use std::{convert::TryFrom, sync::Arc, time::Duration};

use axum::{extract::Path, Extension};
use http_negotiator::{ContentTypeNegotiation, Negotiation};
use serde::{de::Unexpected, Deserialize, Deserializer};
use sqlx::SqlitePool;

use crate::{
    alias::Alias,
    error::{expiration as ExpirationError, Error},
    include_query,
    response::{ApiResponse, ResponseType},
    update::AdminToken,
    upload::{Determiner, Expiration},
};

#[derive(Copy, Clone, Debug)]
pub enum DurationRequest {
    Initial,
    Maximum,
    Custom(u64),
}

impl<'de> Deserialize<'de> for DurationRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let input = <&str>::deserialize(deserializer)?;
        match input {
            "init" | "initial" => Ok(Self::Initial),
            "max" | "maximum" => Ok(Self::Maximum),
            _ => input.parse().map(|n| Self::Custom(n)).map_err(|err| {
                serde::de::Error::invalid_value(Unexpected::Str(&input), &err.to_string().as_str())
            }),
        }
    }
}

pub async fn handler(
    Extension(pool): Extension<SqlitePool>,
    response_type: Negotiation<ContentTypeNegotiation, ResponseType>,
    Extension(determiner): Extension<Arc<Determiner>>,
    AdminToken(admin_token): AdminToken,
    alias: Alias,
    Path((_, duration)): Path<(String, DurationRequest)>,
) -> Result<ApiResponse<Expiration>, ApiResponse<Error>> {
    Ok(ApiResponse(
        *response_type,
        process_extend(pool, determiner, alias, duration, admin_token)
            .await
            .map_err(|err| ApiResponse(*response_type, err))?,
    ))
}

async fn process_extend(
    pool: SqlitePool,
    determiner: Arc<Determiner>,
    alias: Alias,
    duration: DurationRequest,
    admin_token: String,
) -> Result<Expiration, Error> {
    let (id, size, mut conn) = super::authorize(pool, &alias, &admin_token).await?;

    let (default, allowed) = determiner
        .determine(size)
        .ok_or(ExpirationError::TooLarge)?;
    let expiration = Expiration::try_from(match duration {
        DurationRequest::Initial => default,
        DurationRequest::Maximum => {
            if let Some(duration) = allowed {
                duration
            } else {
                return Err(ExpirationError::ExpirationTooHigh);
            }
        }
        DurationRequest::Custom(secs) => {
            let dur = Duration::from_secs(secs);
            if dur > allowed.unwrap_or(default) {
                return Err(ExpirationError::ExpirationTooHigh);
            }
            dur
        }
    })?;

    sqlx::query(include_query!("extend_file"))
        .bind(expiration.timestamp() as i64)
        .bind(id)
        .execute(&mut conn)
        .await
        .map_err(|_| ExpirationError::Database)?;

    Ok(expiration)
}
