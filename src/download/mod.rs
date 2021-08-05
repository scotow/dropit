use std::convert::Infallible;

use hyper::{Body, Request, Response};
use hyper::header::CONTENT_TYPE;
use routerify::ext::RequestExt;
use sqlx::{FromRow, SqlitePool};

use crate::error::download as DownloadError;
use crate::include_query;
use crate::misc::generic_500;
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
    let alias = match req.param("alias") {
        Some(alias) => alias.clone(),
        None => {
            return Response::builder()
                .status(DownloadError::AliasExtract.status_code())
                .header(CONTENT_TYPE, "text/plain")
                .body(DownloadError::AliasExtract.to_string().into())
                .or_else(|_| Ok(generic_500()));
        }
    };
    if alias.contains('+') {
        archive::handler(req).await
    } else {
        file::handler(req).await
    }
}

async fn file_downloaded(pool: &SqlitePool, dir: &Dir, id: &str) {
    let mut conn = pool.acquire().await.unwrap();
    let (downloads,) = sqlx::query_as::<_, (Option<u16>,)>(include_query!("get_file_downloads"))
        .bind(id)
        .fetch_optional(&mut conn).await.unwrap()
        .unwrap();
    match downloads {
        None => (),
        Some(0) => unreachable!("should not be possible"),
        Some(1) => {
            tokio::fs::remove_file(dir.file_path(id)).await.unwrap();
            sqlx::query(include_query!("delete_file"))
                .bind(id)
                .execute(&mut conn).await.unwrap();
        },
        Some(count @ _) => {
            sqlx::query(include_query!("update_file_downloads"))
                .bind(count - 1)
                .bind(id)
                .execute(&mut conn).await.unwrap();
        }
    }
}