use axum::{Extension, Json};
use hyper::{Body, Request, Response, StatusCode};
use itertools::Itertools;
// use routerify::ext::RequestExt;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

use crate::alias::group::AliasGroup;
use crate::alias::Alias;
use crate::error::valid as ValidError;
use crate::error::Error;
use crate::response::{ApiHeader, ApiResponse, ResponseType, SingleLine};
// use crate::response::json_response;

#[derive(Serialize)]
pub struct ValidityCheck {
    valid: Vec<bool>,
}

impl ApiHeader for ValidityCheck {}

impl SingleLine for ValidityCheck {
    fn single_lined(&self) -> String {
        self.valid.iter().join(" ")
    }
}

pub async fn handler(
    Extension(pool): Extension<SqlitePool>,
    AliasGroup(aliases): AliasGroup,
) -> Result<ApiResponse<ValidityCheck>, Error> {
    Ok(ResponseType::JSON.to_api_response(ValidityCheck {
        valid: process_check_validity(pool, aliases).await?,
    }))
    // Ok(json_response(
    //     StatusCode::OK,
    //     process_check_validity(pool, aliases).await.map(|res| {
    //         json!({
    //             "valids": res,
    //         })
    //     })?,
    // )?)
}

async fn process_check_validity(pool: SqlitePool, aliases: Vec<Alias>) -> Result<Vec<bool>, Error> {
    // let aliases = req
    //     .param("alias")
    //     .ok_or(ValidError::AliasExtract)?
    //     .split('+')
    //     .map(|a| a.parse::<Alias>().map_err(|_| ValidError::InvalidAlias))
    //     .collect::<Result<Vec<_>, _>>()?;

    let mut conn = pool.acquire().await.map_err(|_| ValidError::Database)?;

    let mut res = Vec::with_capacity(aliases.len());
    for alias in aliases {
        res.push(alias.is_used(&mut conn).await.ok_or(ValidError::Database)?);
    }

    Ok(res)
}
