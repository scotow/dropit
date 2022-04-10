use hyper::http::HeaderValue;
use hyper::{header, http::Result as HttpResult, Body, Response, StatusCode};
use serde::Serialize;
use serde_json::json;
use serde_json::Value;

use crate::error::Error;

pub trait SingleLine {
    fn single_lined(&self) -> String;
}

pub fn json_response<S: Serialize>(code: StatusCode, content: S) -> HttpResult<Response<Body>> {
    let mut json = serde_json::to_value(content).unwrap();
    json.as_object_mut()
        .unwrap()
        .insert("success".to_owned(), Value::from(true));
    build_response(code, "application/json", json.to_string())
}

pub fn json_error(error: Error) -> HttpResult<Response<Body>> {
    build_response(
        error.status_code(),
        "application/json",
        json!({
            "success": false,
            "error": error.to_string(),
        })
        .to_string(),
    )
}

#[allow(clippy::wildcard_in_or_patterns)]
pub fn adaptive_response<C: SingleLine + Serialize>(
    accept_header: Option<HeaderValue>,
    code: StatusCode,
    content: C,
) -> HttpResult<Response<Body>> {
    let response_type = accept_header.map(|h| h.as_bytes().to_vec());
    match response_type.as_deref() {
        Some(b"text/plain") => build_response(code, "text/plain", content.single_lined()),
        Some(b"application/json") | _ => json_response(code, content),
    }
}

#[allow(clippy::wildcard_in_or_patterns)]
pub fn adaptive_error(
    accept_header: Option<HeaderValue>,
    error: Error,
) -> HttpResult<Response<Body>> {
    let response_type = accept_header.map(|h| h.as_bytes().to_vec());
    match response_type.as_deref() {
        Some(b"text/plain") => {
            build_response(error.status_code(), "text/plain", error.single_lined())
        }
        Some(b"application/json") | _ => json_error(error),
    }
}

fn build_response<T: Into<Body>>(
    code: StatusCode,
    content_type: &str,
    body: T,
) -> HttpResult<Response<Body>> {
    Response::builder()
        .status(code)
        .header(header::CONTENT_TYPE, content_type)
        .body(body.into())
}

pub fn generic_500() -> Response<Body> {
    let mut resp = Response::new(Body::empty());
    *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    resp
}
