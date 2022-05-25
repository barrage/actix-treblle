use actix_http::{h1::Payload, HttpMessage};
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    error::Error,
    web::BytesMut,
};
use futures::{
    future::{ok, Future, Ready},
    task::{Context, Poll},
    StreamExt,
};
use serde_json::Value;
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
        let svc = self.service.clone();
        let api_key = self.api_key.clone();
        let project_id = self.project_id.clone();
        let debug = self.debug;
        let masking_fields = self.masking_fields.clone();

        let skip_treblle = self
            .ignored_routes
            .contains(&req.match_pattern().unwrap_or_else(|| "".to_string()));

        Box::pin(async move {
            // If we are skipping treblle, we will only do the call for the
            // further request and skip anything else.
            if skip_treblle {
                return svc.call(req).await;
            }

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
    let mut request_body = BytesMut::new();
    while let Some(chunk) = sr.take_payload().next().await {
        request_body.extend_from_slice(&chunk?);
    }
    let bytes = request_body.freeze();

    let (_sender, mut orig_payload) = Payload::create(true);
    orig_payload.unread_data(bytes.clone());
    sr.set_payload(actix_http::Payload::from(orig_payload));

    Ok(serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null))
}
