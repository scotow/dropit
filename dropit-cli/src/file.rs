use std::{
    pin::Pin,
    task::{Context, Poll},
    thread::sleep,
    time::Duration,
};

use futures_core::Stream;
use indicatif::ProgressBar;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

pub struct FileWrapper {
    fd: ReaderStream<File>,
    progress: ProgressBar,
}

impl FileWrapper {
    pub fn new(fd: File, progress: ProgressBar) -> Self {
        Self {
            fd: ReaderStream::new(fd),
            progress,
        }
    }

    pub fn progress_bar(&self) -> &ProgressBar {
        &self.progress
    }
}

impl Stream for FileWrapper {
    type Item = <ReaderStream<File> as Stream>::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let polled = Pin::new(&mut self.fd).poll_next(cx);
        if let Poll::Ready(Some(Ok(data))) = &polled {
            self.progress.inc(data.len() as u64);
            sleep(Duration::from_millis(5));
        }
        polled
    }
}
