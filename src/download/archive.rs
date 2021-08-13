use std::collections::HashMap;
use std::convert::Infallible;

use async_tar::{Builder, Header, HeaderMode};
use hyper::{
    Body,
    header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    Request,
    Response,
    StatusCode
};
use routerify::ext::RequestExt;
use sqlx::SqlitePool;
use tokio::fs::File;
use tokio::io::{duplex, DuplexStream};
use tokio_util::compat::TokioAsyncReadCompatExt;
use tokio_util::io::ReaderStream;

use crate::alias::Alias;
use crate::download::FileInfo;
use crate::error::download as DownloadError;
use crate::error::Error;
use crate::include_query;
use crate::misc::generic_500;
use crate::storage::dir::Dir;

pub(super) async fn handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match process_download(req).await {
        Ok(stream) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, "application/x-tar")
                .header(CONTENT_DISPOSITION, r#"attachment; filename="archive.tar""#)
                .body(Body::wrap_stream(ReaderStream::new(stream)))
        },
        Err(err) => {
            Response::builder()
                .status(err.status_code())
                .header(CONTENT_TYPE, "text/plain")
                .body(err.to_string().into())
        }
    }.or_else(|_| Ok(generic_500()))
}

async fn process_download(req: Request<Body>) -> Result<DuplexStream, Error> {
    let alias = req.param("alias")
        .ok_or(DownloadError::AliasExtract)?;

    let aliases = alias.split('+')
        .map(|a| a.parse::<Alias>().map_err(|_| DownloadError::InvalidAlias))
        .collect::<Result<Vec<_>, _>>()?;

    let dir = req.data::<Dir>().ok_or(DownloadError::PathResolve)?.clone();

    let pool = req.data::<SqlitePool>().ok_or(DownloadError::Database)?.clone();
    let mut conn = pool.acquire().await.map_err(|_| DownloadError::Database)?;

    let mut info = Vec::with_capacity(aliases.len());
    for alias in aliases {
        info.push(
            sqlx::query_as::<_, FileInfo>(include_query!("get_file"))
                .bind(alias.inner()).bind(alias.inner())
                .fetch_optional(&mut conn).await.map_err(|_| DownloadError::Database)?
                .ok_or(DownloadError::FileNotFound)?
        );
    }

    let (w, r) = duplex(64000);
    tokio::spawn(async move {
        let mut name_occurrences = HashMap::new();
        let mut ar = Builder::new(w.compat());
        ar.mode(HeaderMode::Deterministic);

        for info in info.iter() {
            let occurrence = name_occurrences.entry(&info.name).or_insert(0u16);
            *occurrence += 1;
            let name = if *occurrence == 1 {
                info.name.clone()
            } else {
                if let Some((name, extension)) = info.name.split_once('.') {
                    format!("{}-{}.{}", name, occurrence, extension)
                } else {
                    format!("{}-{}", info.name, occurrence)
                }
            };

            let mut header = Header::new_gnu();
            match header.set_path(name) {
                Ok(_) => (),
                Err(err) => {
                    log::error!("Failed to write file path to archive: {}", err);
                    break;
                }
            }
            header.set_mode(0o644);
            header.set_size(info.size as u64);
            header.set_cksum();

            let fd = match File::open(dir.file_path(&info.id)).await {
                Ok(fd) => fd,
                Err(err) => {
                    log::error!("Failed to open file for archive streaming: {}", err);
                    break;
                }
            };
            match ar.append(&mut header, fd.compat()).await {
                Ok(_) => (),
                Err(err) => {
                    log::error!("Failed to append file to archive: {}", err);
                    break;
                }
            }
            match super::file_downloaded(&pool, &dir, &info.id).await {
                Ok(_) => (),
                Err(err) => {
                    log::error!("Failed to process file downloads counter update: {}", err);
                    break;
                },
            }
        }

        match ar.finish().await {
            Ok(_) => (),
            Err(err) => log::error!("Failed to write archive's completion data: {}", err),
        }
    });

    Ok(r)
}