use std::{collections::VecDeque, time::Duration};

use flume::Receiver as MpmcReceiver;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use itertools::Itertools;
use tokio::{
    sync::{mpsc, oneshot::Receiver},
    task,
    task::JoinHandle,
};

use crate::{options::Credentials, reporter::Reporter, upload_request::UploadRequest};

pub struct Client {
    endpoint: String,
    credentials: Option<Credentials>,
    progress: Option<Progress>,
    concurrent_uploads: usize,
}

impl Client {
    pub fn new(
        endpoint: String,
        credentials: Option<Credentials>,
        progress: bool,
        concurrent_uploads: usize,
    ) -> Self {
        Self {
            endpoint,
            credentials,
            progress: progress.then(|| Progress::new()),
            concurrent_uploads,
        }
    }

    pub async fn run(&self, mut queue: VecDeque<UploadRequest>) {
        let (workers_tx, workers_rx) = flume::bounded::<(UploadRequest, Reporter)>(0);
        let workers = self.spawn_workers(workers_rx);

        let (printer_tx, mut printer_rx) = mpsc::unbounded_channel::<Receiver<String>>();
        if self.progress.is_none() {
            task::spawn(async move {
                while let Some(finish_rx) = printer_rx.recv().await {
                    let link = finish_rx.await.expect("Failed to get file link");
                    println!("{link}");
                }
            });
        }

        self.update_queue_bar(&queue);
        for _ in 0..queue.len() {
            let next = queue.pop_front().expect("Queue processing error");
            let reporter = if let Some(progress) = &self.progress {
                let (reporter, progress_bar) = Reporter::new_progress_bar(next.size());
                progress.insert(progress_bar.clone());
                if let Some(name) = next.name() {
                    progress_bar.set_prefix(name.to_owned());
                }
                reporter
            } else {
                let (reporter, finish_rx) = Reporter::new_channel();
                printer_tx.send(finish_rx).expect("Queue processing error");
                reporter
            };

            workers_tx
                .send_async((next, reporter))
                .await
                .expect("Queue processing error");
            self.update_queue_bar(&queue);
        }

        drop(workers_tx);
        for worker in workers {
            worker.await.expect("Queue processing error");
        }
    }

    fn spawn_workers(&self, rx: MpmcReceiver<(UploadRequest, Reporter)>) -> Vec<JoinHandle<()>> {
        let mut handlers = Vec::with_capacity(self.concurrent_uploads);
        for _ in 0..self.concurrent_uploads {
            let rx = rx.clone();
            let endpoint = self.endpoint.clone();
            let credentials = self.credentials.clone();
            handlers.push(task::spawn(async move {
                while let Ok((upload, reporter)) = rx.recv_async().await {
                    upload.process(&endpoint, &credentials, reporter).await;
                }
            }));
        }
        handlers
    }

    fn update_queue_bar(&self, queue: &VecDeque<UploadRequest>) {
        let Some(progress) = &self.progress else {
            return
        };

        if queue.is_empty() {
            progress.queue.finish_and_clear();
        } else {
            progress
                .queue
                .set_message(queue.iter().map(|u| u.name().unwrap_or("-")).join(", "));
        }
    }
}

struct Progress {
    group: MultiProgress,
    queue: ProgressBar,
}

impl Progress {
    fn new() -> Self {
        let group = MultiProgress::with_draw_target(ProgressDrawTarget::stderr());
        let queue = group.insert(usize::MAX, ProgressBar::new(1));
        queue.set_style(
            ProgressStyle::with_template("{spinner} {prefix:.bold}: {wide_msg}").unwrap(),
        );
        queue.set_prefix("Queue");
        queue.enable_steady_tick(Duration::from_millis(100));
        Self { group, queue }
    }

    fn insert(&self, bar: ProgressBar) {
        self.group.insert_before(&self.queue, bar);
    }
}
