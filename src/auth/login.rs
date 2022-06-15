use std::sync::Arc;

use axum::response::IntoResponse;
use axum::{Extension, Json};
use hyper::StatusCode;
use serde::Deserialize;
use serde::Serialize;

use crate::auth::Authenticator;
use crate::error::Error;
use crate::response::{ApiHeader, ResponseType, SingleLine};

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
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, Error> {
    // TODO: Add size limiter back.
    // if req.size_hint().upper().unwrap_or(u64::MAX) > 2048 {
    //     return Ok(json_error(AuthError::AuthProcess)?);
    // }

    let token = auth.create_session(&req.username, &req.password).await?;

    Ok(response_type.to_api_response(LoginResponse { token }))
}
