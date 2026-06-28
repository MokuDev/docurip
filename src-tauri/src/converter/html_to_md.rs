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
        let md = strip_ui_boilerplate(&md);
        let md = dedup_markdown(&md);
        strip_trailing_heading_stubs(&md)
    }
}

fn strip_ui_boilerplate(md: &str) -> String {
    use regex::Regex;
    let re = Regex::new(r"(?i)(Copy\s*page\s*(?:Open\s*markdown\s*)?(?:Edit\s*page)?|Edit\s*page|Open\s*markdown)")
        .expect("static regex");
    let lines: Vec<&str> = md.lines().collect();
    let mut result = Vec::new();
    for line in &lines {
        let trimmed = line.trim();
        if !trimmed.is_empty() && re.replace_all(trimmed, "").trim().is_empty() {
            continue;
        }
        result.push(*line);
    }
    result.join("\n")
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
        let all_headings = trimmed.lines().all(|l| {
            let lt = l.trim();
            lt.is_empty() || is_heading(lt)
        });
        if all_headings {
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

fn is_toc_section(section: &str) -> bool {
    let lines: Vec<&str> = section.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() < 3 {
        return false;
    }
    let anchor_links = lines.iter().filter(|l| {
        let t = l.trim().trim_start_matches("* ").trim_start_matches("- ");
        t.starts_with('[') && t.contains("](#")
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
    fn test_strips_trailing_heading_stubs() {
        let md = "# Title\n\nSome real content.\n\nMore content.\n\nEven more.\n\n## Section A\n==========\n\n## Section B\n----------\n\n## Section C\n----------";
        let result = strip_trailing_heading_stubs(md);
        assert!(result.contains("Even more."), "got: {}", result);
        assert!(!result.contains("Section C"), "heading stubs not stripped: {}", result);
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
}
