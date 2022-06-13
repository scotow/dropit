use crate::auth::{Authenticator, Features};
use axum::headers::Cookie;
use axum::response::IntoResponse;
use axum::{Extension, Json, TypedHeader};
use hyper::StatusCode;
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
struct RequiresAuth {
    pub required: bool,
}

pub(super) async fn handler(
    Extension(auth): Extension<Arc<Authenticator>>,
    cookies: Option<TypedHeader<Cookie>>,
) -> impl IntoResponse {
    if !auth.protects(Features::UPLOAD) {
        return (StatusCode::OK, Json(RequiresAuth { required: false }));
    }

    let cookies = match cookies {
        Some(cookies) => cookies.0,
        None => {
            return (StatusCode::OK, Json(RequiresAuth { required: true }));
        }
    };

    (
        StatusCode::OK,
        Json(RequiresAuth {
            required: auth.verify_cookie(cookies).await.is_none(),
        }),
    )
}
