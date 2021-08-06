use std::convert::Infallible;

use hyper::{Body, Request, Response, StatusCode};
use serde_json::json;

use crate::{alias, include_query};
use crate::error::alias as AliasError;
use crate::error::Error;
use crate::misc::{generic_500, request_target};
use crate::response::json_response;

pub async fn handler_short(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    json_response(
        StatusCode::OK,
        process_short(req).await
            .map(|(base, alias)| json!({
                "alias": { "short": &alias },
                "link": { "short": format!("{}/{}", base, &alias) }
            }))
    ).or_else(|_| Ok(generic_500()))
}

pub async fn handler_long(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    json_response(
        StatusCode::OK,
        process_long(req).await
            .map(|(base, alias)| json!({
                "alias": { "long": &alias },
                "link": { "long": format!("{}/{}", base, &alias) }
            }))
    ).or_else(|_| Ok(generic_500()))
}

pub async fn handler_both(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    json_response(
        StatusCode::OK,
        process_both(req).await
            .map(|(base, short, long)| json!({
                "alias": {
                    "short": &short,
                    "long": &long,
                },
                "link": {
                    "short": format!("{}/{}", base, &short),
                    "long": format!("{}/{}", base, &long),
                }
            }))
    ).or_else(|_| Ok(generic_500()))
}

async fn process_short(req: Request<Body>) -> Result<(String, String), Error> {
    let (id, _size, mut conn) = super::authorize(&req).await?;
    let alias = alias::random_unused_short(&mut conn).await
        .ok_or(AliasError::AliasGeneration)?;

    let affected = sqlx::query(include_query!("update_file_short_alias"))
        .bind(&alias)
        .bind(&id)
        .execute(&mut conn).await
        .map_err(|_| AliasError::Database)?
        .rows_affected();

    if affected != 1 {
        return Err(AliasError::UnexpectedFileModification);
    }

    let base = request_target(req.headers()).ok_or(AliasError::Target)?;
    Ok((base, alias))
}

async fn process_long(req: Request<Body>) -> Result<(String, String), Error> {
    let (id, _size, mut conn) = super::authorize(&req).await?;
    let alias = alias::random_unused_long(&mut conn).await
        .ok_or(AliasError::AliasGeneration)?;

    let affected = sqlx::query(include_query!("update_file_long_alias"))
        .bind(&alias)
        .bind(&id)
        .execute(&mut conn).await
        .map_err(|_| AliasError::Database)?
        .rows_affected();

    if affected != 1 {
        return Err(AliasError::UnexpectedFileModification);
    }

    let base = request_target(req.headers()).ok_or(AliasError::Target)?;
    Ok((base, alias))
}

async fn process_both(req: Request<Body>) -> Result<(String, String, String), Error> {
    let (id, _size, mut conn) = super::authorize(&req).await?;
    let (short, long) = alias::random_unused_aliases(&mut conn).await
        .ok_or(AliasError::AliasGeneration)?;

    let affected = sqlx::query(include_query!("update_file_aliases"))
        .bind(&short)
        .bind(&long)
        .bind(&id)
        .execute(&mut conn).await
        .map_err(|_| AliasError::Database)?
        .rows_affected();

    if affected != 1 {
        return Err(AliasError::UnexpectedFileModification);
    }

    let base = request_target(req.headers()).ok_or(AliasError::Target)?;
    Ok((base, short, long))
}