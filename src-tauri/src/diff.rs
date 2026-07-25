//! Crawl diff / change detection.
//!
//! Compares two crawls of the same site and classifies every page as
//! added, removed, modified, unchanged, or unknown. Modification is
//! decided by comparing a stable content fingerprint stored on each
//! [`PageMeta`] (see [`content_fingerprint`]); the actual line-level
//! text diff is computed on demand from the two `.md` files on disk.
//!
//! The comparison logic is kept as free functions with no I/O so it is
//! unit-testable without a filesystem or a running crawl.

use std::collections::HashMap;

use serde::Serialize;

use crate::crawler::job::PageMeta;

/// Stable, version-independent content fingerprint (FNV-1a, 64-bit)
/// rendered as zero-padded hex.
///
/// Deterministic across Rust versions and platforms — unlike
/// `std::hash::DefaultHasher`, whose SipHash output is explicitly not
/// guaranteed stable between releases. Because these fingerprints are
/// persisted with the job and compared against a *future* crawl, they
/// must survive an app or toolchain upgrade, so a fixed algorithm is
/// required rather than the standard hasher.
pub fn content_fingerprint(content: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = FNV_OFFSET;
    for byte in content.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

/// Per-page classification between two crawls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeKind {
    Added,
    Removed,
    Modified,
    Unchanged,
    /// Present in both crawls but at least one side has no content hash
    /// (it was crawled before hashing existed), so we can't tell whether
    /// the content actually changed. Reported honestly rather than
    /// guessed as modified/unchanged.
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageDiff {
    pub url: String,
    pub title: String,
    pub kind: ChangeKind,
}

/// Summary of a crawl-to-crawl comparison: every page plus per-bucket
/// counts so the frontend can render totals without re-scanning.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiffResult {
    pub entries: Vec<PageDiff>,
    pub added: usize,
    pub removed: usize,
    pub modified: usize,
    pub unchanged: usize,
    pub unknown: usize,
}

impl DiffResult {
    fn push(&mut self, entry: PageDiff) {
        match entry.kind {
            ChangeKind::Added => self.added += 1,
            ChangeKind::Removed => self.removed += 1,
            ChangeKind::Modified => self.modified += 1,
            ChangeKind::Unchanged => self.unchanged += 1,
            ChangeKind::Unknown => self.unknown += 1,
        }
        self.entries.push(entry);
    }
}

/// Compare two crawls' page metadata by URL. Pure — no I/O.
///
/// A URL only in `new` is Added; only in `old` is Removed. A URL in both
/// is Modified/Unchanged when both sides carry a content hash, else
/// Unknown. Entries are ordered new-crawl-first (added/modified/…),
/// then the removed pages that only the old crawl had.
pub fn diff_page_metas(old: &[PageMeta], new: &[PageMeta]) -> DiffResult {
    let old_by_url: HashMap<&str, &PageMeta> =
        old.iter().map(|p| (p.url.as_str(), p)).collect();
    let new_by_url: HashMap<&str, &PageMeta> =
        new.iter().map(|p| (p.url.as_str(), p)).collect();

    let mut result = DiffResult::default();

    for page in new {
        let kind = match old_by_url.get(page.url.as_str()) {
            None => ChangeKind::Added,
            Some(old_page) => match (&old_page.content_hash, &page.content_hash) {
                (Some(a), Some(b)) if a == b => ChangeKind::Unchanged,
                (Some(_), Some(_)) => ChangeKind::Modified,
                _ => ChangeKind::Unknown,
            },
        };
        result.push(PageDiff {
            url: page.url.clone(),
            title: page.title.clone(),
            kind,
        });
    }

    for page in old {
        if !new_by_url.contains_key(page.url.as_str()) {
            result.push(PageDiff {
                url: page.url.clone(),
                title: page.title.clone(),
                kind: ChangeKind::Removed,
            });
        }
    }

    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LineTag {
    Equal,
    Insert,
    Delete,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub tag: LineTag,
    pub content: String,
}

/// Line-level text diff between the old and new Markdown of one page.
/// Used on demand when the user opens a Modified page in the diff view.
pub fn diff_lines(old: &str, new: &str) -> Vec<DiffLine> {
    use similar::{ChangeTag, TextDiff};
    let diff = TextDiff::from_lines(old, new);
    diff.iter_all_changes()
        .map(|change| {
            let tag = match change.tag() {
                ChangeTag::Equal => LineTag::Equal,
                ChangeTag::Insert => LineTag::Insert,
                ChangeTag::Delete => LineTag::Delete,
            };
            DiffLine {
                tag,
                content: change.value().trim_end_matches('\n').to_string(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(url: &str, hash: Option<&str>) -> PageMeta {
        PageMeta {
            url: url.to_string(),
            title: format!("Title for {url}"),
            status: 200,
            links_count: 0,
            content_hash: hash.map(|h| h.to_string()),
        }
    }

    #[test]
    fn fingerprint_is_stable_and_distinct() {
        // Fixed expected value pins the algorithm so a future refactor
        // that changes it (breaking persisted-hash comparability) fails.
        assert_eq!(content_fingerprint(""), "cbf29ce484222325");
        assert_eq!(content_fingerprint("hello"), content_fingerprint("hello"));
        assert_ne!(content_fingerprint("hello"), content_fingerprint("hello!"));
    }

    #[test]
    fn diff_detects_added_removed_modified_unchanged() {
        let old = vec![
            meta("https://x/a", Some("h1")),
            meta("https://x/b", Some("h2")),
            meta("https://x/gone", Some("h3")),
        ];
        let new = vec![
            meta("https://x/a", Some("h1")),        // unchanged
            meta("https://x/b", Some("h2-changed")), // modified
            meta("https://x/new", Some("h4")),       // added
        ];
        let d = diff_page_metas(&old, &new);
        assert_eq!(d.added, 1);
        assert_eq!(d.removed, 1);
        assert_eq!(d.modified, 1);
        assert_eq!(d.unchanged, 1);
        assert_eq!(d.unknown, 0);

        let kind = |url: &str| d.entries.iter().find(|e| e.url == url).map(|e| e.kind);
        assert_eq!(kind("https://x/new"), Some(ChangeKind::Added));
        assert_eq!(kind("https://x/gone"), Some(ChangeKind::Removed));
        assert_eq!(kind("https://x/b"), Some(ChangeKind::Modified));
        assert_eq!(kind("https://x/a"), Some(ChangeKind::Unchanged));
    }

    #[test]
    fn missing_hash_on_either_side_is_unknown() {
        let old = vec![meta("https://x/a", None)];
        let new = vec![meta("https://x/a", Some("h1"))];
        let d = diff_page_metas(&old, &new);
        assert_eq!(d.unknown, 1);
        assert_eq!(d.modified, 0);
        assert_eq!(d.unchanged, 0);
    }

    #[test]
    fn line_diff_tags_inserts_and_deletes() {
        let lines = diff_lines("alpha\nbeta\ngamma\n", "alpha\ndelta\ngamma\n");
        assert!(lines.iter().any(|l| l.tag == LineTag::Delete && l.content == "beta"));
        assert!(lines.iter().any(|l| l.tag == LineTag::Insert && l.content == "delta"));
        assert!(lines.iter().any(|l| l.tag == LineTag::Equal && l.content == "alpha"));
    }
}
