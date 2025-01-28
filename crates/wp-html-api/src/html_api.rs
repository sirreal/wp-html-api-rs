pub struct HtmlProcessor {
    html: String,
}

impl HtmlProcessor {
    pub fn create_fragment(html: &str) -> Self {
        Self {
            html: html.to_string(),
        }
    }
}
