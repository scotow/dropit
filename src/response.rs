use hyper::{Body, header, http::Result as HttpResult, Response, StatusCode};
use serde::Serialize;
use serde_json::Value;

use crate::error::Error;

// application/json
pub fn json_response<T: Serialize>(code: StatusCode, res: Result<T, Error>) -> HttpResult<Response<Body>> {
    let (code, mut json) = match &res {
        Ok(data) => (code, serde_json::to_value(data).unwrap()),
        Err(err) => (err.status_code(), serde_json::to_value(err).unwrap()),
    };
    json.as_object_mut().unwrap().insert("success".to_owned(), Value::from(res.is_ok()));
    build_response(code, "application/json", json.to_string())
}

// text/plain
pub fn text_response(code: StatusCode, res: Result<String, Error>) -> HttpResult<Response<Body>> {
    let (code, text) = match res {
        Ok(data) => (code, data),
        Err(err) => (err.status_code(), err.to_string()),
    };
    build_response(code, "text/plain", text)
}

fn build_response<T: Into<Body>>(code: StatusCode, content_type: &str, body: T) -> HttpResult<Response<Body>> {
    Response::builder()
        .status(code)
        .header(header::CONTENT_TYPE, content_type)
        .body(body.into())
}