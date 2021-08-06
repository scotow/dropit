use std::convert::Infallible;

use hyper::{Body, Request, Response, StatusCode};
use routerify::ext::RequestExt;
use serde_json::{Map, Value};

use crate::error::Error;
use crate::error::revoke as RevokeError;
use crate::include_query;
use crate::misc::generic_500;
use crate::response::json_response;
use crate::storage::dir::Dir;

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    json_response(
        StatusCode::OK,
        process_revoke(req).await.map(|_| Value::Object(Map::new()))
    ).or_else(|_| Ok(generic_500()))
}

async fn process_revoke(req: Request<Body>) -> Result<(), Error> {
    let (id, _size, mut conn) = super::authorize(&req).await?;

    tokio::fs::remove_file(
        req.data::<Dir>().ok_or(RevokeError::PathResolve)?.file_path(&id)
    ).await.map_err(|_| RevokeError::RemoveFile)?;

    sqlx::query(include_query!("delete_file"))
        .bind(&id)
        .execute(&mut conn).await
        .map_err(|_| RevokeError::PartialRemove)?;
    Ok(())
}