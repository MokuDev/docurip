# Multi-Format Export — Design Spec

**Date:** 2026-06-13
**Status:** Approved

## Problem

Export currently only supports ZIP download of the raw output directory. Users need:
- Individual .md/.pdf files in a chosen folder
- Single merged .md/.pdf file combining all crawled pages

## Export Formats

| Format | Enum Value | Description |
|--------|-----------|-------------|
| MD Files | `md_files` | Copy .md files to destination, preserving folder structure |
| PDF Files | `pdf_files` | Per-page MD→HTML→PDF via headless Chrome |
| Merged MD | `merged_md` | All pages concatenated into one .md with `---` separators |
| Merged PDF | `merged_pdf` | All pages in one HTML doc → single PDF via headless Chrome |

## Backend

### New Command

```
export_job_v2(job_id: String, format: ExportFormat, destination: String) → Result<String, String>
```

- Registered in `lib.rs` alongside existing commands
- Returns destination path on success

### ExportFormat Enum (`export.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    MdFiles,
    PdfFiles,
    MergedMd,
    MergedPdf,
}
```

### Implementation Strategy

**md_files**: Walk output_dir, copy all .md files to destination preserving relative paths. Use `std::fs::copy`.

**pdf_files**: For each .md file in output_dir:
1. Read .md content
2. Convert MD→HTML (simple wrapper with `<html><body>` + markdown-to-html)
3. Launch headless Chrome, navigate to data URI, print-to-PDF
4. Write PDF to destination with same relative path but `.pdf` extension

**merged_md**: Read all .md files in sorted order, concatenate with `\n\n---\n\n` separator, write single file.

**merged_pdf**: Same as merged_md but build one HTML document, then headless Chrome → single PDF.

### PDF Rendering

- Uses headless Chrome (already feature-gated: `#[cfg(feature = "headless")]`)
- Without feature: return clear error `"PDF export requires headless Chrome support. Rebuild with --features headless."`
- Chrome launcher: reuse `headless_chrome::Browser::new(headless_chrome::LaunchOptions { headless: true, .. })`
- Print-to-PDF via `page.print_to_pdf(None)` 

### Error Cases

| Condition | Error Message |
|-----------|--------------|
| Job not found | `"Job not found: {id}"` |
| Output dir missing | `"Output directory not found for job {id}"` |
| No headless feature | `"PDF export requires headless Chrome support"` |
| Chrome start failure | `"Failed to start Chrome: {err}"` |
| Write failure | `"Failed to write export: {err}"` |

## Frontend

### Export Modal (`ExportModal.tsx`)

Triggered by the existing export button in `History.tsx`. Modal contains:

1. **Format radio group**: 4 options with descriptions
2. **Destination picker**: Button to open Tauri `save_dialog` in directory mode, displays chosen path
3. **Export button**: Disabled until destination selected; loading spinner during export
4. **Cancel button**

### PDF Availability

- Detect headless Chrome support via a new Tauri command:
  ```rust
  #[tauri::command]
  pub fn check_headless_support() -> bool {
      cfg!(feature = "headless")
  }
  ```
- PDF radio options grayed out + tooltip when unsupported

### TypeScript Types

```typescript
type ExportFormat = 'md_files' | 'pdf_files' | 'merged_md' | 'merged_pdf'

interface ExportModalProps {
  jobId: string
  onClose: () => void
}
```

### Integration

- Replace `handleExport` in `History.tsx` to open modal instead of direct download
- Toast on success/error via existing `useToasts` hook
- Existing `export_job` / `export_job_zip` commands remain for backward compat (ZIP download)

## Testing

### Unit Tests (Rust)

- `ExportFormat` serde round-trip (all 4 variants)
- `merged_md` concatenation logic: correct separator, correct ordering
- File copy logic: preserves structure, handles missing files

### Integration Tests

- Full PDF pipeline test behind `#[cfg(feature = "headless")]`: create temp .md files → export → verify .pdf exists and has content
- Error paths: missing job, missing output dir

### Frontend

- Manual testing via `npm run tauri dev`
- Verify modal opens, format selection works, export completes

## Non-Goals

- Custom PDF styling/theming (future enhancement)
- Progress reporting per-page during PDF export (future enhancement)
- Batch export of multiple jobs at once
