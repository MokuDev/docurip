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
}
