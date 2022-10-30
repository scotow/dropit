use axum::{response::IntoResponse, routing::get, Router};
use hyper::{header, Body, Response, StatusCode, Uri};
use rust_embed::RustEmbed;

use crate::error::{assets as AssetsError, Error};

#[derive(RustEmbed)]
#[folder = "src/public/"]
#[prefix = "/"]
struct Assets;

pub async fn handler(uri: Uri) -> Result<impl IntoResponse, Error> {
    let path = uri.path();
    let path = if path.ends_with('/') {
        format!("{}index.html", path)
    } else {
        path.to_owned()
    };

    let mime_type = match path {
        _ if path.ends_with("html") => "text/html",
        _ if path.ends_with("css") => "text/css",
        _ if path.ends_with("js") => "application/javascript",
        _ if path.ends_with("png") => "image/png",
        _ => "text/plain",
    };

    let asset = Assets::get(&path).ok_or(AssetsError::AssetNotFound)?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_type)
        .body(Body::from(asset.data))?)
}

pub fn router() -> Router {
    Router::new()
        .route("/", get(handler))
        .route("/index.html", get(handler))
        .route("/style.css", get(handler))
        .route("/app.js", get(handler))
        .route("/icon.png", get(handler))
        .route("/login/", get(handler))
        .route("/login/index.html", get(handler))
        .route("/login/style.css", get(handler))
        .route("/login/app.js", get(handler))
}
