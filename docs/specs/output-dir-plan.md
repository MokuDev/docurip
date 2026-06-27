# Implementation Plan: Output-Dir-System-Redesign

## Overview

Rework the output directory system so each crawl gets an organized subfolder (`{domain}/{date}-{id}`), the UI uses native folder pickers instead of text inputs, and exports pre-fill from the job's output path.

> **Status (verifiziert gegen v0.3.3):** Großteil umgesetzt, aber die finale Struktur weicht vom ursprünglichen Plan ab — siehe CHANGELOG v0.3.2: vereinfacht zu `{outputDir}/{domain}/main|zip|formats/` ohne Date/ID-Suffix; per-Crawl-Picker in NewCrawl wurde komplett entfernt; ExportModal hat keinen Picker mehr (auto-destination).

## Task List

### Phase 1: Backend — Subfolder Generation

- [x] **Task 1:** `resolve_output_dir` existiert in `orchestrator.rs:22` — erzeugt aber `{base_dir}/{domain}` ohne Date/ID-Suffix (siehe CHANGELOG v0.3.2 — bewusste Vereinfachung)
  - Acceptance: ✅ Orchestrator erzeugt Domain-Subfolder unter Default
  - Verify: ✅ `cargo test` passes
  - Files: `src-tauri/src/crawler/orchestrator.rs`
  - Scope: S

### Phase 2: Frontend — Folder Pickers

- [x] **Task 2:** Native Picker wäre per Plan in NewCrawl gewesen — **mittlerweile entfernt** zugunsten globaler Settings-only-Konfiguration (CHANGELOG v0.3.2). NewCrawl hat keinen Picker mehr, `config.outputDir` bleibt leer und wird vom Orchestrator aufgelöst.
  - Files: `src/views/NewCrawl.tsx`
  - Scope: S

- [x] **Task 3:** Folder-Picker in Settings.tsx vorhanden (`Settings.tsx:182` ruft `@tauri-apps/plugin-dialog open({ directory: true })` auf)
  - Acceptance: ✅ Klick öffnet nativen Picker
  - Verify: ✅ `npm run build` succeeds
  - Files: `src/views/Settings.tsx`
  - Scope: S

### Phase 3: ExportModal Pre-fill

- [x] **Task 4:** ExportModal wurde **weiter vereinfacht** als geplant: kein Pre-fill + Picker mehr, sondern voll automatisches Ziel (`{outputDir}/formats/` bzw. `{outputDir}/zip/`) — siehe CHANGELOG v0.3.2 ("Simplified ExportModal"). `destination: null` im Modal-State.
  - Files: `src/components/ExportModal.tsx`
  - Scope: S

### Phase 4: History Polish

- [x] **Task 5:** "Open folder" in `History.tsx:61-64` öffnet `{outputDir}/main/` direkt (siehe CHANGELOG v0.3.2 "Open folder opens main/ subfolder")
  - Acceptance: ✅ Korrekter Ordner wird geöffnet
  - Verify: ✅ `npm run build` succeeds
  - Files: `src/views/History.tsx`
  - Scope: S

### Checkpoint: Complete
- [x] `cargo test` — alle Tests grün
- [x] `npm run build` — kein Fehler
- [x] App läuft — neuer Crawl legt Subordner `{domain}/{main|zip|formats}` an, Settings-Picker funktioniert, Export schreibt automatisch ins korrekte Unterverzeichnis
