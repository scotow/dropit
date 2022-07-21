use std::{convert::TryFrom, sync::Arc};

use axum::Extension;
use sqlx::SqlitePool;

use crate::{
    alias::Alias,
    error::{expiration as ExpirationError, Error},
    include_query,
    response::{ApiResponse, ResponseType},
    update::AdminToken,
    upload::{Determiner, Expiration},
};

pub async fn handler(
    Extension(pool): Extension<SqlitePool>,
    response_type: ResponseType,
    Extension(determiner): Extension<Arc<Determiner>>,
    AdminToken(admin_token): AdminToken,
    alias: Alias,
) -> Result<ApiResponse<Expiration>, ApiResponse<Error>> {
    Ok(ApiResponse(
        response_type,
        process_extend(pool, determiner, alias, admin_token)
            .await
            .map_err(|err| ApiResponse(response_type, err))?,
    ))
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
