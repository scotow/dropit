use axum::Extension;
use std::convert::TryFrom;
use std::sync::Arc;

use crate::alias::Alias;
use hyper::{Body, Request, Response, StatusCode};
use sqlx::SqlitePool;
// use routerify::ext::RequestExt;

use crate::error::expiration as ExpirationError;
use crate::error::Error;
use crate::include_query;
use crate::response::{ApiResponse, ResponseType};
use crate::update::AdminToken;
// use crate::response::json_response;
use crate::upload::expiration::Determiner;
use crate::upload::file::Expiration;

pub async fn handler(
    Extension(pool): Extension<SqlitePool>,
    response_type: ResponseType,
    Extension(determiner): Extension<Arc<Determiner>>,
    AdminToken(admin_token): AdminToken,
    alias: Alias,
) -> Result<ApiResponse<Expiration>, ApiResponse<Error>> {
    Ok(response_type.to_api_response(
        process_extend(pool, determiner, alias, admin_token)
            .await
            .map_err(|err| response_type.to_api_response(err))?,
    ))
    // Ok(json_response(StatusCode::OK, process_extend(req).await?)?)
}

async fn process_extend(
    pool: SqlitePool,
    determiner: Arc<Determiner>,
    alias: Alias,
    admin_token: String,
) -> Result<Expiration, Error> {
    let (id, size, mut conn) = super::authorize(pool, &alias, &admin_token).await?;

    let expiration = Expiration::try_from(
        determiner
            .determine(size)
            .ok_or(ExpirationError::TooLarge)?,
    )?;

    sqlx::query(include_query!("extend_file"))
        .bind(expiration.timestamp() as i64)
        .bind(id)
        .execute(&mut conn)
        .await
        .map_err(|_| ExpirationError::Database)?;

    Ok(expiration)
}
