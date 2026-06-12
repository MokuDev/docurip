pub struct HtmlToMarkdown;

impl Default for HtmlToMarkdown {
    fn default() -> Self {
        Self::new()
    }
}

impl HtmlToMarkdown {
    pub fn new() -> Self {
        Self
    }

    pub fn convert(&self, html: &str) -> String {
        html2md::parse_html(html)
    }
}
