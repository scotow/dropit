use std::borrow::Cow;

use hyper::{header, Body, Request, Response, StatusCode};
use routerify::ext::RequestExt;

use crate::{AssetsError, Error};

#[cfg(debug_assertions)]
pub struct Assets {
    color: String,
}

#[cfg(debug_assertions)]
impl Assets {
    pub fn new(color: String) -> Self {
        Self { color }
    }

    async fn load_file(file: &str) -> Vec<u8> {
        tokio::fs::read(format!("src/public/{}", file))
            .await
            .unwrap()
    }

    async fn load_string(file: &str) -> String {
        tokio::fs::read_to_string(format!("src/public/{}", file))
            .await
            .unwrap()
    }

    async fn home_html(&self) -> (Cow<'static, [u8]>, &str) {
        (Cow::from(Self::load_file("index.html").await), "text/html")
    }

    async fn home_css(&self) -> (Cow<'static, [u8]>, &str) {
        (
            Cow::from(
                Self::load_string("style.css")
                    .await
                    .replace("TEMPLATE_COLOR", &self.color)
                    .into_bytes(),
            ),
            "text/css",
        )
    }

    async fn home_js(&self) -> (Cow<'static, [u8]>, &str) {
        (
            Cow::from(
                Self::load_string("app.js")
                    .await
                    .replace("TEMPLATE_COLOR", &self.color)
                    .into_bytes(),
            ),
            "application/javascript",
        )
    }

    async fn icon(&self) -> (Cow<'static, [u8]>, &str) {
        (Cow::from(Self::load_file("icon.png").await), "image/png")
    }

    async fn login_html(&self) -> (Cow<'static, [u8]>, &str) {
        (
            Cow::from(Self::load_file("login/index.html").await),
            "text/html",
        )
    }

    async fn login_css(&self) -> (Cow<'static, [u8]>, &str) {
        (
            Cow::from(Self::load_file("login/style.css").await),
            "text/css",
        )
    }

    async fn login_js(&self) -> (Cow<'static, [u8]>, &str) {
        (
            Cow::from(Self::load_file("login/app.js").await),
            "application/javascript",
        )
    }
}

#[cfg(not(debug_assertions))]
pub struct Assets {
    html: Cow<'static, [u8]>,
    css: Cow<'static, [u8]>,
    js: Cow<'static, [u8]>,
    icon: Cow<'static, [u8]>,
}

#[cfg(not(debug_assertions))]
impl Assets {
    pub fn new(color: String) -> Self {
        Self {
            html: Cow::from(include_bytes!("public/index.html").as_ref()),
            css: Cow::from(
                include_str!("public/style.css")
                    .replace("TEMPLATE_COLOR", &color)
                    .into_bytes(),
            ),
            js: Cow::from(
                include_str!("public/app.js")
                    .replace("TEMPLATE_COLOR", &color)
                    .into_bytes(),
            ),
            icon: Cow::from(include_bytes!("public/icon.png").as_ref()),
        }
    }

    async fn html(&self) -> (Cow<'static, [u8]>, &str) {
        (self.html.clone(), "text/html")
    }

    async fn css(&self) -> (Cow<'static, [u8]>, &str) {
        (self.css.clone(), "text/css")
    }

    async fn js(&self) -> (Cow<'static, [u8]>, &str) {
        (self.js.clone(), "application/javascript")
    }

    async fn icon(&self) -> (Cow<'static, [u8]>, &str) {
        (self.icon.clone(), "image/png")
    }
}

impl Assets {
    pub async fn asset_for_path(&self, path: &str) -> Option<(Cow<'static, [u8]>, &str)> {
        match path {
            "/" | "/index.html" => Some(self.home_html().await),
            "/style.css" => Some(self.home_css().await),
            "/app.js" => Some(self.home_js().await),
            "/icon.png" => Some(self.icon().await),
            "/login/" | "/login/index.html" => Some(self.login_html().await),
            "/login/style.css" => Some(self.login_css().await),
            "/login/app.js" => Some(self.login_js().await),
            _ => None,
        }
    }
}

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Error> {
    let assets = req.data::<Assets>().ok_or(AssetsError::AssetsCatalogue)?;
    let (content, mime_type) = assets
        .asset_for_path(req.uri().path())
        .await
        .ok_or(AssetsError::AssetNotFound)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_type)
        .body(Body::from(content))?)
}
