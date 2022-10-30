use async_trait::async_trait;
use axum::extract::{FromRequest, RequestParts};
use hyper::Body;
use percent_encoding::percent_decode_str;
use sanitize_filename::sanitize;

use crate::error::Error as UploadError;

pub struct Filename(pub Option<String>);

#[async_trait]
impl FromRequest<Body> for Filename {
    type Rejection = UploadError;

    async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
        if let Some(header) = req.headers().get("X-Filename") {
            let header = header.to_str().map_err(|_| UploadError::FilenameHeader)?;
            let filename = sanitize(
                percent_decode_str(header)
                    .decode_utf8()
                    .map_err(|_| UploadError::FilenameHeader)?,
            );
            Ok(Self(Some(filename)))
        } else {
            Ok(Self(None))
        }
    }
}
