use axum::Extension;
use sqlx::SqlitePool;

use crate::{
    alias,
    alias::Alias,
    error::{alias as AliasError, Error},
    include_query,
    response::{ApiResponse, ResponseType},
    update::{alias::AliasChange, AdminToken},
    upload::DomainUri,
};

pub async fn handler(
    Extension(pool): Extension<SqlitePool>,
    alias: Alias,
    AdminToken(admin_token): AdminToken,
    DomainUri(domain_uri): DomainUri,
    response_type: ResponseType,
) -> Result<ApiResponse<AliasChange>, Error> {
    let new_alias = process_change(pool, alias, admin_token).await?;
    Ok(ApiResponse(
        response_type,
        AliasChange {
            short: Some((new_alias.clone(), format!("{}/{}", domain_uri, new_alias))),
            long: None,
        },
    ))
}

async fn process_change(
    pool: SqlitePool,
    alias: Alias,
    admin_token: String,
) -> Result<String, Error> {
    let (id, _size, mut conn) = super::super::authorize(pool, &alias, &admin_token).await?;
    let alias = alias::random_unused_short(&mut conn)
        .await
        .ok_or(AliasError::AliasGeneration)?;

    let affected = sqlx::query(include_query!("update_file_short_alias"))
        .bind(&alias)
        .bind(&id)
        .execute(&mut conn)
        .await
        .map_err(|_| AliasError::Database)?
        .rows_affected();

    if affected != 1 {
        return Err(AliasError::UnexpectedFileModification);
    }

    Ok(alias)
}
