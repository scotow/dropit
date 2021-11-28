use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use hyper::{
    Body,
    header::{CONTENT_DISPOSITION, CONTENT_LENGTH}, Request, Response, StatusCode,
};
use routerify::ext::RequestExt;
use sqlx::SqlitePool;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::download::FileInfo;
use crate::error::download as DownloadError;
use crate::error::Error;
use crate::storage::dir::Dir;

pub(super) async fn handler(
    req: Request<Body>,
    info: &FileInfo,
    pool: SqlitePool,
) -> Result<Response<Body>, Error> {
    let dir = req.data::<Dir>().ok_or(DownloadError::PathResolve)?.clone();
    let fd = File::open(dir.file_path(&info.id))
        .await
        .map_err(|_| DownloadError::OpenFile)?;
    let streamer = FileStreamer::new(fd, &info, dir, pool);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_LENGTH, info.size as u64)
        .header(
            CONTENT_DISPOSITION,
            format!(r#"attachment; filename="{}""#, info.name),
        )
        .body(Body::wrap_stream(streamer))?)
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
