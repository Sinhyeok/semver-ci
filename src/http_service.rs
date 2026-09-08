use crate::config;
use crate::errors::{messages, DefaultError, Result, ResultExt};
use log::info;
use reqwest::blocking::Response;
use reqwest::header::HeaderMap;
use serde_json::Value;
use std::collections::HashMap;

fn handle_response(response: Response) -> Result<HashMap<String, Value>> {
    let status = response.status();
    if status.is_success() {
        info!("{:#?}", response);
    }
    let headers = response.headers().clone();
    let body = response.bytes().context(messages::http_body(status))?;
    if !status.is_success() {
        return Err(DefaultError::new(messages::http_status(
            status,
            &headers,
            &String::from_utf8_lossy(&body),
        )));
    }
    let parsed = serde_json::from_slice(&body).context(messages::HTTP_JSON)?;
    info!("    body: {:#?}", parsed);
    Ok(parsed)
}

pub(crate) fn post(
    url: String,
    headers: Option<HeaderMap>,
    body: Option<HashMap<&str, Value>>,
) -> Result<HashMap<String, Value>> {
    let mut request_builder = reqwest::blocking::Client::builder()
        .build()
        .context(messages::HTTP_CLIENT)?
        .post(url);
    if let Some(headers) = headers {
        request_builder = request_builder.headers(headers);
    }
    if let Some(body) = &body {
        request_builder = request_builder.json(body);
    }

    info!("{:#?}", request_builder);
    info!("    body: {:#?}", body);

    if config::is_test()? {
        return Ok(HashMap::new());
    }

    let response = request_builder.send().context(messages::HTTP_SEND)?;

    handle_response(response)
}

pub(crate) fn get(
    url: String,
    headers: Option<HeaderMap>,
    query: Option<HashMap<&str, &str>>,
) -> Result<HashMap<String, Value>> {
    let mut request_builder = reqwest::blocking::Client::builder()
        .build()
        .context(messages::HTTP_CLIENT)?
        .get(url);
    if let Some(headers) = headers {
        request_builder = request_builder.headers(headers);
    }
    if let Some(query) = query {
        request_builder = request_builder.query(&query);
    }

    info!("{:#?}", request_builder);

    if config::is_test()? {
        let mut mock = HashMap::new();
        mock.insert("commits".to_string(), serde_json::json!("[]"));
        mock.insert("web_url".to_string(), serde_json::json!(""));
        return Ok(mock);
    }

    let response = request_builder.send().context(messages::HTTP_SEND)?;

    handle_response(response)
}
