use std::{collections::VecDeque, time::Duration};

use flume::Receiver as MpmcReceiver;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use itertools::Itertools;
use tokio::{
    sync::{
        mpsc, oneshot,
        oneshot::{Receiver, Sender},
    },
    task,
    task::JoinHandle,
};

use crate::{
    options::{Credentials, Options},
    upload_request::UploadRequest,
    DEFAULT_CONCURRENT_UPLOAD,
};

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
            progress: progress.then(|| Progress::default()),
        }
    }

    pub async fn run(&self, mut queue: VecDeque<UploadRequest>) {
        let (tx, rx) = flume::bounded::<(UploadRequest, Sender<String>)>(0);
        let workers = self.spawn_workers(rx);

        let (printer_tx, mut printer_rx) = mpsc::unbounded_channel::<Receiver<String>>();
        task::spawn(async move {
            while let Some(finish_rx) = printer_rx.recv().await {
                let link = finish_rx.await.unwrap();
                println!("{link}");
            }
        });

        // let mut finish_handlers = Vec::with_capacity(queue.len());

        self.update_queue_bar(&queue);
        // let mut previous = None;
        for _ in 0..queue.len() {
            let mut next = queue.pop_front().unwrap();
            if let Some(progress) = &self.progress {
                progress.insert(next.progress_bar(), None);
            }
            // previous = Some(next.progress_bar());
            // let mut progress_bar = ProgressBar::new(next.size).with_style(
            //     ProgressStyle::with_template(
            //         "{prefix:.bold} [{elapsed_precise}] [{wide_bar}] {bytes}/{total_bytes} ({eta})",
            //     )
            //     .unwrap()
            //     .progress_chars("#>-"),
            // );
            // progress_bar = match &previous {
            //     Some(p) => self.progress.unwrap().group.insert_after(p, progress_bar),
            //     None => self.progress.unwrap().group.insert_before(&queue_bar, progress_bar),
            // };
            let (finish_tx, finish_rx) = oneshot::channel();
            printer_tx.send(finish_rx).unwrap();
            // finish_handlers.push(finish_rx);

            // let printer_tx = printer_tx.clone();
            // task::spawn(async move {
            //     let link = finish_rx.await.unwrap();
            //     printer_tx.send(link).unwrap();
            // });

            tx.send_async((next, finish_tx)).await.unwrap();
            self.update_queue_bar(&queue);
        }

        drop(tx);
        for worker in workers {
            worker.await.unwrap();
        }
        // tokio::join!(
        //     async {
        //         for handler in finish_handlers {
        //             let link = handler.await.unwrap();
        //             if self.progress.is_none() {
        //                 println!("{link}");
        //             }
        //         }
        // }
        // );
    }

    fn spawn_workers(
        &self,
        rx: MpmcReceiver<(UploadRequest, Sender<String>)>,
    ) -> Vec<JoinHandle<()>> {
        let mut handlers = Vec::with_capacity(DEFAULT_CONCURRENT_UPLOAD);
        for _ in 0..DEFAULT_CONCURRENT_UPLOAD {
            let rx = rx.clone();
            handlers.push(task::spawn(async move {
                while let Ok((upload, done_channel)) = rx.recv_async().await {
                    let link = upload.process().await;
                    done_channel.send(link).unwrap();
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
            progress.queue.set_message(
                queue
                    .iter()
                    .map(|u| u.name.as_deref())
                    .map(|u| u.unwrap_or("-"))
                    .join(", "),
            );
        }
    }
}

struct Progress {
    group: MultiProgress,
    queue: ProgressBar,
}

impl Progress {
    fn insert(&self, bar: ProgressBar, previous: Option<ProgressBar>) {
        match &previous {
            Some(p) => self.group.insert_after(p, bar),
            None => self.group.insert_before(&self.queue, bar),
        };
    }
}

impl Default for Progress {
    fn default() -> Self {
        let group = MultiProgress::new();
        let queue = group.insert(usize::MAX, ProgressBar::new(1));
        queue.set_style(
            ProgressStyle::with_template("{spinner} {prefix:.bold}: {wide_msg}").unwrap(),
        );
        queue.set_prefix("Queue");
        queue.enable_steady_tick(Duration::from_millis(100));
        Self { group, queue }
    }
}
