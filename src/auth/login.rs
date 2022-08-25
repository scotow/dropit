use std::sync::Arc;

use axum::{extract::ContentLengthLimit, response::IntoResponse, Extension, Json};
use http_negotiator::{ContentTypeNegotiation, Negotiation};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    auth::Authenticator,
    error::Error,
    response::{ApiHeader, ApiResponse, ResponseType, SingleLine},
};

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
    response_type: Negotiation<ContentTypeNegotiation, ResponseType>,
    ContentLengthLimit(Json(req)): ContentLengthLimit<Json<LoginRequest>, 2048>,
) -> Result<impl IntoResponse, Error> {
    Ok(ApiResponse(
        response_type.into_inner(),
        LoginResponse {
            token: auth.create_session(&req.username, &req.password).await?,
        },
    ))
}
