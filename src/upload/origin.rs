use async_trait::async_trait;
use axum::extract::{FromRequest, RequestParts};
use std::net::IpAddr;

use crate::error::Error;
use hyper::Body;

#[derive(Copy, Clone, Debug)]
pub struct RealIp(bool);

impl RealIp {
    pub fn new(behind_proxy: bool) -> Self {
        Self(behind_proxy)
    }

    pub fn resolve(&self, real: IpAddr, forwarded: Option<IpAddr>) -> Option<IpAddr> {
        if self.0 {
            forwarded
        } else {
            Some(real)
        }
    }
}

pub struct ForwardedForHeader(pub IpAddr);

#[async_trait]
impl FromRequest<Body> for ForwardedForHeader {
    type Rejection = Error;

    async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
        Ok(Self(
            req.headers()
                .get("X-Forwarded-For")
                .ok_or_else(|| Error::Origin)?
                .to_str()
                .map_err(|_| Error::Origin)?
                .parse::<IpAddr>()
                .map_err(|_| Error::Origin)?,
        ))
    }
}

pub struct DomainUri(pub String);

#[async_trait]
impl FromRequest<Body> for DomainUri {
    type Rejection = Error;

    async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
        let protocol = if let Some(header) = req.headers().get("X-Forwarded-Proto") {
            header.to_str().map_err(|_| Error::Target)?
        } else {
            "http"
        };
        let host = if let Some(header) = req.headers().get("X-Forwarded-Host") {
            header.to_str().map_err(|_| Error::Target)?
        } else if let Some(header) = req.headers().get("Host") {
            header.to_str().map_err(|_| Error::Target)?
        } else {
            return Err(Error::Target);
        };
        Ok(DomainUri(format!("{}://{}", protocol, host)))
    }
}
