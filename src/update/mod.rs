use hyper::{Body, header, Request};
use routerify::ext::RequestExt;
use sqlx::{Sqlite, SqlitePool};
use sqlx::pool::PoolConnection;

use crate::alias::Alias;
use crate::error::admin as AdminError;
use crate::error::Error;
use crate::include_query;

pub mod revoke;
pub mod alias;

async fn authorize(req: &Request<Body>) -> Result<(String, PoolConnection<Sqlite>), Error> {
    let alias = req.param("alias")
        .ok_or(AdminError::AliasExtract)?
        .parse::<Alias>()
        .map_err(|_| AdminError::InvalidAlias)?;

    let auth = req.headers()
        .get(header::AUTHORIZATION).ok_or(AdminError::InvalidAuthorizationHeader)?
        .to_str().map_err(|_| AdminError::InvalidAuthorizationHeader)?;

    let mut conn = req.data::<SqlitePool>().ok_or(AdminError::Database)?
        .acquire().await.map_err(|_| AdminError::Database)?;
    let (id, admin) = sqlx::query_as::<_, (String, String)>(include_query!("get_file_admin"))
        .bind(alias.inner())
        .bind(alias.inner())
        .fetch_optional(&mut conn).await.map_err(|_| AdminError::Database)?
        .ok_or(AdminError::FileNotFound)?;

    if admin != auth.to_ascii_lowercase() {
        return Err(AdminError::InvalidAdminToken);
    }
    Ok((id, conn))
}