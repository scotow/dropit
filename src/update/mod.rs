use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{FromRequest, RequestParts},
    routing::{delete, patch},
    Extension, Router,
};
use hyper::{header, Body};
use sqlx::{pool::PoolConnection, Sqlite, SqlitePool};

use crate::{
    alias::Alias,
    error::{admin as AdminError, Error},
    include_query,
    storage::Dir,
    upload::Determiner,
};

mod alias;
mod downloads;
mod expiration;
mod revoke;

async fn authorize(
    pool: SqlitePool,
    alias: &Alias,
    admin_token: &str,
) -> Result<(String, u64, PoolConnection<Sqlite>), Error> {
    let mut conn = pool.acquire().await.map_err(|_| AdminError::Database)?;

    let (id, size, admin) =
        sqlx::query_as::<_, (String, i64, String)>(include_query!("get_file_admin"))
            .bind(alias.inner())
            .bind(alias.inner())
            .fetch_optional(&mut conn)
            .await
            .map_err(|_| AdminError::Database)?
            .ok_or(AdminError::FileNotFound)?;

    if admin != admin_token.to_ascii_lowercase() {
        return Err(AdminError::InvalidAdminToken);
    }
    Ok((id, size as u64, conn))
}

pub struct AdminToken(String);

#[async_trait]
impl FromRequest<Body> for AdminToken {
    type Rejection = Error;

    async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
        Ok(Self(
            req.headers()
                .get("X-Authorization") // Prioritize X-Authorization because Safari doesn't overwrite XMLHttpRequest's Authorization header.
                .or_else(|| req.headers().get(header::AUTHORIZATION))
                .ok_or(AdminError::InvalidAuthorizationHeader)?
                .to_str()
                .map_err(|_| AdminError::InvalidAuthorizationHeader)?
                .to_owned(),
        ))
    }
}

pub fn router(pool: SqlitePool, dir: Dir, determiner: Arc<Determiner>) -> Router {
    Router::new()
        .route("/:alias/alias/short", patch(alias::short::handler))
        .route("/:alias/alias/long", patch(alias::long::handler))
        .route("/:alias/alias", patch(alias::both::handler))
        .route("/:alias/downloads/:count", patch(downloads::handler))
        .route("/:alias/expiration/:duration", patch(expiration::handler))
        .route("/:alias", delete(revoke::handler))
        .route_layer(Extension(pool))
        .route_layer(Extension(dir))
        .route_layer(Extension(determiner))
}
