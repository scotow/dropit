use std::convert::Infallible;

use hyper::{
    Body,
    header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE},
    Request,
    Response,
    StatusCode
};
use routerify::ext::RequestExt;
use sqlx::{
    FromRow,
    SqlitePool
};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::alias::Alias;
use crate::download::error::Error as DownloadError;
use crate::include_query;
use crate::storage::dir::Dir;

#[derive(FromRow)]
struct FileInfo {
    id: String,
    name: String,
    size: i64,
}

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok (
        match process_download(req).await {
            Ok((info, fd)) => {
                Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_LENGTH, info.size as u64)
                    .header(CONTENT_DISPOSITION, format!(r#"attachment; filename="{}""#, info.name))
                    .body(Body::wrap_stream(ReaderStream::new(fd)))
            },
            Err(err) => {
                Response::builder()
                    .status(err.status_code())
                    .header(CONTENT_TYPE, "text/plain")
                    .body(err.to_string().into())
            }
        }.unwrap()
    )
}

async fn process_download(req: Request<Body>) -> Result<(FileInfo, File), DownloadError> {
    let alias = req.param("alias")
        .ok_or(DownloadError::AliasExtract)?
        .parse::<Alias>()
        .map_err(|_| DownloadError::InvalidAlias)?;

    let query = match &alias {
        Alias::Short(_) => include_query!("get_file_short"),
        Alias::Long(_) => include_query!("get_file_long"),
    };

    let mut conn = req.data::<SqlitePool>().ok_or(DownloadError::Database)?
        .acquire().await.map_err(|_| DownloadError::Database)?;
    let info = sqlx::query_as::<_, FileInfo>(query)
        .bind(alias.inner())
        .fetch_optional(&mut conn).await.map_err(|_| DownloadError::Database)?
        .ok_or(DownloadError::FileNotFound)?;

    let fd = File::open(
        req.data::<Dir>().ok_or(DownloadError::OpenFile)?.file_path(&info.id)
    ).await.map_err(|_| DownloadError::OpenFile)?;
    Ok((info, fd))
}