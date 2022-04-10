use crate::Error;
use hyper::{header, Body, Request, Response, StatusCode};
use routerify::ext::RequestExt;

pub struct Theme(String);

impl Theme {
    pub fn new(color: &str) -> Self {
        Self(format!(":root {{\n    --theme: {};\n}}", color))
    }
}

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Error> {
    let theme = req.data::<Theme>().ok_or(Error::AssetsCatalogue)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/css")
        .body(Body::from(theme.0.clone()))?)
}
