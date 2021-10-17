use std::convert::Infallible;

use hyper::{Body, Request, Response};
use routerify::ext::RequestExt;
use sqlx::{FromRow, SqlitePool};

use crate::auth::{Access, Authenticator};
use crate::error::auth as AuthError;
use crate::error::download as DownloadError;
use crate::include_query;
use crate::misc::generic_500;
use crate::response::error_text_response;
use crate::storage::dir::Dir;

mod file;
mod archive;

#[derive(FromRow)]
struct FileInfo {
    id: String,
    name: String,
    size: i64,
}

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let auth = match req.data::<Authenticator>() {
        Some(auth) => auth,
        None => return Ok(error_text_response(AuthError::AuthProcess).unwrap_or_else(|_| generic_500())),
    };
    if let Some(resp) = auth.allows(&req, Access::DOWNLOAD) {
        return Ok(resp);
    }

    let alias = match req.param("alias") {
        Some(alias) => alias.clone(),
        None => return error_text_response(DownloadError::AliasExtract).or_else(|_| Ok(generic_500()))
    };
    if alias.contains('+') {
        archive::handler(req).await
    } else {
        file::handler(req).await
    }
}

async fn file_downloaded(pool: &SqlitePool, dir: &Dir, id: &str) -> Result<(), String> {
    let mut conn = pool.acquire().await
        .map_err(|err| format!("Cannot acquire database connect: {:?}", err))?;
    let (downloads,) = sqlx::query_as::<_, (Option<u16>,)>(include_query!("get_file_downloads"))
        .bind(id)
        .fetch_optional(&mut conn).await
        .map_err(|err| format!("Cannot fetch downloads count: {:?}", err))?
        .ok_or_else(|| "Cannot find file for downloads count decrement")?;
    match downloads {
        None => (),
        Some(0) => Err(format!("Found a zero downloads counter file: {}", id))?,
        Some(1) => {
            tokio::fs::remove_file(dir.file_path(id)).await
                .map_err(|err| format!("Failed to delete decremented to zero file from fs {}: {:?}", id, err))?;
            sqlx::query(include_query!("delete_file"))
                .bind(id)
                .execute(&mut conn).await
                .map_err(|err| format!("Failed to delete decremented to zero file from database {}: {:?}", id, err))?;
        },
        Some(count @ _) => {
            sqlx::query(include_query!("update_file_downloads"))
                .bind(count - 1)
                .bind(id)
                .execute(&mut conn).await
                .map_err(|err| format!("Failed to decremented file from database {}: {:?}", id, err))?;
        }
    };
    Ok(())
}