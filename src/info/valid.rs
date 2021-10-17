use std::convert::Infallible;

use hyper::{Body, Request, Response, StatusCode};
use routerify::ext::RequestExt;
use serde_json::json;
use sqlx::SqlitePool;

use crate::alias::Alias;
use crate::error::Error;
use crate::error::valid as ValidError;
use crate::misc::generic_500;
use crate::response::json_response;

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    json_response(
        StatusCode::OK,
        process_valids(req).await.map(|res| json!({
            "valids": res,
        }))
    ).or_else(|_| Ok(generic_500()))
}

async fn process_valids(req: Request<Body>) -> Result<Vec<bool>, Error> {
    let aliases = req.param("alias")
        .ok_or(ValidError::AliasExtract)?
        .split('+')
        .map(|a| a.parse::<Alias>().map_err(|_| ValidError::InvalidAlias))
        .collect::<Result<Vec<_>, _>>()?;

    let mut conn = req.data::<SqlitePool>().ok_or(ValidError::Database)?
        .acquire().await.map_err(|_| ValidError::Database)?;

    let mut res = Vec::with_capacity(aliases.len());
    for alias in aliases {
        res.push(
            alias.is_used(&mut conn).await.ok_or(ValidError::Database)?
        );
    }

    Ok(res)
}