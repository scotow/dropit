use serde::Serialize;

use crate::upload::file::UploadInfo;
use crate::upload::error::{Error as UploadError};
use hyper::{Body, Response, header, StatusCode};

pub trait UploadResponse {
    fn response(&self) -> Response<Body>;
}

#[derive(Serialize)]
pub struct JsonResponse<T: Serialize> {
    #[serde(skip)]
    pub code: StatusCode,
    pub success: bool,
    #[serde(flatten)]
    pub data: T,
}

impl<T: Serialize> UploadResponse for JsonResponse<T> {
    fn response(&self) -> Response<Body> {
        Response::builder()
            .status(self.code)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(self).unwrap()))
            .unwrap()
    }
}

impl From<UploadInfo> for JsonResponse<UploadInfo> {
    fn from(info: UploadInfo) -> Self {
        Self {
            code: StatusCode::CREATED,
            success: true,
            data: info,
        }
    }
}

impl From<UploadError> for JsonResponse<UploadError> {
    fn from(err: UploadError) -> Self {
        Self {
            code: err.status_code(),
            success: false,
            data: err,
        }
    }
}

pub struct PlainTextResponse {
    pub code: StatusCode,
    pub inner: String,
}

impl UploadResponse for PlainTextResponse {
    fn response(&self) -> Response<Body> {
        Response::builder()
            .status(self.code)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Body::from(self.inner.clone()))
            .unwrap()
    }
}

impl From<UploadInfo> for PlainTextResponse {
    fn from(info: UploadInfo) -> Self {
        Self {
            code: StatusCode::CREATED,
            inner: info.link.short,
        }
    }
}

impl From<UploadError> for PlainTextResponse {
    fn from(err: UploadError) -> Self {
        Self {
            code: err.status_code(),
            inner: err.to_string(),
        }
    }
}