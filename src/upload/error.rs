use hyper::StatusCode;
use serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid filename header")]
    FilenameHeader,
    #[error("invalid content length")]
    ContentLength,
    #[error("file too large")]
    TooLarge,
    #[error("cannot calculate expiration")]
    TimeCalculation,
    #[error("cannot generate alias")]
    AliasGeneration,
    #[error("cannot determine origin")]
    Origin,
    #[error("cannot determine upload target")]
    Target,
    #[error("database connection failure")]
    Database,
    #[error("quota determination failure")]
    QuotaAccess,
    #[error("too many uploads")]
    QuotaExceeded,
    #[error("cannot create file")]
    CreateFile,
    #[error("cannot copy file")]
    CopyFile,
    #[error("not matching file size")]
    SizeMismatch,
}

impl Error {
    pub fn status_code(&self) -> StatusCode {
        use Error::*;
        match self {
            FilenameHeader => StatusCode::BAD_REQUEST,
            ContentLength => StatusCode::BAD_REQUEST,
            TooLarge => StatusCode::BAD_REQUEST,
            TimeCalculation => StatusCode::INTERNAL_SERVER_ERROR,
            AliasGeneration => StatusCode::INTERNAL_SERVER_ERROR,
            Origin => StatusCode::BAD_REQUEST,
            Target => StatusCode::BAD_REQUEST,
            Database => StatusCode::INTERNAL_SERVER_ERROR,
            QuotaAccess => StatusCode::INTERNAL_SERVER_ERROR,
            QuotaExceeded => StatusCode::TOO_MANY_REQUESTS,
            CreateFile => StatusCode::INTERNAL_SERVER_ERROR,
            CopyFile => StatusCode::INTERNAL_SERVER_ERROR,
            SizeMismatch => StatusCode::BAD_REQUEST,
        }
    }
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> where
        S: Serializer {
        let mut state = serializer.serialize_struct("Error", 1)?;
        state.serialize_field("error", &self.to_string())?;
        state.end()
    }
}