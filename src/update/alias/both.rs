use crate::alias;
use crate::alias::Alias;
use crate::error::alias as AliasError;
use crate::response::{ApiHeader, ApiResponse, ResponseType, SingleLine};
use crate::update::alias::AliasChange;
use crate::update::AdminToken;
use crate::upload::origin::DomainUri;
use crate::{include_query, Error};
use axum::Extension;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use serde_json::json;
use sqlx::SqlitePool;

pub async fn handler(
    Extension(pool): Extension<SqlitePool>,
    alias: Alias,
    AdminToken(admin_token): AdminToken,
    DomainUri(domain_uri): DomainUri,
    response_type: ResponseType,
) -> Result<ApiResponse<AliasChange>, Error> {
    let (new_short, new_long) = process_change(pool, alias, admin_token).await?;
    Ok(response_type.to_api_response(AliasChange {
        short: Some((new_short.clone(), format!("{}/{}", domain_uri, new_short))),
        long: Some((new_long.clone(), format!("{}/{}", domain_uri, new_long))),
    }))
    // Ok(json_response(
    //     StatusCode::OK,
    //     process_short(req).await.map(|(base, alias)| {
    //         json!({
    //             "alias": { "short": &alias },
    //             "link": { "short": format!("{}/{}", base, &alias) }
    //         })
    //     })?,
    // )?)
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
