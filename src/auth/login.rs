use std::sync::Arc;

use axum::extract::ContentLengthLimit;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use hyper::StatusCode;
use serde::Deserialize;
use serde::Serialize;

use crate::auth::Authenticator;
use crate::error::Error;
use crate::response::{ApiHeader, ApiResponse, ResponseType, SingleLine};

#[derive(Deserialize)]
pub(super) struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub(super) struct LoginResponse {
    pub token: String,
}

impl SingleLine for LoginResponse {
    fn single_lined(&self) -> String {
        self.token.clone()
    }
}

impl ApiHeader for LoginResponse {
    fn status_code(&self) -> StatusCode {
        StatusCode::CREATED
    }
}

pub(super) async fn handler(
    Extension(auth): Extension<Arc<Authenticator>>,
    response_type: ResponseType,
    ContentLengthLimit(Json(req)): ContentLengthLimit<Json<LoginRequest>, 2048>,
) -> Result<impl IntoResponse, Error> {
    Ok(ApiResponse(
        response_type,
        LoginResponse {
            token: auth.create_session(&req.username, &req.password).await?,
        },
    ))
}
