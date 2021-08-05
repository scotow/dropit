use std::convert::Infallible;

use hyper::{Body, header, Request, Response, StatusCode};
use routerify::ext::RequestExt;
use serde_json::json;

use crate::error::downloads as DownloadsError;
use crate::error::Error;
use crate::include_query;
use crate::misc::generic_500;

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match process_downloads(req).await {
        Ok(_) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "success": true }).to_string()))
        },
        Err(err) => {
            Response::builder()
                .status(err.status_code())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(err.json_string()))
        }
    }.or_else(|_| Ok(generic_500()))
}

async fn process_downloads(req: Request<Body>) -> Result<(), Error> {
    let (id, _size, mut conn) = super::authorize(&req).await?;

    let count = req.param("count")
        .ok_or(DownloadsError::InvalidDownloadsCount)?
        .parse::<u16>()
        .map_err(|_| DownloadsError::InvalidDownloadsCount)?;
    let count = if count >= 1 {
        Some(count)
    } else {
        None
    };

    sqlx::query(include_query!("update_file_downloads"))
        .bind(count)
        .bind(&id)
        .execute(&mut conn).await
        .map_err(|_| DownloadsError::UnexpectedFileModification)?;

    Ok(())
}