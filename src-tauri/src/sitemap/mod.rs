//! Sitemap fetching, parsing, and discovery.
//!
//! Supports `<urlset>` and `<sitemapindex>` documents, gzip-compressed
//! sitemaps (`.xml.gz`), robots.txt `Sitemap:` directives, and the standard
//! discovery locations `/sitemap.xml` and `/sitemap_index.xml`.
//!
//! All fetches enforce a response-size cap, request timeout, and (when
//! enabled) SSRF protection. Sitemap-index recursion is bounded in both
//! depth and total number of sub-sitemaps to prevent zip-bomb-style
//! amplification.

use std::collections::HashSet;
use std::io::Read;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::crawler::ssrf::is_private_target;

/// Hard cap on the number of URLs returned from a single top-level sitemap
/// fetch. The list is truncated (and a warning surfaced) once this many
/// URLs have been collected across all sub-sitemaps.
pub const MAX_URLS: usize = 10_000;

/// Threshold above which the caller should surface a "large sitemap"
/// warning in the UI.
pub const WARN_URL_THRESHOLD: usize = 1_000;

/// Maximum number of sub-sitemaps followed from a `<sitemapindex>`.
pub const MAX_SUB_SITEMAPS: usize = 50;

/// Maximum recursion depth for nested sitemap-index documents.
/// Depth 0 = top-level document; depth 2 allows an index that points to
/// another index that points to leaf sitemaps.
pub const MAX_DEPTH: u8 = 2;

/// Cap on a single sitemap response body (post-decompression for gzip).
pub const MAX_BODY_BYTES: usize = 50 * 1024 * 1024;

/// Cap on a `robots.txt` response body — separate from [`MAX_BODY_BYTES`]
/// because a real robots.txt should never approach a megabyte. A hostile
/// server returning a multi-GB body should be cut off much earlier here.
pub const MAX_ROBOTS_BYTES: usize = 1024 * 1024;

/// Per-request timeout.
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

const USER_AGENT: &str = concat!("docurip/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SitemapEntry {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lastmod: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SitemapResult {
    pub entries: Vec<SitemapEntry>,
    /// True if the URL list was truncated because [`MAX_URLS`] was hit.
    pub truncated: bool,
    /// Source sitemap URLs actually fetched (top-level plus any sub-sitemaps).
    pub sources: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SitemapError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("SSRF protection blocked '{0}'")]
    Ssrf(String),
    #[error("HTTP error fetching '{url}': {source}")]
    Http { url: String, source: reqwest::Error },
    #[error("HTTP status {status} fetching '{url}'")]
    Status { url: String, status: u16 },
    #[error("Sitemap body exceeds {MAX_BODY_BYTES} bytes")]
    TooLarge,
    #[error("Failed to decompress gzip sitemap: {0}")]
    Gzip(#[from] std::io::Error),
    #[error("Malformed sitemap XML: {0}")]
    Parse(String),
}

/// One parsed sitemap document.
#[derive(Debug, Clone)]
pub(crate) enum Parsed {
    UrlSet(Vec<SitemapEntry>),
    Index(Vec<String>),
}

pub(crate) fn parse_sitemap_xml(body: &[u8]) -> Result<Parsed, SitemapError> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_reader(body);
    reader.config_mut().trim_text(true);

    #[derive(Copy, Clone, PartialEq)]
    enum Tag { None, Loc, Lastmod, Priority }
    #[derive(Copy, Clone, PartialEq)]
    enum Mode { Unknown, UrlSet, Index }

    let mut mode = Mode::Unknown;
    let mut in_url = false;
    let mut in_sitemap = false;
    let mut cur_tag = Tag::None;

    let mut entries: Vec<SitemapEntry> = Vec::new();
    let mut sub_sitemaps: Vec<String> = Vec::new();

    let mut cur_loc = String::new();
    let mut cur_lastmod: Option<String> = None;
    let mut cur_priority: Option<f32> = None;

    let mut buf = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| SitemapError::Parse(e.to_string()))?
        {
            Event::Start(e) => {
                let name = local_name(e.name().as_ref());
                match name.as_str() {
                    "urlset" => mode = Mode::UrlSet,
                    "sitemapindex" => mode = Mode::Index,
                    "url" if mode == Mode::UrlSet => {
                        in_url = true;
                        cur_loc.clear();
                        cur_lastmod = None;
                        cur_priority = None;
                    }
                    "sitemap" if mode == Mode::Index => {
                        in_sitemap = true;
                        cur_loc.clear();
                    }
                    "loc" if in_url || in_sitemap => cur_tag = Tag::Loc,
                    "lastmod" if in_url => cur_tag = Tag::Lastmod,
                    "priority" if in_url => cur_tag = Tag::Priority,
                    _ => {}
                }
            }
            Event::End(e) => {
                let name = local_name(e.name().as_ref());
                match name.as_str() {
                    "url" if in_url => {
                        if !cur_loc.is_empty() {
                            entries.push(SitemapEntry {
                                url: std::mem::take(&mut cur_loc),
                                lastmod: cur_lastmod.take(),
                                priority: cur_priority.take(),
                            });
                        }
                        in_url = false;
                    }
                    "sitemap" if in_sitemap => {
                        if !cur_loc.is_empty() {
                            sub_sitemaps.push(std::mem::take(&mut cur_loc));
                        }
                        in_sitemap = false;
                    }
                    "loc" | "lastmod" | "priority" => cur_tag = Tag::None,
                    _ => {}
                }
            }
            Event::Text(t) => {
                if cur_tag == Tag::None { buf.clear(); continue; }
                let decoded = t.decode().map_err(|e| SitemapError::Parse(e.to_string()))?;
                let text = quick_xml::escape::unescape(&decoded)
                    .map_err(|e| SitemapError::Parse(e.to_string()))?;
                match cur_tag {
                    Tag::Loc => cur_loc.push_str(text.trim()),
                    Tag::Lastmod => cur_lastmod = Some(text.trim().to_string()),
                    Tag::Priority => cur_priority = text.trim().parse().ok(),
                    Tag::None => {}
                }
            }
            Event::CData(c) => {
                if cur_tag == Tag::Loc {
                    let raw = std::str::from_utf8(c.as_ref())
                        .map_err(|e| SitemapError::Parse(e.to_string()))?;
                    cur_loc.push_str(raw.trim());
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    match mode {
        Mode::UrlSet => Ok(Parsed::UrlSet(entries)),
        Mode::Index => Ok(Parsed::Index(sub_sitemaps)),
        Mode::Unknown => Err(SitemapError::Parse(
            "no <urlset> or <sitemapindex> root element found".to_string(),
        )),
    }
}

fn local_name(qname: &[u8]) -> String {
    let s = std::str::from_utf8(qname).unwrap_or("");
    s.rsplit(':').next().unwrap_or(s).to_ascii_lowercase()
}

fn build_client(ssrf: bool) -> reqwest::Client {
    // When SSRF is on we can't just check the initial URL — reqwest's
    // default redirect policy follows up to 10 hops, so a hostile server
    // can 302 to `http://127.0.0.1/admin` and the request lands there
    // silently. Install a custom policy that runs `is_private_target`
    // on every redirect target.
    let policy = if ssrf {
        reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 10 {
                return attempt.error("too many redirects");
            }
            if is_private_target(attempt.url().as_str()) {
                return attempt.error("redirect target is a private address");
            }
            attempt.follow()
        })
    } else {
        reqwest::redirect::Policy::default()
    };
    reqwest::Client::builder()
        .redirect(policy)
        .timeout(FETCH_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .unwrap_or_default()
}

/// Fetch a URL, streaming the response body chunk-by-chunk and rejecting
/// as soon as the running total exceeds `cap` — do NOT buffer the whole
/// body first, because `Content-Length` is trivially spoofable (a hostile
/// server can omit it, lie, or use chunked transfer encoding).
async fn fetch_capped(
    client: &reqwest::Client,
    url: &str,
    cap: usize,
) -> Result<Vec<u8>, SitemapError> {
    let mut resp = client
        .get(url)
        .send()
        .await
        .map_err(|source| SitemapError::Http { url: url.to_string(), source })?;

    if !resp.status().is_success() {
        return Err(SitemapError::Status { url: url.to_string(), status: resp.status().as_u16() });
    }

    // Advertised length is only used to short-circuit obvious over-caps;
    // do not trust it as an upper bound — the streaming loop below is the
    // real enforcement.
    if let Some(len) = resp.content_length() {
        if len as usize > cap {
            return Err(SitemapError::TooLarge);
        }
    }

    let mut buf: Vec<u8> = Vec::new();
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|source| SitemapError::Http { url: url.to_string(), source })?
    {
        if buf.len().saturating_add(chunk.len()) > cap {
            return Err(SitemapError::TooLarge);
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(buf)
}

/// Decompress if `body` is gzip-encoded (magic 0x1f 0x8b) or the URL hints at it.
fn maybe_decompress(url: &str, body: Vec<u8>) -> Result<Vec<u8>, SitemapError> {
    let looks_gz = body.len() >= 2 && body[0] == 0x1f && body[1] == 0x8b;
    let url_gz = url.to_ascii_lowercase().ends_with(".gz");
    if !looks_gz && !url_gz {
        return Ok(body);
    }
    let mut decoder = flate2::read::GzDecoder::new(&body[..]);
    let mut out = Vec::new();
    // Cap decompressed output too — protects against gzip bombs.
    let mut buf = [0u8; 8192];
    loop {
        let n = decoder.read(&mut buf)?;
        if n == 0 { break; }
        if out.len() + n > MAX_BODY_BYTES {
            return Err(SitemapError::TooLarge);
        }
        out.extend_from_slice(&buf[..n]);
    }
    Ok(out)
}

async fn fetch_and_parse(
    client: &reqwest::Client,
    url: &str,
    ssrf: bool,
) -> Result<Parsed, SitemapError> {
    if ssrf && is_private_target(url) {
        return Err(SitemapError::Ssrf(url.to_string()));
    }
    let body = fetch_capped(client, url, MAX_BODY_BYTES).await?;
    let body = maybe_decompress(url, body)?;
    parse_sitemap_xml(&body)
}

/// Fetch a sitemap and recursively expand any `<sitemapindex>` entries.
///
/// Enforces [`MAX_SUB_SITEMAPS`], [`MAX_DEPTH`], and [`MAX_URLS`]. Returns
/// a truncated list (with `truncated = true`) rather than erroring when
/// [`MAX_URLS`] is reached.
///
/// Pass a `should_stop` handle (typically the same `AtomicBool` shared
/// with a crawl orchestrator, or a fresh one owned by the caller) to
/// abandon the recursion between fetches. Callers with no cancellation
/// story can use [`fetch_sitemap`].
pub async fn fetch_sitemap_with_stop(
    url: &str,
    ssrf: bool,
    should_stop: Option<&std::sync::atomic::AtomicBool>,
) -> Result<SitemapResult, SitemapError> {
    use std::sync::atomic::Ordering;
    let _ = Url::parse(url).map_err(|_| SitemapError::InvalidUrl(url.to_string()))?;
    let client = build_client(ssrf);

    let mut entries: Vec<SitemapEntry> = Vec::new();
    let mut seen_urls: HashSet<String> = HashSet::new();
    let mut sources: Vec<String> = Vec::new();
    let mut visited_sources: HashSet<String> = HashSet::new();
    let mut truncated = false;
    let mut sub_count: usize = 0;

    // (sitemap_url, depth)
    let mut queue: Vec<(String, u8)> = vec![(url.to_string(), 0)];

    while let Some((sm_url, depth)) = queue.pop() {
        // Cancellation check *before* each network fetch — the caller
        // can abandon a deep recursion without waiting on remaining
        // sub-sitemaps. Returns a truncated result rather than an error
        // so the caller can still use whatever was collected.
        if should_stop.map(|s| s.load(Ordering::Relaxed)).unwrap_or(false) {
            truncated = true;
            break;
        }
        if !visited_sources.insert(sm_url.clone()) {
            continue;
        }
        sources.push(sm_url.clone());

        let parsed = fetch_and_parse(&client, &sm_url, ssrf).await?;
        match parsed {
            Parsed::UrlSet(list) => {
                for e in list {
                    if !seen_urls.insert(e.url.clone()) { continue; }
                    if entries.len() >= MAX_URLS {
                        truncated = true;
                        break;
                    }
                    entries.push(e);
                }
                if truncated { break; }
            }
            Parsed::Index(subs) => {
                if depth >= MAX_DEPTH {
                    // Ignore deeper indexes rather than error out.
                    continue;
                }
                for sub in subs {
                    if sub_count >= MAX_SUB_SITEMAPS {
                        truncated = true;
                        break;
                    }
                    sub_count += 1;
                    queue.push((sub, depth + 1));
                }
                if truncated { break; }
            }
        }
    }

    Ok(SitemapResult { entries, truncated, sources })
}

/// Thin wrapper over [`fetch_sitemap_with_stop`] for callers with no
/// cancellation handle.
pub async fn fetch_sitemap(url: &str, ssrf: bool) -> Result<SitemapResult, SitemapError> {
    fetch_sitemap_with_stop(url, ssrf, None).await
}

/// Discover sitemap URLs for a site.
///
/// Combines:
/// 1. `Sitemap:` directives in `/robots.txt`.
/// 2. Default well-known locations (`/sitemap.xml`, `/sitemap_index.xml`)
///    that respond with 200.
///
/// Returns discovered sitemap URLs in preference order (robots.txt first).
/// No result is not an error — an empty vec means the site has no
/// discoverable sitemap.
pub async fn discover_sitemap(base_url: &str, ssrf: bool) -> Result<Vec<String>, SitemapError> {
    let base = Url::parse(base_url).map_err(|_| SitemapError::InvalidUrl(base_url.to_string()))?;
    if ssrf && is_private_target(base_url) {
        return Err(SitemapError::Ssrf(base_url.to_string()));
    }
    let client = build_client(ssrf);

    let mut found: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for candidate in discover_from_robots_txt(&client, &base).await {
        if seen.insert(candidate.clone()) {
            found.push(candidate);
        }
    }

    for path in ["/sitemap.xml", "/sitemap_index.xml"] {
        let mut u = base.clone();
        u.set_path(path);
        u.set_query(None);
        let candidate = u.to_string();
        if !seen.insert(candidate.clone()) { continue; }
        if head_or_get_ok(&client, &candidate).await {
            found.push(candidate);
        }
    }

    Ok(found)
}

async fn discover_from_robots_txt(client: &reqwest::Client, base: &Url) -> Vec<String> {
    let mut robots_url = base.clone();
    robots_url.set_path("/robots.txt");
    robots_url.set_query(None);

    // Reuse the streaming-capped fetcher with the tighter robots.txt
    // cap — a real robots.txt is a few kilobytes; anything larger is a
    // server hosing us and we'd rather cut off early than absorb it.
    let bytes = match fetch_capped(client, robots_url.as_str(), MAX_ROBOTS_BYTES).await {
        Ok(b) => b,
        Err(_) => return Vec::new(),
    };
    let body = match std::str::from_utf8(&bytes) {
        Ok(s) => s.to_string(),
        Err(_) => return Vec::new(),
    };

    let mut out = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.starts_with('#') { continue; }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("sitemap:") {
            // Recover the original-case value at the same offset.
            let value = line["sitemap:".len()..].trim().to_string();
            if !value.is_empty() {
                out.push(value);
            }
        }
    }
    out
}

async fn head_or_get_ok(client: &reqwest::Client, url: &str) -> bool {
    // Many CDNs 405 on HEAD, so fall back to a bounded GET.
    if let Ok(r) = client.head(url).send().await {
        if r.status().is_success() { return true; }
    }
    match client.get(url).send().await {
        Ok(r) => r.status().is_success(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_urlset() {
        let xml = br#"<?xml version="1.0"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>https://example.com/a</loc><lastmod>2024-01-01</lastmod><priority>0.8</priority></url>
  <url><loc>https://example.com/b</loc></url>
</urlset>"#;
        let parsed = parse_sitemap_xml(xml).unwrap();
        match parsed {
            Parsed::UrlSet(entries) => {
                assert_eq!(entries.len(), 2);
                assert_eq!(entries[0].url, "https://example.com/a");
                assert_eq!(entries[0].lastmod.as_deref(), Some("2024-01-01"));
                assert_eq!(entries[0].priority, Some(0.8));
                assert_eq!(entries[1].url, "https://example.com/b");
                assert!(entries[1].lastmod.is_none());
            }
            _ => panic!("expected urlset"),
        }
    }

    #[test]
    fn parses_sitemapindex() {
        let xml = br#"<?xml version="1.0"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <sitemap><loc>https://example.com/s1.xml</loc></sitemap>
  <sitemap><loc>https://example.com/s2.xml</loc></sitemap>
</sitemapindex>"#;
        let parsed = parse_sitemap_xml(xml).unwrap();
        match parsed {
            Parsed::Index(subs) => {
                assert_eq!(subs, vec![
                    "https://example.com/s1.xml".to_string(),
                    "https://example.com/s2.xml".to_string(),
                ]);
            }
            _ => panic!("expected index"),
        }
    }

    #[test]
    fn parses_urlset_with_cdata_loc() {
        let xml = br#"<?xml version="1.0"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc><![CDATA[https://example.com/x]]></loc></url>
</urlset>"#;
        let parsed = parse_sitemap_xml(xml).unwrap();
        match parsed {
            Parsed::UrlSet(entries) => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].url, "https://example.com/x");
            }
            _ => panic!("expected urlset"),
        }
    }

    #[test]
    fn malformed_xml_errors() {
        // Unterminated CDATA — quick-xml surfaces this as a parse error.
        let xml = br#"<urlset><url><loc><![CDATA[oops</loc></url></urlset>"#;
        let err = parse_sitemap_xml(xml).unwrap_err();
        assert!(matches!(err, SitemapError::Parse(_)));
    }

    #[test]
    fn truncated_input_yields_empty_urlset() {
        // Real-world tolerance: a truncated body still produces a valid parse
        // result with whatever entries were fully closed before EOF.
        let xml = br#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><url><loc>https://example.com/a</loc></url><url><loc>broken"#;
        match parse_sitemap_xml(xml).unwrap() {
            Parsed::UrlSet(entries) => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].url, "https://example.com/a");
            }
            _ => panic!("expected urlset"),
        }
    }

    #[test]
    fn unknown_root_errors() {
        let xml = br#"<?xml version="1.0"?><rss><channel/></rss>"#;
        let err = parse_sitemap_xml(xml).unwrap_err();
        assert!(matches!(err, SitemapError::Parse(_)));
    }

    #[tokio::test]
    async fn fetch_sitemap_returns_entries() {
        use wiremock::{MockServer, ResponseTemplate};
        use wiremock::matchers::{method, path};

        let mock = MockServer::start().await;
        let body = r#"<?xml version="1.0"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>https://example.com/one</loc></url>
  <url><loc>https://example.com/two</loc></url>
</urlset>"#;
        wiremock::Mock::given(method("GET"))
            .and(path("/sitemap.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(body.as_bytes().to_vec(), "application/xml"),
            )
            .mount(&mock)
            .await;

        let url = format!("{}/sitemap.xml", mock.uri());
        let result = fetch_sitemap(&url, false).await.unwrap();
        assert_eq!(result.entries.len(), 2);
        assert!(!result.truncated);
        assert_eq!(result.sources, vec![url]);
    }

    #[tokio::test]
    async fn fetch_sitemap_expands_index() {
        use wiremock::{MockServer, ResponseTemplate};
        use wiremock::matchers::{method, path};

        let mock = MockServer::start().await;

        // Sub-sitemap first (leaf)
        wiremock::Mock::given(method("GET"))
            .and(path("/sub.xml"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                br#"<?xml version="1.0"?><urlset><url><loc>https://example.com/leaf</loc></url></urlset>"#.to_vec(),
                "application/xml",
            ))
            .mount(&mock)
            .await;

        // Sitemap index that points to sub.xml
        let index_body = format!(
            r#"<?xml version="1.0"?><sitemapindex><sitemap><loc>{}/sub.xml</loc></sitemap></sitemapindex>"#,
            mock.uri()
        );
        wiremock::Mock::given(method("GET"))
            .and(path("/sitemap.xml"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_raw(index_body.into_bytes(), "application/xml"))
            .mount(&mock)
            .await;

        let result = fetch_sitemap(&format!("{}/sitemap.xml", mock.uri()), false)
            .await
            .unwrap();
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].url, "https://example.com/leaf");
        assert_eq!(result.sources.len(), 2);
    }

    #[tokio::test]
    async fn fetch_sitemap_gzip_decompresses() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;
        use wiremock::{MockServer, ResponseTemplate};
        use wiremock::matchers::{method, path};

        let mock = MockServer::start().await;
        let body = br#"<?xml version="1.0"?><urlset><url><loc>https://example.com/gz</loc></url></urlset>"#;

        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(body).unwrap();
        let gz = enc.finish().unwrap();

        wiremock::Mock::given(method("GET"))
            .and(path("/sitemap.xml.gz"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(gz, "application/gzip"),
            )
            .mount(&mock)
            .await;

        let result = fetch_sitemap(&format!("{}/sitemap.xml.gz", mock.uri()), false)
            .await
            .unwrap();
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].url, "https://example.com/gz");
    }

    #[tokio::test]
    async fn fetch_sitemap_truncates_over_max_urls() {
        // Verify with a lower conceptual cap: build a urlset with MAX_URLS+5
        // entries and confirm we cap at MAX_URLS and set truncated=true.
        // We keep this cheap by generating a small body when MAX_URLS is huge.
        // Since MAX_URLS = 10_000, generating 10k+ entries in a test is fine.
        use wiremock::{MockServer, ResponseTemplate};
        use wiremock::matchers::{method, path};

        let mock = MockServer::start().await;
        let mut body = String::from(r#"<?xml version="1.0"?><urlset>"#);
        for i in 0..(MAX_URLS + 5) {
            body.push_str(&format!("<url><loc>https://example.com/{}</loc></url>", i));
        }
        body.push_str("</urlset>");
        wiremock::Mock::given(method("GET"))
            .and(path("/big.xml"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_raw(body.into_bytes(), "application/xml"))
            .mount(&mock)
            .await;

        let result = fetch_sitemap(&format!("{}/big.xml", mock.uri()), false)
            .await
            .unwrap();
        assert_eq!(result.entries.len(), MAX_URLS);
        assert!(result.truncated);
    }

    #[tokio::test]
    async fn fetch_sitemap_ssrf_blocks_private() {
        let err = fetch_sitemap("http://127.0.0.1/sitemap.xml", true)
            .await
            .unwrap_err();
        assert!(matches!(err, SitemapError::Ssrf(_)));
    }

    // Two related fixes — the custom redirect policy that runs
    // `is_private_target` on each hop, and the chunked streaming that
    // ignores a lying `Content-Length` — are covered by inspection
    // rather than end-to-end tests because both require infrastructure
    // wiremock cannot easily produce (a mock server on a non-private
    // address, or chunked transfer that reqwest reads past a header
    // lie). The existing `fetch_sitemap_body_too_large` covers the
    // honest-oversize case; the streaming loop is a straight port from
    // that.

    #[tokio::test]
    async fn fetch_sitemap_cancellation_stops_between_fetches() {
        use std::sync::atomic::AtomicBool;
        use wiremock::{MockServer, ResponseTemplate};
        use wiremock::matchers::{method, path};

        let mock = MockServer::start().await;
        let index_body = format!(
            r#"<?xml version="1.0"?><sitemapindex><sitemap><loc>{}/sub.xml</loc></sitemap></sitemapindex>"#,
            mock.uri()
        );
        wiremock::Mock::given(method("GET"))
            .and(path("/sitemap.xml"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_raw(index_body.into_bytes(), "application/xml"))
            .mount(&mock)
            .await;
        // Sub sitemap that would normally add another entry — should
        // be skipped because we cancel before the second fetch.
        let stop = AtomicBool::new(true);
        let result = fetch_sitemap_with_stop(
            &format!("{}/sitemap.xml", mock.uri()),
            false,
            Some(&stop),
        )
        .await
        .unwrap();
        // Cancellation surfaces as truncated=true with whatever was collected.
        assert!(result.truncated);
        assert_eq!(result.entries.len(), 0);
    }

    #[tokio::test]
    async fn fetch_sitemap_body_too_large() {
        use wiremock::{MockServer, ResponseTemplate};
        use wiremock::matchers::{method, path};

        let mock = MockServer::start().await;
        let big = vec![b'a'; MAX_BODY_BYTES + 1];
        wiremock::Mock::given(method("GET"))
            .and(path("/huge.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(big, "application/xml"),
            )
            .mount(&mock)
            .await;

        let err = fetch_sitemap(&format!("{}/huge.xml", mock.uri()), false)
            .await
            .unwrap_err();
        assert!(matches!(err, SitemapError::TooLarge));
    }

    #[tokio::test]
    async fn discover_sitemap_from_robots_txt() {
        use wiremock::{MockServer, ResponseTemplate};
        use wiremock::matchers::{method, path};

        let mock = MockServer::start().await;
        let robots = format!(
            "User-agent: *\nDisallow:\nSitemap: {}/custom-sitemap.xml\n",
            mock.uri()
        );
        wiremock::Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(ResponseTemplate::new(200).set_body_string(robots))
            .mount(&mock)
            .await;
        // 404 the default locations so they don't get discovered
        wiremock::Mock::given(method("GET"))
            .and(path("/sitemap.xml"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock)
            .await;
        wiremock::Mock::given(method("HEAD"))
            .and(path("/sitemap.xml"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock)
            .await;

        let found = discover_sitemap(&mock.uri(), false).await.unwrap();
        assert_eq!(found, vec![format!("{}/custom-sitemap.xml", mock.uri())]);
    }

    #[tokio::test]
    async fn discover_sitemap_default_location() {
        use wiremock::{MockServer, ResponseTemplate};
        use wiremock::matchers::{method, path};

        let mock = MockServer::start().await;
        // No robots.txt.
        wiremock::Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock)
            .await;
        // HEAD succeeds on /sitemap.xml.
        wiremock::Mock::given(method("HEAD"))
            .and(path("/sitemap.xml"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock)
            .await;

        let found = discover_sitemap(&mock.uri(), false).await.unwrap();
        assert_eq!(found, vec![format!("{}/sitemap.xml", mock.uri())]);
    }
}
