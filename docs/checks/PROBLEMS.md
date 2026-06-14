# Docurip — Codebase-Analyse: Bugs, Sicherheit, Performance, Code-Smells

> Analyse vom 14.06.2026 | Version: v0.2.5 (App) / v0.2.0 (DOC.md)

---

## Zusammenfassung

| Kategorie | Kritisch | Hoch | Mittel | Niedrig |
|-----------|----------|------|--------|---------|
| **Bugs** | 3 | 4 | 5 | 2 |
| **Sicherheit** | 2 | 2 | 2 | 1 |
| **Performance** | 1 | 4 | 3 | 1 |
| **Code-Smells** | 0 | 2 | 6 | 3 |
| **Gesamt** | **6** | **12** | **16** | **7** |

---

## 1. Bugs

### B1 — `stay_within_domain` wird nicht durchgesetzt
- **Schweregrad:** Kritisch
- **Datei:** `src-tauri/src/crawler/orchestrator.rs:597-609`
- **Beschreibung:** Die `CrawlConfig`-Konfiguration enthält ein Feld `stay_within_domain`, das aber während der Link-Extraktion nie geprüft wird. Der Crawler folgt Links beliebiger Domains.
- **Fix:** In `process_fetched_page` nach `extract_links` die URLs gegen die Domain der Start-URL filtern.

### B2 — `respect_robots_txt` wird nicht durchgesetzt
- **Schweregrad:** Kritisch
- **Datei:** `src-tauri/src/settings/config.rs`, `src-tauri/src/crawler/orchestrator.rs`
- **Beschreibung:** Das `respect_robots_txt`-Feld existiert in `CrawlConfig` (Default: `true`), wird aber nie während des Crawlings geprüft. Die Einstellung hat keinen Effekt.
- **Fix:** robots.txt parsen und Pfade vor dem Fetch prüfen.

### B3 — Cancel-Status ist `Paused` statt `Failed`
- **Schweregrad:** Hoch
- **Datei:** `src-tauri/src/crawler/orchestrator.rs:299`
- **Beschreibung:** Beim Abbrechen eines Crawls wird `JobStatus::Paused` gesetzt statt `Failed` oder `Cancelled`. Das ist semantisch falsch — ein abgebrochener Crawl ist nicht "pausiert".
- **Fix:** `JobStatus::Failed` oder neuen `Cancelled`-Status einführen.

### B4 — `timeout`-Setting wird nicht an HttpFetcher übergeben
- **Schweregrad:** Hoch
- **Datei:** `src-tauri/src/fetcher/http.rs:55`, `src-tauri/src/settings/config.rs`
- **Beschreibung:** `AppSettings.timeout` (Default: 30000ms) existiert, aber `HttpFetcher::new()` hardcodiert 30s. Der Benutzer kann den Timeout in den Einstellungen ändern, es hat aber keinen Effekt.
- **Fix:** Timeout-Parameter an `HttpFetcher::new()` übergeben.

### B5 — Versionsnummern inkonsistent
- **Schweregrad:** Hoch
- **Datei:** Mehrere Dateien
- **Beschreibung:**
  - `App.tsx` Footer: `v0.2.5`
  - `DOC.md`: `v0.2.0`
  - `HttpFetcher` User-Agent: `Docurip/0.1.0`
  - `AppSettings` Default User-Agent: `Docurip/0.1.0`
- **Fix:** Einheitliche Versionsnummer in allen Dateien.

### B6 — Query-Strings in Dateinamen
- **Schweregrad:** Hoch
- **Datei:** `src-tauri/src/writer/fs.rs`
- **Beschreibung:** `url_to_page_path` und `url_to_asset_path` entfernen Query-Parameter nicht. Eine URL wie `page?id=1` wird zu `page?id=1.md`, was unter Windows ein ungültiger Dateiname ist.
- **Fix:** Query-String vor der Pfaderstellung entfernen.

### B7 — Double Update Check
- **Schweregrad:** Hoch
- **Datei:** `src/hooks/useUpdater.ts`
- **Beschreibung:** `installUpdate()` ruft `check()` erneut auf, anstatt den gecachten `updateAvailable`-Wert zu verwenden. Ergebnis: Doppelte Netzwerkanfrage.
- **Fix:** Den gecachten `update`-Objekt verwenden.

### B8 — LiveConsole verliert Events
- **Schweregrad:** Mittel
- **Datei:** `src/components/LiveConsole.tsx`
- **Beschreibung:** Verarbeitet nur `events[events.length - 1]`, überspringt Events die zwischen zwei Renders eintreffen.
- **Fix:** Alle neuen Events seit dem letzten Render verarbeiten (Index-basiert).

### B9 — History Lade-Flackern
- **Schweregrad:** Mittel
- **Datei:** `src/views/History.tsx`
- **Beschreibung:** `loadJobs` setzt `setLoading(true)` bei jedem 3s-Poll, was zu einem unnötigen Spinner-Flackern führt.
- **Fix:** Nur beim initialen Laden `loading=true` setzen.

### B10 — `prefillUrl` nicht erneut auslösend
- **Schweregrad:** Mittel
- **Datei:** `src/views/NewCrawl.tsx`
- **Beschreibung:** `prefillUrl`-useEffect setzt die URL nur wenn `prev.url` leer ist. Nach manuellem Löschen funktioniert der Prefill nicht mehr.
- **Fix:** Depency-Array korrekt nutzen oder State-Management anpassen.

### B11 — AppSettings-Typ unvollständig
- **Schweregrad:** Mittel
- **Datei:** `src/types/index.ts`
- **Beschreibung:** `AppSettings`-Typ fehlt: `defaultDownloadAssets`, `defaultHeadlessStrategy`, `defaultRespectRobotsTxt` (existieren auf der Rust-Seite).
- **Fix:** Fehlende Felder zum TypeScript-Interface hinzufügen.

### B12 — `walk_dir` Duplizierung
- **Schweregrad:** Mittel
- **Datei:** `src-tauri/src/export.rs`, `src-tauri/src/commands.rs`
- **Beschreibung:** Zwei verschiedene Implementierungen des rekursiven Verzeichnis-Walkers, mit unterschiedlichem Verhalten.
- **Fix:** Einheitliche Implementierung in einem Modul.

### B13 — LiveConsole aktualisiert nicht bei mehreren Events
- **Schweregrad:** Niedrig
- **Datei:** `src/components/LiveConsole.tsx`
- **Beschreibung:** Nur das letzte Event wird angezeigt, wenn mehrere Events zwischen Renders eintreffen.

### B14 — useUpdater Error State wird nie angezeigt
- **Schweregrad:** Niedrig
- **Datei:** `src/hooks/useUpdater.ts`
- **Beschreibung:** `error`-State wird gesetzt, aber nie in der UI angezeigt.

---

## 2. Sicherheitsprobleme

### S1 — Kein SSRF-Schutz
- **Schweregrad:** Kritisch
- **Datei:** `src-tauri/src/commands.rs:56-86` (validate_crawl_input)
- **Beschreibung:** Der Crawler kann interne IPs (localhost, 192.168.x.x, 10.x.x.x) crawlen. Bei einer Desktop-Anwendung ist das weniger kritisch als bei einem Server, aber ein Benutzer könnte versehentlich interne Dienste offenlegen.
- **Empfehlung:** Optionale IP-Blacklist oder Warnung bei RFC1918-Adressen.

### S2 — Keine Content-Type-Validierung bei Asset-Downloads
- **Schweregrad:** Hoch
- **Datei:** `src-tauri/src/asset_dl/downloader.rs`
- **Beschreibung:** Assets werden ohne MIME-Type-Validierung heruntergeladen. Eine Datei könnte als `image/png` angefordert werden, aber ein HTML-Dokument zurückgeben.
- **Empfehlung:** Content-Type prüfen und nur erwartete Typen akzeptieren.

### S3 — Keine Dateigrößenbegrenzung bei Asset-Downloads
- **Schweregrad:** Hoch
- **Datei:** `src-tauri/src/asset_dl/downloader.rs`
- **Beschreibung:** Assets werden ohne Größenlimit heruntergeladen. Eine einzige große Datei könnte den Speicher oder die Festplatte erschöpfen.
- **Empfehlung:** Max-Download-Größe konfigurieren (z.B. 50MB).

### S4 — Regex DoS via `exclude_patterns`
- **Schweregrad:** Mittel
- **Datei:** `src-tauri/src/commands.rs:75`
- **Beschreibung:** Benutzer-definierte Regex-Muster könnten katastrophales Backtracking verursachen (ReDoS).
- **Empfehlung:** Regex-Timeout oder `fancy-regex` mit Timeout verwenden.

### S5 — Path Traversal via Query-Strings
- **Schweregrad:** Mittel
- **Datei:** `src-tauri/src/writer/fs.rs`
- **Beschreibung:** Query-Parameter werden nicht aus Dateinamen entfernt. Unter bestimmten Umständen könnte ein Angreifer über URL-Encoding Pfade manipulieren.
- **Empfehlung:** Query-String und Fragment-Identifiers aus allen Dateinamen entfernen.

### S6 — Output-Pfad nicht sanitisiert im Frontend
- **Schweregrad:** Mittel
- **Datei:** `src/views/Settings.tsx`
- **Beschreibung:** Der Benutzer kann beliebige Pfade als Ausgabeverzeichnis eingeben, inkl. Pfade außerhalb erlaubter Bereiche.
- **Empfehlung:** Pfad-Validierung auch auf Backend-Seite (existsiert bereits in Tauri).

### S7 — Keine Benutzereingabe-Sanitisierung
- **Schweregrad:** Niedrig
- **Datei:** `src/views/NewCrawl.tsx`
- **Beschreibung:** Crawl-Konfiguration (URLs, Patterns) wird ohne Sanitisierung an das Backend gesendet. Da Tauri Commands typisiert sind, ist das Risiko gering.
- **Empfehlung:** URL-Validierung auf Frontend-Seite ergänzen.

---

## 3. Performance-Probleme

### P1 — Synchrones Disk-I/O blockiert Tokio-Runtime
- **Schweregrad:** Kritisch
- **Datei:** `src-tauri/src/state.rs`, `src-tauri/src/commands.rs`
- **Beschreibung:** `std::fs::read_to_string`, `std::fs::write`, `std::fs::read_dir` werden durchgängig auf dem Tokio-Runtime verwendet. Dies blockiert Worker-Threads und kann zu Verzögerungen führen.
- **Fix:** `tokio::fs` oder `spawn_blocking` verwenden.

### P2 — Headless Browser pro Fetch neu erstellt
- **Schweregrad:** Hoch
- **Datei:** `src-tauri/src/fetcher/headless.rs`
- **Beschreibung:** Für jeden Fetch wird ein neuer Browser-Prozess erstellt. Das ist extrem langsam (hunderte Millisekunden pro Start).
- **Fix:** Browser-Instanz wiederverwenden (Singleton-Pattern).

### P3 — System-Stats nicht gecacht
- **Schweregrad:** Hoch
- **Datei:** `src-tauri/src/system.rs:12-13`
- **Beschreibung:** `sysinfo::System::new_all()` + `refresh_all()` werden bei jedem Aufruf erstellt (alle 2s). Das ist unnötig teuer.
- **Fix:** `System`-Instanz statisch halten und nur `refresh()` aufrufen.

### P4 — Dashboard pollt 3× parallel
- **Schweregrad:** Hoch
- **Datei:** `src/views/Dashboard.tsx`
- **Beschreibung:** Drei separate Intervalle: Jobs (3s), Stats (3s), Exports (5s). Das erzeugt unnötige Backend-Aufrufe.
- **Fix:** Ein einziges Polling-Interval mit kombinierten Daten.

### P5 — Assets werden sequenziell heruntergeladen
- **Schweregrad:** Hoch
- **Datei:** `src-tauri/src/crawler/orchestrator.rs:484-519`
- **Beschreibung:** Pro Seite werden Assets in einer Schleife einzeln heruntergeladen, obwohl ein `JoinSet` verfügbar wäre.
- **Fix:** Parallele Asset-Downloads mit JoinSet.

### P6 — Logs-Array Copy-on-Write
- **Schweregrad:** Mittel
- **Datei:** `src/views/NewCrawl.tsx`
- **Beschreibung:** `[...prev, log]` erstellt bei jedem Log-Eintrag ein neues Array — O(n) bei jedem Event.
- **Fix:** `useRef` oder zustand store verwenden.

### P7 — Keine Virtualisierung im ResultTree
- **Schweregrad:** Mittel
- **Datei:** `src/components/ResultTree.tsx`
- **Beschreibung:** Alle Knoten werden gerendert, bei 500+ Seiten performance-problematisch.
- **Fix:** `react-window` oder `@tanstack/react-virtual` verwenden.

### P8 — Kein Debounce bei ResultBrowser-Suche
- **Schweregrad:** Mittel
- **Datei:** `src/components/ResultSearch.tsx`
- **Beschreibung:** Jeder Tastendruck löst eine Filterung über alle Seiten aus.
- **Fix:** `useDeferredValue` oder `debounce` (300ms).

### P9 — `request_delay` vor Semaphore-Akquisition
- **Schweregrad:** Niedrig
- **Datei:** `src-tauri/src/crawler/orchestrator.rs:332-334`
- **Beschreibung:** Die Verzögerung wird *vor* dem Erhalt eines Semaphore-Slots angewendet, was bedeutet, dass die Verzögerung vergeudet wird wenn Slots verfügbar sind.
- **Fix:** Delay *nach* Semaphore-Akquisition anwenden.

---

## 4. Code-Smells

### C1 — `is_disk_error` per String-Matching
- **Schweregrad:** Hoch
- **Datei:** `src-tauri/src/crawler/orchestrator.rs:616-628`
- **Beschreibung:** Disk-Fehler werden über Fehlermeldungs-Strings erkannt statt über Error-Typen. Das ist fragil und kann bei lokalisierter Fehlermeldungen brechen.
- **Empfehlung:** `std::io::ErrorKind` verwenden.

### C2 — `is_transient_error` per String-Matching
- **Schweregrad:** Hoch
- **Datei:** `src-tauri/src/fetcher/http.rs:40-47`
- **Beschreibung:** Selbes Muster wie C1 — Error-Typen statt Strings verwenden.
- **Empfehlung:** `reqwest::Error`-Methoden wie `is_connect()`, `is_timeout()` nutzen.

### C3 — `collect_all_jobs` verwendet `try_read`
- **Schweregrad:** Mittel
- **Datei:** `src-tauri/src/commands.rs:241-262`
- **Beschreibung:** `try_read()` statt `read()` — wenn ein Lock gehalten wird, werden stillschweigend Daten weggeworfen.
- **Empfehlung:** `read().await` mit Timeout oder Mindest-Latenz.

### C4 — DashboardStats Cache mit `std::sync::Mutex`
- **Schweregrad:** Mittel
- **Datei:** `src-tauri/src/commands.rs:195-199`
- **Beschreibung:** `std::sync::Mutex` blockiert den async-Runtime bei Lock-Konkurrenz.
- **Empfehlung:** `tokio::sync::Mutex` verwenden.

### C5 — `rewrite_asset_urls` als String-Replacement
- **Schweregrad:** Mittel
- **Datei:** `src-tauri/src/parser/dom.rs:124-134`
- **Beschreibung:** URLs werden als Strings ersetzt, was Teiler-sätze in Attribut-Werten treffen könnte.
- **Empfehlung:** DOM-aware URL-Rewriting mit `scraper`-API.

### C6 — `merge_md_files` lädt alles in einen String
- **Schweregrad:** Mittel
- **Datei:** `src-tauri/src/export.rs:57-64`
- **Beschreibung:** Alle .md-Dateien werden in einen einzigen String zusammengeführt. Bei sehr großen Crawls könnte das OOM verursachen.
- **Empfehlung:** Streaming oder Chunk-basiertes Zusammenführen.

### C7 — `walk_dir` in commands.rs liest gesamtes Verzeichnis in Speicher
- **Schweregrad:** Mittel
- **Datei:** `src-tauri/src/commands.rs` (export_job_zip)
- **Beschreibung:** `read_dir` + `Vec` statt Streaming für ZIP-Export.
- **Empfehlung:** `walkdir`-Crate oder iteratives Lesen.

### C8 — StatusIcon/StatusBadge dupliziert
- **Schweregrad:** Mittel
- **Datei:** `Dashboard.tsx`, `History.tsx`, `NewCrawl.tsx`
- **Beschreibung:** Identische Komponenten in 3 Dateien dupliziert.
- **Empfehlung:** In eine gemeinsame Datei `src/components/StatusBadge.tsx` auslagern.

### C9 — Regex-basiertes Markdown-Rendering
- **Schweregrad:** Niedrig
- **Datei:** `src/components/MarkdownPreview.tsx`
- **Beschreibung:** Markdown wird mit Regex geparst statt mit einem richtigen Parser. Funktioniert, ist aber fragil.
- **Empfehlung:** `react-markdown` oder `unified`/`remark` verwenden.

### C10 — Redundante Log-Speicherung
- **Schweregrad:** Niedrig
- **Datei:** `src/views/NewCrawl.tsx`
- **Beschreibung:** Lokale `logs`-State AND globale Events beide tracking denselben Daten.
- **Empfehlung:** Nur globale Events verwenden.

### C11 — Dashboard Catch-Blocks leer
- **Schweregrad:** Niedrig
- **Datei:** `src/views/Dashboard.tsx`
- **Beschreibung:** Fehler beim Laden werden stillschweigend ignoriert.
- **Empfehlung:** Fehler in Toast-System oder Fallback-UI anzeigen.

---

## 5. Empfohlene Behebungsreihenfolge

### Phase 1 — Kritische Fixes (sofort)
1. **B1+B2** — `stay_within_domain` und `respect_robots_txt` implementieren
2. **P1** — Synchrones Disk-I/O auf `tokio::fs` umstellen
3. **S1** — SSRF-Schutz (Warnung bei internen IPs)

### Phase 2 — Wichtige Fixes (nächste Woche)
4. **B3** — Cancel-Status korrigieren
5. **B4** — Timeout-Setting an HttpFetcher übergeben
6. **B5** — Versionsnummern vereinheitlichen
7. **B6** — Query-Strings aus Dateinamen entfernen
8. **B7** — Double Update Check beheben
9. **P2** — Headless Browser wiederverwenden
10. **P3** — System-Stats cachen
11. **S2+S3** — Content-Type + Größenvalidierung bei Downloads

### Phase 3 — Verbesserungen (nächster Sprint)
12. **P4** — Dashboard-Polling optimieren
13. **P5** — Parallele Asset-Downloads
14. **C1+C2** — String-Matching durch Error-Typen ersetzen
15. **B8** — LiveConsole Event-Verarbeitung fixen
16. **B9** — History Lade-Flackern beheben
17. **C8** — StatusBadge auslagern

### Phase 4 — Nice-to-have
18. **P6-P8** — Frontend-Performance (Virtualisierung, Debounce, Logs)
19. **C9-C11** — Code-Qualität
20. **B10-B14** — Minor Bugfixes

---

## 6. Dokumentations-Backlog

- DOC.md Version von v0.2.0 auf v0.2.5 aktualisieren
- Fehlende Befehle in der DOC.md dokumentieren (`get_job`, `search_job_results`, `get_session_info`, `list_exports`, `export_job_zip`)
- Neue Settings-Felder dokumentieren (`defaultDownloadAssets`, `defaultHeadlessStrategy`, `defaultRespectRobotsTxt`)
- Security-Abschnitt um SSRF-Info ergänzen
- Performance-Abschnitt um bekannte Einschränkungen ergänzen
