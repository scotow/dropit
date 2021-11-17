use std::convert::TryFrom;

use hyper::{Body, Request, Response, StatusCode};
use routerify::ext::RequestExt;

use crate::error::Error;
use crate::error::expiration as ExpirationError;
use crate::include_query;
use crate::response::json_response;
use crate::upload::expiration::Determiner;
use crate::upload::file::Expiration;

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Error> {
    Ok(json_response(StatusCode::OK, process_extend(req).await?)?)
}

async fn process_extend(req: Request<Body>) -> Result<Expiration, Error> {
    let (id, size, mut conn) = super::authorize(&req).await?;

    let determiner = req
        .data::<Determiner>()
        .ok_or(ExpirationError::TimeCalculation)?;
    let expiration = Expiration::try_from(
        determiner
            .determine(size)
            .ok_or(ExpirationError::TooLarge)?,
    )?;

    sqlx::query(include_query!("extend_file"))
        .bind(expiration.timestamp() as i64)
        .bind(id)
        .execute(&mut conn)
        .await
        .map_err(|_| ExpirationError::Database)?;

    Ok(expiration)
}
