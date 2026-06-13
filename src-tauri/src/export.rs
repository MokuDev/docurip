use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    MdFiles,
    PdfFiles,
    MergedMd,
    MergedPdf,
}

pub fn copy_md_files(src_dir: &Path, dst_dir: &Path) -> anyhow::Result<()> {
    for entry in walk_dir(src_dir)? {
        let relative = entry.strip_prefix(src_dir)?;
        let dst_path = dst_dir.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&dst_path)?;
        } else if entry.extension().and_then(|e| e.to_str()) == Some("md") {
            if let Some(parent) = dst_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&entry, &dst_path)?;
        }
    }
    Ok(())
}

fn walk_dir(dir: &Path) -> anyhow::Result<Vec<std::path::PathBuf>> {
    let mut result = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                result.push(path);
            }
        }
    }
    Ok(result)
}

pub fn merge_md_files(src_dir: &Path, dst_file: &Path) -> anyhow::Result<()> {
    let mut files = walk_dir(src_dir)?
        .into_iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
        .collect::<Vec<_>>();
    files.sort();

    let mut merged = String::new();
    for (i, file) in files.iter().enumerate() {
        let content = std::fs::read_to_string(file)?;
        if i > 0 {
            merged.push_str("\n\n---\n\n");
        }
        merged.push_str(&content);
    }

    if let Some(parent) = dst_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(dst_file, merged)?;
    Ok(())
}

#[cfg(feature = "headless")]
fn md_to_html(md_content: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(md_content, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    format!(
        r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><style>
body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; max-width: 800px; margin: 0 auto; padding: 2rem; line-height: 1.6; color: #1a1a1a; }}
pre {{ background: #f5f5f5; padding: 1rem; overflow-x: auto; border-radius: 4px; }}
code {{ background: #f5f5f5; padding: 0.2em 0.4em; border-radius: 3px; font-size: 0.9em; }}
pre code {{ background: none; padding: 0; }}
h1, h2, h3 {{ margin-top: 1.5em; }}
hr {{ border: none; border-top: 1px solid #ddd; margin: 2rem 0; }}
table {{ border-collapse: collapse; width: 100%; }}
th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
th {{ background: #f5f5f5; }}
</style></head>
<body>{}</body>
</html>"#,
        html_output
    )
}

#[cfg(feature = "headless")]
pub fn export_pdf_files(src_dir: &Path, dst_dir: &Path) -> anyhow::Result<()> {
    use headless_chrome::{Browser, LaunchOptions};

    let mut files = walk_dir(src_dir)?
        .into_iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
        .collect::<Vec<_>>();

    if files.is_empty() {
        return Ok(());
    }

    files.sort();

    let browser = Browser::new(LaunchOptions {
        headless: true,
        ..Default::default()
    })?;

    let tmp_dir = tempfile::TempDir::new()?;

    for file in files {
        let relative = file.strip_prefix(src_dir)?;
        let dst_path = dst_dir.join(relative).with_extension("pdf");
        if let Some(parent) = dst_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let md_content = std::fs::read_to_string(&file)?;
        let html_content = md_to_html(&md_content);
        let tmp_html = tmp_dir.path().join(format!("{}.html", relative.display()));
        if let Some(parent) = tmp_html.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&tmp_html, &html_content)?;

        let tab = browser.new_tab()?;
        let file_url = format!("file:///{}", tmp_html.display().to_string().replace('\\', "/"));
        tab.navigate_to(&file_url)?;
        tab.wait_until_navigated()?;
        let pdf_bytes = tab.print_to_pdf(None)?;
        std::fs::write(&dst_path, pdf_bytes)?;
        drop(tab);
    }

    Ok(())
}

#[cfg(not(feature = "headless"))]
pub fn export_pdf_files(_src_dir: &Path, _dst_dir: &Path) -> anyhow::Result<()> {
    anyhow::bail!("PDF export requires headless Chrome support. Rebuild with --features headless.")
}

#[cfg(feature = "headless")]
pub fn export_merged_pdf(src_dir: &Path, dst_file: &Path) -> anyhow::Result<()> {
    use headless_chrome::{Browser, LaunchOptions};

    let mut files = walk_dir(src_dir)?
        .into_iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
        .collect::<Vec<_>>();
    files.sort();

    let mut merged_md = String::new();
    for (i, file) in files.iter().enumerate() {
        let content = std::fs::read_to_string(file)?;
        if i > 0 {
            merged_md.push_str("\n\n---\n\n");
        }
        merged_md.push_str(&content);
    }

    let html_content = md_to_html(&merged_md);
    let tmp_dir = tempfile::TempDir::new()?;
    let tmp_html = tmp_dir.path().join("merged.html");
    std::fs::write(&tmp_html, &html_content)?;

    let browser = Browser::new(LaunchOptions {
        headless: true,
        ..Default::default()
    })?;
    let tab = browser.new_tab()?;
    let file_url = format!("file:///{}", tmp_html.display().to_string().replace('\\', "/"));
    tab.navigate_to(&file_url)?;
    tab.wait_until_navigated()?;
    let pdf_bytes = tab.print_to_pdf(None)?;

    if let Some(parent) = dst_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(dst_file, pdf_bytes)?;
    Ok(())
}

#[cfg(not(feature = "headless"))]
pub fn export_merged_pdf(_src_dir: &Path, _dst_file: &Path) -> anyhow::Result<()> {
    anyhow::bail!("PDF export requires headless Chrome support. Rebuild with --features headless.")
}

pub fn zip_directory(src_dir: &Path, dst_file: &Path) -> anyhow::Result<()> {
    let file = File::create(dst_file)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip_directory_inner(src_dir, src_dir, &mut zip, &options)?;
    zip.finish()?;
    Ok(())
}

fn zip_directory_inner(
    base: &Path,
    current: &Path,
    zip: &mut zip::ZipWriter<File>,
    options: &SimpleFileOptions,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.strip_prefix(base)?.to_string_lossy();
        if path.is_file() {
            zip.start_file(name, *options)?;
            let mut f = File::open(&path)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        } else if path.is_dir() {
            zip.add_directory(name, *options)?;
            zip_directory_inner(base, &path, zip, options)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn export_format_serde_roundtrip() {
        let formats = vec![
            ExportFormat::MdFiles,
            ExportFormat::PdfFiles,
            ExportFormat::MergedMd,
            ExportFormat::MergedPdf,
        ];
        for fmt in &formats {
            let json = serde_json::to_string(fmt).unwrap();
            let back: ExportFormat = serde_json::from_str(&json).unwrap();
            assert!(matches!(
                (fmt, &back),
                (ExportFormat::MdFiles, ExportFormat::MdFiles)
                    | (ExportFormat::PdfFiles, ExportFormat::PdfFiles)
                    | (ExportFormat::MergedMd, ExportFormat::MergedMd)
                    | (ExportFormat::MergedPdf, ExportFormat::MergedPdf)
            ));
        }
    }

    #[test]
    fn export_format_json_values() {
        assert_eq!(serde_json::to_string(&ExportFormat::MdFiles).unwrap(), "\"md_files\"");
        assert_eq!(serde_json::to_string(&ExportFormat::PdfFiles).unwrap(), "\"pdf_files\"");
        assert_eq!(serde_json::to_string(&ExportFormat::MergedMd).unwrap(), "\"merged_md\"");
        assert_eq!(serde_json::to_string(&ExportFormat::MergedPdf).unwrap(), "\"merged_pdf\"");
    }

    #[test]
    fn copy_md_files_preserves_structure() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();
        let sub = src.path().join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(src.path().join("a.md"), b"# A").unwrap();
        std::fs::write(sub.join("b.md"), b"# B").unwrap();
        std::fs::write(src.path().join("c.txt"), b"ignored").unwrap();

        copy_md_files(src.path(), dst.path()).unwrap();

        assert!(dst.path().join("a.md").exists());
        assert!(dst.path().join("sub").join("b.md").exists());
        assert!(!dst.path().join("c.txt").exists());
    }

    #[test]
    fn merge_md_files_concatenates_with_separator() {
        let src = TempDir::new().unwrap();
        std::fs::write(src.path().join("a.md"), b"# A").unwrap();
        std::fs::write(src.path().join("b.md"), b"# B").unwrap();

        let dst = TempDir::new().unwrap();
        let out = dst.path().join("merged.md");
        merge_md_files(src.path(), &out).unwrap();

        let content = std::fs::read_to_string(&out).unwrap();
        assert!(content.contains("# A"));
        assert!(content.contains("# B"));
        assert!(content.contains("---"));
    }
}
