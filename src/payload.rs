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
    pub load_time: Option<String>,
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

        self.data.response.load_time = Some(get_seconds_with_micro(self.start, None));

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

    /// Send where we don't wait for the execution of the request to finish
    pub fn send(self) {
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let _ = client
                .post("https://rocknrolla.treblle.com")
                .timeout(std::time::Duration::from_secs(2))
                .header("x-api-key", &self.api_key)
                .json(&self)
                .send()
                .await;
        });
    }

    /// Send payload to Treblle
    pub async fn send_debug(self) {
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

                let body = res.text().await.unwrap_or_else(|_| "".to_string());
                log::debug!("Response body: {}", body);
            }
            Err(e) => panic!("{:#?}", e),
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

/// Get microseconds from start and end date, the reason for the shenanings in this
/// function is to get the proper microseconds without a f64 error where 0.1 + 0.2 != 0.3
fn get_seconds_with_micro(start: DateTime<Utc>, end: Option<DateTime<Utc>>) -> String {
    let end = end.unwrap_or_else(Utc::now);
    let start_seconds = start.timestamp() as f64;
    let start_micros = start.timestamp_subsec_micros() as f64 / 1000000_f64;
    let start_with_micros = start_seconds as f64 + start_micros as f64;

    let end_seconds = end.timestamp() as f64;
    let end_micros = end.timestamp_subsec_micros() as f64 / 1000000_f64;
    let end_with_micros = end_seconds as f64 + end_micros as f64;

    let duration = end_with_micros - start_with_micros;

    format!("{:.5}", duration)
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
            .checked_add_signed(chrono::Duration::microseconds(2000))
            .unwrap();

        assert_eq!(super::get_seconds_with_micro(start, Some(end)), "0.00200");

        let start = chrono::Utc::now();
        let end = start
            .clone()
            .checked_add_signed(chrono::Duration::milliseconds(200))
            .unwrap();

        assert_eq!(super::get_seconds_with_micro(start, Some(end)), "0.20000");

        let start = chrono::Utc::now();
        let end = start
            .clone()
            .checked_add_signed(chrono::Duration::seconds(500))
            .unwrap();

        assert_eq!(super::get_seconds_with_micro(start, Some(end)), "500.00000");
    }
}
