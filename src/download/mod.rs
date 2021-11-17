use hyper::{Body, Request, Response};
use routerify::ext::RequestExt;
use sqlx::{FromRow, SqlitePool};

use crate::{Error, include_query};
use crate::auth::{Access, Authenticator};
use crate::error::auth as AuthError;
use crate::error::download as DownloadError;
use crate::storage::dir::Dir;

mod file;
mod archive;

#[derive(FromRow)]
struct FileInfo {
    id: String,
    name: String,
    size: i64,
}

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Error> {
    let auth = req.data::<Authenticator>()
        .ok_or(AuthError::AuthProcess)?;
    if let Some(resp) = auth.allows(&req, Access::DOWNLOAD) {
        return Ok(resp);
    }

    let alias = req.param("alias")
        .ok_or(DownloadError::AliasExtract)?;
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
        .ok_or("Cannot find file for downloads count decrement")?;
    match downloads {
        None => (),
        Some(0) => return Err(format!("Found a zero downloads counter file: {}", id)),
        Some(1) => {
            tokio::fs::remove_file(dir.file_path(id)).await
                .map_err(|err| format!("Failed to delete decremented to zero file from fs {}: {:?}", id, err))?;
            sqlx::query(include_query!("delete_file"))
                .bind(id)
                .execute(&mut conn).await
                .map_err(|err| format!("Failed to delete decremented to zero file from database {}: {:?}", id, err))?;
        },
        Some(count) => {
            sqlx::query(include_query!("update_file_downloads"))
                .bind(count - 1)
                .bind(id)
                .execute(&mut conn).await
                .map_err(|err| format!("Failed to decremented file from database {}: {:?}", id, err))?;
        }
    };
    Ok(())
}
