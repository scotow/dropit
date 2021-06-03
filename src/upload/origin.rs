use std::net::IpAddr;

use hyper::{Body, Request};
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