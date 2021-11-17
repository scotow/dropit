use std::collections::HashMap;

use hyper::{
    Body,
    header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE}, Request, Response, StatusCode,
};
use routerify::ext::RequestExt;
use sqlx::SqlitePool;
use tokio::fs::File;
use tokio::io::{duplex, DuplexStream};
use tokio_util::io::ReaderStream;
use zipit::{Archive, archive_size, FileDateTime};

use crate::alias::Alias;
use crate::download::FileInfo;
use crate::error::download as DownloadError;
use crate::error::Error;
use crate::include_query;
use crate::storage::dir::Dir;

pub(super) async fn handler(req: Request<Body>) -> Result<Response<Body>, Error> {
    let (size, stream) = process_download(req).await?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_LENGTH, size)
        .header(CONTENT_TYPE, "application/zip")
        .header(CONTENT_DISPOSITION, r#"attachment; filename="archive.zip""#)
        .body(Body::wrap_stream(ReaderStream::new(stream)))?)
}

async fn process_download(req: Request<Body>) -> Result<(usize, DuplexStream), Error> {
    let alias = req.param("alias").ok_or(DownloadError::AliasExtract)?;

    let aliases = alias
        .split('+')
        .map(|a| a.parse::<Alias>().map_err(|_| DownloadError::InvalidAlias))
        .collect::<Result<Vec<_>, _>>()?;

    let dir = req.data::<Dir>().ok_or(DownloadError::PathResolve)?.clone();

    let pool = req
        .data::<SqlitePool>()
        .ok_or(DownloadError::Database)?
        .clone();
    let mut conn = pool.acquire().await.map_err(|_| DownloadError::Database)?;

    let mut info = Vec::with_capacity(aliases.len());
    for alias in aliases {
        info.push(
            sqlx::query_as::<_, FileInfo>(include_query!("get_file"))
                .bind(alias.inner())
                .bind(alias.inner())
                .fetch_optional(&mut conn)
                .await
                .map_err(|_| DownloadError::Database)?
                .ok_or(DownloadError::FileNotFound)?,
        );
    }

    let mut name_occurrences = HashMap::new();
    for mut info in &mut info {
        let occurrence = name_occurrences.entry(info.name.clone()).or_insert(0u16);
        *occurrence += 1;
        if *occurrence >= 2 {
            if let Some((name, extension)) = info.name.split_once('.') {
                info.name = format!("{}-{}.{}", name, occurrence, extension);
            } else {
                info.name = format!("{}-{}", info.name, occurrence);
            }
        }
    }
    let archive_size = archive_size(info.iter().map(|f| (f.name.as_ref(), f.size as usize)));

    let (w, r) = duplex(64000);
    tokio::spawn(async move {
        let mut archive = Archive::new(w);
        for info in info {
            let mut fd = match File::open(dir.file_path(&info.id)).await {
                Ok(fd) => fd,
                Err(err) => {
                    log::error!("Failed to open file for archive streaming: {}", err);
                    break;
                }
            };
            match archive
                .append(info.name, FileDateTime::now(), &mut fd)
                .await
            {
                Ok(fd) => fd,
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
                }
            }
        }
        match archive.finalize().await {
            Ok(_) => (),
            Err(err) => log::error!("Failed to write archive's completion data: {}", err),
        }
    });

    Ok((archive_size, r))
}
