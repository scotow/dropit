use axum::extract::{FromRequest, RequestParts};
use axum::{Extension, Router};
use hyper::{header, Body, Request};
use std::sync::Arc;
// use routerify::ext::RequestExt;
use async_trait::async_trait;
use axum::routing::{delete, patch};
use sqlx::pool::PoolConnection;
use sqlx::{Sqlite, SqlitePool};

use crate::alias::Alias;
use crate::error::admin as AdminError;
use crate::error::Error;
use crate::{include_query, Determiner, Dir};

pub mod alias;
pub mod downloads;
pub mod expiration;
pub mod revoke;

async fn authorize(
    pool: SqlitePool,
    alias: &Alias,
    admin_token: &str,
) -> Result<(String, u64, PoolConnection<Sqlite>), Error> {
    // let alias = req
    //     .param("alias")
    //     .ok_or(AdminError::AliasExtract)?
    //     .parse::<Alias>()
    //     .map_err(|_| AdminError::InvalidAlias)?;

    // let headers = req.headers();
    // let auth = headers
    //     .get("X-Authorization") // Prioritize X-Authorization because Safari doesn't overwrite XMLHttpRequest's Authorization header.
    //     .or_else(|| headers.get(header::AUTHORIZATION))
    //     .ok_or(AdminError::InvalidAuthorizationHeader)?
    //     .to_str()
    //     .map_err(|_| AdminError::InvalidAuthorizationHeader)?;

    let mut conn = pool.acquire().await.map_err(|_| AdminError::Database)?;
    // let mut conn = req
    //     .data::<SqlitePool>()
    //     .ok_or(AdminError::Database)?
    //     .acquire()
    //     .await
    //     .map_err(|_| AdminError::Database)?;
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
        .route("/:alias/expiration", patch(expiration::handler))
        .route("/:alias", delete(revoke::handler))
        .route_layer(Extension(pool))
        .route_layer(Extension(dir))
        .route_layer(Extension(determiner))
}
