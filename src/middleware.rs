use actix_http::{h1::Payload, HttpMessage};
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    error::Error,
    http::header,
    web::BytesMut,
};
use futures::{
    future::{ok, Future, Ready},
    task::{Context, Poll},
    StreamExt,
};
use serde_json::{Map, Value};
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;

use super::payload::TreblleData;
use super::treblle::Treblle;

impl<S: 'static> Transform<S, ServiceRequest> for Treblle
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type Transform = TreblleMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(TreblleMiddleware {
            project_id: self.project_id.clone(),
            api_key: self.api_key.clone(),
            debug: self.debug,
            masking_fields: self.masking_fields.clone(),
            ignored_routes: self.ignored_routes.clone(),
            service: Rc::new(RefCell::new(service)),
        })
    }
}

pub struct TreblleMiddleware<S> {
    pub(crate) project_id: String,
    pub(crate) api_key: String,
    pub(crate) debug: bool,
    pub(crate) masking_fields: Vec<String>,
    pub(crate) ignored_routes: Vec<String>,
    service: Rc<RefCell<S>>,
}

impl<S> Service<ServiceRequest> for TreblleMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let skip_treblle = self
            .ignored_routes
            .contains(&req.match_pattern().unwrap_or_else(|| "".to_string()));

        // If we are skipping treblle, we will only do the call for the
        // further request and skip anything else.
        if skip_treblle {
            let fut = self.service.call(req);

            return Box::pin(async move { fut.await });
        }

        let svc = self.service.clone();
        let api_key = self.api_key.clone();
        let project_id = self.project_id.clone();
        let debug = self.debug;
        let masking_fields = self.masking_fields.clone();

        Box::pin(async move {
            let mut treblle = TreblleData::new(api_key, project_id);
            treblle.add_request_body(get_request_body(&mut req).await?);

            let service_response: ServiceResponse = svc.call(req).await?;

            let (service_response, mut data) = treblle.collect_data(service_response);

            // Run field masking on the data
            data.mask_fields(masking_fields);

            if debug {
                log::debug!("Treblle payload data:\n{:#?}", &data);
                data.send_debug().await;
            } else {
                data.send();
            }

            Ok(service_response)
        })
    }
}

/// Clone and extract any type of body received from the request into a Value type
/// that is universal JSON holder. If the deserialization of the request data fails, we'll treat
/// it as a Null.
async fn get_request_body(sr: &mut ServiceRequest) -> Result<Value, Error> {
    let content_type = sr
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.clone().to_str().unwrap_or("").to_string())
        .unwrap_or_else(|| "".to_string())
        .to_lowercase();

    // TODO: Content type that is not application json won't be logged since it can cause
    // harm in some setups, this might be a feature to implement sometimes in the future,
    // once we get a proper chance to test it and figure out all the bugs that keep happening,
    // but for now we will simply set it as Null value in the log.
    //
    // Issue that we got was that some multipart forms weren't recognized properly after
    // the things we did here below to them, the issue couldn't be reproduced in a local
    // setting, but it was happening within the cluster.
    //
    // Payload would apear okay in treblle.com, but later methods that were supposed
    // to handle that payload reported invalid multipart data, or form data.
    if content_type != "application/json" {
        return Ok(Value::Null);
    }

    let mut request_body = BytesMut::new();
    while let Some(chunk) = sr.take_payload().next().await {
        request_body.extend_from_slice(&chunk?);
    }
    let bytes = request_body.freeze();

    let (_sender, mut orig_payload) = Payload::create(true);
    orig_payload.unread_data(bytes.clone());
    sr.set_payload(actix_http::Payload::from(orig_payload));

    if bytes.is_empty() {
        return Ok(Value::Null);
    }

    Ok(match serde_json::from_slice::<Value>(&bytes) {
        Ok(v) => v,
        Err(_) => match String::from_utf8(bytes.to_vec()) {
            Ok(s) => {
                let mut map = Map::new();
                map.insert("request_as_a_string".to_string(), Value::String(s));

                Value::Object(map)
            }
            Err(_) => {
                let mut map = Map::new();
                map.insert(
                    "request_as_raw_bytes".to_string(),
                    Value::String(format!("{:?}", bytes)),
                );

                Value::Object(map)
            }
        },
    })
}
