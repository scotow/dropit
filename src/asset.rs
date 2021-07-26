use std::borrow::Cow;

#[cfg(debug_assertions)]
pub struct Assets {
    color: String,
}

#[cfg(debug_assertions)]
impl Assets {
    pub fn new(color: String) -> Self {
        Self {
            color
        }
    }

    async fn load_file(file: &str) -> String {
        tokio::fs::read_to_string(format!("src/public/{}", file)).await.unwrap()
    }

    async fn html(&self) -> (Cow<'static, str>, &str) {
        (Cow::from(Self::load_file("index.html").await), "text/html")
    }

    async fn css(&self) -> (Cow<'static, str>, &str) {
        (
            Cow::from(Self::load_file("style.css").await.replace("TEMPLATE_COLOR", &self.color)),
            "text/css"
        )
    }

    async fn js(&self) -> (Cow<'static, str>, &str) {
        (
            Cow::from(Self::load_file("app.js").await.replace("TEMPLATE_COLOR", &self.color)),
            "application/javascript"
        )
    }
}

#[cfg(not(debug_assertions))]
pub struct Assets {
    html: Cow<'static, str>,
    css: Cow<'static, str>,
    js: Cow<'static, str>,
}

#[cfg(not(debug_assertions))]
impl Assets {
    pub fn new(color: String) -> Self {
        Self {
            html: Cow::from(include_str!("public/index.html")),
            css: Cow::from(
                include_str!("public/style.css")
                    .replace("TEMPLATE_COLOR", &color)
            ),
            js: Cow::from(
                include_str!("public/app.js")
                    .replace("TEMPLATE_COLOR", &color)
            ),
        }
    }

    async fn html(&self) -> (Cow<'static, str>, &str) {
        (self.html.clone(), "text/html")
    }

    async fn css(&self) -> (Cow<'static, str>, &str) {
        (self.css.clone(), "text/css")
    }

    async fn js(&self) -> (Cow<'static, str>, &str) {
        (self.js.clone(), "application/javascript")
    }
}

impl Assets {
    pub async fn asset_for_path(&self, path: &str) -> Option<(Cow<'static, str>, &str)> {
        match path {
            "/" | "/index.html" => Some(self.html().await),
            "/style.css" => Some(self.css().await),
            "/app.js" => Some(self.js().await),
            _ => None,
        }
    }
}

