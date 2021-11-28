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

pub(super) async fn handler(
    req: Request<Body>,
    mut files_info: Vec<FileInfo>,
    pool: SqlitePool,
) -> Result<Response<Body>, Error> {
    let dir = req.data::<Dir>().ok_or(DownloadError::PathResolve)?.clone();

    let mut name_occurrences = HashMap::new();
    for mut info in &mut files_info {
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
    let archive_size = archive_size(
        files_info
            .iter()
            .map(|f| (f.name.as_ref(), f.size as usize)),
    );

    let (w, r) = duplex(64000);
    tokio::spawn(async move {
        let mut archive = Archive::new(w);
        for info in files_info {
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

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_LENGTH, archive_size)
        .header(CONTENT_TYPE, "application/zip")
        .header(CONTENT_DISPOSITION, r#"attachment; filename="archive.zip""#)
        .body(Body::wrap_stream(ReaderStream::new(r)))?)
}
