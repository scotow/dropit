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
            Short(a) => &a,
            Long(a) => &a,
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

#[allow(unused)]
pub fn random_aliases() -> Option<(String, String)> {
    short::random().and_then(|s| long::random().map(|l| (s, l)))
}

pub async fn random_unused_aliases(conn: &mut SqliteConnection) -> Option<(String, String)> {
    let mut aliases = (None, None);
    for _ in 0..GENERATION_MAX_TENTATIVES {
        // Short alias.

        if aliases.0.is_none() {
            let alias = short::random()?;
            if alias_is_unused(&alias, include_query!("exist_alias_short"), conn).await? {
                aliases.0 = Some(alias);
            }
        }

        // Long alias.
        if aliases.1.is_none() {
            let alias = long::random()?;
            if alias_is_unused(&alias, include_query!("exist_alias_long"), conn).await? {
                aliases.1 = Some(alias);
            }
        }

        if let (Some(short), Some(long)) = aliases {
            return Some((short, long))
        }
    }
    None
}

pub async fn alias_is_unused(alias: &str, query: &str, conn: &mut SqliteConnection) -> Option<bool> {
    sqlx::query(query)
        .bind(alias)
        .fetch_optional(conn).await.ok()?
        .is_none().into()
}