use hyper::{header, Body, Request, Response, StatusCode};
use rust_embed::RustEmbed;

use crate::AssetsError;
use crate::Error;

#[derive(RustEmbed)]
#[folder = "src/public/"]
#[prefix = "/"]
struct Assets;

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Error> {
    let path = req.uri().path();
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
