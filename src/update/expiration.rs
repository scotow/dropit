use std::convert::{Infallible, TryFrom};

use hyper::{Body, header, Request, Response, StatusCode};
use routerify::ext::RequestExt;
use serde_json::Value;

use crate::error::Error;
use crate::error::expiration as ExpirationError;
use crate::include_query;
use crate::misc::generic_500;
use crate::upload::expiration::Determiner;
use crate::upload::file::Expiration;

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match process_extend(req).await {
        Ok(expiration) => {
            let mut json = serde_json::to_value(expiration).unwrap();
            json.as_object_mut().unwrap().insert("success".to_owned(), Value::from(true));
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json.to_string()))
        },
        Err(err) => {
            Response::builder()
                .status(err.status_code())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(err.json_string()))
        }
    }.or_else(|_| Ok(generic_500()))
}

async fn process_extend(req: Request<Body>) -> Result<Expiration, Error> {
    let (id, size, mut conn) = super::authorize(&req).await?;

    let determiner = req.data::<Determiner>().ok_or(ExpirationError::TimeCalculation)?;
    let expiration = Expiration::try_from(
        determiner.determine(size).ok_or(ExpirationError::TooLarge)?
    )?;

    sqlx::query(include_query!("extend_file"))
        .bind(expiration.timestamp() as i64)
        .bind(id)
        .execute(&mut conn).await
        .map_err(|_| ExpirationError::Database)?;

    Ok(expiration)
}