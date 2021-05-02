use serde::Serialize;
use byte_unit::Byte;
use std::convert::TryFrom;
use std::time::{Duration, UNIX_EPOCH, SystemTime};
use crate::upload::error::Error as UploadError;
use humantime::{format_duration, format_rfc3339_seconds};

#[derive(Serialize)]
pub struct UploadInfo {
    pub(super) name: String,
    pub(super) size: Size,
    pub(super) alias: Aliases,
    pub(super) link: Links,
    pub(super) expiration: Expiration,
}

impl UploadInfo {
    pub fn new(name: String, size: u64, alias: (String, String), link_base: String, expiration: Expiration) -> Self {
        Self {
            name,
            size: Size::from(size),
            alias: Aliases {
                short: alias.0.clone(),
                long: alias.1.clone(),
            },
            link: Links {
                short: format!("{}/{}", link_base, &alias.0),
                long: format!("{}/{}", link_base, &alias.1),
            },
            expiration
        }
    }
}

#[derive(Serialize)]
pub struct Size {
    pub(super) bytes: u64,
    pub(super) readable: String,
}

impl From<u64> for Size {
    fn from(bytes: u64) -> Self {
        Self {
            bytes,
            readable: Byte::from_bytes(bytes).get_appropriate_unit(false).to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct Aliases {
    pub(super) short: String,
    pub(super) long: String,
}

#[derive(Serialize)]
pub struct Links {
    pub(super) short: String,
    pub(super) long: String,
}

#[derive(Serialize)]
pub struct Expiration {
    pub(super) duration: ExpirationDuration,
    pub(super) date: ExpirationDate,
}

impl Expiration {
    pub fn timestamp(&self) -> u64 {
        self.date.timestamp
    }
}

impl TryFrom<Duration> for Expiration {
    type Error = UploadError;

    fn try_from(duration: Duration) -> Result<Self, Self::Error> {
        Ok(
            Self {
                duration: ExpirationDuration::from(duration),
                date: ExpirationDate::try_from(duration)?,
            }
        )
    }
}

#[derive(Serialize)]
pub struct ExpirationDuration {
    pub(super) seconds: u64,
    pub(super) readable: String,
}

impl From<Duration> for ExpirationDuration {
    fn from(duration: Duration) -> Self {
        Self {
            seconds: duration.as_secs(),
            readable: format_duration(duration).to_string().replace(' ', ""),
        }
    }
}

#[derive(Serialize)]
pub struct ExpirationDate {
    pub(super) timestamp: u64,
    pub(super) readable: String,
}

impl TryFrom<Duration> for ExpirationDate {
    type Error = UploadError;

    fn try_from(duration: Duration) -> Result<Self, Self::Error> {
        let expiration = SystemTime::now() + duration;
        Ok(
            Self {
                timestamp: expiration.duration_since(UNIX_EPOCH).map_err(|_| UploadError::TimeCalculation)?.as_secs(),
                readable: {
                    let mut full = format_rfc3339_seconds(expiration).to_string();
                    full.truncate(full.len() - 4);
                    full.replace('T', " ").replace('-', "/")
                },
            }
        )
    }
}