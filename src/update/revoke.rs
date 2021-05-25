use std::convert::Infallible;

use hyper::{Body, header, Request, Response, StatusCode};
use routerify::ext::RequestExt;
use serde_json::json;
use sqlx::SqlitePool;

use crate::alias::Alias;
use crate::error::Error;
use crate::error::revoke as RevokeError;
use crate::include_query;
use crate::misc::generic_500;
use crate::storage::dir::Dir;

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match process_revoke(req).await {
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

pub async fn process_revoke(req: Request<Body>) -> Result<(), Error> {
    let alias = req.param("alias")
        .ok_or(RevokeError::AliasExtract)?
        .parse::<Alias>()
        .map_err(|_| RevokeError::InvalidAlias)?;

    let auth = req.headers()
        .get(header::AUTHORIZATION).ok_or(RevokeError::InvalidAuthorizationHeader)?
        .to_str().map_err(|_| RevokeError::InvalidAuthorizationHeader)?;

    let mut conn = req.data::<SqlitePool>().ok_or(RevokeError::Database)?
        .acquire().await.map_err(|_| RevokeError::Database)?;
    let (id, admin) = sqlx::query_as::<_, (String, String)>(include_query!("get_file_admin"))
        .bind(alias.inner())
        .bind(alias.inner())
        .fetch_optional(&mut conn).await.map_err(|_| RevokeError::Database)?
        .ok_or(RevokeError::FileNotFound)?;

    if admin != auth.to_ascii_lowercase() {
        return Err(RevokeError::InvalidAdminToken);
    }

    tokio::fs::remove_file(
        req.data::<Dir>().ok_or(RevokeError::PathResolve)?.file_path(&id)
    ).await.map_err(|_| RevokeError::RemoveFile)?;

    sqlx::query(include_query!("delete_file"))
        .bind(&id)
        .execute(&mut conn).await
        .map_err(|_| RevokeError::PartialRemove)?;
    Ok(())
}