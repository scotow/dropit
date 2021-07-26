use std::convert::TryFrom;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use byte_unit::Byte;
use humantime::{format_duration, format_rfc3339_seconds};
use serde::Serialize;

use crate::error::Error;
use crate::error::upload as UploadError;

#[derive(Serialize)]
pub struct UploadInfo {
    admin: String,
    name: String,
    size: Size,
    alias: Aliases,
    link: Links,
    expiration: Expiration,
}

impl UploadInfo {
    pub fn new(admin: String, name: String, size: u64, alias: (String, String), link_base: String, expiration: Expiration) -> Self {
        Self {
            admin,
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

    pub fn short_link(self) -> String {
        self.link.short
    }
}

#[derive(Serialize)]
pub struct Size {
    bytes: u64,
    readable: String,
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
    short: String,
    long: String,
}

#[derive(Serialize)]
pub struct Links {
    short: String,
    long: String,
}

#[derive(Serialize)]
pub struct Expiration {
    duration: ExpirationDuration,
    date: ExpirationDate,
}

impl Expiration {
    pub fn timestamp(&self) -> u64 {
        self.date.timestamp
    }
}

impl TryFrom<Duration> for Expiration {
    type Error = Error;

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
    type Error = Error;

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