use std::{collections::VecDeque, error::Error};

use futures::future::try_join_all;

use crate::{client::Client, options::Options, upload_request::UploadRequest};

mod client;
mod options;
mod upload_request;

const DEFAULT_CONCURRENT_UPLOAD: usize = 4;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let options = Options::parse();

    let client = Client::new(
        options.endpoint.clone(),
        options.credentials(),
        options.progress_bar(),
    );
    let queue =
        VecDeque::from(try_join_all(options.paths.iter().map(|p| UploadRequest::new(p))).await?);
    client.run(queue).await;

    Ok(())
}
