use std::sync::Arc;

use axum::headers::Cookie;
use axum::response::IntoResponse;
use axum::{Extension, Json, TypedHeader};
use hyper::StatusCode;
use serde::Serialize;

use crate::auth::{AuthStatus, Authenticator, Features};

#[derive(Serialize)]
struct RequiresAuth {
    pub required: bool,
}

pub(super) async fn handler(
    Extension(auth): Extension<Arc<Authenticator>>,
    cookie: Option<TypedHeader<Cookie>>,
) -> impl IntoResponse {
    let required = match auth
        .allows(None, cookie.map(|c| c.0), Features::UPLOAD)
        .await
    {
        AuthStatus::NotNeeded | AuthStatus::Valid(_) => false,
        AuthStatus::Prompt | AuthStatus::Error(_) => true,
    };

    (StatusCode::OK, Json(RequiresAuth { required }))
}
