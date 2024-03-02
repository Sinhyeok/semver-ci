use reqwest::header::HeaderMap;
use serde_json::Value;
use std::collections::HashMap;

pub(crate) fn post(
    url: String,
    headers: Option<HeaderMap>,
    body: Option<HashMap<&str, String>>,
) -> HashMap<String, Value> {
    let mut request_builder = reqwest::blocking::Client::new().post(url);
    if headers.is_some() {
        request_builder = request_builder.headers(headers.unwrap());
    }
    if body.is_some() {
        request_builder = request_builder.json(&body.unwrap());
    }

    let response = request_builder.send().unwrap_or_else(|e| panic!("{}", e));

    let status = response.status();
    if status.is_success() {
        response
            .json::<HashMap<String, Value>>()
            .unwrap_or_else(|e| panic!("{}", e))
    } else {
        let headers = response.headers().clone();
        let body = response.text().unwrap();
        panic!(
            "Status: {}\nHeaders:\n{:#?}\nBody:\n{}",
            status, headers, body
        )
    }
}
