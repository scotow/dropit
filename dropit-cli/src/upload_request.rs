use std::{
    error::Error,
    path::Path,
    pin::Pin,
    task::{Context, Poll},
    thread::sleep,
    time::Duration,
};

use futures::Stream;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{header, Body, Client};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::options::Credentials;

pub struct UploadRequest {
    pub fd: File,
    pub name: Option<String>,
    pub size: u64,
    progress_bar: Option<ProgressBar>,
}

impl UploadRequest {
    pub async fn new(path: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let fd = File::open(&path).await?;
        let metadata = fd.metadata().await?;
        if !metadata.is_file() {
            return Err("File is not a regular file.".into());
        }
        let name = Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.to_owned());
        Ok(Self {
            fd,
            name,
            size: metadata.len(),
            progress_bar: None,
        })
    }

    pub fn progress_bar(&mut self) -> ProgressBar {
        self.progress_bar
            .get_or_insert_with(|| {
                ProgressBar::new(self.size).with_style(
                ProgressStyle::with_template(
                    "{prefix:.bold} [{elapsed_precise}] [{wide_bar}] {bytes}/{total_bytes} ({eta})",
                )
                    .unwrap()
                    .progress_chars("#>-")
            )
            })
            .clone()
    }

    pub async fn process(self, endpoint: &str, credentials: &Option<Credentials>) -> String {
        let client = Client::new();
        let mut req = client
            .post(endpoint)
            .header(header::CONTENT_LENGTH, self.size)
            .header(header::ACCEPT, "text/plain");
        if let Some(credentials) = credentials {
            req = req.basic_auth(&credentials.username, Some(&credentials.password));
        }

        if let Some(name) = &self.name {
            if let Some(progress) = &self.progress_bar {
                progress.set_prefix(name.clone());
            }
            req = req.header("X-Filename", name);
        }

        let resp = req
            .body(Body::wrap_stream(UploadStream::new(
                self.fd,
                self.progress_bar.clone(),
            )))
            .send()
            .await
            .unwrap();
        let link = resp.text().await.unwrap();

        if let Some(progress) = self.progress_bar {
            progress.set_style(ProgressStyle::with_template("{prefix:.bold} {msg}").unwrap());
            progress.finish_with_message(link.clone());
        };
        link
    }
}

struct UploadStream {
    fd: ReaderStream<File>,
    progress: Option<ProgressBar>,
}

impl UploadStream {
    pub fn new(fd: File, progress: Option<ProgressBar>) -> Self {
        Self {
            fd: ReaderStream::new(fd),
            progress,
        }
    }
}

impl Stream for UploadStream {
    type Item = <ReaderStream<File> as Stream>::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let polled = Pin::new(&mut self.fd).poll_next(cx);
        if let Poll::Ready(Some(Ok(data))) = &polled {
            if let Some(progress) = &self.progress {
                progress.inc(data.len() as u64);
            }
            sleep(Duration::from_millis(5));
        }
        polled
    }
}
