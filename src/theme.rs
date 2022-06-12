use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Extension, Router};
use hyper::{header, Body, Request, Response, StatusCode};

use crate::Error;

#[derive(Clone, Debug)]
pub struct Theme(String);

impl Theme {
    pub fn new(color: &str) -> Self {
        Self(format!(":root {{\n\t--theme: {};\n}}", color))
    }
}

async fn handler(Extension(theme): Extension<Theme>) -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/css")],
        theme.0,
    )
}

pub fn router(color: &str) -> Router {
    Router::new()
        .route("/theme.css", get(handler))
        .layer(Extension(Theme::new(color)))
}
