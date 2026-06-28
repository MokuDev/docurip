use regex::Regex;
use std::collections::HashMap;

pub struct CleanerConfig {
    pub remove_headers_footers: bool,
    pub remove_page_numbers: bool,
    pub remove_footnotes: bool,
    pub remove_boilerplate: bool,
}

impl Default for CleanerConfig {
    fn default() -> Self {
        Self {
            remove_headers_footers: true,
            remove_page_numbers: true,
            remove_footnotes: true,
            remove_boilerplate: true,
        }
    }
}

pub fn clean_pages(pages: &[String], config: &CleanerConfig) -> Vec<String> {
    let mut result: Vec<String> = pages
        .iter()
        .map(|p| trim_blank_lines(p))
        .collect();

    if config.remove_headers_footers {
        remove_headers_footers(&mut result);
    }
    if config.remove_page_numbers {
        remove_page_numbers(&mut result);
    }
    if config.remove_footnotes {
        remove_footnotes(&mut result);
    }
    if config.remove_boilerplate {
        remove_boilerplate(&mut result);
    }

    result
        .into_iter()
        .map(|p| collapse_blank_lines(&p))
        .filter(|p| !p.trim().is_empty())
        .collect()
}

fn trim_blank_lines(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let first = lines.iter().position(|l| !l.trim().is_empty()).unwrap_or(0);
    let last = lines.iter().rposition(|l| !l.trim().is_empty()).unwrap_or(0);
    if first > last {
        return String::new();
    }
    lines[first..=last].join("\n")
}

fn non_blank_indices(lines: &[&str]) -> Vec<usize> {
    lines.iter().enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, _)| i)
        .collect()
}

fn normalize_signature(line: &str) -> String {
    let trimmed = line.trim();
    let no_digits: String = trimmed.chars().map(|c| if c.is_ascii_digit() { ' ' } else { c }).collect();
    no_digits.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

fn remove_headers_footers(pages: &mut [String]) {
    if pages.len() < 4 {
        return;
    }

    let threshold = (pages.len() as f64 * 0.6).ceil() as usize;
    let zone_count = 5;

    let mut top_sigs: HashMap<String, usize> = HashMap::new();
    let mut bottom_sigs: HashMap<String, usize> = HashMap::new();

    for page in pages.iter() {
        let lines: Vec<&str> = page.lines().collect();
        let nb = non_blank_indices(&lines);
        let ez = zone_count.min(nb.len() / 2);

        for &idx in nb.iter().take(ez) {
            let sig = normalize_signature(lines[idx]);
            if !sig.is_empty() {
                *top_sigs.entry(sig).or_insert(0) += 1;
            }
        }
        for &idx in nb.iter().rev().take(ez) {
            let sig = normalize_signature(lines[idx]);
            if !sig.is_empty() {
                *bottom_sigs.entry(sig).or_insert(0) += 1;
            }
        }
    }

    let frequent_top: Vec<String> = top_sigs
        .into_iter()
        .filter(|(_, count)| *count >= threshold)
        .map(|(sig, _)| sig)
        .collect();

    let frequent_bottom: Vec<String> = bottom_sigs
        .into_iter()
        .filter(|(_, count)| *count >= threshold)
        .map(|(sig, _)| sig)
        .collect();

    for page in pages.iter_mut() {
        let lines: Vec<&str> = page.lines().collect();
        let nb = non_blank_indices(&lines);
        let ez = zone_count.min(nb.len() / 2);
        let mut keep = vec![true; lines.len()];

        for &idx in nb.iter().take(ez) {
            let sig = normalize_signature(lines[idx]);
            if frequent_top.contains(&sig) {
                keep[idx] = false;
            }
        }

        for &idx in nb.iter().rev().take(ez) {
            let sig = normalize_signature(lines[idx]);
            if frequent_bottom.contains(&sig) {
                keep[idx] = false;
            }
        }

        *page = lines
            .iter()
            .enumerate()
            .filter(|(i, _)| keep[*i])
            .map(|(_, l)| *l)
            .collect::<Vec<_>>()
            .join("\n");
    }
}

fn remove_page_numbers(pages: &mut [String]) {
    let bare_number = Regex::new(r"^\s*\d{1,4}\s*$").unwrap();
    let page_x = Regex::new(r"(?i)^\s*page\s+\d{1,4}\s*$").unwrap();
    let dash_number = Regex::new(r"^\s*[-–—]\s*\d{1,4}\s*[-–—]\s*$").unwrap();
    let x_of_y = Regex::new(r"(?i)^\s*\d{1,4}\s+of\s+\d{1,4}\s*$").unwrap();
    let roman = Regex::new(r"(?i)^\s*[ivxlcdm]+\s*$").unwrap();

    let zone_count = 5;

    // Phase 1: detect sequential bare numbers across pages (strongest signal)
    let seq_positions = detect_sequential_numbers(pages);

    for (page_idx, page) in pages.iter_mut().enumerate() {
        let lines: Vec<&str> = page.lines().collect();
        let nb = non_blank_indices(&lines);
        let mut keep = vec![true; lines.len()];

        // Remove sequential page numbers found anywhere
        if let Some(line_idx) = seq_positions.get(&page_idx) {
            keep[*line_idx] = false;
        }

        // Zone-based removal for patterned page numbers
        let ez = zone_count.min(nb.len() / 2);
        let top_zone: Vec<usize> = nb.iter().take(ez).copied().collect();
        let bottom_zone: Vec<usize> = nb.iter().rev().take(ez).copied().collect();

        for &idx in top_zone.iter().chain(bottom_zone.iter()) {
            let line = lines[idx];
            if page_x.is_match(line)
                || dash_number.is_match(line)
                || x_of_y.is_match(line)
            {
                keep[idx] = false;
            }
            if bare_number.is_match(line) || roman.is_match(line) {
                keep[idx] = false;
            }
        }

        *page = lines
            .iter()
            .enumerate()
            .filter(|(i, _)| keep[*i])
            .map(|(_, l)| *l)
            .collect::<Vec<_>>()
            .join("\n");
    }
}

fn detect_sequential_numbers(pages: &[String]) -> HashMap<usize, usize> {
    if pages.len() < 3 {
        return HashMap::new();
    }

    let bare_number = Regex::new(r"^\s*(\d{1,4})\s*$").unwrap();
    let mut best: HashMap<usize, usize> = HashMap::new();

    // Try each possible starting page number (1-based or 0-based)
    for start_num in 0..=5u32 {
        let mut matches: HashMap<usize, usize> = HashMap::new();

        for (page_idx, page) in pages.iter().enumerate() {
            let expected = start_num + page_idx as u32;
            for (line_idx, line) in page.lines().enumerate() {
                if let Some(caps) = bare_number.captures(line) {
                    if let Ok(n) = caps[1].parse::<u32>() {
                        if n == expected {
                            matches.insert(page_idx, line_idx);
                            break;
                        }
                    }
                }
            }
        }

        // Need matches on at least 60% of pages to be confident
        let threshold = (pages.len() as f64 * 0.6).ceil() as usize;
        if matches.len() >= threshold && matches.len() > best.len() {
            best = matches;
        }
    }

    best
}

fn remove_footnotes(pages: &mut [String]) {
    let inline_marker = Regex::new(r"\[\d{1,3}\]").unwrap();
    let footnote_line = Regex::new(r"^\d{1,3}[\.\)]\s").unwrap();
    let separator_line = Regex::new(r"^\s*[-_]{3,}\s*$").unwrap();

    for page in pages.iter_mut() {
        let mut text = inline_marker.replace_all(page, "").to_string();

        let lines: Vec<&str> = text.lines().collect();
        let mut cut_from: Option<usize> = None;

        for i in (0..lines.len()).rev() {
            if footnote_line.is_match(lines[i]) {
                cut_from = Some(i);
            } else if separator_line.is_match(lines[i]) && cut_from.is_some() {
                cut_from = Some(i);
                break;
            } else if cut_from.is_some() {
                break;
            }
        }

        if let Some(start) = cut_from {
            text = lines[..start].join("\n");
        }

        *page = text;
    }
}

fn remove_boilerplate(pages: &mut [String]) {
    let boilerplate = Regex::new(
        r"(?i)(©|\bcopyright\b|\ball rights reserved\b|\bconfidential\b|\bdraft\s*[-–—]\s*do not distribute\b)"
    ).unwrap();

    let zone_count = 5;

    for page in pages.iter_mut() {
        let lines: Vec<&str> = page.lines().collect();
        let nb = non_blank_indices(&lines);
        let effective_zone = zone_count.min(nb.len() / 2);
        let mut keep = vec![true; lines.len()];

        for &idx in nb.iter().take(effective_zone) {
            if boilerplate.is_match(lines[idx]) {
                keep[idx] = false;
            }
        }

        for &idx in nb.iter().rev().take(effective_zone) {
            if boilerplate.is_match(lines[idx]) {
                keep[idx] = false;
            }
        }

        *page = lines
            .iter()
            .enumerate()
            .filter(|(i, _)| keep[*i])
            .map(|(_, l)| *l)
            .collect::<Vec<_>>()
            .join("\n");
    }
}

fn collapse_blank_lines(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut out: Vec<&str> = Vec::with_capacity(lines.len());
    let mut blank_count = 0;

    for line in &lines {
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count <= 1 {
                out.push("");
            }
        } else {
            blank_count = 0;
            out.push(line);
        }
    }

    let joined = out.join("\n");
    joined.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pages(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn removes_repeated_headers() {
        let input = pages(&[
            "ACME Corp Manual\nFirst page content here",
            "ACME Corp Manual\nSecond page content here",
            "ACME Corp Manual\nThird page content here",
            "ACME Corp Manual\nFourth page content here",
            "ACME Corp Manual\nFifth page content here",
        ]);
        let result = clean_pages(&input, &CleanerConfig { remove_headers_footers: true, ..Default::default() });
        for page in &result {
            assert!(!page.contains("ACME Corp Manual"), "Header should be removed: {}", page);
        }
    }

    #[test]
    fn removes_repeated_footers() {
        let input = pages(&[
            "Content A\nwww.example.com",
            "Content B\nwww.example.com",
            "Content C\nwww.example.com",
            "Content D\nwww.example.com",
        ]);
        let result = clean_pages(&input, &CleanerConfig { remove_headers_footers: true, ..Default::default() });
        for page in &result {
            assert!(!page.contains("www.example.com"), "Footer should be removed: {}", page);
        }
    }

    #[test]
    fn skips_header_detection_for_short_docs() {
        let input = pages(&[
            "Same Header\nPage 1 content",
            "Same Header\nPage 2 content",
        ]);
        let result = clean_pages(&input, &CleanerConfig { remove_headers_footers: true, ..Default::default() });
        assert!(result[0].contains("Same Header"));
    }

    #[test]
    fn removes_bare_page_numbers() {
        let input = pages(&[
            "42\nSome real content\n43",
        ]);
        let result = clean_pages(&input, &CleanerConfig {
            remove_page_numbers: true,
            remove_headers_footers: false,
            remove_footnotes: false,
            remove_boilerplate: false,
        });
        assert!(!result[0].starts_with("42"));
        assert!(!result[0].ends_with("43"));
        assert!(result[0].contains("Some real content"));
    }

    #[test]
    fn removes_page_x_format() {
        let input = pages(&["Page 5\nContent here\nPage 6"]);
        let config = CleanerConfig {
            remove_page_numbers: true,
            remove_headers_footers: false,
            remove_footnotes: false,
            remove_boilerplate: false,
        };
        let result = clean_pages(&input, &config);
        assert!(!result[0].contains("Page 5"));
    }

    #[test]
    fn removes_dash_number_format() {
        let input = pages(&["- 3 -\nMain text\n— 4 —"]);
        let config = CleanerConfig {
            remove_page_numbers: true,
            remove_headers_footers: false,
            remove_footnotes: false,
            remove_boilerplate: false,
        };
        let result = clean_pages(&input, &config);
        assert!(!result[0].contains("- 3 -"));
    }

    #[test]
    fn removes_roman_numeral_page_numbers() {
        let input = pages(&["iv\nContent here"]);
        let config = CleanerConfig {
            remove_page_numbers: true,
            remove_headers_footers: false,
            remove_footnotes: false,
            remove_boilerplate: false,
        };
        let result = clean_pages(&input, &config);
        assert!(!result[0].contains("iv"));
    }

    #[test]
    fn preserves_numbers_in_body() {
        let input = pages(&[
            "Header\nThe answer is 42 and that matters\nAnother line\nMore text\nFooter",
        ]);
        let config = CleanerConfig {
            remove_page_numbers: true,
            remove_headers_footers: false,
            remove_footnotes: false,
            remove_boilerplate: false,
        };
        let result = clean_pages(&input, &config);
        assert!(result[0].contains("42"));
    }

    #[test]
    fn removes_inline_footnote_markers() {
        let input = pages(&["This is text[1] with footnotes[2] embedded."]);
        let config = CleanerConfig {
            remove_footnotes: true,
            remove_headers_footers: false,
            remove_page_numbers: false,
            remove_boilerplate: false,
        };
        let result = clean_pages(&input, &config);
        assert!(!result[0].contains("[1]"));
        assert!(!result[0].contains("[2]"));
        assert!(result[0].contains("This is text with footnotes embedded."));
    }

    #[test]
    fn removes_footnote_blocks() {
        let input = pages(&[
            "Main content here.\n___\n1. First footnote\n2. Second footnote",
        ]);
        let config = CleanerConfig {
            remove_footnotes: true,
            remove_headers_footers: false,
            remove_page_numbers: false,
            remove_boilerplate: false,
        };
        let result = clean_pages(&input, &config);
        assert!(result[0].contains("Main content here."));
        assert!(!result[0].contains("First footnote"));
        assert!(!result[0].contains("Second footnote"));
    }

    #[test]
    fn removes_copyright_from_edges() {
        let input = pages(&[
            "© 2024 Acme Corp\nActual content\nAll rights reserved.",
        ]);
        let config = CleanerConfig {
            remove_boilerplate: true,
            remove_headers_footers: false,
            remove_page_numbers: false,
            remove_footnotes: false,
        };
        let result = clean_pages(&input, &config);
        assert!(!result[0].contains("© 2024 Acme Corp"));
        assert!(!result[0].contains("All rights reserved"));
        assert!(result[0].contains("Actual content"));
    }

    #[test]
    fn preserves_copyright_in_body() {
        let input = pages(&[
            "Line 1\nLine 2\nLine 3\nThe copyright symbol © is used legally.\nLine 5\nLine 6\nLine 7",
        ]);
        let config = CleanerConfig {
            remove_boilerplate: true,
            remove_headers_footers: false,
            remove_page_numbers: false,
            remove_footnotes: false,
        };
        let result = clean_pages(&input, &config);
        assert!(result[0].contains("copyright symbol ©"));
    }

    #[test]
    fn collapses_excessive_blank_lines() {
        let input = pages(&["Line 1\n\n\n\n\nLine 2"]);
        let config = CleanerConfig {
            remove_headers_footers: false,
            remove_page_numbers: false,
            remove_footnotes: false,
            remove_boilerplate: false,
        };
        let result = clean_pages(&input, &config);
        assert!(!result[0].contains("\n\n\n"));
    }

    #[test]
    fn drops_empty_pages() {
        let input = pages(&["Real content", "   ", "More content"]);
        let config = CleanerConfig {
            remove_headers_footers: false,
            remove_page_numbers: false,
            remove_footnotes: false,
            remove_boilerplate: false,
        };
        let result = clean_pages(&input, &config);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn individual_flags_disable() {
        let input = pages(&[
            "© Header\n42\nText[1] here\n___\n1. Note\n99",
        ]);
        let config = CleanerConfig {
            remove_headers_footers: false,
            remove_page_numbers: false,
            remove_footnotes: false,
            remove_boilerplate: false,
        };
        let result = clean_pages(&input, &config);
        assert!(result[0].contains("© Header"));
        assert!(result[0].contains("[1]"));
        assert!(result[0].contains("1. Note"));
    }

    #[test]
    fn full_pipeline_integration() {
        let input = pages(&[
            "ACME Manual v3\n\nIntroduction to the system[1].\n\n___\n1. See appendix.\n\n© 2024 ACME",
            "ACME Manual v3\n\n2\n\nChapter 1 details[2].\n\n___\n2. Reference doc.\n\n© 2024 ACME",
            "ACME Manual v3\n\n3\n\nChapter 2 continues.\n\n© 2024 ACME",
            "ACME Manual v3\n\n4\n\nConclusion of the manual.\n\n© 2024 ACME",
        ]);
        let result = clean_pages(&input, &CleanerConfig::default());

        for page in &result {
            assert!(!page.contains("© 2024 ACME"), "Boilerplate should be removed");
            assert!(!page.contains("[1]"), "Footnote markers should be removed");
            assert!(!page.contains("[2]"), "Footnote markers should be removed");
            assert!(!page.contains("See appendix"), "Footnote blocks should be removed");
        }
        assert!(result[0].contains("Introduction to the system"));
    }

    #[test]
    fn x_of_y_page_number_removed() {
        let input = pages(&["1 of 10\nContent\n2 of 10"]);
        let config = CleanerConfig {
            remove_page_numbers: true,
            remove_headers_footers: false,
            remove_footnotes: false,
            remove_boilerplate: false,
        };
        let result = clean_pages(&input, &config);
        assert!(!result[0].contains("1 of 10"));
    }

    #[test]
    fn header_with_varying_numbers_detected() {
        let input = pages(&[
            "Chapter 1 - Guide\nContent A",
            "Chapter 2 - Guide\nContent B",
            "Chapter 3 - Guide\nContent C",
            "Chapter 4 - Guide\nContent D",
            "Chapter 5 - Guide\nContent E",
        ]);
        let result = clean_pages(&input, &CleanerConfig { remove_headers_footers: true, ..Default::default() });
        for page in &result {
            assert!(!page.contains("Guide"), "Normalized header should be removed: {}", page);
        }
    }

    #[test]
    fn trims_leading_trailing_blank_lines() {
        let input = pages(&["\n\n\nActual content\n\n\n"]);
        let config = CleanerConfig {
            remove_headers_footers: false,
            remove_page_numbers: false,
            remove_footnotes: false,
            remove_boilerplate: false,
        };
        let result = clean_pages(&input, &config);
        assert_eq!(result[0], "Actual content");
    }

    #[test]
    fn removes_page_numbers_after_blank_lines() {
        let input = pages(&["\n\n\n42\n\nReal content here\n\n\n43\n\n"]);
        let config = CleanerConfig {
            remove_page_numbers: true,
            remove_headers_footers: false,
            remove_footnotes: false,
            remove_boilerplate: false,
        };
        let result = clean_pages(&input, &config);
        assert!(!result[0].contains("42"), "Page number after blanks should be removed");
        assert!(!result[0].contains("43"), "Page number before blanks should be removed");
        assert!(result[0].contains("Real content here"));
    }

    #[test]
    fn sequential_page_numbers_detected_anywhere() {
        let input = pages(&[
            "Content A\n1\nMore A",
            "Content B\n2\nMore B",
            "Content C\n3\nMore C",
            "Content D\n4\nMore D",
            "Content E\n5\nMore E",
        ]);
        let config = CleanerConfig {
            remove_page_numbers: true,
            remove_headers_footers: false,
            remove_footnotes: false,
            remove_boilerplate: false,
        };
        let result = clean_pages(&input, &config);
        for (i, page) in result.iter().enumerate() {
            let num = format!("{}", i + 1);
            assert!(!page.lines().any(|l| l.trim() == num),
                "Sequential page number {} should be removed from page {}", num, i);
        }
    }

    #[test]
    fn realistic_pdf_page_with_blanks() {
        let input = pages(&[
            "\n\n  My Book Title  \n\n\nSome introduction text.\nMore text.\n\n  © 2024 Publisher  \n\n\n  1  \n\n",
            "\n\n  My Book Title  \n\n\nChapter 1 content.\nDetails here.\n\n  © 2024 Publisher  \n\n\n  2  \n\n",
            "\n\n  My Book Title  \n\n\nChapter 2 content.\nMore details.\n\n  © 2024 Publisher  \n\n\n  3  \n\n",
            "\n\n  My Book Title  \n\n\nChapter 3 content.\nFinal text.\n\n  © 2024 Publisher  \n\n\n  4  \n\n",
        ]);
        let result = clean_pages(&input, &CleanerConfig::default());
        for page in &result {
            assert!(!page.contains("My Book Title"), "Repeated header should be removed: {}", page);
            assert!(!page.contains("© 2024 Publisher"), "Copyright should be removed: {}", page);
        }
    }
}
