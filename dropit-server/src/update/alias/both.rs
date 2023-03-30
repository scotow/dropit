use axum::Extension;
use http_negotiator::{ContentTypeNegotiation, Negotiation};
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
    response_type: Negotiation<ContentTypeNegotiation, ResponseType>,
) -> Result<ApiResponse<AliasChange>, Error> {
    let (new_short, new_long) = process_change(pool, alias, admin_token).await?;
    Ok(ApiResponse(
        response_type.into_inner(),
        AliasChange {
            short: Some((new_short.clone(), format!("{}/{}", domain_uri, new_short))),
            long: Some((new_long.clone(), format!("{}/{}", domain_uri, new_long))),
        },
    ))
}

async fn process_change(
    pool: SqlitePool,
    alias: Alias,
    admin_token: String,
) -> Result<(String, String), Error> {
    let (id, _size, mut conn) = super::super::authorize(pool, &alias, &admin_token).await?;
    let (short, long) = alias::random_unused_aliases(&mut conn)
        .await
        .ok_or(AliasError::AliasGeneration)?;

    let affected = sqlx::query(include_query!("update_file_aliases"))
        .bind(&short)
        .bind(&long)
        .bind(&id)
        .execute(&mut conn)
        .await
        .map_err(|_| AliasError::Database)?
        .rows_affected();

    if affected != 1 {
        return Err(AliasError::UnexpectedFileModification);
    }

    Ok((short, long))
}
