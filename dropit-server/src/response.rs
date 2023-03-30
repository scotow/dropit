use axum::{
    response::{IntoResponse, Response},
    Json,
};
use http_negotiator::AsNegotiationStr;
use hyper::{HeaderMap, StatusCode};
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

impl AsNegotiationStr for ResponseType {
    fn as_str(&self) -> &str {
        match self {
            ResponseType::Json => "application/json",
            ResponseType::Text => "text/plain",
        }
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
