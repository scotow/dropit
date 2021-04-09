use hyper::{Request, Body, HeaderMap};
use hyper::http::HeaderValue;
use std::net::IpAddr;
use routerify::ext::RequestExt;

pub fn real_ip(req: &Request<Body>) -> Option<IpAddr> {
    if let Some(header) = req.headers().get("X-Forwarded-For") {
        header.to_str().ok()?.parse::<IpAddr>().ok()
    } else {
        Some(req.remote_addr().ip())
    }
}

fn target_protocol(headers: &HeaderMap<HeaderValue>) -> Option<String> {
    if let Some(header) = headers.get("X-Forwarded-Proto") {
        header.to_str().map(|s| s.to_owned()).ok()
    } else {
        Some("http".to_owned())
    }
}

fn target_host(headers: &HeaderMap<HeaderValue>) -> Option<String> {
    if let Some(header) = headers.get("X-Forwarded-Host") {
        header.to_str().map(|s| s.to_owned()).ok()
    } else if let Some(header) = headers.get("Host") {
        header.to_str().map(|s| s.to_owned()).ok()
    } else {
        None
    }
}

pub fn upload_base(headers: &HeaderMap<HeaderValue>) -> Option<String> {
    Some(format!("{}://{}", target_protocol(headers)?, target_host(headers)?))
}