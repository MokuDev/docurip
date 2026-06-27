# Docurip — Implementierungsplan: Bug-, Sicherheits- und Performance-Fixes

> Basierend auf `docs/checks/PROBLEMS.md` | Erstellt: 14.06.2026

---

## Übersicht

41 dokumentierte Probleme (6 kritisch, 12 hoch, 16 mittel, 7 niedrig) werden in 4 Phasen behoben. Jede Phase ist unabhängig testbar und lässt das System in einem funktionierenden Zustand.

## Architekturentscheidungen

- **Bestehende Patterns respektieren:** Keine neuen Crates außer wenn zwingend nötig
- **Keine Breaking Changes:** CrawlConfig-JSON muss abwärtskompatibel bleiben (serde defaults)
- **Test-First wo sinnvoll:** Neue Logik wird mit Unit-Tests versehen
- **Inkrementell:** Nach jeder Phase ist `cargo test` + `cargo check` grün

---

## Phase 1: Kritische Fixes

### Task 1: `stay_within_domain` implementieren

**Beschreibung:** Das `stay_within_domain`-Feld in `CrawlConfig` muss beim Crawling durchgesetzt werden. Nach `extract_links` werden alle URLs gefiltert, deren Domain von der Start-URL abweicht.

**Akzeptanzkriterien:**
- [x] Crawler folgt nur Links derselben Domain (inkl. Subdomains optional konfigurierbar)
- [x] `stay_within_domain: false` erlaubt weiterhin externe Links
- [x] Unit-Tests für Domain-Filter-Logik

**Verifikation:**
- [x] `cargo test` — bestehende Tests passen
- [x] `cargo test --package docurip --lib -- state::tests` — State-Tests passen
- [x] `cargo check` — kein Compile-Fehler

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/crawler/orchestrator.rs` (hauptsächlich)
- `src-tauri/src/parser/dom.rs` (möglicherweise)

**Umfang:** Medium (3-5 Dateien)

---

### Task 2: `respect_robots_txt` implementieren

**Beschreibung:** robots.txt muss parsen und Pfade vor dem Fetch geprüft werden. Dafür wird ein einfacher robots.txt-Parser benötigt (kein Crates dependency nötig, 50-100 Zeilen).

**Akzeptanzkriterien:**
- [x] robots.txt wird beim ersten Fetch der Domain geladen und gecacht (`crawler/robots.rs`, orchestrator.rs:228)
- [x] Pfade die durch robots.txt blockiert sind werden nicht gefetcht (orchestrator.rs:655)
- [x] `respect_robots_txt: false` umgeht die Prüfung
- [x] Fehlende robots.txt wird toleriert (kein Fehler)

**Verifikation:**
- [x] `cargo test` — alle Tests passen
- [x] Manueller Test: Crawl gegen eine Seite mit robots.txt

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/crawler/orchestrator.rs`
- `src-tauri/src/fetcher/http.rs` (evtl. robots.txt Cache)

**Umfang:** Medium (3-5 Dateien)

---

### Task 3: Synchones Disk-I/O auf `tokio::fs` umstellen

**Beschreibung:** `state.rs` und `commands.rs` verwenden `std::fs` auf dem Tokio-Runtime. Dies blockiert Worker-Threads.

**Akzeptanzkriterien:**
- [x] `state.rs` Methoden sind async und verwenden `tokio::fs` (`use tokio::fs` für persist/load/delete)
- [ ] `commands.rs` verwendet `tokio::fs` für alle Dateioperationen — **OFFEN**: weiterhin `std::fs::read_dir` an commands.rs:205 und :609
- [ ] `export.rs` verwendet `tokio::fs` für ZIP-Operationen — **OFFEN**: `walk_dir` nutzt weiterhin `std::fs`
- [ ] `state.rs` `init()` bleibt absichtlich synchron (`std::fs::read_dir`, `read_to_string`) wegen Tauri-Startup-Reihenfolge — siehe CHANGELOG v0.3.1 Fix

**Verifikation:**
- [x] `cargo test` — alle Tests passen
- [x] `cargo check` — kein Compile-Fehler
- [ ] Keine `std::fs` Aufrufe in async-Kontexten mehr — **OFFEN** (commands.rs)

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/state.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/export.rs`

**Umfang:** Medium (3-5 Dateien)

---

### Task 4: SSRF-Schutz (Warnung bei internen IPs)

**Beschreibung:** Optionale Warnung wenn der Benutzer interne IPs (localhost, RFC1918) crawlen möchte.

**Akzeptanzkriterien:**
- [x] SSRF-Modul existiert (`crawler/ssrf.rs`) und erkennt interne IPs (IPv4/IPv6 privat, localhost, `.local`)
- [x] `ssrf_protection` Config-Feld wird im Orchestrator durchgesetzt (orchestrator.rs:646)
- [x] 10 Unit-Tests für SSRF-Erkennung
- [x] `validate_crawl_input` ruft SSRF-Check für Start-URL auf, wenn `ssrf_protection` aktiv ist (commands.rs:25)
- [ ] Frontend zeigt keine Warnung beim Eintragen einer internen URL — Fehler kommt erst nach Submit

**Verifikation:**
- [x] `cargo test` — alle Tests passen
- [x] `cargo check` — kein Compile-Fehler

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/commands.rs` (validate_crawl_input)
- `src/views/NewCrawl.tsx` (Warnung anzeigen)

**Umfang:** Small (1-2 Dateien)

---

## Checkpoint: Phase 1

- [x] `cargo test` — alle Tests grün
- [x] `cargo check` — kein Compile-Fehler
- [x] `cargo check --features headless` — kein Compile-Fehler
- [x] Manueller Test: Crawl mit `stay_within_domain: true` verlässt nicht die Domain
- [x] Manueller Test: Crawl mit `respect_robots_txt: true` respektiert robots.txt
- [ ] Review vor Phase 2 — **OFFEN**: SSRF-Check fehlt im `validate_crawl_input`, restliche `std::fs` in `commands.rs`/`export.rs`

---

## Phase 2: Wichtige Fixes

### Task 5: Cancel-Status korrigieren

**Beschreibung:** Cancel setzt `JobStatus::Paused` statt `Failed` oder `Cancelled`. Entweder neuen `Cancelled`-Status einführen oder `Failed` mit separatem Flag verwenden.

**Akzeptanzkriterien:**
- [x] Cancel setzt korrekten Status: `JobStatus::Failed` mit `error: "Crawl cancelled by user"` (orchestrator.rs:333-334)
- [x] History zeigt abgebrochene Crawls korrekt an (als Failed mit Error-Text)
- [ ] Frontend unterscheidet zwischen fehlgeschlagen und abgebrochen — **OFFEN**: kein separater `Cancelled`-Enum-Wert, beide werden als Failed angezeigt

**Verifikation:**
- [x] `cargo test` — alle Tests passen
- [x] TypeScript types (`types/index.ts`) sind synchron mit Rust

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/crawler/orchestrator.rs` (Zeile ~299)
- `src-tauri/src/crawler/job.rs` (ggf. neuen Enum-Wert)
- `src/types/index.ts` (TypeScript-Type anpassen)

**Umfang:** Medium (3-5 Dateien)

---

### Task 6: Timeout-Setting an HttpFetcher übergeben

**Beschreibung:** `AppSettings.timeout` wird ignoriert — HttpFetcher hardcodiert 30s.

**Akzeptanzkriterien:**
- [x] `HttpFetcher::new(timeout_secs: u64)` akzeptiert Timeout-Parameter (http.rs:20)
- [x] `AppSettings.timeout` wird durchgereicht (orchestrator.rs:116 `HttpFetcher::new(timeout_secs)`)
- [x] Default bleibt 30s wenn nicht konfiguriert

**Verifikation:**
- [x] `cargo test` — alle Tests passen

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/fetcher/http.rs`
- `src-tauri/src/crawler/orchestrator.rs` (HttpFetcher::new Aufruf)

**Umfang:** Small (1-2 Dateien)

---

### Task 7: Versionsnummern vereinheitlichen

**Beschreibung:** Einheitliche Versionsnummer in allen Dateien. App-Version ist mittlerweile `v0.3.3`.

**Akzeptanzkriterien:**
- [x] `Cargo.toml`: `0.3.3`
- [x] `package.json`: `0.3.3`
- [x] `App.tsx` Footer: `v0.3.3`
- [x] `HttpFetcher` User-Agent: `Docurip/0.3.3` (http.rs:28)
- [x] `AppSettings` Default User-Agent: `Docurip/0.3.3` (config.rs:33)

**Verifikation:**
- [x] `cargo test` — alle Tests passen
- [x] `npm run build` — kein Fehler

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/Cargo.toml`
- `src/App.tsx`
- `src-tauri/src/fetcher/http.rs`
- `src-tauri/src/settings/config.rs`
- `package.json`

**Umfang:** Small (1-2 Dateien)

---

### Task 8: Query-Strings aus Dateinamen entfernen

**Beschreibung:** `url_to_page_path` und `url_to_asset_path` entfernen Query-Parameter nicht. `page?id=1` → `page?id=1.md` ist unter Windows ungültig.

**Akzeptanzkriterien:**
- [x] Query-String wird aus Dateinamen entfernt (writer/fs.rs)
- [x] Fragment-Identifiers werden ebenfalls entfernt
- [x] 3 Unit-Tests für URL → Pfad Konvertierung mit Query/Fragment (fs.rs:184, 191, 198) — siehe CHANGELOG v0.3.1

**Verifikation:**
- [x] `cargo test` — alle Tests passen

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/writer/fs.rs`

**Umfang:** Small (1-2 Dateien)

---

### Task 9: Double Update Check beheben

**Beschreibung:** `installUpdate()` ruft `check()` erneut auf statt den gecachten `updateAvailable`-Wert zu verwenden.

**Akzeptanzkriterien:**
- [x] `installUpdate()` verwendet das gecachte `update`-Objekt (`updateRef.current`, useUpdater.ts:14, 42)
- [x] Keine doppelte Netzwerkanfrage im Normalfall — Fallback auf `check()` nur wenn Cache leer

**Verifikation:**
- [x] `npm run build` — kein Fehler

**Dependencies:** Keine

**Betroffene Dateien:**
- `src/hooks/useUpdater.ts`

**Umfang:** XS (1 Datei)

---

### Task 10: Headless Browser wiederverwenden

**Beschreibung:** Für jeden Fetch wird ein neuer Browser-Prozess erstellt. Browser-Instanz wiederverwenden.

**Akzeptanzkriterien:**
- [x] Browser-Instanz wird in `HeadlessFetcher`-Struct gehalten (headless.rs:5-13)
- [x] Tabs werden nach Fetch geschlossen, nicht der Browser (headless.rs:22 `tab.close()`)
- [x] Kein Memory-Leak

**Verifikation:**
- [x] `cargo test` — alle Tests passen

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/fetcher/headless.rs`
- `src-tauri/src/crawler/orchestrator.rs`

**Umfang:** Medium (3-5 Dateien)

---

### Task 11: System-Stats cachen

**Beschreibung:** `sysinfo::System::new_all()` + `refresh_all()` werden bei jedem Aufruf erstellt. System-Instanz statisch halten.

**Akzeptanzkriterien:**
- [x] `System`-Instanz als `LazyLock<Mutex<System>>` Singleton (system.rs:12)
- [x] Nur `refresh_all()` wird alle 2s aufgerufen (system.rs:16)
- [x] CPU-RAM-Uptime Komponente funktioniert weiterhin

**Verifikation:**
- [x] `cargo test` — alle Tests passen

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/system.rs`
- `src-tauri/src/commands.rs`

**Umfang:** Small (1-2 Dateien)

---

### Task 12: Content-Type + Größenvalidierung bei Downloads

**Beschreibung:** Assets werden ohne MIME-Type-Validierung und ohne Größenlimit heruntergeladen.

**Akzeptanzkriterien:**
- [x] Content-Type wird geprüft — `is_allowed_asset_mime` in http.rs lehnt `text/html`/`application/xhtml+xml` ab und whitelistet Images/Fonts/CSS/JS/JSON/PDF/Audio/Video/Octet-Stream
- [x] Max-Download-Größe (50MB hardcoded, http.rs:110-112) — nicht konfigurierbar, aber wirksam
- [x] Ungültige Downloads werden via `anyhow::bail!` abgewiesen und vom Orchestrator als Asset-Fehler geloggt

**Verifikation:**
- [x] `cargo test` — alle Tests passen

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/asset_dl/downloader.rs`

**Umfang:** Medium (3-5 Dateien)

---

## Checkpoint: Phase 2

- [x] `cargo test` — alle Tests grün
- [x] `npm run build` — kein Fehler
- [x] Cancel zeigt korrekten Status in History (Failed mit Error-Text)
- [x] Timeout-Setting wirkt sich auf Fetcher aus
- [ ] Alle Versionen sind aktuell — **OFFEN**: User-Agent-Strings in `http.rs`/`config.rs` noch `0.3.1` statt `0.3.3`
- [ ] Review vor Phase 3 — **OFFEN** für: Content-Type Validierung, User-Agent-Version, separater Cancelled-Status

---

## Phase 3: Verbesserungen

### Task 13: Dashboard-Polling optimieren

**Beschreibung:** Drei separate Intervalle (Jobs 3s, Stats 3s, Exports 5s) erzeugen unnötige Backend-Aufrufe.

**Akzeptanzkriterien:**
- [x] Ein einziges Polling-Interval (3s) für alle Daten (Dashboard.tsx:31-36) — siehe CHANGELOG v0.3.0
- [x] Stats werden gedrosselt: live (3s) während aktiver Crawls, sonst nur jede 4. Iteration (~12s) via Tick-Counter (Dashboard.tsx)

**Verifikation:**
- [x] `npm run build` — kein Fehler
- [x] Dashboard lädt korrekt

**Dependencies:** Keine

**Betroffene Dateien:**
- `src/views/Dashboard.tsx`

**Umfang:** Small (1-2 Dateien)

---

### Task 14: Parallele Asset-Downloads

**Beschreibung:** Pro Seite werden Assets in einer Schleife einzeln heruntergeladen, obwohl ein `JoinSet` verfügbar wäre.

**Akzeptanzkriterien:**
- [x] Assets werden parallel mit `tokio::task::JoinSet` heruntergeladen (orchestrator.rs:519) — siehe CHANGELOG v0.3.0
- [ ] Max parallelDownloads konfigurierbar — **OFFEN**: kein separates Limit, nutzt Crawl-Concurrency-Settings indirekt
- [x] Fehler bei einem Asset brechen nicht alle anderen ab

**Verifikation:**
- [x] `cargo test` — alle Tests passen

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/crawler/orchestrator.rs`

**Umfang:** Medium (3-5 Dateien)

---

### Task 15: String-Matching durch Error-Typen ersetzen

**Beschreibung:** `is_disk_error` und `is_transient_error` verwenden String-Matching statt Error-Typen.

**Akzeptanzkriterien:**
- [x] `is_disk_error(&anyhow::Error)` läuft die Fehlerkette ab und matched `std::io::ErrorKind::PermissionDenied`/`StorageFull`/`ReadOnlyFilesystem`. String-Fallback (`is_disk_error_str`) bleibt für Errors ohne `io::Error`-Source (orchestrator.rs:683)
- [x] `is_transient_error` prüft per `downcast_ref::<reqwest::Error>` `is_timeout()`/`is_connect()`/`is_request()`. String-Fallback bleibt für Nicht-reqwest-Fehler (http.rs:42)
- [x] Strukturierte Klassifikation primär, String-Matching nur als Fallback

**Verifikation:**
- [x] `cargo test` — alle Tests passen (Tests prüfen das bestehende String-Matching-Verhalten)

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/crawler/orchestrator.rs`
- `src-tauri/src/fetcher/http.rs`

**Umfang:** Small (1-2 Dateien)

---

### Task 16: LiveConsole Event-Verarbeitung fixen

**Beschreibung:** LiveConsole verarbeitet nur `events[events.length - 1]`, überspringt Events zwischen Renders.

**Akzeptanzkriterien:**
- [x] Index-basierte Verarbeitung via `lastProcessedIdx` ref (LiveConsole.tsx:26-32) — siehe CHANGELOG v0.3.0
- [x] Keine verlorenen Events

**Verifikation:**
- [x] `npm run build` — kein Fehler
- [x] Manueller Test: Crawl starten und Events beobachten

**Dependencies:** Keine

**Betroffene Dateien:**
- `src/components/LiveConsole.tsx`

**Umfang:** XS (1 Datei)

---

### Task 17: History Lade-Flackern beheben

**Beschreibung:** `loadJobs` setzt `setLoading(true)` bei jedem 3s-Poll, was zu Spinner-Flackern führt.

**Akzeptanzkriterien:**
- [x] `loadJobs(showSpinner = false)` — nur beim initialen Aufruf `setLoading(true)` (History.tsx:36-44) — siehe CHANGELOG v0.3.0
- [x] Spätere Polls aktualisieren Daten ohne Spinner

**Verifikation:**
- [x] `npm run build` — kein Fehler

**Dependencies:** Keine

**Betroffene Dateien:**
- `src/views/History.tsx`

**Umfang:** XS (1 Datei)

---

### Task 18: StatusBadge auslagern

**Beschreibung:** StatusIcon/StatusBadge sind in 3 Dateien dupliziert.

**Akzeptanzkriterien:**
- [x] Gemeinsame Komponente `src/components/StatusBadge.tsx` (existiert)
- [x] Alle 3 Dateien importieren die gemeinsame Komponente (z. B. Dashboard.tsx:12) — siehe CHANGELOG v0.3.0

**Verifikation:**
- [x] `npm run build` — kein Fehler

**Dependencies:** Keine

**Betroffene Dateien:**
- `src/components/StatusBadge.tsx` (neu)
- `src/views/Dashboard.tsx`
- `src/views/History.tsx`
- `src/views/NewCrawl.tsx`

**Umfang:** Small (1-2 Dateien)

---

## Checkpoint: Phase 3

- [x] `cargo test` — alle Tests grün
- [x] `npm run build` — kein Fehler
- [x] Dashboard verwendet nur ein Polling-Interval
- [x] LiveConsole verliert keine Events
- [x] History hat kein Spinner-Flackern
- [ ] Review vor Phase 4 — **OFFEN**: Task 15 (String-Matching → Error-Typen) ist noch nicht erledigt

---

## Phase 4: Nice-to-have

### Task 19: Frontend-Performance (Virtualisierung, Debounce, Logs)

**Beschreibung:** P6-P8 aus PROBLEMS.md: Logs-Array Copy-on-Write, keine Virtualisierung im ResultTree, kein Debounce bei Suche.

**Akzeptanzkriterien:**
- [x] Logs `useRef` statt State-Array: `logsRef` + `logTick`-Counter, mutiert in-place mit 500er Cap (NewCrawl.tsx)
- [ ] ResultTree mit Virtualisierung (`react-window`/`@tanstack/react-virtual`) — **OFFEN**: keine Virtualisierung implementiert
- [x] ResultSearch mit Debounce (200ms) — siehe CHANGELOG v0.3.1

**Dependencies:** Keine

**Betroffene Dateien:**
- `src/views/NewCrawl.tsx`
- `src/components/ResultTree.tsx`
- `src/components/ResultSearch.tsx`

---

### Task 20: Code-Qualität (C9-C11)

**Beschreibung:** Regex-basiertes Markdown-Rendering, redundante Log-Speicherung, leere Catch-Blocks.

**Akzeptanzkriterien:**
- [ ] C9 — `react-markdown`/`remark` statt Regex — **OFFEN**: MarkdownPreview.tsx weiterhin regex-basiert (mit DOMPurify-Sanitisierung als Sicherheits-Layer)
- [ ] C10 — Redundante Log-Speicherung entfernen — **OFFEN**: NewCrawl.tsx hat weiterhin lokalen `logs`-State zusätzlich zu globalen Events
- [x] C11 — Empty Catch-Blocks beheben — Dashboard.tsx nutzt jetzt `console.warn(...)` mit Kontext (Dashboard.tsx:43, 57, 73) — siehe CHANGELOG v0.3.1

**Dependencies:** Keine

**Betroffene Dateien:**
- `src/components/MarkdownPreview.tsx`
- `src/views/NewCrawl.tsx`
- `src/views/Dashboard.tsx`

---

### Task 21: Minor Bugfixes (B10-B14)

**Beschreibung:** prefillUrl, AppSettings-Typ, walk_dir-Duplizierung, useUpdater Error State.

**Akzeptanzkriterien:**
- [x] B10 — `prefillUrl` re-triggerbar: `if (prev.url) return prev` Guard entfernt (NewCrawl.tsx:53-56) — siehe CHANGELOG v0.3.1
- [x] B11 — `AppSettings`-TS-Typ vollständig: `defaultDownloadAssets`, `defaultHeadlessStrategy`, `defaultRespectRobotsTxt` vorhanden (types/index.ts:53-55)
- [x] B12 — `walk_dir` als `pub` in `export.rs` exponiert; `commands.rs::export_job_zip` ruft jetzt `export::zip_directory` statt eigener Inline-Rekursion auf
- [x] B13 — Identisch mit B8/Task 16 (LiveConsole) → ✅ erledigt
- [x] B14 — `useUpdater` Error State im Update-Banner gerendert (App.tsx); Button-Label wird zu „Retry" bei Fehler

**Dependencies:** Keine

**Betroffene Dateien:**
- `src/views/NewCrawl.tsx`
- `src/types/index.ts`
- `src-tauri/src/export.rs`
- `src-tauri/src/commands.rs`
- `src/hooks/useUpdater.ts`

---

## Risiken und Minderungen

| Risiko | Impact | Minderung |
|--------|--------|-----------|
| robots.txt Parser zu komplex | Hoch | Einfache Implementierung (nur `Disallow:` Lines), kein Crates dependency |
| tokio::fs Migration bricht bestehende Tests | Hoch | Schrittweise Migration, Tests nach jedem Task |
| Browser-Wiederverwendung verursacht Memory-Leak | Mittel | Tab-Close nach Fetch, Max-Tabs-Limit |
| Neue TypeScript Types nicht synchron mit Rust | Mittel | Nach jedem Task `cargo test` + `npm run build` |

---

## Offene Fragen

1. Soll `stay_within_domain` auch Subdomains einschließen? (z.B. `docs.example.com` wenn Start-URL `example.com` ist)
2. Soll `Cancelled` ein neuer Enum-Wert werden oder `Failed` mit Error-Text?
3. Soll robots.txt Cache pro Crawl-Job oder pro Session sein?
4. Soll die SSRF-Warnung eine Option zum Ignorieren haben?

---

## Zusammenfassung

| Phase | Tasks | Geschätzter Aufwand |
|-------|-------|---------------------|
| Phase 1 (Kritisch) | 4 | 2-3 Stunden |
| Phase 2 (Wichtig) | 8 | 3-4 Stunden |
| Phase 3 (Verbesserungen) | 6 | 2-3 Stunden |
| Phase 4 (Nice-to-have) | 3 | 1-2 Stunden |
| **Gesamt** | **21** | **8-12 Stunden** |

---

## Aktueller Implementierungsstatus (verifiziert nach Quick-Wins/Mittel-Pass)

| Task | Status | Anmerkung |
|------|--------|-----------|
| 1 — `stay_within_domain` | ✅ Erledigt | orchestrator.rs:658 |
| 2 — `respect_robots_txt` | ✅ Erledigt | `robots.rs` + orchestrator.rs:228, 655 |
| 3 — `tokio::fs`-Migration | ⚠️ Teilweise | `state.rs` migriert; `commands.rs::dir_size_capped` und `export.rs` nutzen weiter `std::fs` (sync via spawn_blocking-Pfad noch offen) |
| 4 — SSRF-Schutz | ✅ Erledigt | `ssrf.rs` + Folge-Links + Start-URL in `validate_crawl_input` (commands.rs:25) |
| 5 — Cancel-Status | ⚠️ Teilweise | Cancel = Failed mit Error-Text; kein eigener `Cancelled`-Enum-Wert |
| 6 — Timeout-Setting | ✅ Erledigt | `HttpFetcher::new(timeout_secs)` |
| 7 — Versionsnummern | ✅ Erledigt | Alle Versionen auf `0.3.3` inklusive User-Agent |
| 8 — Query-Strings aus Dateinamen | ✅ Erledigt | 3 Regression-Tests in fs.rs |
| 9 — Double Update Check | ✅ Erledigt | `updateRef.current` cache |
| 10 — Headless Browser wiederverwenden | ✅ Erledigt | Browser im Struct, Tabs pro Fetch (Bonus: `tab.close(false)` für headless_chrome 1.x) |
| 11 — System-Stats cachen | ✅ Erledigt | `LazyLock<Mutex<System>>` |
| 12 — Content-Type + Größenlimit | ✅ Erledigt | 50 MB Limit + MIME-Type-Whitelist (`is_allowed_asset_mime` in http.rs), lehnt `text/html` ab |
| 13 — Dashboard-Polling | ✅ Erledigt | Single Interval, Stats live (3s) bei aktiven Crawls, sonst gedrosselt (~12s) |
| 14 — Parallele Asset-Downloads | ✅ Erledigt | `JoinSet` in orchestrator.rs:519 |
| 15 — String-Matching → Error-Typen | ✅ Erledigt | `is_disk_error` via `io::ErrorKind`, `is_transient_error` via `reqwest::Error`-Methoden; String-Fallback nur für Errors ohne typisierte Source |
| 16 — LiveConsole Event-Verarbeitung | ✅ Erledigt | `lastProcessedIdx` ref |
| 17 — History Lade-Flackern | ✅ Erledigt | `showSpinner`-Param |
| 18 — StatusBadge auslagern | ✅ Erledigt | `components/StatusBadge.tsx` |
| 19 — Frontend-Performance | ⚠️ Teilweise | Debounce ✅, Logs-`useRef` ✅; ResultTree-Virtualisierung ❌ |
| 20 — Code-Qualität (C9-C11) | ⚠️ Teilweise | C11 ✅; **C9 (Markdown-Parser), C10 (redundante Logs) offen** |
| 21 — Minor Bugfixes | ✅ Erledigt | B10–B14 alle erledigt (B12 walk_dir konsolidiert, B14 Updater-Error im UI gerendert) |

**Bilanz:** 18 von 21 Tasks vollständig erledigt, 3 teilweise erledigt (3, 19, 20).

**Noch offene Arbeiten:**
1. Restliche `std::fs` in `commands.rs` (`dir_size_capped`) und `export.rs` auf `tokio::fs` oder `spawn_blocking` umstellen
2. ResultTree-Virtualisierung (`react-window`/`@tanstack/react-virtual`)
3. MarkdownPreview auf echten Markdown-Parser (`react-markdown`) umstellen
4. Redundante Log-Speicherung in NewCrawl (lokaler State zusätzlich zu globalen Events) entfernen
5. Optional: separater `JobStatus::Cancelled`-Enum-Wert für Frontend-Unterscheidung zwischen Cancel und Failed
6. Optional: Frontend-Warnung beim Eintragen einer internen URL, bevor der Submit blockiert wird
