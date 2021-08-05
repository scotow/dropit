use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use hyper::{
    Body,
    header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE},
    Request,
    Response,
    StatusCode
};
use routerify::ext::RequestExt;
use sqlx::SqlitePool;
use tokio::fs::File;
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
        Ok((info, stream)) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_LENGTH, info.size as u64)
                .header(CONTENT_DISPOSITION, format!(r#"attachment; filename="{}""#, info.name))
                .body(Body::wrap_stream(stream))
        },
        Err(err) => {
            Response::builder()
                .status(err.status_code())
                .header(CONTENT_TYPE, "text/plain")
                .body(err.to_string().into())
        }
    }.or_else(|_| Ok(generic_500()))
}

async fn process_download(req: Request<Body>) -> Result<(FileInfo, FileStreamer), Error> {
    let alias = req.param("alias")
        .ok_or(DownloadError::AliasExtract)?
        .parse::<Alias>()
        .map_err(|_| DownloadError::InvalidAlias)?;

    let pool = req.data::<SqlitePool>().ok_or(DownloadError::Database)?.clone();
    let mut conn = pool.acquire().await.map_err(|_| DownloadError::Database)?;
    let info = sqlx::query_as::<_, FileInfo>(include_query!("get_file"))
        .bind(alias.inner()).bind(alias.inner())
        .fetch_optional(&mut conn).await.map_err(|_| DownloadError::Database)?
        .ok_or(DownloadError::FileNotFound)?;

    let dir = req.data::<Dir>().ok_or(DownloadError::PathResolve)?.clone();
    let fd = File::open(dir.file_path(&info.id)
    ).await.map_err(|_| DownloadError::OpenFile)?;

    let streamer = FileStreamer::new(fd, &info, dir, pool);
    Ok((info, streamer))
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
            super::file_downloaded(&pool, &dir, &id).await;
        });
    }
}

impl Stream for FileStreamer {
    type Item = <ReaderStream<File> as Stream>::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let polled = Pin::new(&mut self.file).poll_next(cx);
        match &polled {
            Poll::Ready(Some(Ok(data))) => {
                self.streamed += data.len();
                if !self.decremented && self.streamed * 100 / self.total >= 95 {
                    self.downloaded();
                }
            },
            _ => (),
        }
        polled
    }
}
