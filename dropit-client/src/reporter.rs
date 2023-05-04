use indicatif::{ProgressBar, ProgressStyle};
use tokio::sync::{
    oneshot,
    oneshot::{Receiver, Sender},
};

pub enum Reporter {
    Channel(Sender<String>),
    ProgressBar(ProgressBar),
}

impl Reporter {
    pub fn new_channel() -> (Self, Receiver<String>) {
        let (tx, rx) = oneshot::channel();
        (Self::Channel(tx), rx)
    }

    pub fn new_progress_bar(size: u64) -> (Self, ProgressBar) {
        let progress_bar = ProgressBar::new(size).with_style(
            ProgressStyle::with_template(
                "{prefix:.bold} [{elapsed_precise}] [{wide_bar}] {bytes}/{total_bytes} ({eta})",
            )
            .expect("Invalid progress bar template")
            .progress_chars("#>-"),
        );
        (Self::ProgressBar(progress_bar.clone()), progress_bar)
    }

    pub fn progress_bar(&self) -> Option<ProgressBar> {
        match self {
            Reporter::Channel(_) => None,
            Reporter::ProgressBar(progress_bar) => Some(progress_bar.clone()),
        }
    }

    pub fn finalize<S: Into<String>>(self, message: Result<S, S>) {
        let message = match message {
            Ok(msg) => msg.into(),
            Err(err) => format!("error: {}", err.into()),
        };
        match self {
            Reporter::Channel(ch) => ch.send(message).expect("Cannot send last message"),
            Reporter::ProgressBar(progress_bar) => {
                progress_bar.set_style(
                    ProgressStyle::with_template("{prefix:.bold} {msg}")
                        .expect("Invalid final progress bar template"),
                );
                progress_bar.finish_with_message(message);
            }
        }
    }
}
