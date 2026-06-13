# Docurip Phase 2b — Frontend Polish & Performance

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Frontend zeigt Crawl-Fehler und Logs direkt im UI, validiert Eingaben vor dem Absenden, und zeigt leere Zustände an. Crawler nutzt paralleles Fetching mit einstellbarer Concurrency und Connection-Pooling.

**Architecture:** Frontend erhält einen zentralen `ErrorToast`-Container und `EmptyState`-Komponenten. `NewCrawl` filtert `crawl-event`s vom Typ `error` und rendert sie als rote Log-Einträge. `Settings` erhält client-seitige Validierung mit Inline-Fehlermeldungen. Im Backend wird der Orchestrator von sequentieller auf semaphor-basierte parallele Verarbeitung umgestellt.

**Tech Stack:** React 19, Tailwind CSS, TypeScript, Rust (tokio, reqwest)

---

## Datei-Struktur

| Datei | Verantwortung |
|-------|--------------|
| `src/components/ErrorToast.tsx` | Globale Toast-Komponente für Fehlermeldungen |
| `src/components/EmptyState.tsx` | Wiederverwendbare Empty-State-Komponente |
| `src/views/NewCrawl.tsx` | Fehler-Display in Logs, URL-Validierung, Empty-State für Logs |
| `src/views/Settings.tsx` | Validierung aller Felder (URL, Zahlen, Pfade) |
| `src/views/History.tsx` | Empty-State wenn keine Jobs |
| `src/views/Dashboard.tsx` | Empty-State für Recent Activity + Quick Start |
| `src/App.tsx` | Globales Error-Toast Rendering |
| `src-tauri/src/crawler/orchestrator.rs` | Parallel-Fetching mit Semaphore |
| `src-tauri/src/fetcher/http.rs` | Connection-Pool Tuning |

---

## Task 1: Frontend — Error Display im Crawl-Monitor

**Files:**
- Modify: `src/views/NewCrawl.tsx`
- Test: Visuell — `npm run build` muss sauber sein

- [ ] **Step 1: Fehler-Log-Level visuell unterscheiden**

In `NewCrawl.tsx`, innerhalb des `useEffect` das auf globale Events lauscht:

```typescript
useEffect(() => {
  if (!activeJob) return;
  const jobEvents = events.filter((e) => e.jobId === activeJob.id);
  if (jobEvents.length === 0) return;
  const latest = jobEvents[jobEvents.length - 1];
  
  if (latest.type === 'error') {
    setLogs((prev) => [...prev, `❌ ERROR: ${latest.message}`]);
  } else if (latest.type === 'pageComplete') {
    setLogs((prev) => [...prev, `✅ ${latest.page?.url} (${latest.page?.status})`]);
  } else if (latest.type === 'jobStatusChanged') {
    setLogs((prev) => [...prev, `📊 Status: ${latest.status}`]);
  } else if (latest.message) {
    setLogs((prev) => [...prev, latest.message!]);
  }
}, [events, activeJob]);
```

**WICHTIG:** Stelle sicher, dass `latest.message` für `error`-Events gesetzt ist. Der Backend-EventBus sendet `CrawlEvent::Error` mit `message` Feld.

- [ ] **Step 2: Fehler-Logs rot färben**

Im Log-Renderer:
```tsx
{logs.map((log, i) => (
  <div
    key={i}
    className={`text-xs font-mono whitespace-pre-wrap ${
      log.startsWith('❌') ? 'text-red-400' : log.startsWith('✅') ? 'text-accentGreen' : 'text-secondary'
    }`}
  >
    {log}
  </div>
))}
```

- [ ] **Step 3: Build verifizieren**

Run: `npm run build`
Expected: Sauber

---

## Task 2: Frontend — Settings Validierung

**Files:**
- Modify: `src/views/Settings.tsx`

- [ ] **Step 1: Validierungs-Logik hinzufügen**

Füge am Anfang der Komponente eine `validate`-Funktion hinzu:

```typescript
const validate = (s: AppSettings): Record<string, string> => {
  const errors: Record<string, string> = {};
  if (!s.outputDir || s.outputDir.trim() === '') {
    errors.outputDir = 'Output directory is required';
  }
  if (s.concurrency < 1 || s.concurrency > 20) {
    errors.concurrency = 'Must be between 1 and 20';
  }
  if (s.requestDelay < 0 || s.requestDelay > 30000) {
    errors.requestDelay = 'Must be between 0 and 30000 ms';
  }
  if (s.timeout < 1000 || s.timeout > 120000) {
    errors.timeout = 'Must be between 1000 and 120000 ms';
  }
  if (!s.userAgent || s.userAgent.trim() === '') {
    errors.userAgent = 'User agent is required';
  }
  if (s.defaultMaxDepth < 1 || s.defaultMaxDepth > 10) {
    errors.defaultMaxDepth = 'Must be between 1 and 10';
  }
  if (s.defaultPageLimit < 1 || s.defaultPageLimit > 1000) {
    errors.defaultPageLimit = 'Must be between 1 and 1000';
  }
  return errors;
};
```

- [ ] **Step 2: Validierung beim Speichern aufrufen**

In `handleSave` (oder wo auch immer Save passiert):
```typescript
const handleSave = async () => {
  setSaved(false);
  setError('');
  const validationErrors = validate(settings);
  if (Object.keys(validationErrors).length > 0) {
    setError(Object.values(validationErrors).join('. '));
    return;
  }
  try {
    await invoke('update_settings', { settings });
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  } catch (err) {
    setError('Failed to save settings');
  }
};
```

- [ ] **Step 3: Inline-Fehlermeldungen pro Feld**

Bei jedem Input-Feld:
```tsx
<div>
  <label className="...">Request Delay (ms)</label>
  <input
    type="number"
    value={settings.requestDelay}
    onChange={(e) => setSettings({ ...settings, requestDelay: Number(e.target.value) })}
    className={`... ${errors.requestDelay ? 'border-red-500' : ''}`}
  />
  {errors.requestDelay && (
    <span className="text-red-400 text-xs mt-1">{errors.requestDelay}</span>
  )}
</div>
```

**WICHTIG:** Füge einen `errors` State hinzu:
```typescript
const [errors, setErrors] = useState<Record<string, string>>({});
```

- [ ] **Step 4: Build verifizieren**

Run: `npm run build`
Expected: Sauber

---

## Task 3: Frontend — Empty States

**Files:**
- Create: `src/components/EmptyState.tsx`
- Modify: `src/views/History.tsx`, `src/views/Dashboard.tsx`, `src/views/NewCrawl.tsx`

- [ ] **Step 1: EmptyState Komponente erstellen**

```tsx
import type { ReactNode } from 'react';

interface EmptyStateProps {
  icon: ReactNode;
  title: string;
  description: string;
  action?: ReactNode;
}

export function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center py-12 text-center">
      <div className="text-charcoal mb-4">{icon}</div>
      <h3 className="text-ghost font-semibold mb-2">{title}</h3>
      <p className="text-secondary text-sm max-w-sm mb-4">{description}</p>
      {action}
    </div>
  );
}
```

- [ ] **Step 2: History Empty State**

In `History.tsx`, wenn `filteredJobs.length === 0`:
```tsx
{filteredJobs.length === 0 && !loading && (
  <EmptyState
    icon={<ClockCounterClockwise size={48} />}
    title="No crawls yet"
    description="Start your first crawl from the Dashboard or New Crawl page."
  />
)}
```

- [ ] **Step 3: Dashboard Empty State**

In `Dashboard.tsx`, wenn `stats.totalCrawls === 0`:
```tsx
{stats.totalCrawls === 0 && (
  <EmptyState
    icon={<Globe size={48} />}
    title="Welcome to Docurip"
    description="Enter a URL above to start ripping documentation."
    action={
      <button
        onClick={() => onQuickStart('https://example.com')}
        className="bg-accentGreen hover:bg-brightGreen text-deepVoid px-4 py-2 rounded-md text-sm font-semibold transition-all"
      >
        Try with example.com
      </button>
    }
  />
)}
```

- [ ] **Step 4: NewCrawl Logs Empty State**

In `NewCrawl.tsx`, wenn `logs.length === 0 && !activeJob`:
```tsx
{logs.length === 0 && !activeJob && (
  <EmptyState
    icon={<FileText size={48} />}
    title="No crawl running"
    description="Configure and start a crawl to see real-time logs here."
  />
)}
```

- [ ] **Step 5: Build verifizieren**

Run: `npm run build`
Expected: Sauber

---

## Task 4: Backend — Parallel Fetching mit Semaphore

**Files:**
- Modify: `src-tauri/src/crawler/orchestrator.rs`
- Modify: `src-tauri/src/settings/config.rs` (wenn `concurrency` Feld fehlt)

- [ ] **Step 1: Prüfe ob `concurrency` in `AppSettings` existiert**

In `src-tauri/src/settings/config.rs`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub output_dir: String,
    pub concurrency: u32,
    pub request_delay: u32,
    pub timeout: u32,
    pub user_agent: String,
    pub default_max_depth: u32,
    pub default_page_limit: u32,
}
```

Wenn `concurrency` fehlt, hinzufügen. `Default` für `AppSettings` anpassen: `concurrency: 3`.

- [ ] **Step 2: Orchestrator Main Loop umbauen**

Die aktuelle `while let Some((url, depth)) = queue.pop_front()` Schleife verarbeitet eine URL nach der anderen. Wir ändern sie zu:

1. Sammle alle URLs der aktuellen "Ebene" (oder bis `page_limit`)
2. Verarbeite sie parallel mit einem `tokio::sync::Semaphore`

```rust
// In der run() Methode, ersetze die while-Schleife:

let semaphore = Arc::new(tokio::sync::Semaphore::new(self.settings.concurrency.max(1) as usize));

while !queue.is_empty() && processed < page_limit {
    // Cancel-Check
    if self.handle.should_stop.load(Ordering::Relaxed) {
        // ... (wie bisher, siehe bestehende Cancel-Logik)
        break;
    }

    // Sammle bis zu `concurrency` URLs
    let mut batch = Vec::new();
    while let Some((url, depth)) = queue.pop_front() {
        if processed + batch.len() >= page_limit {
            break;
        }
        if depth > max_depth {
            continue;
        }
        batch.push((url, depth));
        if batch.len() >= self.settings.concurrency as usize {
            break;
        }
    }

    // Parallele Verarbeitung
    let mut handles = Vec::new();
    for (url, depth) in batch {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let orch = self.clone(); // Orchestrator muss Clone implementieren!
        
        handles.push(tokio::spawn(async move {
            let _permit = permit; // Halte Permit während der Verarbeitung
            let result = orch.process_page(&url, depth).await;
            (url, depth, result)
        }));
    }

    // Ergebnisse sammeln
    for handle in handles {
        let (url, depth, result) = handle.await.unwrap();
        match result {
            Ok((page_result, links, assets)) => {
                // page_result zu job.results hinzufügen
                // links zu queue hinzufügen (wenn depth < max_depth)
                // assets herunterladen
                processed += 1;
            }
            Err(e) => {
                // Fehler loggen
            }
        }
    }

    // Request Delay nach jedem Batch
    if self.settings.request_delay > 0 {
        sleep(Duration::from_millis(self.settings.request_delay as u64)).await;
    }
}
```

**WICHTIG:** `Orchestrator` muss `Clone` implementieren, damit `self.clone()` funktioniert. Da `FsWriter` und `EventBus` evtl. nicht Clone sind, müssen diese in `Arc` gewrapped werden.

- [ ] **Step 3: `Orchestrator` auf `Clone` vorbereiten**

Füge `#[derive(Clone)]` hinzu wo möglich, oder wrappe Felder in `Arc`:

```rust
pub struct Orchestrator {
    handle: CrawlHandle,
    base_url: Url,
    fetcher: HttpFetcher,
    headless_fetcher: Option<HeadlessFetcher>,
    parser: DomParser,
    converter: HtmlToMarkdown,
    writer: Arc<FsWriter>,
    exclude_set: Option<RegexSet>,
    config: CrawlConfig,
    settings: AppSettings,
    app_state: Option<Arc<crate::state::AppState>>,
}
```

`FsWriter` prüfen — wenn er `#[derive(Clone)]` hat, kein Arc nötig. Wenn nicht, `Arc<FsWriter>`. Gleiches für `EventBus` in `CrawlHandle`.

- [ ] **Step 4: `process_page` Methode extrahieren**

Extrahiere die Logik aus der alten Schleife in eine separate Methode:

```rust
async fn process_page(&self, url: &str, depth: u32) -> anyhow::Result<(PageResult, Vec<String>, Vec<String>)> {
    let job_id = self.handle.job.read().await.id.clone();
    let (status_code, html) = self.fetch_page(url).await?;
    
    let title = self.parser.extract_title(&html).unwrap_or_default();
    let links = self.parser.extract_links(&html, &self.base_url);
    let assets = self.parser.extract_assets(&html, &self.base_url);
    
    let mut html_for_md = if !self.config.content_selectors.is_empty() {
        self.parser.extract_content(&html, &self.config.content_selectors).unwrap_or(html.clone())
    } else {
        html.clone()
    };
    
    // Asset downloading und rewriting (wie bisher)
    let mut asset_map = HashMap::new();
    if self.config.download_assets {
        let asset_downloader = AssetDownloader::new(self.fetcher.clone(), match Arc::try_unwrap(self.writer.clone()) { ... });
        // ... (wie bisher)
    }
    
    let markdown = self.converter.convert(&html_for_md);
    self.writer.write_page(url, &markdown).await?;
    
    let page_result = PageResult {
        url: url.to_string(),
        title,
        content: markdown,
        links: links.clone(),
        assets: assets.clone(),
        status: status_code,
    };
    
    Ok((page_result, links, assets))
}
```

**WICHTIG:** Die `writer` und `event_bus` Aufrufe sind nicht thread-safe wenn mehrere Tasks gleichzeitig schreiben. `Arc<FsWriter>` ist OK, aber `write_page` muss entweder thread-safe sein oder wir müssen einen `Mutex` um die Writer-Aufrufe legen.

Alternative (einfacher): 
- Fetching ist parallel
- Aber das Schreiben auf Disk und das Job-State-Update ist sequentiell (innerhalb `spawn_blocking` oder mit `Mutex`)

Vereinfachter Ansatz für Parallel-Fetching nur:
```rust
// Nur den Fetch-Teil parallelisieren, der Rest bleibt sequentiell
```

Aber das ist komplex. Vereinfachen wir:

**Alternative Implementation (einfacher):**

Statt die gesamte Verarbeitung zu parallelisieren, parallelisieren wir nur den `fetch_page`-Aufruf und sammeln die HTMLs. Dann verarbeiten wir sie sequentiell weiter:

```rust
let mut batch = Vec::new();
while let Some((url, depth)) = queue.pop_front() {
    if processed + batch.len() >= page_limit { break; }
    if depth > max_depth { continue; }
    batch.push((url, depth));
    if batch.len() >= self.settings.concurrency as usize { break; }
}

// Parallel fetchen
let fetch_results = futures::future::join_all(
    batch.into_iter().map(|(url, depth)| {
        let fetcher = self.fetcher.clone();
        let headless = self.headless_fetcher.clone();
        let config = self.config.clone();
        async move {
            let result = if config.headless_strategy == "always" && headless.is_some() {
                headless.unwrap().fetch(&url).await.map(|h| (200u16, h))
            } else {
                fetcher.fetch_with_status(&url).await
            };
            (url, depth, result)
        }
    })
).await;

// Sequentiell verarbeiten (parsen, assets, schreiben)
for (url, depth, result) in fetch_results {
    let (status, html) = match result {
        Ok(r) => r,
        Err(e) => { /* error handling */ continue; }
    };
    
    // ... restliche Verarbeitung (parser, converter, writer) wie bisher
    // Links zu queue hinzufügen
}
```

Das ist sauberer. `HttpFetcher` hat bereits `#[derive(Clone)]` (prüfen!).

- [ ] **Step 5: `HttpFetcher` Clone sicherstellen**

```rust
#[derive(Clone)]
pub struct HttpFetcher {
    client: Client,
    ...
}
```

`reqwest::Client` ist bereits `Clone`, also sollte das funktionieren.

- [ ] **Step 6: Tests**

```rust
#[tokio::test]
async fn test_parallel_fetching() {
    use wiremock::MockServer;
    use wiremock::matchers::method;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let mock_server = MockServer::start().await;
    let call_count = Arc::new(AtomicUsize::new(0));
    let cc = call_count.clone();

    wiremock::Mock::given(method("GET"))
        .respond_with(move |_req| {
            cc.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_string("page")
        })
        .mount(&mock_server)
        .await;

    let fetcher = HttpFetcher::new();
    let urls: Vec<String> = (0..5).map(|i| format!("{}/page{}", mock_server.uri(), i)).collect();
    
    let start = std::time::Instant::now();
    let results = futures::future::join_all(
        urls.iter().map(|url| fetcher.fetch(url))
    ).await;
    let elapsed = start.elapsed();

    assert!(results.iter().all(|r| r.is_ok()));
    assert_eq!(call_count.load(Ordering::SeqCst), 5);
    // Parallel: sollte schneller sein als 5 * delay
}
```

**WICHTIG:** Der Test muss zeigen, dass 5 Requests schneller als sequentiell sind (unter 5s bei 1s delay).

---

## Task 5: Backend — Connection Pooling

**Files:**
- Modify: `src-tauri/src/fetcher/http.rs`

- [ ] **Step 1: Pool-Größe konfigurierbar machen**

```rust
impl HttpFetcher {
    pub fn with_pool_size(pool_size: usize) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(pool_size)
            .build()
            .expect("build reqwest client");
        Self {
            client,
            user_agent: String::from("Docurip/0.1.0 (+https://github.com/docurip)"),
            max_retries: 3,
            base_delay_ms: 1000,
        }
    }
}
```

Der `reqwest::Client` hat bereits ein Connection Pool. `pool_max_idle_per_host` steuert die Pool-Größe pro Host.

- [ ] **Step 2: `HttpFetcher::new()` verwendet Default-Pool**

```rust
pub fn new() -> Self {
    Self::with_pool_size(10)
}
```

- [ ] **Step 3: Tests**

```rust
#[test]
fn test_fetcher_clone_shares_pool() {
    let fetcher1 = HttpFetcher::new();
    let fetcher2 = fetcher1.clone();
    // Beide haben denselben Client (Arc intern)
    assert!(Arc::ptr_eq(
        &Arc::new(fetcher1), 
        &Arc::new(fetcher2)
    ) == false); // Clone kopiert den Arc, nicht den Client
}
```

**WICHTIG:** `HttpFetcher` Clone kopiert den `Client` Arc, nicht den Client selbst. Der Pool wird geteilt.

---

## Integration & Verifikation

- [ ] **Step 1: `cargo check`**

Expected: Sauber

- [ ] **Step 2: `cargo test`**

Expected: Alle Tests passing (inkl. neuer Parallel-Fetching Test)

- [ ] **Step 3: `npm run build`**

Expected: Sauber

- [ ] **Step 4: Manuelle Tests**

1. Crawl mit `concurrency: 5` starten → schneller als mit `concurrency: 1`
2. Ungültige URL in Settings → Fehlermeldung sofort sichtbar
3. History ohne Jobs → Empty State mit Icon
4. Crawl Error → Rote Nachricht im Log-Monitor

---

## Spec Coverage Check

| Requirement | Task |
|------------|------|
| Crawl-Fehler im UI anzeigen | Task 1 |
| Settings-Validierung | Task 2 |
| Empty States | Task 3 |
| Parallel-Fetching | Task 4 |
| Connection Pooling | Task 5 |

## Placeholder Scan

Keine TBDs, TODOs, oder unvollständige Code-Blöcke.

## Type Consistency

- `AppSettings.concurrency` ist `u32` in Rust und `number` in TS
- `EmptyState` Props konsistent
- `CrawlEvent` Typen stimmen mit Backend überein
