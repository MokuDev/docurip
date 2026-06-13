# Docurip Phase 4 — Spec Gap Coverage Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close remaining gaps between the implementation and `docs/superpowers/specs/2026-06-12-docurip-design.md`. Add Pause/Resume, Disk-Error recovery, spec-compliant Dashboard stats, Recent Exports panel, System Status Bars, global Toast Container, and an E2E test.

**Architecture:** Backend adds `should_pause` to `CrawlHandle` (alongside existing `should_stop`); Orchestrator main loop yields to a `Notify` when paused and resumes on signal. Frontend exposes Pause/Resume buttons in `NewCrawl`, lifts `error` events into a global toast system, and adds the missing Dashboard widgets + chrome bars.

**Tech Stack:** Rust (tokio, tauri, zip, uuid), React 19, Tailwind CSS, TypeScript, framer-motion.

---

## Datei-Struktur

| Datei | Verantwortung |
|-------|--------------|
| `src-tauri/src/crawler/orchestrator.rs` | `should_pause` flag, `tokio::sync::Notify` für Resume-Signal, Disk-Error-Recovery |
| `src-tauri/src/commands.rs` | `pause_crawl`, `resume_crawl` Commands; `list_exports` |
| `src-tauri/src/lib.rs` | Commands registrieren |
| `src-tauri/src/exports.rs` | `list_recent_exports(app_data_dir, n)` helper (scannt `app_data_dir/exports/`) |
| `src/views/NewCrawl.tsx` | Pause/Resume Buttons |
| `src/views/Dashboard.tsx` | Stats erweitern (Crawl Velocity, Total Size, Fail Rate) + Recent Exports Panel |
| `src/components/TopStatusBar.tsx` | Session-ID + Uptime (live counter) |
| `src/components/SystemStatusBar.tsx` | CPU%, RAM%, aktiver Output-Pfad (poll every 2 s) |
| `src/components/ToastContainer.tsx` | Bottom-left global toast renderer |
| `src/App.tsx` | Mount Top/System Bars + Toast Container |
| `src/hooks/useSystemStats.ts` | Pollt system stats via Tauri command |
| `src-tauri/src/commands.rs` | `get_system_stats`, `get_session_info` Commands |
| `src-tauri/src/system.rs` | `sysinfo` crate wrapper (CPU, RAM) |
| `src-tauri/Cargo.toml` | `sysinfo = "0.31"`, `uuid = { version = "1", features = ["v4"] }` |
| `src-tauri/tests/e2e_crawl.rs` | E2E test mit wiremock static site fixture |
| `src-tauri/tests/fixtures/site/` | Statisches HTML Fixture (index + 2 sub-pages + 1 image) |

---

## Task 1: Pause / Resume Commands (Backend + Frontend)

**Files:**
- Modify: `src-tauri/src/crawler/orchestrator.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/views/NewCrawl.tsx`

Trenne `should_stop` (cancel/final) von `should_pause` (soft). Orchestrator hält bei `should_pause=true` an, speichert State, wartet auf `Notify` aus `resume_crawl`.

**Steps:**

- [x] **Step 1: `CrawlHandle` erweitern**

  ```rust
  pub struct CrawlHandle {
      pub job: Arc<RwLock<CrawlJob>>,
      pub should_stop: Arc<AtomicBool>,
      pub should_pause: Arc<AtomicBool>,
      pub resume_notify: Arc<tokio::sync::Notify>,
      pub event_bus: EventBus,
  }
  ```

  Update alle `CrawlHandle { ... }` Konstruktionen (grep nach `should_stop:`).

- [x] **Step 2: Orchestrator Main Loop — Pause-Check + Wait**

  Im `loop` (orchestrator.rs:222) VOR dem `should_stop` Check:

  ```rust
  if self.handle.should_pause.load(Ordering::Relaxed) {
      {
          let mut job = self.handle.job.write().await;
          if job.status != JobStatus::Paused {
              job.status = JobStatus::Paused;
              let job_id = job.id.clone();
              drop(job);
              self.handle.event_bus.emit(CrawlEvent::JobStatusChanged {
                  job_id: job_id.clone(),
                  status: JobStatus::Paused,
              });
              if let Some(ref app_state) = self.app_state {
                  let job = self.handle.job.read().await.clone();
                  let _ = app_state.persist_job(&job).await;
              }
          }
      }
      // Abort in-flight tasks gracefully (keep state) and wait for resume
      pending.abort_all();
      while let Some(_) = pending.join_next().await {}
      self.handle.resume_notify.notified().await;
      // After resume, set status back to Running
      {
          let mut job = self.handle.job.write().await;
          job.status = JobStatus::Running;
          let job_id = job.id.clone();
          drop(job);
          self.handle.event_bus.emit(CrawlEvent::JobStatusChanged {
              job_id,
              status: JobStatus::Running,
          });
      }
  }
  ```

  Achtung: `should_pause` wird im Loop vor `should_stop` geprüft, damit ein pausierter Job bei resume nicht sofort wieder stoppt.

- [x] **Step 3: `pause_crawl` Command**

  ```rust
  #[tauri::command]
  pub async fn pause_crawl(job_id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
      let app = state.inner().clone();
      let handle = app.get_handle(&job_id).await.ok_or("Job not found")?;
      handle.should_pause.store(true, Ordering::Relaxed);
      Ok(())
  }
  ```

  Anpassen an die echte `AppState::get_handle` Signatur (siehe `commands.rs` für `stop_crawl`).

- [x] **Step 4: `resume_crawl` Command**

  ```rust
  #[tauri::command]
  pub async fn resume_crawl(job_id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
      let app = state.inner().clone();
      let handle = app.get_handle(&job_id).await.ok_or("Job not found")?;
      handle.should_pause.store(false, Ordering::Relaxed);
      handle.resume_notify.notify_one();
      Ok(())
  }
  ```

- [x] **Step 5: `lib.rs` registrieren**

  ```rust
  .invoke_handler(tauri::generate_handler![
      ..., pause_crawl, resume_crawl,
  ])
  ```

- [x] **Step 6: Frontend — Pause/Resume Buttons in `NewCrawl.tsx`**

  Zustand `paused` lokal oder via `job.status === 'paused'`. Buttons:
  - Beim `running`: "Pause" (orange) + "Cancel" (rot)
  - Beim `paused`: "Resume" (green) + "Cancel" (rot)
  - `invoke('pause_crawl', { jobId })` / `invoke('resume_crawl', { jobId })`

  Style: kleinere Buttons neben dem Live Monitor, mit Phosphor-Icons `Pause` / `Play`.

- [x] **Step 7: Build verifizieren**

  ```bash
  cargo check
  cargo test
  npm run build
  ```

  Expected: alle 40+ Tests passing.

---

## Task 2: Resume nach Disk-Error

**Files:**
- Modify: `src-tauri/src/crawler/orchestrator.rs`
- Modify: `src-tauri/src/writer/fs.rs`

Spec §Error Handling: "Disk errors (write failures): pause the job, emit error, allow user to resume after fixing the path."

**Steps:**

- [x] **Step 1: `FsWriter` — `write_markdown` Error als soft-fail markieren**

  Füge `pub enum WriteError { Disk(...), Other(...) }` hinzu (oder nutze `anyhow::Error` mit Marker-Variante). Wichtig: Errors die mit `Disk` getaggt sind, sollen unterscheidbar sein.

  Minimal-Invasiv: in `orchestrator.rs::handle_task_result`, wenn `writer.write_*` fehlschlägt UND die `error.message()` "No space" / "Permission denied" / "Read-only" enthält → setze `job.status = Paused` und `should_pause.store(true)`, emit `CrawlEvent::Error` mit Hint "fix output path then resume".

- [x] **Step 2: Disk-Error-Detection im `handle_task_result`**

  Im `Err` branch von `handle_task_result` (orchestrator.rs ~360), zusätzlich zum bestehenden Error-Log:

  ```rust
  if let Some(ref app_state) = self.app_state {
      let mut job = self.handle.job.write().await;
      if error_msg.contains("Permission denied") || error_msg.contains("No space") || error_msg.contains("Read-only") {
          job.status = JobStatus::Paused;
          self.handle.should_pause.store(true, Ordering::Relaxed);
          // Persist + emit
      }
  }
  ```

  HINWEIS: Disk-Error ist schreibend → Paused-Status, NICHT Failed. User kann Output-Pfad in Settings ändern, dann resume.

- [x] **Step 3: Test — synthetischer Disk-Error**

  Unit-Test in `orchestrator.rs` oder `writer/fs.rs`: writer der in `tempdir` versucht zu schreiben, Pfad wird read-only gemacht → expect Paused-Status. (Skip falls kompliziert; manuelle Verifikation OK.)

- [x] **Step 4: Build verifizieren**

  ```bash
  cargo check
  cargo test
  ```

---

## Task 3: Dashboard Stats erweitern

**Files:**
- Modify: `src/views/Dashboard.tsx`
- Modify: `src-tauri/src/commands.rs` (size-Berechnung)
- Modify: `src-tauri/src/state.rs` (expose `total_output_size` via list_jobs)

Spec §Visual Design: 4 metric cards: **Pages Saved, Total Size, Crawl Velocity, Fail Rate**.

**Steps:**

- [x] **Step 1: `total_output_size` im Backend**

  In `AppState::list_jobs` (oder neuer Helper `compute_stats`):
  - Iteriere alle completed jobs
  - Für jeden: `std::fs::read_dir(job.config.output_dir).map(|d| ...).sum::<u64>()` (mit Cache um I/O zu schonen)
  - Return neben den jobs: `{ total_size_bytes: u64 }`

  ODER: in `commands.rs` einen neuen Command `get_dashboard_stats` der nur die Metriken liefert.

- [x] **Step 2: `Crawl Velocity` berechnen**

  Velocity = `pages_done_total / (latest_job.completed_at - latest_job.started_at)` in pages/min.

- [x] **Step 3: `Fail Rate`**

  `failed / total` über alle jobs.

- [x] **Step 4: Dashboard.tsx — 4 neue Cards**

  Layout: 2x2 grid statt 4x1, oder bleibt 4x1 aber cards: "Pages Saved" (=total pages) | "Total Size" (in MB) | "Crawl Velocity" (pages/min) | "Fail Rate" (%).

  Visual: behalte StatCard-Komponente, nur andere Daten.

- [x] **Step 5: Build verifizieren**

  ```bash
  cargo check
  npm run build
  ```

---

## Task 4: Recent Exports Panel

**Files:**
- Create: `src-tauri/src/exports.rs`
- Modify: `src-tauri/src/commands.rs` (new `list_exports` command)
- Modify: `src-tauri/src/lib.rs` (register)
- Modify: `src/views/Dashboard.tsx` (new panel)

**Steps:**

- [x] **Step 1: `exports.rs` — `RecentExport` struct + scannen**

  ```rust
  pub struct RecentExport {
      pub path: String,
      pub job_id: String,
      pub created_at: String,
      pub size_bytes: u64,
  }

  pub fn list_recent_exports(app_data_dir: &Path, n: usize) -> anyhow::Result<Vec<RecentExport>> {
      // 1. mkdir app_data_dir/exports/ if not exists
      // 2. read_dir, filter *.zip
      // 3. for each: get metadata (size, modified)
      // 4. sort by modified desc, take n
      // 5. parse job_id from filename (e.g. "<job_id>.zip")
  }
  ```

- [x] **Step 2: `list_exports` Command**

  ```rust
  #[tauri::command]
  pub async fn list_exports(app: AppHandle, limit: Option<usize>) -> Result<Vec<RecentExport>, String> {
      let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
      let n = limit.unwrap_or(5);
      crate::exports::list_recent_exports(&dir, n).map_err(|e| e.to_string())
  }
  ```

- [x] **Step 3: `lib.rs` registrieren**

- [x] **Step 4: Dashboard.tsx — Recent Exports Panel**

  Auf der rechten Seite oder unter dem Recent-Activity. Empty state wenn keine exports. Zeigt: filename, size (human-readable), date, click → öffnet Pfad (nutze `@tauri-apps/plugin-shell` oder `revealItemInDir`).

- [x] **Step 5: Build verifizieren**

---

## Task 5: System Status Bars (Top + Bottom)

**Files:**
- Create: `src-tauri/src/system.rs`
- Modify: `src-tauri/Cargo.toml` (`sysinfo = "0.31"`)
- Modify: `src-tauri/src/commands.rs` (`get_system_stats`, `get_session_info`)
- Modify: `src-tauri/src/lib.rs` (register; session_id in app state)
- Create: `src/components/TopStatusBar.tsx`
- Create: `src/components/SystemStatusBar.tsx`
- Create: `src/hooks/useSystemStats.ts`
- Modify: `src/App.tsx`

**Steps:**

- [x] **Step 1: `Cargo.toml` — `sysinfo = "0.31"`, `uuid` features v4**

- [x] **Step 2: `system.rs` — SystemStats struct + collector**

  ```rust
  pub struct SystemStats {
      pub cpu_percent: f32,
      pub mem_used_mb: u64,
      pub mem_total_mb: u64,
  }

  pub fn collect() -> SystemStats {
      let mut sys = sysinfo::System::new_all();
      sys.refresh_all();
      let cpu = sys.global_cpu_info().cpu_usage();
      let mem_used = sys.used_memory() / 1024 / 1024;
      let mem_total = sys.total_memory() / 1024 / 1024;
      SystemStats { cpu_percent: cpu, mem_used_mb: mem_used, mem_total_mb: mem_total }
  }
  ```

- [x] **Step 3: Session-ID**

  In `AppState::new()` oder `init()`: `pub session_id: String` = `Uuid::new_v4().to_string()`. Start-time: `Instant::now()` für Uptime.

- [x] **Step 4: `get_system_stats` + `get_session_info` Commands**

  ```rust
  #[tauri::command]
  pub async fn get_system_stats() -> Result<SystemStats, String> {
      Ok(crate::system::collect())
  }

  #[tauri::command]
  pub async fn get_session_info(state: State<'_, Arc<AppState>>) -> Result<SessionInfo, String> {
      let s = state.inner().clone();
      Ok(SessionInfo { id: s.session_id.clone(), uptime_secs: s.uptime_secs() })
  }
  ```

  `uptime_secs()` ist eine Methode auf `AppState` die `start_time.elapsed().as_secs()` zurückgibt.

- [x] **Step 5: `TopStatusBar.tsx`**

  Fixed top bar, 24px high, zeigt:
  - Links: Logo + "Session: <first 8 hex chars>"
  - Mitte: Uptime live (HH:MM:SS, updated every 1s)
  - Rechts: optional Job-Counter (active jobs)

- [x] **Step 6: `SystemStatusBar.tsx`**

  Fixed bottom bar, 20px high, zeigt:
  - Links: CPU% (mit sparkline optional)
  - Mitte: RAM used/total MB
  - Rechts: Current output path (active job) oder "(idle)"

- [x] **Step 7: `useSystemStats` hook**

  Pollt `get_system_stats` alle 2s via `setInterval`, returnt `{ cpu, mem_used, mem_total }`.

- [x] **Step 8: `App.tsx` — Bars mounten**

  Top bar: oberhalb des `main` (full width).
  Bottom bar: unterhalb des `main` (full width).
  Adjust `h-screen` layout zu `h-[calc(100vh-44px)]` für main content (24+20).

- [x] **Step 9: Build verifizieren**

  ```bash
  cargo check
  npm run build
  ```

  ACHTUNG: `sysinfo` kann auf Windows beim ersten Build lange dauern (compile time). Geduld.

---

## Task 6: Global Toast Container (Bottom-Left)

**Files:**
- Create: `src/components/ToastContainer.tsx`
- Create: `src/hooks/useToasts.ts` (eigener Toast-Store, simple useState oder Context)
- Modify: `src/App.tsx` (mount container)
- Modify: `src/hooks/useCrawlEvents.tsx` (emit to global toast on error)

**Steps:**

- [x] **Step 1: `useToasts` — minimaler Toast-Store**

  ```typescript
  interface Toast { id: string; type: 'error' | 'info' | 'success'; message: string; }
  // useToasts() returns { toasts, pushToast, dismissToast }
  // Simple useState<Toast[]> + addToast(message, type) + removeToast(id)
  ```

- [x] **Step 2: `ToastContainer.tsx`**

  - Fixed bottom-left (`fixed bottom-12 left-4 z-50`)
  - Framer-motion `AnimatePresence` für slide-in/slide-out
  - Max 3 sichtbar, dismissable mit X-Button
  - Auto-dismiss nach 6s (außer `error`)

- [x] **Step 3: `useCrawlEvents` — push errors to global toast**

  Im bestehenden Event-Listener (`useCrawlEvents.tsx`), zusätzlich zum `error` state:
  ```typescript
  if (event.type === 'error') {
      pushToast('error', event.message);
  }
  ```

- [x] **Step 4: `App.tsx` — mount container**

  Vor dem schließenden `</div>`.

- [x] **Step 5: Build verifizieren**

  ```bash
  npm run build
  ```

---

## Task 7: E2E Test mit Static-Site-Fixture

**Files:**
- Create: `src-tauri/tests/e2e_crawl.rs`
- Create: `src-tauri/tests/fixtures/site/index.html`
- Create: `src-tauri/tests/fixtures/site/page1.html`
- Create: `src-tauri/tests/fixtures/site/page2.html`
- Create: `src-tauri/tests/fixtures/site/logo.png` (1x1 transparent PNG)
- Modify: `src-tauri/Cargo.toml` (verify `wiremock` in dev-deps)

**Steps:**

- [x] **Step 1: Cargo.toml — `wiremock` in dev-dependencies**

  Falls nicht vorhanden: `wiremock = "0.6"` unter `[dev-dependencies]`.

- [x] **Step 2: Static Site Fixtures erstellen**

  - `index.html`: enthält link zu `page1.html` + image `<img src="logo.png">`
  - `page1.html`: simple content + link zu `page2.html`
  - `page2.html`: simple content
  - `logo.png`: 1x1 transparent (kann via base64 inline erstellt werden)

- [x] **Step 3: `e2e_crawl.rs` — full crawl**

  ```rust
  use wiremock::{MockServer, Mock, ResponseTemplate};
  use wiremock::matchers::{method, path};

  #[tokio::test]
  async fn end_to_end_crawl_writes_files() {
      // 1. Start MockServer
      // 2. Mount routes for index.html, page1.html, page2.html, logo.png
      // 3. Create temp output dir
      // 4. Construct Orchestrator with CrawlConfig { start_url: server.uri, max_depth: 2, page_limit: 10, ... }
      // 5. Run orchestrator.run() (await)
      // 6. Assert: output dir contains 3 *.md files + 1 logo.png in assets/
      // 7. Assert: each markdown contains the rewritten relative asset path
  }
  ```

- [x] **Step 4: Run test**

  ```bash
  cargo test --test e2e_crawl
  ```

  Expected: 1 test passing.

- [x] **Step 5: Full build verify**

  ```bash
  cargo test
  npm run build
  ```

---

## Integration & Verifikation

- [x] **Step 1: `cargo check`**
- [x] **Step 2: `cargo test`** (alle Tests, inkl. neuem e2e)
- [x] **Step 3: `npm run build`**
- [x] **Step 4: Manuelle Smoke Tests**
  1. Crawl starten → Pause klicken → "Paused" Status + persistiert → Resume → weiter
  2. Output-Pfad ungültig machen (read-only) → Disk-Error → Paused → Pfad fixen → Resume
  3. Dashboard zeigt neue Stats + Recent Exports
  4. Top + Bottom Bars live aktualisiert
  5. Error-Toast erscheint bottom-left

---

## Spec Coverage Check

| Requirement | Task |
|------------|------|
| Pause/Resume via cancellation token | Task 1 |
| Disk-Error Resume | Task 2 |
| Pages Saved / Total Size / Crawl Velocity / Fail Rate | Task 3 |
| Recent Exports panel | Task 4 |
| Top status bar (session, uptime) | Task 5 |
| Bottom system bar (CPU, RAM, path) | Task 5 |
| System alert toasts bottom-left | Task 6 |
| E2E test for full crawl | Task 7 |

## Placeholder Scan

Keine TBDs.

## Type Consistency

- `SystemStats` identisch Rust ↔ TS
- `SessionInfo` identisch
- `RecentExport` identisch
- `Toast` TS-only
- `CrawlHandle` hat neue Felder `should_pause` + `resume_notify` — kein TS-Impact (Backend-only)
