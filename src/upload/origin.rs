use hyper::{Request, Body, HeaderMap};
use hyper::http::HeaderValue;
use std::net::IpAddr;
use routerify::ext::RequestExt;

pub struct RealIp(bool);

impl RealIp {
    pub fn new(behind_proxy: bool) -> Self {
        Self(behind_proxy)
    }

    pub fn find(&self, req: &Request<Body>) -> Option<IpAddr> {
        if self.0 {
            req.headers().get("X-Forwarded-For")?.to_str().ok()?.parse::<IpAddr>().ok()
        } else {
            Some(req.remote_addr().ip())
        }
    }
}

fn protocol(headers: &HeaderMap<HeaderValue>) -> Option<String> {
    if let Some(header) = headers.get("X-Forwarded-Proto") {
        header.to_str().map(|s| s.to_owned()).ok()
    } else {
        Some("http".to_owned())
    }
}

fn host(headers: &HeaderMap<HeaderValue>) -> Option<String> {
    if let Some(header) = headers.get("X-Forwarded-Host") {
        header.to_str().map(|s| s.to_owned()).ok()
    } else if let Some(header) = headers.get("Host") {
        header.to_str().map(|s| s.to_owned()).ok()
    } else {
        None
    }
}

pub fn upload_base(headers: &HeaderMap<HeaderValue>) -> Option<String> {
    Some(format!("{}://{}", protocol(headers)?, host(headers)?))
}