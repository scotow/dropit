use std::collections::HashMap;
use std::str::FromStr;

use async_trait::async_trait;
use axum::extract::{FromRequest, Path, RequestParts};
use hyper::Body;
use serde::de::Error as SerdeError;
use serde::{Deserialize, Deserializer};
use sqlx::SqliteConnection;

use crate::alias::Alias::{Long, Short};
use crate::error::Error;
use crate::include_query;

pub use group::AliasGroup;

mod group;
mod long;
mod short;

const GENERATION_MAX_TENTATIVES: u8 = 20;

#[derive(Clone, Debug, PartialEq)]
pub enum Alias {
    Short(String),
    Long(String),
}

impl Alias {
    pub fn inner(&self) -> &str {
        match self {
            Short(a) => a,
            Long(a) => a,
        }
    }

    pub async fn is_used(&self, conn: &mut SqliteConnection) -> Option<bool> {
        match self {
            Short(s) => is_alias_used(s, include_query!("exist_alias_short"), conn).await,
            Long(s) => is_alias_used(s, include_query!("exist_alias_long"), conn).await,
        }
    }
}

impl FromStr for Alias {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if short::is_match(s) {
            Ok(Short(s.to_owned()))
        } else if long::is_match(s) {
            Ok(Long(s.to_owned()))
        } else {
            Err(Error::InvalidAlias)
        }
    }
}

#[async_trait]
impl FromRequest<Body> for Alias {
    type Rejection = Error;

    async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
        Ok(Path::<HashMap<String, String>>::from_request(req)
            .await
            .map_err(|_| Error::InvalidAlias)?
            .0
            .get("alias")
            .ok_or_else(|| Error::AliasExtract)?
            .parse()?)
    }
}

impl<'de> Deserialize<'de> for Alias {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = Deserialize::deserialize(deserializer)?;
        Self::from_str(s).map_err(|_| SerdeError::unknown_variant(s, &["short", "long"]))
    }
}

async fn is_alias_used(alias: &str, query: &str, conn: &mut SqliteConnection) -> Option<bool> {
    sqlx::query(query)
        .bind(alias)
        .fetch_optional(conn)
        .await
        .ok()?
        .is_some()
        .into()
}

async fn random_unused<F>(
    conn: &mut SqliteConnection,
    generator: F,
    exist_query: &str,
) -> Option<String>
where
    F: Fn() -> Option<String>,
{
    for _ in 0..GENERATION_MAX_TENTATIVES {
        let alias = generator()?;
        if !is_alias_used(&alias, exist_query, conn).await? {
            return Some(alias);
        }
    }
    None
}

pub async fn random_unused_short(conn: &mut SqliteConnection) -> Option<String> {
    random_unused(conn, short::random, include_query!("exist_alias_short")).await
}

pub async fn random_unused_long(conn: &mut SqliteConnection) -> Option<String> {
    random_unused(conn, long::random, include_query!("exist_alias_long")).await
}

pub async fn random_unused_aliases(conn: &mut SqliteConnection) -> Option<(String, String)> {
    Some((
        random_unused_short(conn).await?,
        random_unused_long(conn).await?,
    ))
}
