use std::convert::Infallible;

use async_trait::async_trait;
use axum::{
    extract::{FromRequest, RequestParts},
    response::{IntoResponse, Response},
    Json,
};
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
    Json,
    Text,
}

impl Default for ResponseType {
    fn default() -> Self {
        Self::Json
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
                Some("application/json") => Self::Json,
                Some("text/plain") => Self::Text,
                _ => Self::default(),
            },
        )
    }
}

pub struct ApiResponse<T>(pub ResponseType, pub T);

impl<T> IntoResponse for ApiResponse<T>
where
    T: ApiHeader + Serialize + SingleLine,
{
    fn into_response(self) -> Response {
        (
            self.1.status_code(),
            self.1.additional_headers(),
            match self.0 {
                ResponseType::Json => {
                    #[derive(Serialize)]
                    struct JsonResponse<T> {
                        success: bool,
                        #[serde(flatten)]
                        data: T,
                    }
                    Json(JsonResponse {
                        success: self.1.success(),
                        data: self.1,
                    })
                    .into_response()
                }
                ResponseType::Text => self.1.single_lined().into_response(),
            },
        )
            .into_response()
    }
}
