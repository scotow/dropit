use hyper::{Response, Body, StatusCode};

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