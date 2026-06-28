use scraper::{Html, Selector};
use std::collections::HashMap;
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
        let selector = Selector::parse("a[href]").expect("static selector 'a[href]' is valid CSS");
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

    pub fn auto_extract_content(&self, html: &str) -> Option<String> {
        let candidates = [
            "main",
            "article",
            "[role=\"main\"]",
            "#content",
            ".content",
            ".main-content",
            ".post-content",
            ".article-content",
            ".page-content",
            ".docs-content",
        ];
        for sel_str in candidates {
            if let Ok(selector) = Selector::parse(sel_str) {
                let document = Html::parse_document(html);
                if let Some(el) = document.select(&selector).next() {
                    let inner = el.inner_html();
                    if inner.trim().len() > 100 {
                        return Some(inner);
                    }
                }
            }
        }
        None
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

    pub fn rewrite_asset_urls(
        &self,
        html: &str,
        base_url: &Url,
        asset_map: &HashMap<String, String>,
    ) -> String {
        let document = Html::parse_fragment(html);
        let mut replacements: Vec<(String, String, String)> = Vec::new();

        let selectors = [
            ("img[src]", "src"),
            ("link[rel='stylesheet'][href]", "href"),
            ("script[src]", "src"),
        ];

        for (sel_str, attr) in selectors {
            if let Ok(sel) = Selector::parse(sel_str) {
                for el in document.select(&sel) {
                    if let Some(original_attr) = el.value().attr(attr) {
                        if let Ok(abs_url) = base_url.join(original_attr) {
                            let abs_str = abs_url.to_string();
                            if let Some(rel_path) = asset_map.get(&abs_str) {
                                replacements.push((
                                    attr.to_string(),
                                    original_attr.to_string(),
                                    rel_path.clone(),
                                ));
                            }
                        }
                    }
                }
            }
        }

        let mut result = html.to_string();
        for (attr, original_val, rel_path) in replacements {
            let double = format!(r#"{}="{}""#, attr, original_val);
            let single = format!(r#"{}='{}'"#, attr, original_val);
            if result.contains(&double) {
                result = result.replace(&double, &format!(r#"{}="{}""#, attr, rel_path));
            } else if result.contains(&single) {
                result = result.replace(&single, &format!(r#"{}="{}""#, attr, rel_path));
            }
        }
        result
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

    #[test]
    fn test_rewrite_asset_urls_img() {
        let parser = DomParser::new();
        let base = Url::parse("https://example.com/").unwrap();
        let mut map = HashMap::new();
        map.insert(
            "https://example.com/images/logo.png".to_string(),
            "assets/example.com/images/logo.png".to_string(),
        );
        let html = r#"<img src="/images/logo.png" alt="Logo">"#;
        let result = parser.rewrite_asset_urls(html, &base, &map);
        assert!(
            result.contains(r#"src="assets/example.com/images/logo.png""#),
            "Expected rewritten img src, got: {}",
            result
        );
    }

    #[test]
    fn test_rewrite_asset_urls_link() {
        let parser = DomParser::new();
        let base = Url::parse("https://example.com/").unwrap();
        let mut map = HashMap::new();
        map.insert(
            "https://example.com/css/style.css".to_string(),
            "assets/example.com/css/style.css".to_string(),
        );
        let html = r#"<link rel="stylesheet" href="/css/style.css">"#;
        let result = parser.rewrite_asset_urls(html, &base, &map);
        assert!(
            result.contains(r#"href="assets/example.com/css/style.css""#),
            "Expected rewritten link href, got: {}",
            result
        );
    }

    #[test]
    fn test_rewrite_asset_urls_script() {
        let parser = DomParser::new();
        let base = Url::parse("https://example.com/").unwrap();
        let mut map = HashMap::new();
        map.insert(
            "https://example.com/js/app.js".to_string(),
            "assets/example.com/js/app.js".to_string(),
        );
        let html = r#"<script src="/js/app.js"></script>"#;
        let result = parser.rewrite_asset_urls(html, &base, &map);
        assert!(
            result.contains(r#"src="assets/example.com/js/app.js""#),
            "Expected rewritten script src, got: {}",
            result
        );
    }

    #[test]
    fn test_rewrite_asset_urls_unknown_untouched() {
        let parser = DomParser::new();
        let base = Url::parse("https://example.com/").unwrap();
        let map = HashMap::new();
        let html = r#"<img src="/images/unknown.png" alt="Logo">"#;
        let result = parser.rewrite_asset_urls(html, &base, &map);
        assert!(
            result.contains(r#"src="/images/unknown.png""#),
            "Expected unchanged src, got: {}",
            result
        );
    }

    #[test]
    fn test_rewrite_asset_urls_no_match() {
        let parser = DomParser::new();
        let base = Url::parse("https://example.com/").unwrap();
        let map = HashMap::new();
        let html = "<p>Hello</p>";
        let result = parser.rewrite_asset_urls(html, &base, &map);
        assert_eq!(result, html);
    }
}
