use super::Alias;
use crate::error::Error;
use async_trait::async_trait;
use axum::extract::{FromRequest, Path, RequestParts};
use hyper::Body;
use std::collections::HashMap;

pub struct AliasGroup(pub Vec<Alias>);

// We could use axum::extract::Path with customize-path-rejection, but implementing FromRequest is easier / cleaner.
// Requires the path param to be set to ":alias".
#[async_trait]
impl FromRequest<Body> for AliasGroup {
    type Rejection = Error;

    async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
        Ok(Self(
            Path::<HashMap<String, String>>::from_request(req)
                .await
                .map_err(|_| Error::InvalidAlias)?
                .0
                .get("alias")
                .ok_or_else(|| Error::AliasExtract)?
                .split('+')
                .map(|alias| alias.parse())
                .collect::<Result<_, _>>()?,
        ))
    }
}
