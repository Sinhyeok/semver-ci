use crate::config;
use crate::default_error::DefaultError;
use reqwest::blocking::Response;
use reqwest::header::HeaderMap;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;

fn handle_response(response: Response) -> Result<HashMap<String, Value>, Box<dyn Error>> {
    let status = response.status();
    if status.is_success() {
        if !config::is_production() {
            println!("{:#?}", response);
        }

        let parsed = response
            .json::<HashMap<String, Value>>()
            .unwrap_or_else(|e| panic!("{}", e));
        if !config::is_production() {
            println!("    body: {:#?}", parsed);
        }

        Ok(parsed)
    } else {
        let headers = response.headers().clone();
        let body = response.text().unwrap();
        Err(Box::new(DefaultError {
            message: format!(
                "Status: {}\nHeaders:\n{:#?}\nBody:\n{}",
                status, headers, body
            ),
            source: None,
        }))
    }
}

pub(crate) fn post(
    url: String,
    headers: Option<HeaderMap>,
    body: Option<HashMap<&str, Value>>,
) -> Result<HashMap<String, Value>, Box<dyn Error>> {
    let mut request_builder = reqwest::blocking::Client::new().post(url);
    if headers.is_some() {
        request_builder = request_builder.headers(headers.unwrap());
    }
    if body.is_some() {
        request_builder = request_builder.json(&body.clone().unwrap());
    }

    if !config::is_production() {
        println!("{:#?}", request_builder);
        println!("    body: {:#?}", body);
    }

    if config::is_test() {
        return Ok(HashMap::new());
    }

    let response = request_builder.send()?;

    handle_response(response)
}

pub(crate) fn get(
    url: String,
    headers: Option<HeaderMap>,
    query: Option<HashMap<&str, &str>>,
) -> Result<HashMap<String, Value>, Box<dyn Error>> {
    let mut request_builder = reqwest::blocking::Client::new().get(url);
    if headers.is_some() {
        request_builder = request_builder.headers(headers.unwrap());
    }
    if query.is_some() {
        request_builder = request_builder.query(&query.unwrap());
    }

    if !config::is_production() {
        println!("{:#?}", request_builder);
    }

    if config::is_test() {
        let mut mock = HashMap::new();
        mock.insert("commits".to_string(), serde_json::json!("[]"));
        mock.insert("web_url".to_string(), serde_json::json!(""));
        return Ok(mock);
    }

    let response = request_builder.send()?;

    handle_response(response)
}
