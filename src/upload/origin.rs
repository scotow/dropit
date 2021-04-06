use hyper::{Request, Body, HeaderMap};
use hyper::http::HeaderValue;
use std::net::IpAddr;
use routerify::ext::RequestExt;

pub fn real_ip(req: &Request<Body>) -> Result<IpAddr, ()> {
    Ok(
        if let Some(header) = req.headers().get("X-Forwarded-For") {
            header.to_str().map_err(|_| ())?.parse::<IpAddr>().map_err(|_| ())?
        } else {
            req.remote_addr().ip()
        }
    )
}

fn target_protocol(headers: &HeaderMap<HeaderValue>) -> Result<String, ()> {
    Ok(
        if let Some(header) = headers.get("X-Forwarded-Proto") {
            header.to_str().map(|s| s.to_owned()).map_err(|_| ())?
        } else {
            "http".to_owned()
        }
    )
}

fn target_host(headers: &HeaderMap<HeaderValue>) -> Result<String, ()> {
    if let Some(header) = headers.get("X-Forwarded-Host") {
        header.to_str().map(|s| s.to_owned()).map_err(|_| ())
    } else if let Some(header) = headers.get("Host") {
        header.to_str().map(|s| s.to_owned()).map_err(|_| ())
    } else {
        Err(())
    }
}

pub fn upload_base(headers: &HeaderMap<HeaderValue>) -> Result<String, ()> {
    Ok(format!("{}://{}", target_protocol(headers)?, target_host(headers)?))
}