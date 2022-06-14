use crate::alias;
use crate::alias::Alias;
use crate::error::alias as AliasError;
use crate::response::{ApiHeader, ApiResponse, ResponseType, SingleLine};
use crate::update::AdminToken;
use crate::upload::origin::DomainUri;
use crate::{include_query, Error};
use axum::Extension;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use serde_json::json;
use sqlx::SqlitePool;

pub struct ShortAliasChange {
    alias: String,
    link: String,
}

impl ApiHeader for ShortAliasChange {}

impl Serialize for ShortAliasChange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ShortAliasChange", 2)?;
        state.serialize_field(
            "alias",
            &json!({
                "short": self.alias,
            }),
        )?;
        state.serialize_field(
            "link",
            &json!({
                "short": self.link,
            }),
        )?;
        state.end()
    }
}

impl SingleLine for ShortAliasChange {
    fn single_lined(&self) -> String {
        self.link.clone()
    }
}

pub async fn handler(
    Extension(pool): Extension<SqlitePool>,
    alias: Alias,
    AdminToken(admin_token): AdminToken,
    DomainUri(domain_uri): DomainUri,
    response_type: ResponseType,
) -> Result<ApiResponse<ShortAliasChange>, Error> {
    let new_alias = process_change(pool, alias, admin_token).await?;
    Ok(response_type.to_api_response(ShortAliasChange {
        alias: new_alias.clone(),
        link: format!("{}/{}", domain_uri, new_alias),
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
