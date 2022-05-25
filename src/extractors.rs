use actix_web::{
    body::{BodySize, BoxBody, MessageBody},
    dev::ServiceResponse,
    http::header::map::HeaderMap,
};
use chrono::Utc;
use serde_json::{Map, Value};
use std::collections::HashMap;

pub struct Extractor {
    sr: ServiceResponse,
}

impl Extractor {
    pub fn new(sr: ServiceResponse) -> Extractor {
        Extractor { sr }
    }

    /// Get the protocol of the request
    pub fn get_protocol(&self) -> String {
        format!(
            "{}/x",
            self.sr
                .request()
                .connection_info()
                .scheme()
                .to_string()
                .to_uppercase()
        )
    }

    /// Get the status code of the response
    pub fn get_code(&self) -> u16 {
        self.sr.response().status().as_u16()
    }

    /// Get the size of the body in the response
    pub fn get_size(&self) -> u64 {
        match self.sr.response().body().size() {
            BodySize::Sized(v) => v,
            _ => 0,
        }
    }

    /// Extract and covert response headers into easily serializable HashMap
    pub fn get_response_headers(&self) -> HashMap<String, String> {
        headermap_into_hashmap(self.sr.response().headers().clone())
    }

    /// In case of an error prepare it for sending to Treblle as an Value vector
    pub fn get_errors(&self) -> Vec<Value> {
        let mut errors: Vec<Value> = vec![];
        if let Some(e) = self.sr.response().error() {
            let mut map = Map::new();
            let message = format!("{:?}", e);
            let r#type = message
                .split('(')
                .collect::<Vec<&str>>()
                .get(0)
                .unwrap_or(&"")
                .split(" {{")
                .collect::<Vec<&str>>()
                .get(0)
                .unwrap_or(&"")
                .to_string();

            map.insert("source".to_string(), Value::String("onError".to_string()));
            map.insert("message".to_string(), Value::String(message));
            map.insert("type".to_string(), Value::String(r#type));

            errors.push(Value::Object(map));
        }

        errors
    }

    /// Get the properly formated timestamp for the payload
    pub fn get_timestamp(&self) -> String {
        format!("{}", Utc::now().format("%F %T"))
    }

    /// Get the realip address of the user making the request
    pub fn get_ip(&self) -> String {
        self.sr
            .request()
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("127.0.0.1")
            .to_string()
    }

    /// Get the call url from the request
    pub fn get_url(&self) -> String {
        format!(
            "{}://{}{}",
            self.sr.request().connection_info().scheme(),
            self.sr.request().connection_info().host(),
            self.sr.request().uri()
        )
    }

    /// Convert headers into easily serializable HashMap
    pub fn get_request_headers(&self) -> HashMap<String, String> {
        headermap_into_hashmap(self.sr.request().headers().clone())
    }

    /// Extract the user agent from the headers
    pub fn get_user_agent(&self) -> Option<String> {
        self.sr
            .request()
            .headers()
            .get("user-agent")
            .map(|v| v.to_str().unwrap_or(""))
            .map(|v| v.to_string())
    }

    /// Extract the request method
    pub fn get_method(&self) -> String {
        self.sr.request().method().to_string()
    }

    /// Clone the response body and extract it into Value if its possible,
    /// if not, we'll treat it as Null.
    pub fn get_response_body(self) -> (ServiceResponse, Value) {
        let mut bytes = None;
        let sr = self
            .sr
            .map_body(|_, old_body| match old_body.try_into_bytes() {
                Ok(b) => {
                    bytes = Some(b.clone());

                    BoxBody::new(b)
                }
                Err(same_old_body) => same_old_body,
            });

        (
            sr,
            match bytes {
                Some(b) => {
                    if b.is_empty() {
                        Value::Null
                    } else {
                        match serde_json::from_slice::<Value>(&b) {
                            Ok(v) => v,
                            Err(_) => match String::from_utf8(b.to_vec()) {
                                Ok(s) => Value::String(s),
                                Err(_) => Value::String(format!("{:?}", b)),
                            },
                        }
                    }
                }
                None => Value::Null,
            },
        )
    }
}

/// Convert HeaderMap into HashMap of Strings
fn headermap_into_hashmap(headers: HeaderMap) -> HashMap<String, String> {
    let mut map = HashMap::<String, String>::new();
    for (k, v) in headers.iter() {
        map.insert(k.to_string(), v.to_str().unwrap_or("").to_string());
    }

    map
}
