use hyper::{Body, HeaderMap, Response, StatusCode};
use hyper::header::HeaderValue;

#[macro_export]
macro_rules! exit_error {
    ($($arg:tt)+) => {
        {
            log::error!($($arg)+);
            std::process::exit(1)
        }
    }
}

pub fn generic_500() -> Response<Body> {
    let mut resp = Response::new(Body::empty());
    *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    resp
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