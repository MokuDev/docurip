pub mod batch;
pub mod job;
pub mod orchestrator;
pub mod robots;
pub mod ssrf;

/// Validate that a URL uses HTTP or HTTPS scheme only.
pub fn is_valid_url(url: &str) -> bool {
    match url::Url::parse(url) {
        Ok(u) if u.scheme() == "http" || u.scheme() == "https" => true,
        Ok(u) if u.cannot_be_a_base() => {
            // Handle base URLs that are relative (shouldn't happen for crawl targets)
            false
        }
        Ok(_) => false,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_urls() {
        assert!(is_valid_url("http://example.com"));
        assert!(is_valid_url("https://example.com"));
        assert!(is_valid_url("https://example.com:8080/path?q=1"));
    }

    #[test]
    fn test_invalid_urls() {
        assert!(!is_valid_url("ftp://example.com"));
        assert!(!is_valid_url("file:///etc/passwd"));
        assert!(!is_valid_url("javascript:alert(1)"));
        assert!(!is_valid_url("mailto:test@example.com"));
        assert!(!is_valid_url("not-a-url"));
        assert!(!is_valid_url(""));
    }
}
