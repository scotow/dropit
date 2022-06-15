use std::convert::Infallible;

use async_trait::async_trait;
use axum::extract::{FromRequest, RequestParts};
use axum::response::{IntoResponse, Response};
use axum::Json;
use hyper::{header, Body, HeaderMap, StatusCode};
use serde::Serialize;

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
