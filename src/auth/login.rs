use crate::response::{ApiResponse, ResponseType, SingleLine, Status};
use crate::{Authenticator, Error};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use hyper::StatusCode;
use std::sync::Arc;

// use std::sync::Arc;
//
// use hyper::body::HttpBody;
// use hyper::{header, Body, Request, Response, StatusCode};
use serde::Deserialize;
use serde::Serialize;
//
// // use crate::response::{json_error, json_response};
// // use crate::{Authenticator, Error as AuthError, Error};
//
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

impl Status for LoginResponse {}

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

    Ok((
        StatusCode::CREATED,
        response_type.to_api_response(LoginResponse { token }),
    ))
}
