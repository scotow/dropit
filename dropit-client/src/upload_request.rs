use std::{
    error::Error,
    path::Path,
    pin::Pin,
    task::{Context, Poll},
};

use aes_stream::{AesKeySize, AesSteam};
use bytes::Bytes;
use futures::{ready, Stream};
use indicatif::{ProgressBar, ProgressStyle};
use num_integer::Integer;
use reqwest::{header, Body, Client};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::options::Credentials;

pub struct UploadRequest {
    fd: File,
    name: Option<String>,
    size: u64,
    progress_bar: Option<ProgressBar>,
    mode: Mode,
}

impl UploadRequest {
    pub async fn new(path: &str, mode: Mode) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let fd = File::open(&path).await?;
        let metadata = fd.metadata().await?;
        if !metadata.is_file() {
            return Err("File is not a regular file.".into());
        }
        let name = Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.to_owned());
        let size = match mode {
            Mode::Raw => metadata.len(),
            Mode::Encrypted { .. } => Integer::next_multiple_of(&(metadata.len() + 1), &16),
        };
        Ok(Self {
            fd,
            name,
            size,
            progress_bar: None,
            mode,
        })
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
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
                self.mode,
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

enum DynamicSteam {
    Raw(ReaderStream<File>),
    Encrypted(AesSteam<ReaderStream<File>>),
}

struct UploadStream {
    inner: DynamicSteam,
    progress: Option<ProgressBar>,
}

impl UploadStream {
    fn new(fd: File, progress: Option<ProgressBar>, mode: Mode) -> Self {
        Self {
            inner: match mode {
                Mode::Raw => DynamicSteam::Raw(ReaderStream::new(fd)),
                Mode::Encrypted { .. } => DynamicSteam::Encrypted(
                    AesSteam::new(
                        ReaderStream::new(fd),
                        AesKeySize::Aes128,
                        &[0x42; 16],
                        &[0; 16],
                    )
                    .unwrap(),
                ),
            },
            progress,
        }
    }
}

impl Stream for UploadStream {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let chunk = ready!(match &mut self.inner {
            DynamicSteam::Raw(s) => Pin::new(s).poll_next(cx),
            DynamicSteam::Encrypted(s) => Pin::new(s).poll_next(cx),
        });
        if let Some(Ok(chunk)) = &chunk {
            if let Some(progress) = &self.progress {
                progress.inc(chunk.len() as u64);
            }
        }
        Poll::Ready(chunk)
    }
}

pub enum Mode {
    Raw,
    Encrypted { as_command: bool },
}
