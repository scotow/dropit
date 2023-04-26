use std::{collections::VecDeque, time::Duration};

use flume::Receiver as MpmcReceiver;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use itertools::Itertools;
use tokio::{
    sync::{
        mpsc, oneshot,
        oneshot::{Receiver, Sender},
    },
    task,
    task::JoinHandle,
};

use crate::{options::Credentials, upload_request::UploadRequest, DEFAULT_CONCURRENT_UPLOAD};

pub struct Client {
    endpoint: String,
    credentials: Option<Credentials>,
    progress: Option<Progress>,
}

impl Client {
    pub fn new(endpoint: String, credentials: Option<Credentials>, progress: bool) -> Self {
        Self {
            endpoint,
            credentials,
            progress: progress.then(|| Progress::new()),
        }
    }

    pub async fn run(&self, mut queue: VecDeque<UploadRequest>) {
        let (workers_tx, workers_rx) = flume::bounded::<(UploadRequest, Sender<String>)>(0);
        let workers = self.spawn_workers(workers_rx);

        let (printer_tx, mut printer_rx) = mpsc::unbounded_channel::<Receiver<String>>();
        if self.progress.is_none() {
            task::spawn(async move {
                while let Some(finish_rx) = printer_rx.recv().await {
                    let link = finish_rx.await.unwrap();
                    println!("{link}");
                }
            });
        }

        self.update_queue_bar(&queue);
        for _ in 0..queue.len() {
            let mut next = queue.pop_front().unwrap();
            if let Some(progress) = &self.progress {
                progress.insert(next.progress_bar(), None);
            }

            let (finish_tx, finish_rx) = oneshot::channel();
            printer_tx.send(finish_rx).unwrap();
            workers_tx.send_async((next, finish_tx)).await.unwrap();

            self.update_queue_bar(&queue);
        }

        drop(workers_tx);
        for worker in workers {
            worker.await.unwrap();
        }
    }

    fn spawn_workers(
        &self,
        rx: MpmcReceiver<(UploadRequest, Sender<String>)>,
    ) -> Vec<JoinHandle<()>> {
        let mut handlers = Vec::with_capacity(DEFAULT_CONCURRENT_UPLOAD);
        for _ in 0..DEFAULT_CONCURRENT_UPLOAD {
            let rx = rx.clone();
            let endpoint = self.endpoint.clone();
            let credentials = self.credentials.clone();
            handlers.push(task::spawn(async move {
                while let Ok((upload, finish_tx)) = rx.recv_async().await {
                    finish_tx
                        .send(upload.process(&endpoint, &credentials).await)
                        .unwrap();
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

    fn insert(&self, bar: ProgressBar, previous: Option<ProgressBar>) {
        match &previous {
            Some(p) => self.group.insert_after(p, bar),
            None => self.group.insert_before(&self.queue, bar),
        };
    }
}
