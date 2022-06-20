use std::pin::Pin;
use std::task::{Context, Poll};

use axum::body::StreamBody;
use axum::response::{IntoResponse, Response};
use futures::Stream;
use hyper::http::HeaderValue;
use hyper::{
    header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE},
    StatusCode,
};
use sqlx::SqlitePool;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::download::FileInfo;
use crate::error::download as DownloadError;
use crate::error::Error;
use crate::storage::Dir;

pub(super) async fn handler(
    pool: SqlitePool,
    info: &FileInfo,
    dir: Dir,
) -> Result<Response, Error> {
    let fd = dir
        .open_file(&info.id)
        .await
        .map_err(|_| DownloadError::OpenFile)?;
    let streamer = FileStreamer::new(fd, info, dir, pool);

    Ok((
        StatusCode::OK,
        [
            (CONTENT_LENGTH, HeaderValue::from(info.size as u64)),
            (
                CONTENT_TYPE,
                HeaderValue::from_static("application/octet-stream"),
            ),
            (
                CONTENT_DISPOSITION,
                HeaderValue::try_from(format!(r#"attachment; filename="{}""#, info.name))
                    .map_err(|_| DownloadError::FilenameHeader)?,
            ),
        ],
        StreamBody::new(streamer),
    )
        .into_response())
}

struct FileStreamer {
    streamed: usize,
    total: usize,
    decremented: bool,
    file: ReaderStream<File>,
    id: String,
    dir: Dir,
    pool: SqlitePool,
}

impl FileStreamer {
    fn new(file: File, info: &FileInfo, dir: Dir, pool: SqlitePool) -> Self {
        Self {
            streamed: 0,
            total: info.size as usize,
            decremented: false,
            file: ReaderStream::new(file),
            id: info.id.clone(),
            dir,
            pool,
        }
    }

    fn downloaded(&mut self) {
        self.decremented = true;
        let id = self.id.clone();
        let dir = self.dir.clone();
        let pool = self.pool.clone();
        tokio::spawn(async move {
            match super::file_downloaded(&pool, &dir, &id).await {
                Ok(_) => (),
                Err(err) => log::error!("Failed to process file downloads counter update: {}", err),
            }
        });
    }
}

impl Stream for FileStreamer {
    type Item = <ReaderStream<File> as Stream>::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let polled = Pin::new(&mut self.file).poll_next(cx);
        if let Poll::Ready(Some(Ok(data))) = &polled {
            self.streamed += data.len();
            if !self.decremented && self.streamed * 100 / self.total >= 95 {
                self.downloaded();
            }
        }
        polled
    }
}
