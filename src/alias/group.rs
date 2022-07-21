use std::{collections::HashMap, str::FromStr};

use async_trait::async_trait;
use axum::extract::{FromRequest, Path, RequestParts};
use hyper::Body;

use super::Alias;
use crate::error::Error;

pub struct AliasGroup(pub Vec<Alias>);

impl FromStr for AliasGroup {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            s.split('+')
                .map(|alias| alias.parse())
                .collect::<Result<_, _>>()?,
        ))
    }
}

// We could use axum::extract::Path with customize-path-rejection, but implementing FromRequest is easier / cleaner.
// Requires the path param to be set to ":alias".
#[async_trait]
impl FromRequest<Body> for AliasGroup {
    type Rejection = Error;

    async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
        Path::<HashMap<String, String>>::from_request(req)
            .await
            .map_err(|_| Error::InvalidAlias)?
            .0
            .get("alias")
            .ok_or(Error::AliasExtract)?
            .parse()
    }
}
