use std::{collections::VecDeque, error::Error};

use futures::future::try_join_all;

use crate::{client::Client, options::Options, upload_request::UploadRequest};

mod client;
mod options;
mod upload_request;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let options = Options::parse();

    let client = Client::new(
        options.server.clone(),
        options.credentials(),
        options.progress_bar(),
        options.concurrent_uploads,
    );
    let queue = VecDeque::from(
        try_join_all(
            options
                .paths
                .iter()
                .map(|p| UploadRequest::new(p, options.mode())),
        )
        .await?,
    );
    client.run(queue).await;

    Ok(())
}
