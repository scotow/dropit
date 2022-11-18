use std::{
    collections::VecDeque, env::args, error::Error, path::Path, thread, thread::sleep,
    time::Duration,
};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use itertools::Itertools;
use reqwest::{header, Body, Client};
use tokio::{fs::File, task};

use crate::file::FileWrapper;

mod file;

const DEFAULT_CONCURRENT_UPLOAD: usize = 4;

struct Upload {
    fd: File,
    name: Option<String>,
    size: u64,
    index: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut queue = VecDeque::new();
    for (i, path) in args().skip(1).enumerate() {
        let fd = File::open(&path).await?;
        let metadata = fd.metadata().await?;
        if !metadata.is_file() {
            Err("File is not a regular file.")?;
        }
        queue.push_back(Upload {
            fd,
            name: Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.to_owned()),
            size: metadata.len(),
            index: i,
        });
    }

    let (tx, rx) = flume::bounded::<(Upload, ProgressBar)>(0);
    let mut handlers = Vec::with_capacity(DEFAULT_CONCURRENT_UPLOAD);

    let pb_group = MultiProgress::new();
    let queue_bar = pb_group.insert(usize::MAX, ProgressBar::new(1));
    queue_bar
        .set_style(ProgressStyle::with_template("{spinner} {prefix:.bold}: {wide_msg}").unwrap());
    queue_bar.set_prefix("Queue");
    queue_bar.enable_steady_tick(Duration::from_millis(100));

    for _ in 0..DEFAULT_CONCURRENT_UPLOAD {
        let rx = rx.clone();
        handlers.push(task::spawn(async move {
            while let Ok((upload, progress_bar)) = rx.recv_async().await {
                upload_file(upload, progress_bar).await;
            }
        }));
    }

    update_queue_bar(&queue, &queue_bar);
    let previous = None;
    for _ in 0..queue.len() {
        let next = queue.pop_front().unwrap();
        let mut progress_bar = ProgressBar::new(next.size).with_style(
            ProgressStyle::with_template(
                "{prefix:.bold} [{elapsed_precise}] [{wide_bar}] {bytes}/{total_bytes} ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
        );
        progress_bar = match &previous {
            Some(p) => pb_group.insert_after(p, progress_bar),
            None => pb_group.insert_before(&queue_bar, progress_bar),
        };
        tx.send_async((next, progress_bar)).await.unwrap();
        update_queue_bar(&queue, &queue_bar);
    }

    drop(tx);
    for handler in handlers {
        handler.await.unwrap();
    }

    Ok(())
}

fn update_queue_bar(queue: &VecDeque<Upload>, bar: &ProgressBar) {
    if queue.is_empty() {
        bar.finish_and_clear();
    } else {
        bar.set_message(
            queue
                .iter()
                .map(|u| u.name.as_deref())
                .map(|u| u.unwrap_or("-"))
                .join(", "),
        );
    }
}

async fn upload_file(upload: Upload, progress_bar: ProgressBar) {
    let client = Client::new();
    let mut req = client
        .post("http://localhost:8080")
        .header(header::CONTENT_LENGTH, upload.size)
        .header(header::ACCEPT, "text/plain");

    if let Some(name) = upload.name {
        progress_bar.set_prefix(name.clone());
        req = req.header("X-Filename", &name);
    }

    let resp = req
        .body(Body::wrap_stream(FileWrapper::new(
            upload.fd,
            progress_bar.clone(),
        )))
        .send()
        .await
        .unwrap();
    let link = resp.text().await.unwrap();
    progress_bar.set_style(ProgressStyle::with_template("{prefix:.bold} {msg}").unwrap());
    progress_bar.finish_with_message(link);
}
