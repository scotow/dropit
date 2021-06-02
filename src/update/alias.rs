use std::convert::Infallible;

use hyper::{Body, header, Request, Response, StatusCode};
use serde_json::json;

use crate::{alias, include_query};
use crate::error::alias as AliasError;
use crate::error::Error;
use crate::misc::generic_500;

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match process_new_aliases(req).await {
        Ok((short, long)) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({
                    "success": true,
                    "short": short,
                    "long": long,
                }).to_string()))
        },
        Err(err) => {
            Response::builder()
                .status(err.status_code())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(err.json_string()))
        }
    }.or_else(|_| Ok(generic_500()))
}

pub async fn process_new_aliases(req: Request<Body>) -> Result<(String, String), Error> {
    let (id, mut conn) = super::authorize(&req).await?;
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

    Ok((short, long))
}