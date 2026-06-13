use scraper::{Html, Selector};
use url::Url;

pub struct DomParser;

impl Default for DomParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DomParser {
    pub fn new() -> Self {
        Self
    }

    pub fn extract_title(&self, html: &str) -> Option<String> {
        let document = Html::parse_document(html);
        let selector = Selector::parse("title").ok()?;
        document
            .select(&selector)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
    }

    pub fn extract_links(&self, html: &str, base_url: &Url) -> Vec<String> {
        let document = Html::parse_document(html);
        let selector = Selector::parse("a[href]").unwrap();
        document
            .select(&selector)
            .filter_map(|e| e.value().attr("href"))
            .filter_map(|href| base_url.join(href).ok())
            .map(|u| u.to_string())
            .collect()
    }

    pub fn extract_assets(&self, html: &str, base_url: &Url) -> Vec<String> {
        let document = Html::parse_document(html);
        let mut assets = Vec::new();

        if let Ok(sel) = Selector::parse("img[src]") {
            assets.extend(
                document
                    .select(&sel)
                    .filter_map(|e| e.value().attr("src"))
                    .filter_map(|src| base_url.join(src).ok())
                    .map(|u| u.to_string()),
            );
        }
        if let Ok(sel) = Selector::parse("link[rel='stylesheet'][href]") {
            assets.extend(
                document
                    .select(&sel)
                    .filter_map(|e| e.value().attr("href"))
                    .filter_map(|href| base_url.join(href).ok())
                    .map(|u| u.to_string()),
            );
        }
        if let Ok(sel) = Selector::parse("script[src]") {
            assets.extend(
                document
                    .select(&sel)
                    .filter_map(|e| e.value().attr("src"))
                    .filter_map(|src| base_url.join(src).ok())
                    .map(|u| u.to_string()),
            );
        }

        assets
    }

    pub fn extract_content(&self, html: &str, selectors: &[String]) -> Option<String> {
        let document = Html::parse_document(html);
        let mut parts = Vec::new();
        for sel_str in selectors {
            if let Ok(selector) = Selector::parse(sel_str) {
                for el in document.select(&selector) {
                    parts.push(el.inner_html());
                }
            }
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_HTML: &str = r#"<!DOCTYPE html>
<html>
<head><title>Test Page</title></head>
<body>
    <main id="content">
        <h1>Hello World</h1>
        <p>This is a test paragraph.</p>
        <a href="/about">About</a>
        <a href="https://external.com">External</a>
        <a href="page2.html">Page 2</a>
    </main>
    <aside>
        <img src="/images/logo.png" alt="Logo">
        <link rel="stylesheet" href="/css/style.css">
        <script src="/js/app.js"></script>
    </aside>
</body>
</html>"#;

    #[test]
    fn test_extract_title() {
        let parser = DomParser::new();
        assert_eq!(
            parser.extract_title(TEST_HTML),
            Some("Test Page".to_string())
        );
    }

    #[test]
    fn test_extract_title_missing() {
        let parser = DomParser::new();
        assert_eq!(parser.extract_title("<html><body></body></html>"), None);
    }

    #[test]
    fn test_extract_links() {
        let parser = DomParser::new();
        let base = Url::parse("https://example.com/docs/").unwrap();
        let links = parser.extract_links(TEST_HTML, &base);

        assert!(links.contains(&"https://example.com/about".to_string()));
        assert!(links.contains(&"https://external.com/".to_string()));
        assert!(links.contains(&"https://example.com/docs/page2.html".to_string()));
        assert_eq!(links.len(), 3);
    }

    #[test]
    fn test_extract_links_empty() {
        let parser = DomParser::new();
        let base = Url::parse("https://example.com/").unwrap();
        let links = parser.extract_links("<html><body></body></html>", &base);
        assert!(links.is_empty());
    }

    #[test]
    fn test_extract_assets() {
        let parser = DomParser::new();
        let base = Url::parse("https://example.com/").unwrap();
        let assets = parser.extract_assets(TEST_HTML, &base);

        assert!(assets.contains(&"https://example.com/images/logo.png".to_string()));
        assert!(assets.contains(&"https://example.com/css/style.css".to_string()));
        assert!(assets.contains(&"https://example.com/js/app.js".to_string()));
        assert_eq!(assets.len(), 3);
    }

    #[test]
    fn test_extract_assets_empty() {
        let parser = DomParser::new();
        let base = Url::parse("https://example.com/").unwrap();
        let assets = parser.extract_assets("<html><body></body></html>", &base);
        assert!(assets.is_empty());
    }

    #[test]
    fn test_extract_content_with_selectors() {
        let parser = DomParser::new();
        let selectors = vec!["main#content".to_string()];
        let content = parser.extract_content(TEST_HTML, &selectors);

        assert!(content.is_some());
        let html = content.unwrap();
        assert!(html.contains("Hello World"));
        assert!(!html.contains("<aside>"));
    }

    #[test]
    fn test_extract_content_multiple_selectors() {
        let parser = DomParser::new();
        let selectors = vec!["h1".to_string(), "p".to_string()];
        let content = parser.extract_content(TEST_HTML, &selectors);

        assert!(content.is_some());
        let html = content.unwrap();
        assert!(html.contains("Hello World"));
        assert!(html.contains("test paragraph"));
    }

    #[test]
    fn test_extract_content_no_match() {
        let parser = DomParser::new();
        let selectors = vec!["div.nonexistent".to_string()];
        assert_eq!(parser.extract_content(TEST_HTML, &selectors), None);
    }

    #[test]
    fn test_extract_content_empty_selectors() {
        let parser = DomParser::new();
        let selectors: Vec<String> = vec![];
        assert_eq!(parser.extract_content(TEST_HTML, &selectors), None);
    }
}
