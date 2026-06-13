# Multi-Format Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the single ZIP export with 4 export formats: individual MD files, individual PDF files, merged MD, merged PDF.

**Architecture:** A new `export_job_v2` Rust command dispatches on an `ExportFormat` enum. PDF rendering uses the existing headless Chrome feature gate. Frontend gets an `ExportModal` component triggered from the History view.

**Tech Stack:** Rust, Tauri v2, headless_chrome (feature-gated), React 19, TypeScript, @phosphor-icons/react, framer-motion

---

### Task 1: ExportFormat Enum + Backend Core Logic

**Files:**
- Modify: `src-tauri/src/export.rs`
- Modify: `src-tauri/src/export.rs` (add `#[cfg(test)] mod tests`)

- [ ] **Step 1: Add `ExportFormat` enum to `export.rs`**

Add after the existing `use` statements at the top of `src-tauri/src/export.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    MdFiles,
    PdfFiles,
    MergedMd,
    MergedPdf,
}
```

- [ ] **Step 2: Add `copy_md_files` function**

Add after `zip_directory_inner` function (after line 37):

```rust
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
```

- [ ] **Step 3: Add `merge_md_files` function**

Add after `copy_md_files`:

```rust
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
```

- [ ] **Step 4: Add unit tests for `ExportFormat` serde + `copy_md_files` + `merge_md_files`**

Add at the bottom of `src-tauri/src/export.rs`:

```rust
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
```

- [ ] **Step 5: Run tests**

Run: `cd src-tauri && cargo test --lib export`
Expected: All 4 new tests PASS, existing zip tests still PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/export.rs
git commit -m "feat(export): add ExportFormat enum + md_files and merged_md export logic"
```

---

### Task 2: PDF Export Logic (feature-gated)

**Files:**
- Modify: `src-tauri/src/export.rs`

- [ ] **Step 1: Add `pulldown-cmark` dependency**

Add to `src-tauri/Cargo.toml` in `[dependencies]`:

```toml
pulldown-cmark = "0.12"
```

Run: `cd src-tauri && cargo check`
Expected: Compiles successfully.

- [ ] **Step 2: Add `md_to_html` helper function**

Add after `merge_md_files` in `src-tauri/src/export.rs`:

```rust
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
```

- [ ] **Step 3: Add `export_pdf_files` function (headless-gated)**

Add after `md_to_html`. Uses temp HTML files + `file://` URLs for Chrome navigation:

```rust
#[cfg(feature = "headless")]
pub fn export_pdf_files(src_dir: &Path, dst_dir: &Path) -> anyhow::Result<()> {
    use headless_chrome::{Browser, LaunchOptions};

    let browser = Browser::new(LaunchOptions {
        headless: true,
        ..Default::default()
    })?;

    let files = walk_dir(src_dir)?
        .into_iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
        .collect::<Vec<_>>();

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
    }

    Ok(())
}

#[cfg(not(feature = "headless"))]
pub fn export_pdf_files(_src_dir: &Path, _dst_dir: &Path) -> anyhow::Result<()> {
    anyhow::bail!("PDF export requires headless Chrome support. Rebuild with --features headless.")
}
```

- [ ] **Step 4: Add `export_merged_pdf` function (headless-gated)**

```rust
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
```

- [ ] **Step 5: Run all export tests**

Run: `cd src-tauri && cargo test --lib export`
Expected: All tests PASS.

- [ ] **Step 6: Run headless feature check**

Run: `cd src-tauri && cargo check --features headless`
Expected: Compiles successfully (or fails with missing headless_chrome binary — that's OK, we just check Rust compilation).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/export.rs src-tauri/Cargo.toml
git commit -m "feat(export): add PDF export functions with headless Chrome feature gate"
```

---

### Task 3: `export_job_v2` Command + `check_headless_support`

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `export_job_v2` command to `commands.rs`**

Add after the existing `export_job` function (after line 408):

```rust
#[tauri::command]
pub async fn export_job_v2(
    job_id: String,
    format: crate::export::ExportFormat,
    destination: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let job = {
        let jobs = state.active_jobs.read().await;
        if let Some(handle) = jobs.get(&job_id) {
            handle.job.read().await.clone()
        } else {
            let jobs = state.persisted_jobs.read().await;
            jobs.get(&job_id).cloned().ok_or("Job not found")?
        }
    };

    let output_dir = std::path::PathBuf::from(&job.config.output_dir);
    if !output_dir.exists() {
        return Err("Output directory not found for job".to_string());
    }

    let dest = std::path::PathBuf::from(&destination);

    match format {
        crate::export::ExportFormat::MdFiles => {
            crate::export::copy_md_files(&output_dir, &dest)
                .map_err(|e| format!("Export failed: {}", e))?;
        }
        crate::export::ExportFormat::PdfFiles => {
            crate::export::export_pdf_files(&output_dir, &dest)
                .map_err(|e| format!("PDF export failed: {}", e))?;
        }
        crate::export::ExportFormat::MergedMd => {
            let out_file = dest.join(format!("{}-merged.md", job_id));
            crate::export::merge_md_files(&output_dir, &out_file)
                .map_err(|e| format!("Export failed: {}", e))?;
        }
        crate::export::ExportFormat::MergedPdf => {
            let out_file = dest.join(format!("{}-merged.pdf", job_id));
            crate::export::export_merged_pdf(&output_dir, &out_file)
                .map_err(|e| format!("PDF export failed: {}", e))?;
        }
    }

    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub fn check_headless_support() -> bool {
    cfg!(feature = "headless")
}
```

- [ ] **Step 2: Add `tempfile` to `[dependencies]` in Cargo.toml**

`tempfile` is currently only in `[dev-dependencies]`. PDF export needs it at runtime.

Move `tempfile = "3"` from `[dev-dependencies]` to `[dependencies]` in `src-tauri/Cargo.toml`.

- [ ] **Step 3: Register new commands in `lib.rs`**

In `src-tauri/src/lib.rs`, add to the `invoke_handler` list (after `commands::export_job`):

```rust
commands::export_job_v2,
commands::check_headless_support,
```

- [ ] **Step 4: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: Compiles successfully.

- [ ] **Step 5: Run all tests**

Run: `cd src-tauri && cargo test`
Expected: All existing tests still PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat(export): add export_job_v2 command and check_headless_support"
```

---

### Task 4: Frontend TypeScript Types

**Files:**
- Modify: `src/types/index.ts`

- [ ] **Step 1: Add `ExportFormat` type**

Add at the end of `src/types/index.ts`:

```typescript
export type ExportFormat = 'md_files' | 'pdf_files' | 'merged_md' | 'merged_pdf';

export interface ExportOption {
  format: ExportFormat;
  label: string;
  description: string;
  requiresHeadless: boolean;
}

export const EXPORT_OPTIONS: ExportOption[] = [
  {
    format: 'md_files',
    label: 'Markdown Files',
    description: 'Individual .md files in folder structure',
    requiresHeadless: false,
  },
  {
    format: 'merged_md',
    label: 'Merged Markdown',
    description: 'All pages combined into one .md file',
    requiresHeadless: false,
  },
  {
    format: 'pdf_files',
    label: 'PDF Files',
    description: 'Individual .pdf files per page',
    requiresHeadless: true,
  },
  {
    format: 'merged_pdf',
    label: 'Merged PDF',
    description: 'All pages in one PDF document',
    requiresHeadless: true,
  },
];
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `npm run build`
Expected: No TypeScript errors.

- [ ] **Step 3: Commit**

```bash
git add src/types/index.ts
git commit -m "feat(export): add ExportFormat types and export options config"
```

---

### Task 5: ExportModal Component

**Files:**
- Create: `src/components/ExportModal.tsx`

- [ ] **Step 1: Create `ExportModal.tsx`**

Create `src/components/ExportModal.tsx`:

```tsx
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { motion, AnimatePresence } from 'framer-motion';
import { X, Download, FolderOpen, SpinnerGap } from '@phosphor-icons/react';
import { useToasts } from '../hooks/useToasts';
import { EXPORT_OPTIONS } from '../types';
import type { ExportFormat } from '../types';

interface ExportModalProps {
  jobId: string;
  onClose: () => void;
}

export function ExportModal({ jobId, onClose }: ExportModalProps) {
  const [selectedFormat, setSelectedFormat] = useState<ExportFormat>('md_files');
  const [destination, setDestination] = useState('');
  const [headlessSupported, setHeadlessSupported] = useState(false);
  const [exporting, setExporting] = useState(false);
  const { addToast } = useToasts();

  useEffect(() => {
    invoke<boolean>('check_headless_support').then(setHeadlessSupported).catch(() => {});
  }, []);

  const handlePickDestination = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Export Destination',
      });
      if (selected) {
        setDestination(selected);
      }
    } catch (err) {
      console.error('Failed to open directory picker', err);
    }
  };

  const handleExport = async () => {
    if (!destination) return;
    setExporting(true);
    try {
      await invoke('export_job_v2', {
        jobId,
        format: selectedFormat,
        destination,
      });
      addToast(`Export completed: ${selectedFormat}`, 'success');
      onClose();
    } catch (err) {
      addToast(`Export failed: ${err}`, 'error');
    } finally {
      setExporting(false);
    }
  };

  const isDisabled = (requiresHeadless: boolean) => requiresHeadless && !headlessSupported;

  return (
    <AnimatePresence>
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        className="fixed inset-0 bg-black/40 z-40"
        onClick={onClose}
      />
      <motion.div
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        exit={{ opacity: 0, scale: 0.95 }}
        transition={{ type: 'spring', damping: 25, stiffness: 300 }}
        className="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-[440px] bg-deepVoid border border-abyssal/50 rounded-xl z-50 shadow-2xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-abyssal/50">
          <h2 className="text-ghost font-semibold text-base">Export Job</h2>
          <button
            onClick={onClose}
            className="p-1.5 text-charcoal hover:text-ghost hover:bg-abyssal rounded transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        {/* Content */}
        <div className="p-5 space-y-4">
          {/* Format selection */}
          <div className="space-y-2">
            <label className="text-xs text-charcoal uppercase tracking-wider">Format</label>
            <div className="grid grid-cols-2 gap-2">
              {EXPORT_OPTIONS.map((opt) => {
                const disabled = isDisabled(opt.requiresHeadless);
                return (
                  <button
                    key={opt.format}
                    onClick={() => !disabled && setSelectedFormat(opt.format)}
                    disabled={disabled}
                    className={`p-3 rounded-lg border text-left transition-all ${
                      selectedFormat === opt.format
                        ? 'border-accentGreen/60 bg-accentGreen/10'
                        : disabled
                        ? 'border-abyssal/30 bg-surface/20 opacity-40 cursor-not-allowed'
                        : 'border-abyssal/50 bg-surface/30 hover:border-abyssal hover:bg-surface/50'
                    }`}
                  >
                    <p className={`text-sm font-medium ${selectedFormat === opt.format ? 'text-accentGreen' : 'text-ghost'}`}>
                      {opt.label}
                    </p>
                    <p className="text-[10px] text-charcoal mt-0.5">{opt.description}</p>
                    {disabled && (
                      <p className="text-[10px] text-crimson mt-1">Requires headless Chrome</p>
                    )}
                  </button>
                );
              })}
            </div>
          </div>

          {/* Destination picker */}
          <div className="space-y-2">
            <label className="text-xs text-charcoal uppercase tracking-wider">Destination</label>
            <div className="flex items-center space-x-2">
              <div className="flex-1 bg-surface/30 border border-abyssal/50 rounded-md px-3 py-2 text-sm text-ghost truncate min-h-[36px]">
                {destination || 'No folder selected'}
              </div>
              <button
                onClick={handlePickDestination}
                className="p-2 text-charcoal hover:text-ghost hover:bg-abyssal rounded transition-colors"
                title="Pick folder"
              >
                <FolderOpen size={18} />
              </button>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end space-x-3 px-5 py-4 border-t border-abyssal/50">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm text-charcoal hover:text-ghost transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={handleExport}
            disabled={!destination || exporting}
            className="flex items-center gap-2 px-4 py-2 text-sm bg-accentGreen/20 text-accentGreen border border-accentGreen/30 rounded-md hover:bg-accentGreen/30 transition-all disabled:opacity-40 disabled:cursor-not-allowed"
          >
            {exporting ? (
              <>
                <SpinnerGap size={14} className="animate-spin" />
                Exporting...
              </>
            ) : (
              <>
                <Download size={14} />
                Export
              </>
            )}
          </button>
        </div>
      </motion.div>
    </AnimatePresence>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `npm run build`
Expected: No TypeScript errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/ExportModal.tsx
git commit -m "feat(export): add ExportModal component with format picker"
```

---

### Task 6: Wire ExportModal into History.tsx

**Files:**
- Modify: `src/views/History.tsx`

- [ ] **Step 1: Add import for ExportModal**

Add at the top of `src/views/History.tsx` after existing imports:

```typescript
import { ExportModal } from '../components/ExportModal';
```

- [ ] **Step 2: Add state for export modal**

Inside the `HistoryView` function, after the existing `useState` declarations (after line 27):

```typescript
const [exportJobId, setExportJobId] = useState<string | null>(null);
```

- [ ] **Step 3: Replace `handleExport` function**

Replace the existing `handleExport` function (lines 54-61) with:

```typescript
const handleExport = (jobId: string) => {
  setExportJobId(jobId);
};
```

- [ ] **Step 4: Add ExportModal render before closing `</div>`**

Add before the final closing tag of the component:

```tsx
{exportJobId && (
  <ExportModal
    jobId={exportJobId}
    onClose={() => setExportJobId(null)}
  />
)}
```

- [ ] **Step 5: Verify TypeScript compiles**

Run: `npm run build`
Expected: No TypeScript errors.

- [ ] **Step 6: Commit**

```bash
git add src/views/History.tsx
git commit -m "feat(export): wire ExportModal into History view"
```

---

### Task 7: Full Build Verification

**Files:**
- None (verification only)

- [ ] **Step 1: Run backend checks**

Run: `cd src-tauri && cargo check && cargo test`
Expected: `cargo check` clean, all tests PASS.

- [ ] **Step 2: Run frontend build**

Run: `npm run build`
Expected: No errors, successful build.

- [ ] **Step 3: Run headless feature check**

Run: `cd src-tauri && cargo check --features headless`
Expected: Compiles (may fail if headless_chrome binary not installed — that's OK for CI without Chrome).

- [ ] **Step 4: Verify old export still works**

Confirm `export_job` and `export_job_zip` commands still registered and compiled. No removal of backward-compatible code.

- [ ] **Step 5: Final commit if needed**

If any fixes were needed during verification, commit them:

```bash
git add -A
git commit -m "fix(export): address build verification issues"
```
