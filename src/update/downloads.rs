use std::convert::Infallible;

use hyper::{Body, Request, Response, StatusCode};
use routerify::ext::RequestExt;
use serde_json::{Map, Value};

use crate::error::downloads as DownloadsError;
use crate::error::Error;
use crate::include_query;
use crate::misc::generic_500;
use crate::response::json_response;

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    json_response(
        StatusCode::OK,
        process_downloads(req).await.map(|_| Value::Object(Map::new()))
    ).or_else(|_| Ok(generic_500()))
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