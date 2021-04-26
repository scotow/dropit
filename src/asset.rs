pub struct Assets {
    html: &'static str,
    css: &'static str,
    js: &'static str,
}

impl Assets {
    pub fn new(color: &str) -> Self {
        Self {
            html: include_str!("public/index.html"),
            css: Box::leak(
                include_str!("public/style.css")
                    .replace("TEMPLATE_COLOR", color)
                    .into_boxed_str()
            ),
            js: Box::leak(
                include_str!("public/app.js")
                    .replace("TEMPLATE_COLOR", color)
                    .into_boxed_str()
            ),
        }
    }

    fn html(&self) -> (&'static str, &str) {
        (self.html, "text/html")
    }

    fn css(&self) -> (&'static str, &str) {
        (self.css, "text/css")
    }

    fn js(&self) -> (&'static str, &str) {
        (self.js, "application/javascript")
    }

    pub fn asset_for_path(&self, path: &str) -> Option<(&'static str, &str)> {
        match path {
            "/" | "/index.html" => Some(self.html()),
            "/style.css" => Some(self.css()),
            "/app.js" => Some(self.js()),
            _ => None,
        }
    }
}

