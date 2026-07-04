use regex::Regex;

fn strip_script_and_style(html: &str) -> String {
    let script_re = Regex::new(r"(?is)<script[^>]*>.*?</script>").expect("static regex");
    let style_re = Regex::new(r"(?is)<style[^>]*>.*?</style>").expect("static regex");
    let html = script_re.replace_all(html, "");
    let html = style_re.replace_all(&html, "");
    html.to_string()
}

fn strip_empty_links(html: &str) -> String {
    let re = Regex::new(r#"(?i)<a[^>]*href="[^"]*"[^>]*>\s*</a>"#).expect("static regex");
    re.replace_all(html, "").to_string()
}

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
        let html = strip_script_and_style(html);
        let html = strip_empty_links(&html);
        let md = html2md::parse_html(&html);
        let md = strip_ui_boilerplate(&md);
        let md = dedup_markdown(&md);
        let md = strip_trailing_heading_stubs(&md);
        let md = collapse_blank_lines(&md);
        let md = remove_empty_links(&md);
        remove_broken_images(&md)
    }
}

fn strip_ui_boilerplate(md: &str) -> String {
    use regex::Regex;
    let boilerplate_patterns = [
        r"(?i)(Copy\s*page\s*(?:Open\s*markdown\s*)?(?:Edit\s*page)?|Edit\s*page|Open\s*markdown)",
        r"(?i)^.*we\s+use\s+cookies.*$",
        r"(?i)Subscribe\s+to\s+our\s+newsletter",
        r"(?i)Copy\s+code",
    ];
    
    let mut result = md.to_string();
    
    for pattern in &boilerplate_patterns {
        let re = Regex::new(pattern).expect("static regex");
        let lines: Vec<&str> = result.lines().collect();
        let mut filtered = Vec::new();
        for line in &lines {
            let trimmed = line.trim();
            if !trimmed.is_empty() && re.replace_all(trimmed, "").trim().is_empty() {
                continue;
            }
            filtered.push(*line);
        }
        result = filtered.join("\n");
    }
    
    result
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
        if is_toc_section(trimmed) {
            continue;
        }
        if trimmed.len() > 80 {
            if !seen.insert(trimmed) {
                continue;
            }
        }
        result.push(*section);
    }

    result.join("\n\n")
}

fn is_heading(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('#')
        || t.chars().all(|c| c == '=')
        || t.chars().all(|c| c == '-')
}

fn is_stub_section(section: &str) -> bool {
    let trimmed = section.trim();
    let non_empty: Vec<&str> = trimmed.lines().filter(|l| !l.trim().is_empty()).collect();
    if non_empty.is_empty() {
        return true;
    }
    // ATX heading: "## Foo"
    if non_empty.len() == 1 && is_heading(non_empty[0]) {
        return true;
    }
    // Setext heading: "Title\n=========="
    if non_empty.len() == 2 {
        let second = non_empty[1].trim();
        if (second.chars().all(|c| c == '=') || second.chars().all(|c| c == '-')) && !second.is_empty() {
            return true;
        }
    }
    // Short fragment with no sentence structure (e.g. "💡Tip")
    non_empty.len() == 1 && non_empty[0].trim().len() < 30 && !non_empty[0].contains('.')
}

fn strip_trailing_heading_stubs(md: &str) -> String {
    let sections: Vec<&str> = md.split("\n\n").collect();
    if sections.len() < 4 {
        return md.to_string();
    }

    let mut last_content_idx = sections.len();
    for i in (0..sections.len()).rev() {
        let trimmed = sections[i].trim();
        if trimmed.is_empty() {
            continue;
        }
        if is_stub_section(trimmed) {
            last_content_idx = i;
        } else {
            break;
        }
    }

    if last_content_idx < sections.len() && last_content_idx > 0 {
        sections[..last_content_idx].join("\n\n").trim_end().to_string()
    } else {
        md.to_string()
    }
}

fn collapse_blank_lines(md: &str) -> String {
    let re = Regex::new(r"\n{3,}").expect("static regex");
    re.replace_all(md, "\n\n").to_string()
}

fn remove_empty_links(md: &str) -> String {
    let re = Regex::new(r"\[([^\]]+)\]\(\s*\)").expect("static regex");
    re.replace_all(md, "$1").to_string()
}

fn remove_broken_images(md: &str) -> String {
    let re = Regex::new(r"!\[([^\]]*)\]\(\s*\)").expect("static regex");
    re.replace_all(md, "").to_string()
}

fn is_toc_section(section: &str) -> bool {
    let lines: Vec<&str> = section.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() < 3 {
        return false;
    }
    let anchor_links = lines.iter().filter(|l| {
        let t = l.trim().trim_start_matches("* ").trim_start_matches("- ");
        t.starts_with('[') && t.contains('#') && t.ends_with(')')
    }).count();
    anchor_links * 2 >= lines.len()
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

    #[test]
    fn test_strips_copy_page_boilerplate() {
        let input = "# Title\n\nSome content here.\n\nCopy pageOpen markdownEdit page\n\nCopy page";
        let result = strip_ui_boilerplate(input);
        assert!(!result.contains("Copy page"), "got: {}", result);
        assert!(!result.contains("Edit page"), "got: {}", result);
        assert!(result.contains("# Title"));
        assert!(result.contains("Some content here."));
    }

    #[test]
    fn test_strips_toc_section() {
        let md = "# Title\n\nParagraph one.\n\nParagraph two.\n\nParagraph three.\n\nParagraph four.\n\nParagraph five.\n\n* [Section A](#section-a)\n* [Section B](#section-b)\n* [Section C](#section-c)";
        let result = dedup_markdown(md);
        assert!(!result.contains("[Section A](#section-a)"), "TOC not removed: {}", result);
        assert!(result.contains("Paragraph one."));
    }

    #[test]
    fn test_strips_toc_with_full_path_links() {
        let md = "# Title\n\nParagraph one.\n\nParagraph two.\n\nParagraph three.\n\nParagraph four.\n\nParagraph five.\n\n* [Where to Use Kilo](/docs/getting-started#where-to-use-kilo)\n* [What Kilo Can Do](/docs/getting-started#what-kilo-can-do)\n* [Quick Start](/docs/getting-started#quick-start)";
        let result = dedup_markdown(md);
        assert!(!result.contains("where-to-use-kilo"), "TOC with full paths not removed: {}", result);
        assert!(result.contains("Paragraph one."));
    }

    #[test]
    fn test_strips_trailing_heading_stubs() {
        let md = "# Title\n\nSome real content.\n\nMore content.\n\nEven more.\n\n## Section A\n==========\n\n## Section B\n----------\n\n## Section C\n----------";
        let result = strip_trailing_heading_stubs(md);
        assert!(result.contains("Even more."), "got: {}", result);
        assert!(!result.contains("Section C"), "heading stubs not stripped: {}", result);
    }

    #[test]
    fn test_strips_trailing_stubs_with_short_fragments() {
        let md = "# Title\n\nSome real content.\n\nMore content.\n\nEven more.\n\nIntro\n==========\n\nSection A\n----------\n\n💡Tip\n\nSection B\n----------";
        let result = strip_trailing_heading_stubs(md);
        assert!(result.contains("Even more."), "got: {}", result);
        assert!(!result.contains("💡Tip"), "short fragment not stripped: {}", result);
        assert!(!result.contains("Section B"), "heading stubs not stripped: {}", result);
    }

    #[test]
    fn test_preserves_real_heading_with_content() {
        let md = "# Title\n\nIntro paragraph.\n\n## Details\n\nDetail content here.\n\n## More\n\nMore content.";
        let result = strip_trailing_heading_stubs(md);
        assert!(result.contains("## Details"), "got: {}", result);
        assert!(result.contains("## More"), "got: {}", result);
        assert!(result.contains("More content."), "got: {}", result);
    }

    #[test]
    fn test_preserves_normal_list() {
        let md = "# Title\n\nParagraph.\n\nSecond.\n\nThird.\n\nFourth.\n\nFifth.\n\nSixth.\n\n* Item one\n* Item two\n* Item three";
        let result = dedup_markdown(md);
        assert!(result.contains("Item one"), "normal list removed: {}", result);
    }

    #[test]
    fn test_strips_residual_html_tags() {
        let converter = HtmlToMarkdown::new();
        let html = r#"<div class="content"><section><p>Real content</p></section></div>"#;
        let md = converter.convert(html);
        assert!(!md.contains("<div"), "div tag not stripped: {}", md);
        assert!(!md.contains("<section"), "section tag not stripped: {}", md);
        assert!(!md.contains("</div>"), "closing div not stripped: {}", md);
        assert!(md.contains("Real content"), "content lost: {}", md);
    }

    #[test]
    fn test_strips_html_comments() {
        let converter = HtmlToMarkdown::new();
        let html = r#"<p>Visible content</p><!-- This is a comment --><p>More content</p>"#;
        let md = converter.convert(html);
        assert!(!md.contains("<!--"), "comment not stripped: {}", md);
        assert!(!md.contains("This is a comment"), "comment text not stripped: {}", md);
        assert!(md.contains("Visible content"), "content lost: {}", md);
    }

    #[test]
    fn test_strips_span_tags() {
        let converter = HtmlToMarkdown::new();
        let html = r#"<p>Text with <span class="highlight">highlighted</span> content</p>"#;
        let md = converter.convert(html);
        assert!(!md.contains("<span"), "span tag not stripped: {}", md);
        assert!(md.contains("highlighted"), "span content lost: {}", md);
    }

    #[test]
    fn test_strips_script_content() {
        let converter = HtmlToMarkdown::new();
        let html = r#"<p>Real content</p><script>var x = 1; alert(x);</script><p>More content</p>"#;
        let md = converter.convert(html);
        assert!(!md.contains("alert"), "script content leaked: {}", md);
        assert!(!md.contains("<script"), "script tag leaked: {}", md);
        assert!(md.contains("Real content"), "content lost: {}", md);
    }

    #[test]
    fn test_strips_style_content() {
        let converter = HtmlToMarkdown::new();
        let html = r#"<p>Real content</p><style>.foo { color: red; }</style><p>More content</p>"#;
        let md = converter.convert(html);
        assert!(!md.contains("color: red"), "style content leaked: {}", md);
        assert!(!md.contains("<style"), "style tag leaked: {}", md);
        assert!(md.contains("Real content"), "content lost: {}", md);
    }

    #[test]
    fn test_no_excessive_blank_lines() {
        let converter = HtmlToMarkdown::new();
        let html = r#"<div><div><div><p>Content</p></div></div></div>"#;
        let md = converter.convert(html);
        let consecutive_blank_lines = md.contains("\n\n\n");
        assert!(!consecutive_blank_lines, "excessive blank lines in: {:?}", md);
    }

    #[test]
    fn test_strips_empty_links() {
        let converter = HtmlToMarkdown::new();
        let html = r##"<p>Text</p><a href="#"></a><p>More text</p>"##;
        let md = converter.convert(html);
        assert!(!md.contains("[](#)"), "empty link not stripped: {}", md);
    }

    #[test]
    fn test_strips_nav_footer_header_boilerplate() {
        let converter = HtmlToMarkdown::new();
        let html = r#"<nav><a href="/">Home</a><a href="/docs">Docs</a></nav><article><p>Real content here</p></article><footer><p>Copyright 2024</p></footer>"#;
        let md = converter.convert(html);
        assert!(md.contains("Real content"), "article content lost: {}", md);
    }

    #[test]
    fn test_strips_cookie_banner() {
        let input = "# Title\n\nSome content here.\n\nWe use cookies to improve your experience.\n\nMore content.";
        let result = strip_ui_boilerplate(input);
        assert!(!result.contains("We use cookies"), "cookie banner not stripped: {}", result);
        assert!(result.contains("Some content here."), "content lost: {}", result);
    }

    #[test]
    fn test_strips_newsletter_signup() {
        let input = "# Title\n\nSome content here.\n\nSubscribe to our newsletter\n\nMore content.";
        let result = strip_ui_boilerplate(input);
        assert!(!result.contains("Subscribe to our newsletter"), "newsletter not stripped: {}", result);
        assert!(result.contains("Some content here."), "content lost: {}", result);
    }

    #[test]
    fn test_strips_copy_code_button() {
        let input = "# Title\n\nSome content here.\n\nCopy code\n\nMore content.";
        let result = strip_ui_boilerplate(input);
        assert!(!result.contains("Copy code"), "copy code button not stripped: {}", result);
        assert!(result.contains("Some content here."), "content lost: {}", result);
    }

    #[test]
    fn test_collapses_multiple_blank_lines() {
        let input = "# Title\n\n\n\n\nSome content\n\n\n\n\nMore content";
        let result = collapse_blank_lines(input);
        assert!(!result.contains("\n\n\n"), "multiple blank lines not collapsed: {}", result);
        assert!(result.contains("Some content"), "content lost: {}", result);
    }

    #[test]
    fn test_removes_empty_links() {
        let input = "# Title\n\n[Click here]()\n\nReal content with [valid link](https://example.com)";
        let result = remove_empty_links(input);
        assert!(!result.contains("[Click here]()"), "empty link not removed: {}", result);
        assert!(result.contains("[valid link](https://example.com)"), "valid link removed: {}", result);
    }

    #[test]
    fn test_removes_broken_images() {
        let input = "# Title\n\n![Alt text]()\n\n![Valid image](https://example.com/image.png)";
        let result = remove_broken_images(input);
        assert!(!result.contains("![Alt text]()"), "broken image not removed: {}", result);
        assert!(result.contains("![Valid image](https://example.com/image.png)"), "valid image removed: {}", result);
    }
}
