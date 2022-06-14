use axum::response::{IntoResponse, Response};
use axum::Json;
use hyper::http::HeaderValue;
use hyper::{header, HeaderMap, StatusCode};
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use serde_json::json;
use thiserror::Error;

use crate::response::{ApiHeader, SingleLine};

#[derive(Error, Debug)]
pub enum Error {
    #[error("internal server error")]
    Generic,
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
    #[error("missing authorization header")]
    MissingAuthorization,
    #[error("missing or invalid authorization header")]
    InvalidAuthorizationHeader,
    #[error("mismatching admin token")]
    InvalidAdminToken,
    #[error("invalid credentials or authentication process")]
    AccessForbidden,
    #[error("an unexpected error happen while updating file metadata")]
    UnexpectedFileModification,
    #[error("invalid downloads count")]
    InvalidDownloadsCount,
    #[error("authorization process failure")]
    AuthProcess,
    #[error("assets catalogue connection failure")]
    AssetsCatalogue,
    #[error("asset not found")]
    AssetNotFound,
}

impl Error {
    pub fn status_code(&self) -> StatusCode {
        use Error::*;
        match self {
            Generic => StatusCode::INTERNAL_SERVER_ERROR,
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
            MissingAuthorization => StatusCode::UNAUTHORIZED,
            InvalidAuthorizationHeader => StatusCode::UNAUTHORIZED,
            InvalidAdminToken => StatusCode::FORBIDDEN,
            AccessForbidden => StatusCode::FORBIDDEN,
            UnexpectedFileModification => StatusCode::INTERNAL_SERVER_ERROR,
            InvalidDownloadsCount => StatusCode::BAD_REQUEST,
            AuthProcess => StatusCode::INTERNAL_SERVER_ERROR,
            AssetsCatalogue => StatusCode::INTERNAL_SERVER_ERROR,
            AssetNotFound => StatusCode::NOT_FOUND,
        }
    }
}

impl ApiHeader for Error {
    fn status_code(&self) -> StatusCode {
        self.status_code()
    }

    fn additional_headers(&self) -> HeaderMap {
        use Error::*;
        match self {
            MissingAuthorization => [(header::WWW_AUTHENTICATE, HeaderValue::from_static("Basic"))]
                .into_iter()
                .collect(),
            _ => HeaderMap::default(),
        }
    }

    fn success(&self) -> bool {
        false
    }
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Error", 3)?;
        state.serialize_field("error", &self.to_string())?;
        state.end()
    }
}

impl SingleLine for Error {
    fn single_lined(&self) -> String {
        self.to_string()
    }
}

// JSON only.
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (
            self.status_code(),
            self.additional_headers(),
            Json(json!({
                "success": false,
                "error": self.to_string(),
            })),
        )
            .into_response()
    }
}

impl From<hyper::http::Error> for Error {
    fn from(_: hyper::http::Error) -> Self {
        Self::Generic
    }
}

impl From<hyper::Error> for Error {
    fn from(_: hyper::Error) -> Self {
        Self::Generic
    }
}

pub mod upload {
    pub use super::Error::{
        AliasGeneration, ContentLength, CopyFile, CreateFile, Database, FilenameHeader, Origin,
        PathResolve, QuotaAccess, QuotaExceeded, SizeMismatch, Target, TimeCalculation, TooLarge,
    };
}

pub mod download {
    pub use super::Error::{
        AliasExtract, Database, FileNotFound, FilenameHeader, InvalidAlias, OpenFile, PathResolve,
    };
}

pub mod admin {
    pub use super::Error::{
        AliasExtract, Database, FileNotFound, InvalidAdminToken, InvalidAlias,
        InvalidAuthorizationHeader,
    };
}

pub mod revoke {
    pub use super::Error::{PartialRemove, PathResolve, RemoveFile};
}

pub mod alias {
    pub use super::Error::{AliasGeneration, Database, Target, UnexpectedFileModification};
}

pub mod expiration {
    pub use super::Error::{Database, TimeCalculation, TooLarge};
}

pub mod downloads {
    pub use super::Error::{InvalidDownloadsCount, UnexpectedFileModification};
}

pub mod valid {
    pub use super::Error::{AliasExtract, Database, InvalidAlias};
}

pub mod assets {
    pub use super::Error::{AssetNotFound, AssetsCatalogue};
}

pub mod auth {
    pub use super::Error::{
        AccessForbidden, AuthProcess, InvalidAuthorizationHeader, MissingAuthorization,
    };
}
