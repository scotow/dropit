use std::str::FromStr;

use sqlx::SqliteConnection;

use crate::alias::Alias::{Long, Short};
use crate::include_query;

pub mod short;
pub mod long;

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
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if short::is_match(s) {
            Ok(Short(s.to_owned()))
        } else if long::is_match(s) {
            Ok(Long(s.to_owned()))
        } else {
            Err(())
        }
    }
}

async fn is_alias_used(alias: &str, query: &str, conn: &mut SqliteConnection) -> Option<bool> {
    sqlx::query(query)
        .bind(alias)
        .fetch_optional(conn).await.ok()?
        .is_some().into()
}

async fn random_unused<F>(conn: &mut SqliteConnection, generator: F, exist_query: &str) -> Option<String>
where F: Fn() -> Option<String> {
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
    let mut aliases = (None, None);
    for _ in 0..GENERATION_MAX_TENTATIVES {
        // Short alias.
        if aliases.0.is_none() {
            let alias = short::random()?;
            if !is_alias_used(&alias, include_query!("exist_alias_short"), conn).await? {
                aliases.0 = Some(alias);
            }
        }

        // Long alias.
        if aliases.1.is_none() {
            let alias = long::random()?;
            if !is_alias_used(&alias, include_query!("exist_alias_long"), conn).await? {
                aliases.1 = Some(alias);
            }
        }

        if let (Some(short), Some(long)) = aliases {
            return Some((short, long))
        }
    }
    None
}