use std::path::Path;
use anyhow::Context;
use epub::doc::EpubDoc;

use super::{ensure_output_dir, sanitize_filename, ImportResult};

pub fn import_epub(file_path: &Path, output_dir: &Path) -> anyhow::Result<ImportResult> {
    let images_dir = ensure_output_dir(output_dir)?;

    let mut doc = EpubDoc::new(file_path)
        .map_err(|e| anyhow::anyhow!("Failed to open EPUB: {}", e))?;

    let title = doc
        .get_title()
        .unwrap_or_else(|| {
            file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("document")
                .to_string()
        });

    let safe_name = sanitize_filename(&title);

    let mut image_count = 0usize;
    let mut image_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    let resource_ids: Vec<(String, String)> = doc
        .resources
        .iter()
        .map(|(id, res)| (id.clone(), res.mime.clone()))
        .collect();

    for (res_id, mime) in &resource_ids {
        if !mime.starts_with("image/") {
            continue;
        }
        let ext = match mime.as_str() {
            "image/jpeg" | "image/jpg" => "jpg",
            "image/png" => "png",
            "image/gif" => "gif",
            "image/svg+xml" => "svg",
            "image/webp" => "webp",
            _ => "bin",
        };

        if let Some((data, _)) = doc.get_resource(res_id) {
            image_count += 1;
            let img_name = format!("{}_{}.{}", safe_name, image_count, ext);
            let img_path = images_dir.join(&img_name);
            if std::fs::write(&img_path, &data).is_ok() {
                let orig_filename = doc
                    .resources
                    .get(res_id)
                    .map(|r| {
                        r.path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !orig_filename.is_empty() {
                    image_map.insert(orig_filename, format!("images/{}", img_name));
                }
            }
        }
    }

    let spine_idrefs: Vec<String> = doc.spine.iter().map(|s| s.idref.clone()).collect();
    let mut chapters = Vec::new();

    for spine_id in &spine_idrefs {
        if let Some((content, _mime)) = doc.get_resource_str(spine_id) {
            let mut md = html2md::parse_html(&content);
            for (orig_name, local_path) in &image_map {
                md = md.replace(orig_name, local_path);
            }
            chapters.push(md);
        }
    }

    let page_count = chapters.len().max(1);

    let mut markdown = String::new();
    markdown.push_str(&format!("# {}\n\n", title));

    for (i, chapter) in chapters.iter().enumerate() {
        let trimmed = chapter.trim();
        if trimmed.is_empty() {
            continue;
        }
        if i > 0 {
            markdown.push_str("\n\n---\n\n");
        }
        markdown.push_str(trimmed);
        markdown.push_str("\n\n");
    }

    let md_path = output_dir.join(format!("{}.md", safe_name));
    std::fs::write(&md_path, &markdown)
        .with_context(|| format!("Failed to write markdown to {}", md_path.display()))?;

    Ok(ImportResult {
        markdown_path: md_path.to_string_lossy().to_string(),
        images_dir: if image_count > 0 {
            Some(images_dir.to_string_lossy().to_string())
        } else {
            None
        },
        page_count,
        image_count,
        title,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn import_epub_missing_file_returns_error() {
        let output = TempDir::new().unwrap();
        let result = import_epub(&PathBuf::from("/nonexistent.epub"), output.path());
        assert!(result.is_err());
    }
}
