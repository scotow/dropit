use axum::Extension;
use itertools::Itertools;
use serde::Serialize;
use sqlx::SqlitePool;

use crate::{
    alias::{Alias, AliasGroup},
    error::{valid as ValidError, Error},
    response::{ApiHeader, ApiResponse, ResponseType, SingleLine},
};

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
    Ok(ApiResponse(
        ResponseType::Json,
        ValidityCheck {
            valid: process_check_validity(pool, aliases).await?,
        },
    ))
}

async fn process_check_validity(pool: SqlitePool, aliases: Vec<Alias>) -> Result<Vec<bool>, Error> {
    let mut conn = pool.acquire().await.map_err(|_| ValidError::Database)?;

    let mut res = Vec::with_capacity(aliases.len());
    for alias in aliases {
        res.push(alias.is_used(&mut conn).await.ok_or(ValidError::Database)?);
    }

    Ok(res)
}
