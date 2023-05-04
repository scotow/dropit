use std::{
    error::Error,
    path::Path,
    pin::Pin,
    task::{Context, Poll},
};

use aes_stream::{AesKeySize, AesSteam};
use bytes::Bytes;
use futures::{ready, Stream};
use indicatif::ProgressBar;
use num_integer::Integer;
use rand::random;
use reqwest::{header, Body, Client};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::{
    options::{Credentials, Mode},
    reporter::Reporter,
};

pub struct UploadRequest {
    fd: Option<File>,
    name: Option<String>,
    size: u64,
    mode: OutputMode,
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
            Mode::Link => metadata.len(),
            Mode::Encrypted | Mode::EncryptedRaw => {
                Integer::next_multiple_of(&(metadata.len() + 1), &16)
            }
        };
        let mode = match mode {
            Mode::Link => OutputMode::Link,
            Mode::Encrypted | Mode::EncryptedRaw => OutputMode::Encrypted {
                as_command: matches!(mode, Mode::Encrypted),
                key: [(); 16].map(|_| random()),
                iv: [(); 16].map(|_| random()),
            },
        };
        Ok(Self {
            fd: Some(fd),
            name,
            size,
            mode,
        })
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub async fn process(
        mut self,
        endpoint: &str,
        credentials: &Option<Credentials>,
        reporter: Reporter,
    ) {
        let client = Client::new();
        let mut req = client
            .post(endpoint)
            .header(header::CONTENT_LENGTH, self.size)
            .header(header::ACCEPT, "text/plain");
        if let Some(credentials) = credentials {
            req = req.basic_auth(&credentials.username, Some(&credentials.password));
        }

        if let Some(name) = &self.name {
            req = req.header("X-Filename", name);
        }

        let resp = req
            .body(Body::wrap_stream(UploadStream::new(
                self.fd.take().expect("file already uploaded"),
                reporter.progress_bar(),
                &self.mode,
            )))
            .send()
            .await;
        let resp = match resp {
            Ok(resp) => resp,
            Err(_) => {
                reporter.finalize(Err("connection error"));
                return;
            }
        };

        let status = resp.status();
        let text = match resp.text().await {
            Ok(link) => link,
            Err(_) => {
                reporter.finalize(Err("invalid body"));
                return;
            }
        };

        if !status.is_success() {
            reporter.finalize(Err(text));
            return;
        }

        let output = match &self.mode {
            OutputMode::Link => text,
            OutputMode::Encrypted {
                as_command,
                key,
                iv,
            } => {
                if *as_command {
                    format!(
                        "curl -s {} | openssl enc -aes-128-cbc -d -K {} -iv {} -nosalt",
                        text,
                        hex::encode(&key),
                        hex::encode(&iv)
                    )
                } else {
                    format!("{} key={} iv={}", text, hex::encode(&key), hex::encode(&iv))
                }
            }
        };

        reporter.finalize(Ok(output))
    }
}

enum OutputMode {
    Link,
    Encrypted {
        as_command: bool,
        key: [u8; 16],
        iv: [u8; 16],
    },
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
    fn new(fd: File, progress: Option<ProgressBar>, mode: &OutputMode) -> Self {
        Self {
            inner: match mode {
                OutputMode::Link => DynamicSteam::Raw(ReaderStream::new(fd)),
                OutputMode::Encrypted {
                    as_command: _as_command,
                    key,
                    iv,
                } => DynamicSteam::Encrypted(
                    AesSteam::new(ReaderStream::new(fd), AesKeySize::Aes128, key, iv)
                        .expect("Invalid key or iv"),
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
