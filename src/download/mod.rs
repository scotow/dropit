use hyper::{Body, Request, Response};
use routerify::ext::RequestExt;
use sqlx::{FromRow, SqlitePool};

use crate::alias::Alias;
use crate::auth::{Authenticator, Features};
use crate::error::auth as AuthError;
use crate::error::download as DownloadError;
use crate::storage::dir::Dir;
use crate::{include_query, Error};

mod archive;
mod file;
mod open_graph;

#[derive(FromRow)]
struct FileInfo {
    id: String,
    name: String,
    size: i64,
}

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Error> {
    let auth = req.data::<Authenticator>().ok_or(AuthError::AuthProcess)?;
    if let Err(resp) = auth.allows(&req, Features::DOWNLOAD).await {
        return Ok(resp);
    }

    let alias = req.param("alias").ok_or(DownloadError::AliasExtract)?;
    let aliases = alias
        .split('+')
        .map(|a| a.parse::<Alias>().map_err(|_| DownloadError::InvalidAlias))
        .collect::<Result<Vec<_>, _>>()?;

    let pool = req
        .data::<SqlitePool>()
        .ok_or(DownloadError::Database)?
        .clone();
    let mut conn = pool.acquire().await.map_err(|_| DownloadError::Database)?;

    let mut files_info = Vec::with_capacity(aliases.len());
    for alias in aliases {
        files_info.push(
            sqlx::query_as::<_, FileInfo>(include_query!("get_file"))
                .bind(alias.inner())
                .bind(alias.inner())
                .fetch_optional(&mut conn)
                .await
                .map_err(|_| DownloadError::Database)?
                .ok_or(DownloadError::FileNotFound)?,
        );
    }

    if let Some(og_resp) = open_graph::proxy_request(&req, &files_info) {
        return Ok(og_resp);
    }

    match files_info.len() {
        0 => Err(DownloadError::AliasExtract),
        1 => file::handler(req, &files_info[0], pool).await,
        _ => archive::handler(req, files_info, pool).await,
    }
}

async fn file_downloaded(pool: &SqlitePool, dir: &Dir, id: &str) -> Result<(), String> {
    let mut conn = pool
        .acquire()
        .await
        .map_err(|err| format!("Cannot acquire database connect: {:?}", err))?;
    let (downloads,) = sqlx::query_as::<_, (Option<u16>,)>(include_query!("get_file_downloads"))
        .bind(id)
        .fetch_optional(&mut conn)
        .await
        .map_err(|err| format!("Cannot fetch downloads count: {:?}", err))?
        .ok_or("Cannot find file for downloads count decrement")?;
    match downloads {
        None => (),
        Some(0) => return Err(format!("Found a zero downloads counter file: {}", id)),
        Some(1) => {
            tokio::fs::remove_file(dir.file_path(id))
                .await
                .map_err(|err| {
                    format!(
                        "Failed to delete decremented to zero file from fs {}: {:?}",
                        id, err
                    )
                })?;
            sqlx::query(include_query!("delete_file"))
                .bind(id)
                .execute(&mut conn)
                .await
                .map_err(|err| {
                    format!(
                        "Failed to delete decremented to zero file from database {}: {:?}",
                        id, err
                    )
                })?;
        }
        Some(count) => {
            sqlx::query(include_query!("update_file_downloads"))
                .bind(count - 1)
                .bind(id)
                .execute(&mut conn)
                .await
                .map_err(|err| {
                    format!("Failed to decremented file from database {}: {:?}", id, err)
                })?;
        }
    };
    Ok(())
}
