use actix_web::dev::ServiceResponse;
use chrono::{DateTime, Local, Utc};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::HashMap;

use crate::extractors::Extractor;

#[derive(Serialize, Debug, Default)]
pub(crate) struct TreblleResponseData {
    pub headers: HashMap<String, String>,
    pub code: Option<u16>,
    pub size: Option<u64>,
    pub load_time: Option<f32>,
    pub body: Option<serde_json::Value>,
}

#[derive(Serialize, Debug, Default)]
pub(crate) struct TreblleRequestData {
    pub timestamp: Option<String>,
    pub ip: Option<String>,
    pub url: Option<String>,
    pub user_agent: Option<String>,
    pub method: Option<String>,
    pub headers: HashMap<String, String>,
    pub body: Option<serde_json::Value>,
}

#[derive(Serialize, Debug)]
pub(crate) struct TreblleLanguageData {
    pub name: String,
    pub version: String,
}

impl Default for TreblleLanguageData {
    fn default() -> TreblleLanguageData {
        TreblleLanguageData {
            name: "rust".to_string(),
            version: rustc_version_runtime::version().to_string(),
        }
    }
}

#[derive(Serialize, Debug)]
pub(crate) struct TreblleServerOsData {
    pub name: String,
    pub release: String,
    pub architecture: String,
}

impl Default for TreblleServerOsData {
    fn default() -> TreblleServerOsData {
        TreblleServerOsData {
            name: std::env::consts::FAMILY.to_string(),
            release: std::env::consts::OS.to_string(),
            architecture: std::env::consts::ARCH.to_string(),
        }
    }
}

#[derive(Serialize, Debug)]
pub(crate) struct TreblleServerData {
    pub timezone: String,
    pub os: TreblleServerOsData,
    pub software: Option<String>,
    pub signature: Option<String>,
    pub protocol: Option<String>,
}

impl Default for TreblleServerData {
    fn default() -> TreblleServerData {
        TreblleServerData {
            timezone: Local::now().format("%Z").to_string(),
            os: TreblleServerOsData::default(),
            software: None,
            signature: None,
            protocol: None,
        }
    }
}

#[derive(Serialize, Debug, Default)]
pub(crate) struct TreblleDataInner {
    pub server: TreblleServerData,
    pub language: TreblleLanguageData,
    pub request: TreblleRequestData,
    pub response: TreblleResponseData,
    pub errors: Vec<serde_json::Value>,
}

#[derive(Serialize, Debug)]
pub(crate) struct TreblleData {
    #[serde(skip_serializing)]
    pub start: DateTime<Utc>,
    pub api_key: String,
    pub project_id: String,
    pub version: String,
    pub sdk: String,
    pub data: TreblleDataInner,
}

impl TreblleData {
    /// Start the timer and create a new data instance
    pub fn new(api_key: String, project_id: String) -> TreblleData {
        Self {
            start: Utc::now(),
            api_key,
            project_id,
            sdk: "rust".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            data: TreblleDataInner::default(),
        }
    }

    /// Insert the request body before the execution of it starts
    pub fn add_request_body(&mut self, body: serde_json::Value) {
        self.data.request.body = Some(body);
    }

    /// Mark the end of the request processing
    pub fn stop_timer(&mut self) {
        let end = Utc::now();
        self.data.response.load_time = Some(get_micros_from(self.start, end));
    }

    /// Collect the data from the service response and return it back
    pub fn collect_data(mut self, sr: ServiceResponse) -> (ServiceResponse, TreblleData) {
        let extractor = Extractor::new(sr);

        self.data.server.protocol = Some(extractor.get_protocol());

        self.data.request.timestamp = Some(extractor.get_timestamp());
        self.data.request.ip = Some(extractor.get_ip());
        self.data.request.url = Some(extractor.get_url());
        self.data.request.user_agent = extractor.get_user_agent();
        self.data.request.method = Some(extractor.get_method());
        self.data.request.headers = extractor.get_request_headers();

        self.data.response.headers = extractor.get_response_headers();
        self.data.response.code = Some(extractor.get_code());
        self.data.response.size = Some(extractor.get_size());
        self.data.errors = extractor.get_errors();

        let (sr, body) = extractor.get_response_body();
        self.data.response.body = Some(body);

        (sr, self)
    }

    /// Run through request and response and mask all the fields
    /// String fields will be converted into '*', any other will be simply deleted.
    pub fn mask_fields(&mut self, fields: Vec<String>) {
        let body = self.data.request.body.clone();
        self.data.request.body = body.map(|mut value| {
            clear_value(&mut value, &fields);

            value
        });

        let body = self.data.response.body.clone();
        self.data.response.body = body.map(|mut value| {
            clear_value(&mut value, &fields);

            value
        });

        clear_hashmap(&mut self.data.request.headers, &fields);
        clear_hashmap(&mut self.data.response.headers, &fields);
    }

    /// Send payload to Treblle
    pub async fn send(self, debug: bool) {
        let client = reqwest::Client::new();
        let req = client
            .post("https://rocknrolla.treblle.com")
            .timeout(std::time::Duration::from_secs(2))
            .header("x-api-key", &self.api_key)
            .json(&self)
            .send()
            .await;

        match req {
            Ok(res) => {
                log::debug!("Response: {:#?}", res);

                if debug {
                    let body = res.text().await.unwrap_or_else(|_| "".to_string());
                    log::debug!("Response body: {}", body);
                }
            }
            Err(e) => match debug {
                true => {
                    panic!("{:#?}", e);
                }
                false => log::error!("{:#?}", e),
            },
        };
    }
}

/// Replace given fields in the value with "*" or Null
fn clear_value(value: &mut Value, fields: &[String]) {
    if let Value::Object(map) = value {
        clear_map(map, fields);
    }
}

/// Replace given fields in the value's map with "*" or Null
fn clear_map(map: &mut Map<String, Value>, fields: &[String]) {
    for (key, value) in map.into_iter() {
        match value {
            // Object field will be sent through the same process of clearing (recursion)
            Value::Object(m) => clear_map(m, fields),

            // String value will be checked that the key should be masked, and if it has to be masked,
            // we will replace it with "*"
            Value::String(v) => {
                if fields.contains(key) {
                    *v = "******".to_string()
                }
            }

            // Any other value will be checked if it should be masked and we will mask it
            _ => {
                if fields.contains(key) {
                    *value = Value::Null
                }
            }
        }
    }
}

/// Clear given fields out of a HashMap
fn clear_hashmap(map: &mut HashMap<String, String>, fields: &[String]) {
    for (key, value) in map.iter_mut() {
        if key.to_lowercase() == "authorization" {
            let v = value.split(' ').collect::<Vec<&str>>();
            *value = format!("{} {}", v.get(0).unwrap_or(&""), "******");
        } else if fields.contains(key) {
            *value = "******".to_string();
        }
    }
}

/// Get microseconds from start and end date
fn get_micros_from(start: DateTime<Utc>, end: DateTime<Utc>) -> f32 {
    let duration = end - start;

    duration.num_microseconds().unwrap_or_default() as f32 / 100000_f32
}

#[cfg(test)]
mod test {
    use super::clear_value;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct TestParent {
        pub password: String,
        pub child: TestChild,
        pub message: String,
    }

    #[derive(Serialize, Deserialize)]
    struct TestChild {
        pub description: String,
        pub ccv: Option<i32>,
    }

    #[test]
    fn clear_test_map() {
        let item = TestParent {
            password: "password".to_string(),
            child: TestChild {
                description: "description".to_string(),
                ccv: Some(123),
            },
            message: "message".to_string(),
        };

        let mut value = serde_json::to_value(item).unwrap();

        clear_value(&mut value, &vec!["password".to_string(), "ccv".to_string()]);

        let item = serde_json::from_value::<TestParent>(value).unwrap();

        assert_eq!(&item.password, "******");
        assert!(item.child.ccv.is_none());
    }

    #[test]
    fn get_microseconds_duration() {
        let start = chrono::Utc::now();
        let end = start
            .clone()
            .checked_add_signed(chrono::Duration::seconds(1))
            .unwrap();

        assert_eq!(super::get_micros_from(start, end), 10.0_f32);
    }
}
