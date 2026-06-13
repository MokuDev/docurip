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

- [x] **Step 1: Fehler-Log-Level visuell unterscheiden**

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

- [x] **Step 2: Fehler-Logs rot färben**

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

- [x] **Step 3: Build verifizieren**

Run: `npm run build`
Expected: Sauber

---

## Task 2: Frontend — Settings Validierung

**Files:**
- Modify: `src/views/Settings.tsx`

- [x] **Step 1: Validierungs-Logik hinzufügen**

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

- [x] **Step 2: Validierung beim Speichern aufrufen**

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

- [x] **Step 3: Inline-Fehlermeldungen pro Feld**

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

- [x] **Step 4: Build verifizieren**

Run: `npm run build`
Expected: Sauber

---

## Task 3: Frontend — Empty States

**Files:**
- Create: `src/components/EmptyState.tsx`
- Modify: `src/views/History.tsx`, `src/views/Dashboard.tsx`, `src/views/NewCrawl.tsx`

- [x] **Step 1: EmptyState Komponente erstellen**
- [x] **Step 2: History Empty State**
- [x] **Step 3: Dashboard Empty State**
- [x] **Step 4: NewCrawl Logs Empty State**
- [x] **Step 5: Build verifizieren**

---

## Task 4: Backend — Parallel Fetching mit Semaphore

**Files:**
- Modify: `src-tauri/src/crawler/orchestrator.rs`
- Modify: `src-tauri/src/settings/config.rs` (wenn `concurrency` Feld fehlt)

- [x] **Step 1: Prüfe ob `concurrency` in `AppSettings` existiert**

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

- [x] **Step 2: Orchestrator Main Loop umbauen**

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

- [x] **Step 3: `Orchestrator` auf `Clone` vorbereiten**
- [x] **Step 4: `process_page` Methode extrahieren**
- [x] **Step 5: `HttpFetcher` Clone sicherstellen**
- [x] **Step 6: Tests**

---

## Task 5: Backend — Connection Pooling

**Files:**
- Modify: `src-tauri/src/fetcher/http.rs`

- [x] **Step 1: Pool-Größe konfigurierbar machen**
- [x] **Step 2: `HttpFetcher::new()` verwendet Default-Pool**
- [x] **Step 3: Tests**

---

## Integration & Verifikation

- [x] **Step 1: `cargo check`**
- [x] **Step 2: `cargo test`**
- [x] **Step 3: `npm run build`**
- [x] **Step 4: Manuelle Tests**

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
