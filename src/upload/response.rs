use hyper::{Body, header, Response, StatusCode};
use serde_json::Value;

use crate::upload::file::UploadInfo;
use crate::upload::UploadResult;

// application/json
pub fn json_response(res: UploadResult<UploadInfo>) -> Response<Body> {
    let (code, success, mut json) = match res {
        Ok(info) => (StatusCode::CREATED, true, serde_json::to_value(info).unwrap()),
        Err(err) => (err.status_code(), false, serde_json::to_value(err).unwrap()),
    };
    json.as_object_mut().unwrap().insert("success".to_owned(), Value::from(success));
    build_response(code, "application/json", serde_json::to_vec(&json).unwrap())
}

// text/plain
pub fn text_response(res: UploadResult<UploadInfo>) -> Response<Body> {
    let (code, text) = match res {
        Ok(info) => (StatusCode::CREATED, info.link.short),
        Err(err) => (err.status_code(), err.to_string()),
    };
    build_response(code, "text/plain", text)
}

fn build_response<T: Into<Body>>(code: StatusCode, content_type: &str, body: T) -> Response<Body> {
    Response::builder()
        .status(code)
        .header(header::CONTENT_TYPE, content_type)
        .body(body.into())
        .unwrap()
}