use hyper::StatusCode;
use serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;
use serde_json::json;
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
    #[error("cannot extract alias")]
    AliasExtract,
    #[error("invalid alias format")]
    InvalidAlias,
    #[error("cannot resolve file path")]
    PathResolve,
    #[error("cannot find file")]
    FileNotFound,
    #[error("cannot open file")]
    OpenFile,
    #[error("cannot remove file")]
    RemoveFile,
    #[error("file was partially removed")]
    PartialRemove,
    #[error("missing or invalid authorization header")]
    InvalidAuthorizationHeader,
    #[error("mismatching admin token")]
    InvalidAdminToken,
    #[error("an unexpected error happen while updating file metadata")]
    UnexpectedFileModification,
    #[error("invalid downloads count")]
    InvalidDownloadsCount,
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
            AliasExtract => StatusCode::INTERNAL_SERVER_ERROR,
            InvalidAlias => StatusCode::BAD_REQUEST,
            FileNotFound => StatusCode::NOT_FOUND,
            PathResolve => StatusCode::INTERNAL_SERVER_ERROR,
            OpenFile => StatusCode::INTERNAL_SERVER_ERROR,
            RemoveFile => StatusCode::INTERNAL_SERVER_ERROR,
            PartialRemove => StatusCode::INTERNAL_SERVER_ERROR,
            InvalidAuthorizationHeader => StatusCode::UNAUTHORIZED,
            InvalidAdminToken => StatusCode::FORBIDDEN,
            UnexpectedFileModification => StatusCode::INTERNAL_SERVER_ERROR,
            InvalidDownloadsCount => StatusCode::BAD_REQUEST,
        }
    }

    pub fn json_string(&self) -> String {
        json!({
            "success": false,
            "error": self.to_string(),
        }).to_string()
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

pub mod upload {
    pub use super::Error::{
        AliasGeneration,
        ContentLength,
        CopyFile,
        CreateFile,
        Database,
        FilenameHeader,
        Origin,
        PathResolve,
        QuotaAccess,
        QuotaExceeded,
        SizeMismatch,
        Target,
        TimeCalculation,
        TooLarge,
    };
}

pub mod download {
    pub use super::Error::{
        AliasExtract,
        Database,
        FileNotFound,
        InvalidAlias,
        OpenFile,
        PathResolve,
    };
}

pub mod admin {
    pub use super::Error::{
        AliasExtract,
        Database,
        FileNotFound,
        InvalidAdminToken,
        InvalidAlias,
        InvalidAuthorizationHeader,
    };
}

pub mod revoke {
    pub use super::Error::{
        PartialRemove,
        PathResolve,
        RemoveFile,
    };
}

pub mod alias {
    pub use super::Error::{
        AliasGeneration,
        Database,
        Target,
        UnexpectedFileModification,
    };
}

pub mod expiration {
    pub use super::Error::{
        Database,
        TimeCalculation,
        TooLarge,
    };
}

pub mod downloads {
    pub use super::Error::*;
}