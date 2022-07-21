use axum::{response::IntoResponse, routing::get, Extension, Router};
use hyper::{header, StatusCode};

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
