use std::path::Path;
use anyhow::Context;
use lopdf::Document;

use super::{ensure_output_dir, sanitize_filename, ImportResult};

pub fn import_pdf(file_path: &Path, output_dir: &Path) -> anyhow::Result<ImportResult> {
    let images_dir = ensure_output_dir(output_dir)?;

    let text = pdf_extract::extract_text(file_path)
        .with_context(|| format!("Failed to extract text from {}", file_path.display()))?;

    let mut image_count = 0usize;
    let mut image_refs = Vec::new();

    if let Ok(doc) = Document::load(file_path) {
        for (page_num, page_id) in doc.page_iter().enumerate() {
            if let Ok(images) = doc.get_page_images(page_id) {
                for img in images {
                    let is_jpeg = img
                        .filters
                        .as_ref()
                        .map(|f| f.iter().any(|name| name == "DCTDecode"))
                        .unwrap_or(false);

                    if !is_jpeg {
                        continue;
                    }

                    if img.content.is_empty() {
                        continue;
                    }

                    image_count += 1;
                    let img_name = format!("page{}_{}.jpg", page_num + 1, image_count);
                    let img_path = images_dir.join(&img_name);
                    if std::fs::write(&img_path, img.content).is_ok() {
                        image_refs
                            .push(format!("![Image {}](images/{})", image_count, img_name));
                    }
                }
            }
        }
    }

    let file_stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let title = file_stem.to_string();
    let safe_name = sanitize_filename(file_stem);

    let pages: Vec<&str> = text.split('\u{0C}').collect();
    let page_count = pages.len().max(1);

    let mut markdown = String::new();
    markdown.push_str(&format!("# {}\n\n", title));

    if !image_refs.is_empty() {
        markdown.push_str("## Extracted Images\n\n");
        for img_ref in &image_refs {
            markdown.push_str(img_ref);
            markdown.push_str("\n\n");
        }
        markdown.push_str("---\n\n");
    }

    for (i, page) in pages.iter().enumerate() {
        let trimmed = page.trim();
        if trimmed.is_empty() {
            continue;
        }
        if pages.len() > 1 {
            markdown.push_str(&format!("## Page {}\n\n", i + 1));
        }
        markdown.push_str(trimmed);
        markdown.push_str("\n\n");
    }

    let md_path = output_dir.join(format!("{}.md", safe_name));
    std::fs::write(&md_path, &markdown)?;

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
    fn import_pdf_missing_file_returns_error() {
        let output = TempDir::new().unwrap();
        let result = import_pdf(&PathBuf::from("/nonexistent.pdf"), output.path());
        assert!(result.is_err());
    }
}
