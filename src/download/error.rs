use hyper::StatusCode;
use serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot extract alias")]
    AliasExtract,
    #[error("invalid alias format")]
    InvalidAlias,
    #[error("database connection failure")]
    Database,
    #[error("cannot find file")]
    FileNotFound,
    #[error("cannot open file")]
    OpenFile,
}

impl Error {
    pub fn status_code(&self) -> StatusCode {
        use Error::*;
        match self {
            AliasExtract => StatusCode::INTERNAL_SERVER_ERROR,
            InvalidAlias => StatusCode::BAD_REQUEST,
            Database => StatusCode::INTERNAL_SERVER_ERROR,
            FileNotFound => StatusCode::NOT_FOUND,
            OpenFile => StatusCode::INTERNAL_SERVER_ERROR,
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