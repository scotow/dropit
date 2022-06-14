use async_trait::async_trait;
use axum::extract::{FromRequest, RequestParts};
use axum::response::{IntoResponse, Response};
use axum::Json;
use hyper::http::HeaderValue;
use hyper::{header, http::Result as HttpResult, Body, HeaderMap, StatusCode};
use serde::Serialize;
use serde_json::json;
use serde_json::Value;
use std::convert::Infallible;

use crate::error::Error;

pub trait ApiHeader {
    fn status_code(&self) -> StatusCode {
        StatusCode::OK
    }

    fn additional_headers(&self) -> HeaderMap {
        HeaderMap::default()
    }

    fn success(&self) -> bool {
        true
    }
}

impl ApiHeader for () {}

pub trait SingleLine {
    fn single_lined(&self) -> String;
}

impl SingleLine for () {
    fn single_lined(&self) -> String {
        String::new()
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ResponseType {
    JSON,
    Text,
}

impl ResponseType {
    pub fn to_api_response<T>(self, data: T) -> ApiResponse<T> {
        ApiResponse { data, format: self }
    }
}

impl Default for ResponseType {
    fn default() -> Self {
        Self::JSON
    }
}

#[async_trait]
impl FromRequest<Body> for ResponseType {
    type Rejection = Infallible;

    async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
        Ok(
            match req
                .headers()
                .get(header::ACCEPT)
                .and_then(|h| h.to_str().ok())
            {
                Some("application/json") => Self::JSON,
                Some("text/plain") => Self::Text,
                _ => Default::default(),
            },
        )
    }
}

pub struct ApiResponse<T> {
    data: T,
    format: ResponseType,
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: ApiHeader + Serialize + SingleLine,
{
    fn into_response(self) -> Response {
        match self.format {
            ResponseType::JSON => {
                #[derive(Serialize)]
                struct JsonResponse<T> {
                    success: bool,
                    #[serde(flatten)]
                    data: T,
                }
                (
                    self.data.status_code(),
                    self.data.additional_headers(),
                    Json(JsonResponse {
                        success: self.data.success(),
                        data: self.data,
                    }),
                )
                    .into_response()
            }
            ResponseType::Text => self.data.single_lined().into_response(),
        }
    }
}

// pub fn json_response<S: Serialize>(code: StatusCode, content: S) -> HttpResult<Response<Body>> {
//     let mut json = serde_json::to_value(content).unwrap();
//     json.as_object_mut()
//         .unwrap()
//         .insert("success".to_owned(), Value::from(true));
//     build_response(code, "application/json", json.to_string())
// }
//
// pub fn json_error(error: Error) -> HttpResult<Response<Body>> {
//     build_response(
//         error.status_code(),
//         "application/json",
//         json!({
//             "success": false,
//             "error": error.to_string(),
//         })
//         .to_string(),
//     )
// }
//
// #[allow(clippy::wildcard_in_or_patterns)]
// pub fn adaptive_response<C: SingleLine + Serialize>(
//     accept_header: Option<HeaderValue>,
//     code: StatusCode,
//     content: C,
// ) -> HttpResult<Response<Body>> {
//     let response_type = accept_header.map(|h| h.as_bytes().to_vec());
//     match response_type.as_deref() {
//         Some(b"text/plain") => build_response(code, "text/plain", content.single_lined()),
//         Some(b"application/json") | _ => json_response(code, content),
//     }
// }
//
// #[allow(clippy::wildcard_in_or_patterns)]
// pub fn adaptive_error(
//     accept_header: Option<HeaderValue>,
//     error: Error,
// ) -> HttpResult<Response<Body>> {
//     let response_type = accept_header.map(|h| h.as_bytes().to_vec());
//     match response_type.as_deref() {
//         Some(b"text/plain") => {
//             build_response(error.status_code(), "text/plain", error.single_lined())
//         }
//         Some(b"application/json") | _ => json_error(error),
//     }
// }
//
// fn build_response<T: Into<Body>>(
//     code: StatusCode,
//     content_type: &str,
//     body: T,
// ) -> HttpResult<Response<Body>> {
//     Response::builder()
//         .status(code)
//         .header(header::CONTENT_TYPE, content_type)
//         .body(body.into())
// }
//
// pub fn generic_500() -> Response<Body> {
//     let mut resp = Response::new(Body::empty());
//     *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
//     resp
// }
