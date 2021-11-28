use std::borrow::Cow;

use hyper::{Body, header, Request, Response, StatusCode};
use routerify::ext::RequestExt;

use crate::{Access, AssetsError, Authenticator, AuthError, Error};

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

    async fn html(&self) -> (Cow<'static, [u8]>, &str) {
        (Cow::from(Self::load_file("index.html").await), "text/html")
    }

    async fn css(&self) -> (Cow<'static, [u8]>, &str) {
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

    async fn js(&self) -> (Cow<'static, [u8]>, &str) {
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
            "/" | "/index.html" => Some(self.html().await),
            "/style.css" => Some(self.css().await),
            "/app.js" => Some(self.js().await),
            "/icon.png" => Some(self.icon().await),
            _ => None,
        }
    }
}

pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Error> {
    let auth = req.data::<Authenticator>().ok_or(AuthError::AuthProcess)?;
    if let Some(resp) = auth.allows(&req, Access::WEB_UI) {
        return Ok(resp);
    }

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
