use std::convert::Infallible;

use hyper::{Body, Request, Response};
use hyper::header::CONTENT_TYPE;
use routerify::ext::RequestExt;
use sqlx::FromRow;

use crate::error::download as DownloadError;
use crate::misc::generic_500;

mod file;
mod archive;

#[derive(FromRow)]
struct FileInfo {
    id: String,
    name: String,
    size: i64,
}

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let alias = match req.param("alias") {
        Some(alias) => alias.clone(),
        None => {
            return Response::builder()
                .status(DownloadError::AliasExtract.status_code())
                .header(CONTENT_TYPE, "text/plain")
                .body(DownloadError::AliasExtract.to_string().into())
                .or_else(|_| Ok(generic_500()));
        }
    };
    if alias.contains('+') {
        archive::handler(req).await
    } else {
        file::handler(req).await
    }
}