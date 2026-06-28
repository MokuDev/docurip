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
        let md = html2md::parse_html(html);
        dedup_markdown(&md)
    }
}

fn dedup_markdown(md: &str) -> String {
    let sections: Vec<&str> = md.split("\n\n").collect();
    if sections.len() < 6 {
        return md.to_string();
    }

    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for section in &sections {
        let trimmed = section.trim();
        if trimmed.is_empty() {
            result.push(*section);
            continue;
        }
        // Only dedup substantial blocks (> 80 chars) to avoid removing
        // short repeated elements like separators or short paragraphs
        if trimmed.len() > 80 {
            if !seen.insert(trimmed) {
                continue;
            }
        }
        result.push(*section);
    }

    result.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_heading() {
        let converter = HtmlToMarkdown::new();
        let md = converter.convert("<h1>Title</h1>");
        assert!(md.contains("Title"), "Expected heading text, got: {}", md);
        assert!(
            md.contains("==========") || md.contains("# Title"),
            "Expected heading marker, got: {}",
            md
        );
    }

    #[test]
    fn test_convert_paragraph() {
        let converter = HtmlToMarkdown::new();
        let md = converter.convert("<p>Hello world</p>");
        assert!(md.contains("Hello world"), "Expected paragraph text, got: {}", md);
    }

    #[test]
    fn test_convert_link() {
        let converter = HtmlToMarkdown::new();
        let md = converter.convert(r#"<a href="https://example.com">Link</a>"#);
        assert!(
            md.contains("[Link](https://example.com)"),
            "Expected markdown link, got: {}",
            md
        );
    }

    #[test]
    fn test_convert_list() {
        let converter = HtmlToMarkdown::new();
        let html = "<ul><li>Item 1</li><li>Item 2</li></ul>";
        let md = converter.convert(html);
        assert!(md.contains("Item 1"), "Expected list item, got: {}", md);
        assert!(md.contains("Item 2"), "Expected list item, got: {}", md);
        assert!(
            md.contains("* Item") || md.contains("- Item"),
            "Expected list marker, got: {}",
            md
        );
    }

    #[test]
    fn test_convert_code_block() {
        let converter = HtmlToMarkdown::new();
        let html = "<pre><code>fn main() {}</code></pre>";
        let md = converter.convert(html);
        assert!(md.contains("```"), "Expected code fence, got: {}", md);
        assert!(md.contains("fn main()"), "Expected code content, got: {}", md);
    }

    #[test]
    fn test_convert_empty() {
        let converter = HtmlToMarkdown::new();
        let md = converter.convert("");
        assert_eq!(md.trim(), "");
    }

    #[test]
    fn test_convert_complex_html() {
        let converter = HtmlToMarkdown::new();
        let html = r#"<h1>Guide</h1>
<p>Welcome to the <a href="/start">getting started</a> guide.</p>
<ul>
<li>Step 1</li>
<li>Step 2</li>
</ul>"#;
        let md = converter.convert(html);
        assert!(md.contains("Guide"), "Expected heading text, got: {}", md);
        assert!(
            md.contains("[getting started](/start)"),
            "Expected link, got: {}",
            md
        );
        assert!(md.contains("Step 1"), "Expected list item, got: {}", md);
        assert!(md.contains("Step 2"), "Expected list item, got: {}", md);
    }
}
