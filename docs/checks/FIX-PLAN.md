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
- [ ] robots.txt wird beim ersten Fetch der Domain geladen und gecacht
- [ ] Pfade die durch robots.txt blockiert sind werden nicht gefetcht
- [ ] `respect_robots_txt: false` umgeht die Prüfung
- [ ] Fehlende robots.txt wird toleriert (kein Fehler)

**Verifikation:**
- [ ] `cargo test` — alle Tests passen
- [ ] Manueller Test: Crawl gegen eine Seite mit robots.txt

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/crawler/orchestrator.rs`
- `src-tauri/src/fetcher/http.rs` (evtl. robots.txt Cache)

**Umfang:** Medium (3-5 Dateien)

---

### Task 3: Synchones Disk-I/O auf `tokio::fs` umstellen

**Beschreibung:** `state.rs` und `commands.rs` verwenden `std::fs` auf dem Tokio-Runtime. Dies blockiert Worker-Threads.

**Akzeptanzkriterien:**
- [ ] `state.rs` Methoden sind async und verwenden `tokio::fs`
- [ ] `commands.rs` verwendet `tokio::fs` für alle Dateioperationen
- [ ] `export.rs` verwendet `tokio::fs` für ZIP-Operationen

**Verifikation:**
- [ ] `cargo test` — alle Tests passen
- [ ] `cargo check` — kein Compile-Fehler
- [ ] Keine `std::fs` Aufrufe in async-Kontexten mehr

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
- [ ] `validate_crawl_input` erkennt interne IPs und gibt Warnung zurück
- [ ] Frontend zeigt Warnung an (nicht blockierend)
- [ ] Kein Hard-Block (Desktop-App, nicht Server)

**Verifikation:**
- [ ] `cargo test` — alle Tests passen
- [ ] `cargo check` — kein Compile-Fehler

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/commands.rs` (validate_crawl_input)
- `src/views/NewCrawl.tsx` (Warnung anzeigen)

**Umfang:** Small (1-2 Dateien)

---

## Checkpoint: Phase 1

- [ ] `cargo test` — alle Tests grün
- [ ] `cargo check` — kein Compile-Fehler
- [ ] `cargo check --features headless` — kein Compile-Fehler
- [ ] Manueller Test: Crawl mit `stay_within_domain: true` verlässt nicht die Domain
- [ ] Manueller Test: Crawl mit `respect_robots_txt: true` respektiert robots.txt
- [ ] Review vor Phase 2

---

## Phase 2: Wichtige Fixes

### Task 5: Cancel-Status korrigieren

**Beschreibung:** Cancel setzt `JobStatus::Paused` statt `Failed` oder `Cancelled`. Entweder neuen `Cancelled`-Status einführen oder `Failed` mit separatem Flag verwenden.

**Akzeptanzkriterien:**
- [ ] Cancel setzt korrekten Status (Cancelled oder Failed mit Error-Text)
- [ ] History zeigt abgebrochene Crawls korrekt an
- [ ] Frontend unterscheidet zwischen fehlgeschlagen und abgebrochen

**Verifikation:**
- [ ] `cargo test` — alle Tests passen
- [ ] TypeScript types (`types/index.ts`) sind synchron mit Rust

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
- [ ] `HttpFetcher::new()` akzeptiert Timeout-Parameter
- [ ] `AppSettings.timeout` wird durchgereicht
- [ ] Default bleibt 30s wenn nicht konfiguriert

**Verifikation:**
- [ ] `cargo test` — alle Tests passen

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/fetcher/http.rs`
- `src-tauri/src/crawler/orchestrator.rs` (HttpFetcher::new Aufruf)

**Umfang:** Small (1-2 Dateien)

---

### Task 7: Versionsnummern vereinheitlichen

**Beschreibung:** Einheitliche Versionsnummer in allen Dateien.

**Akzeptanzkriterien:**
- [ ] `Cargo.toml`, `App.tsx`, `HttpFetcher`, `AppSettings` Default — alle `0.2.5`
- [ ] `package.json` — `0.2.5`

**Verifikation:**
- [ ] `cargo test` — alle Tests passen
- [ ] `npm run build` — kein Fehler

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
- [ ] Query-String wird aus Dateinamen entfernt
- [ ] Fragment-Identifiers werden ebenfalls entfernt
- [ ] Unit-Tests für URL → Pfad Konvertierung

**Verifikation:**
- [ ] `cargo test` — alle Tests passen

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/writer/fs.rs`

**Umfang:** Small (1-2 Dateien)

---

### Task 9: Double Update Check beheben

**Beschreibung:** `installUpdate()` ruft `check()` erneut auf statt den gecachten `updateAvailable`-Wert zu verwenden.

**Akzeptanzkriterien:**
- [ ] `installUpdate()` verwendet das gecachte `update`-Objekt
- [ ] Keine doppelte Netzwerkanfrage

**Verifikation:**
- [ ] `npm run build` — kein Fehler

**Dependencies:** Keine

**Betroffene Dateien:**
- `src/hooks/useUpdater.ts`

**Umfang:** XS (1 Datei)

---

### Task 10: Headless Browser wiederverwenden

**Beschreibung:** Für jeden Fetch wird ein neuer Browser-Prozess erstellt. Browser-Instanz wiederverwenden.

**Akzeptanzkriterien:**
- [ ] Browser-Instanz wird pro Crawl-Job erstellt und wiederverwendet
- [ ] Tabs werden nach Fetch geschlossen (nicht der Browser)
- [ ] Kein Memory-Leak

**Verifikation:**
- [ ] `cargo test` — alle Tests passen

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/fetcher/headless.rs`
- `src-tauri/src/crawler/orchestrator.rs`

**Umfang:** Medium (3-5 Dateien)

---

### Task 11: System-Stats cachen

**Beschreibung:** `sysinfo::System::new_all()` + `refresh_all()` werden bei jedem Aufruf erstellt. System-Instanz statisch halten.

**Akzeptanzkriterien:**
- [ ] `System`-Instanz wird einmal erstellt und wiederverwendet
- [ ] Nur `refresh_all()` wird alle 2s aufgerufen
- [ ] CPU-RAM-Uptime Komponente funktioniert weiterhin

**Verifikation:**
- [ ] `cargo test` — alle Tests passen

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/system.rs`
- `src-tauri/src/commands.rs`

**Umfang:** Small (1-2 Dateien)

---

### Task 12: Content-Type + Größenvalidierung bei Downloads

**Beschreibung:** Assets werden ohne MIME-Type-Validierung und ohne Größenlimit heruntergeladen.

**Akzeptanzkriterien:**
- [ ] Content-Type wird geprüft (nur erwartete Typen: image/*, font/*, text/css, application/javascript)
- [ ] Max-Download-Größe konfigurierbar (Default: 50MB)
- [ ] Ungültige Downloads werden übersprungen mit Log-Eintrag

**Verifikation:**
- [ ] `cargo test` — alle Tests passen

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/asset_dl/downloader.rs`

**Umfang:** Medium (3-5 Dateien)

---

## Checkpoint: Phase 2

- [ ] `cargo test` — alle Tests grün
- [ ] `npm run build` — kein Fehler
- [ ] Cancel zeigt korrekten Status in History
- [ ] Timeout-Setting wirkt sich auf Fetcher aus
- [ ] Alle Versionen sind `0.2.5`
- [ ] Review vor Phase 3

---

## Phase 3: Verbesserungen

### Task 13: Dashboard-Polling optimieren

**Beschreibung:** Drei separate Intervalle (Jobs 3s, Stats 3s, Exports 5s) erzeugen unnötige Backend-Aufrufe.

**Akzeptanzkriterien:**
- [ ] Ein einziges Polling-Interval (3s) für alle Daten
- [ ] Stats werden nur alle 10s gefreshed

**Verifikation:**
- [ ] `npm run build` — kein Fehler
- [ ] Dashboard lädt korrekt

**Dependencies:** Keine

**Betroffene Dateien:**
- `src/views/Dashboard.tsx`

**Umfang:** Small (1-2 Dateien)

---

### Task 14: Parallele Asset-Downloads

**Beschreibung:** Pro Seite werden Assets in einer Schleife einzeln heruntergeladen, obwohl ein `JoinSet` verfügbar wäre.

**Akzeptanzkriterien:**
- [ ] Assets werden parallel mit JoinSet heruntergeladen
- [ ] Max parallelDownloads konfigurierbar
- [ ] Fehler bei einem Asset brechen nicht alle anderen ab

**Verifikation:**
- [ ] `cargo test` — alle Tests passen

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/crawler/orchestrator.rs`

**Umfang:** Medium (3-5 Dateien)

---

### Task 15: String-Matching durch Error-Typen ersetzen

**Beschreibung:** `is_disk_error` und `is_transient_error` verwenden String-Matching statt Error-Typen.

**Akzeptanzkriterien:**
- [ ] `is_disk_error` verwendet `std::io::ErrorKind`
- [ ] `is_transient_error` verwendet `reqwest::Error`-Methoden
- [ ] Keine String-Vergleiche mehr für Error-Klassifikation

**Verifikation:**
- [ ] `cargo test` — alle Tests passen

**Dependencies:** Keine

**Betroffene Dateien:**
- `src-tauri/src/crawler/orchestrator.rs`
- `src-tauri/src/fetcher/http.rs`

**Umfang:** Small (1-2 Dateien)

---

### Task 16: LiveConsole Event-Verarbeitung fixen

**Beschreibung:** LiveConsole verarbeitet nur `events[events.length - 1]`, überspringt Events zwischen Renders.

**Akzeptanzkriterien:**
- [ ] Alle neuen Events seit dem letzten Render werden verarbeitet (Index-basiert)
- [ ] Keine verlorenen Events

**Verifikation:**
- [ ] `npm run build` — kein Fehler
- [ ] Manueller Test: Crawl starten und Events beobachten

**Dependencies:** Keine

**Betroffene Dateien:**
- `src/components/LiveConsole.tsx`

**Umfang:** XS (1 Datei)

---

### Task 17: History Lade-Flackern beheben

**Beschreibung:** `loadJobs` setzt `setLoading(true)` bei jedem 3s-Poll, was zu Spinner-Flackern führt.

**Akzeptanzkriterien:**
- [ ] Nur beim initialen Laden `loading=true` setzen
- [ ] Spätere Polls aktualisieren Daten ohne Spinner

**Verifikation:**
- [ ] `npm run build` — kein Fehler

**Dependencies:** Keine

**Betroffene Dateien:**
- `src/views/History.tsx`

**Umfang:** XS (1 Datei)

---

### Task 18: StatusBadge auslagern

**Beschreibung:** StatusIcon/StatusBadge sind in 3 Dateien dupliziert.

**Akzeptanzkriterien:**
- [ ] Gemeinsame Komponente `src/components/StatusBadge.tsx`
- [ ] Alle 3 Dateien importieren die gemeinsame Komponente

**Verifikation:**
- [ ] `npm run build` — kein Fehler

**Dependencies:** Keine

**Betroffene Dateien:**
- `src/components/StatusBadge.tsx` (neu)
- `src/views/Dashboard.tsx`
- `src/views/History.tsx`
- `src/views/NewCrawl.tsx`

**Umfang:** Small (1-2 Dateien)

---

## Checkpoint: Phase 3

- [ ] `cargo test` — alle Tests grün
- [ ] `npm run build` — kein Fehler
- [ ] Dashboard verwendet nur ein Polling-Interval
- [ ] LiveConsole verliert keine Events
- [ ] History hat kein Spinner-Flackern
- [ ] Review vor Phase 4

---

## Phase 4: Nice-to-have

### Task 19: Frontend-Performance (Virtualisierung, Debounce, Logs)

**Beschreibung:** P6-P8 aus PROBLEMS.md: Logs-Array Copy-on-Write, keine Virtualisierung im ResultTree, kein Debounce bei Suche.

**Akzeptanzkriterien:**
- [ ] Logs使用 `useRef` statt State-Array
- [ ] ResultTree使用 `react-window` oder `@tanstack/react-virtual`
- [ ] ResultSearch使用 `useDeferredValue`

**Dependencies:** Keine

**Betroffene Dateien:**
- `src/views/NewCrawl.tsx`
- `src/components/ResultTree.tsx`
- `src/components/ResultSearch.tsx`

---

### Task 20: Code-Qualität (C9-C11)

**Beschreibung:** Regex-basiertes Markdown-Rendering, redundante Log-Speicherung, leere Catch-Blocks.

**Dependencies:** Keine

**Betroffene Dateien:**
- `src/components/MarkdownPreview.tsx`
- `src/views/NewCrawl.tsx`
- `src/views/Dashboard.tsx`

---

### Task 21: Minor Bugfixes (B10-B14)

**Beschreibung:** prefillUrl, AppSettings-Typ, walk_dir-Duplizierung, useUpdater Error State.

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
