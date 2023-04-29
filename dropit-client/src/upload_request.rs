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
use rand::random;
use reqwest::{header, Body, Client};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::options::Credentials;

pub struct UploadRequest {
    fd: Option<File>,
    name: Option<String>,
    size: u64,
    progress_bar: Option<ProgressBar>,
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
            Mode::Raw => metadata.len(),
            Mode::Encrypted { .. } => Integer::next_multiple_of(&(metadata.len() + 1), &16),
        };
        let mode = match mode {
            Mode::Raw => OutputMode::Raw,
            Mode::Encrypted { as_command } => OutputMode::Encrypted {
                as_command,
                key: [(); 16].map(|_| random()),
                iv: [(); 16].map(|_| random()),
            },
        };
        Ok(Self {
            fd: Some(fd),
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
                    .expect("Invalid progress bar template")
                    .progress_chars("#>-")
            )
            })
            .clone()
    }

    pub async fn process(mut self, endpoint: &str, credentials: &Option<Credentials>) -> String {
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
                self.fd.take().expect("file already uploaded"),
                self.progress_bar.clone(),
                &self.mode,
            )))
            .send()
            .await;
        let resp = match resp {
            Ok(resp) => resp,
            Err(_) => {
                self.finalize_bar(Err("connection error"));
                return "connection error".to_owned();
            }
        };

        let status = resp.status();
        let text = match resp.text().await {
            Ok(link) => link,
            Err(_) => {
                self.finalize_bar(Err("invalid body"));
                return "invalid body".to_owned();
            }
        };

        if !status.is_success() {
            self.finalize_bar(Err(&text));
            return text;
        }

        let output = match &self.mode {
            OutputMode::Raw => text,
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

        self.finalize_bar(Ok(&output));
        output
    }

    fn finalize_bar(&self, message: Result<&str, &str>) {
        if let Some(progress) = &self.progress_bar {
            progress.set_style(
                ProgressStyle::with_template("{prefix:.bold} {msg}")
                    .expect("Invalid final progress bar template"),
            );
            progress.finish_with_message(match message {
                Ok(m) => m.to_owned(),
                Err(m) => format!("error: {}", m),
            });
        };
    }
}

enum OutputMode {
    Raw,
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
                OutputMode::Raw => DynamicSteam::Raw(ReaderStream::new(fd)),
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

pub enum Mode {
    Raw,
    Encrypted { as_command: bool },
}
