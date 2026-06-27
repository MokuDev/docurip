# Spec: Output-Dir-System-Redesign

## Objective

Rework the output directory system so that:
1. Each crawl writes to a dedicated subfolder under the global output dir (e.g., `~/.docurip/docs.example.com/2026-06-14-abc123/`)
2. The NewCrawl view uses a native folder picker instead of a plain text input
3. History view shows the output path and provides quick access
4. ExportModal defaults to "export next to crawl output" instead of requiring a manual picker every time
5. Dashboard shows recent exports with one-click open

## Current State

| Component | Current behavior | Problem |
|-----------|-----------------|---------|
| `AppSettings.output_dir` | Global path (default `~/.docurip`) | All crawls dump into the same flat dir |
| `CrawlConfig.output_dir` | Per-crawl override (empty = use global) | Plain text input, no picker, no validation |
| `Orchestrator` | Resolves to `config.output_dir || settings.output_dir` | No subfolder organization |
| `NewCrawl.tsx` | Text input with placeholder "Leave empty for default" | No folder picker, easy to mistype |
| `Settings.tsx` | Text input for default output dir | No folder picker |
| `ExportModal` | Manual destination picker every time | Tedious for repeated exports |
| `History.tsx` | "Open folder" button uses `job.config.outputDir` | Only works if output dir exists |
| `export_job_zip` | Zips from `job.config.output_dir` | No organized location for zip |
| `exports.rs` | Lists zips from `app_data_dir/exports/` | Separate from actual crawl output |

## Design Decisions

### Output folder structure
```
{settings.output_dir}/
  {domain}/
    {YYYY-MM-DD}-{job_id_short}/
      index.md
      getting-started.md
      assets/
        logo.png
        styles.css
```
- Domain extracted from crawl URL (e.g., `docs.example.com`)
- Date prefix for chronological sorting
- Job ID short (first 8 chars) for uniqueness
- This keeps crawls organized without user intervention

### Backward compatibility
- `CrawlConfig.output_dir` stays in the schema but becomes read-only after crawl starts (it stores the resolved path)
- Empty `CrawlConfig.output_dir` → orchestrator creates `{domain}/{date}-{id}` subfolder under `settings.output_dir`
- Non-empty `CrawlConfig.output_dir` → use as-is (user explicitly chose a path via folder picker)
- Existing persisted jobs keep their old `output_dir` values — no migration needed

### Export destination
- ExportModal pre-fills destination with the job's output directory
- User can still change it via folder picker
- "Export here" button (one-click) exports to the crawl's own output dir

## Files to Modify

### Rust Backend
1. **`src-tauri/src/crawler/orchestrator.rs`** — Generate subfolder path from domain + date + job_id
2. **`src-tauri/src/commands.rs`** — Pass job_id to orchestrator for subfolder naming
3. **`src-tauri/src/settings/config.rs`** — No changes needed (output_dir stays)

### Frontend
4. **`src/views/NewCrawl.tsx`** — Replace text input with folder picker button + display
5. **`src/views/Settings.tsx`** — Replace text input with folder picker button + display
6. **`src/components/ExportModal.tsx`** — Pre-fill destination from job output dir
7. **`src/views/History.tsx`** — Show output path, ensure "Open folder" works
8. **`src/views/Dashboard.tsx`** — Add recent exports section (optional, lower priority)

## Success Criteria

> **Status (verifiziert gegen v0.3.3):** Umgesetzt, aber mit zwei bewussten Spec-Abweichungen (siehe CHANGELOG v0.3.2): (1) Subfolder vereinfacht zu `{domain}` (ohne Date/ID), Inhalt unterteilt in `main|zip|formats`. (2) NewCrawl-Picker ganz entfernt, Output-Dir nur noch in Settings.

- [x] Neuer Crawl erzeugt automatisch `{outputDir}/{domain}/{main|zip|formats}` (abweichend: kein Date/ID-Suffix)
- [ ] NewCrawl mit Folder-Picker — **bewusst entfernt** (Settings-only)
- [x] Settings mit nativen Folder-Picker (`Settings.tsx:182`)
- [x] ExportModal benötigt kein Destination-Picker mehr (auto-destination, weiter vereinfacht als ursprünglich geplant)
- [x] History "Open folder" funktioniert, öffnet `main/` (History.tsx:61-64)
- [x] Alle Tests bestehen (`cargo test` grün laut CHANGELOG)
- [x] Keine Breaking Changes an persistierten Jobs

## Resolved Questions

1. **Folder picker scope:** Picks the parent directory only. The orchestrator always creates the `{domain}/{date}-{id}` subfolder. No manual override for subfolder name.
2. **Dashboard exports:** No Dashboard changes for now — History view is sufficient.
