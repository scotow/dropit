use std::collections::HashMap;

use axum::body::StreamBody;
use axum::response::{IntoResponse, Response};
use hyper::header::HeaderValue;
use hyper::{
    header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE},
    StatusCode,
};
use sqlx::SqlitePool;
use tokio::io::duplex;
use tokio_util::io::ReaderStream;
use zipit::{archive_size, Archive, FileDateTime};

use crate::download::FileInfo;
use crate::error::Error;
use crate::storage::Dir;

pub(super) async fn handler(
    pool: SqlitePool,
    mut files_info: Vec<FileInfo>,
    dir: Dir,
) -> Result<Response, Error> {
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
            let mut fd = match dir.open_file(&info.id).await {
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

    Ok((
        StatusCode::OK,
        [
            (CONTENT_LENGTH, HeaderValue::from(archive_size)),
            (CONTENT_TYPE, HeaderValue::from_static("application/zip")),
            (
                CONTENT_DISPOSITION,
                HeaderValue::from_static(r#"attachment; filename="archive.zip""#),
            ),
        ],
        StreamBody::new(ReaderStream::new(r)),
    )
        .into_response())
}
