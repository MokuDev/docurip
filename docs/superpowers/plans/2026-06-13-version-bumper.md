# Version-Bumper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a standalone `version-bumper.html` that bumps versions across 4 project files, with optional git-commit and build/release command generation.

**Architecture:** Single HTML file with embedded CSS/JS. Uses File System Access API (Chromium) for direct file manipulation, with upload/download fallback for other browsers. Git/npm steps generate copy-paste shell commands since browsers cannot execute them directly.

**Tech Stack:** Vanilla HTML/CSS/JS, File System Access API, no build step or dependencies.

> **Status (verifiziert gegen v0.3.3):** ✅ Alle Tasks erledigt. `version-bumper.html` existiert im Projekt-Root (468 Zeilen) mit Directory-Picker, Versionserkennung, Preview-Diffs, Schreiblogik, Git/Release-Command-Generierung und Fallback für Non-Chromium-Browser.

---

### Task 1: HTML Skeleton + CSS Styling

**Files:**
- Create: `version-bumper.html`

- [x] **Step 1: Write the HTML skeleton**

Create `version-bumper.html` with the following structure:

```html
<!DOCTYPE html>
<html lang="de">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>docurip Version Bumper</title>
  <style>
    /* styles go here in Task 2 */
  </style>
</head>
<body>
  <div class="container">
    <h1>🚀 docurip Version Bumper</h1>

    <section class="step">
      <h2>1. Projektordner wählen</h2>
      <button id="pickFolderBtn">📁 Ordner auswählen</button>
      <span id="folderName"></span>
    </section>

    <section class="step" id="versionSection" style="display:none;">
      <h2>2. Version bumpen</h2>
      <div class="current-version">
        Aktuelle Version: <span id="currentVersion">-</span>
      </div>
      <label>Neue Version:</label>
      <input type="text" id="newVersion" placeholder="z.B. 0.3.0" />
      <button id="previewBtn">🔍 Preview</button>
      <button id="bumpBtn" disabled>✅ Bump!</button>
    </section>

    <section class="step" id="previewSection" style="display:none;">
      <h2>3. Preview</h2>
      <div id="previewContent"></div>
    </section>

    <section class="step" id="optionsSection" style="display:none;">
      <h2>4. Optionen</h2>
      <label><input type="checkbox" id="gitCommitCheck" /> Git-Commit erstellen</label>
      <label><input type="checkbox" id="buildReleaseCheck" /> Build + Release ausführen</label>
      <div id="commandOutput" style="display:none;">
        <h3>Auszuführende Commands:</h3>
        <pre id="commandBlock"></pre>
        <button id="copyCommandsBtn">📋 Kopieren</button>
      </div>
    </section>

    <section class="step" id="fallbackSection" style="display:none;">
      <h2>Browser nicht unterstützt</h2>
      <p>Dein Browser unterstützt das Dateisystem nicht direkt. Bitte lade die 4 Dateien hoch:</p>
      <input type="file" id="fallbackFiles" multiple accept=".json,.toml,.tsx" />
      <div id="fallbackOutput"></div>
    </section>

    <div id="statusMessage"></div>
  </div>

  <script>
    // JavaScript goes here in subsequent tasks
  </script>
</body>
</html>
```

- [x] **Step 2: Commit**

```bash
git add version-bumper.html
git commit -m "feat: add version-bumper HTML skeleton"
```

---

### Task 2: CSS Styling

**Files:**
- Modify: `version-bumper.html` (replace `<style>` content)

- [x] **Step 1: Replace the `<style>` block with complete styling**

Replace the empty `<style>` tag in `version-bumper.html` with:

```css
* { box-sizing: border-box; margin: 0; padding: 0; }

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  background: #0f172a;
  color: #e2e8f0;
  padding: 2rem;
  line-height: 1.6;
}

.container {
  max-width: 800px;
  margin: 0 auto;
}

h1 {
  font-size: 1.5rem;
  margin-bottom: 2rem;
  color: #4ade80;
}

.step {
  background: #1e293b;
  border: 1px solid #334155;
  border-radius: 8px;
  padding: 1.5rem;
  margin-bottom: 1.5rem;
}

.step h2 {
  font-size: 1rem;
  margin-bottom: 1rem;
  color: #94a3b8;
}

button {
  background: #4ade80;
  color: #0f172a;
  border: none;
  padding: 0.5rem 1rem;
  border-radius: 4px;
  font-weight: 600;
  cursor: pointer;
  margin-right: 0.5rem;
  margin-top: 0.5rem;
}

button:hover:not(:disabled) {
  background: #22c55e;
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

input[type="text"] {
  background: #0f172a;
  border: 1px solid #334155;
  color: #e2e8f0;
  padding: 0.5rem;
  border-radius: 4px;
  width: 200px;
  font-size: 1rem;
  margin-right: 0.5rem;
}

input[type="checkbox"] {
  margin-right: 0.5rem;
}

label {
  display: block;
  margin: 0.5rem 0;
}

.current-version {
  font-size: 1.1rem;
  margin-bottom: 1rem;
  color: #94a3b8;
}

#currentVersion {
  color: #4ade80;
  font-weight: bold;
  font-family: monospace;
}

.preview-file {
  background: #0f172a;
  border: 1px solid #334155;
  border-radius: 4px;
  padding: 1rem;
  margin: 0.5rem 0;
  font-family: monospace;
  font-size: 0.85rem;
}

.preview-file h3 {
  font-size: 0.85rem;
  color: #94a3b8;
  margin-bottom: 0.5rem;
}

.diff-line {
  display: block;
  padding: 0.15rem 0.5rem;
  border-radius: 2px;
}

.diff-remove {
  background: #7f1d1d;
  color: #fca5a5;
}

.diff-add {
  background: #14532d;
  color: #86efac;
}

#commandBlock {
  background: #0f172a;
  border: 1px solid #334155;
  border-radius: 4px;
  padding: 1rem;
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-all;
}

#statusMessage {
  margin-top: 1rem;
  padding: 0.75rem;
  border-radius: 4px;
  display: none;
}

#statusMessage.success {
  background: #14532d;
  color: #86efac;
  display: block;
}

#statusMessage.error {
  background: #7f1d1d;
  color: #fca5a5;
  display: block;
}

#folderName {
  margin-left: 1rem;
  color: #4ade80;
  font-family: monospace;
}
```

- [x] **Step 2: Commit**

```bash
git add version-bumper.html
git commit -m "style: add complete CSS for version-bumper"
```

---

### Task 3: Directory Picker + File Reading (Chromium)

**Files:**
- Modify: `version-bumper.html` (replace `<script>` content)

- [x] **Step 1: Implement directory picker and version extraction**

Replace the `<script>` tag content in `version-bumper.html` with:

```javascript
const TARGET_FILES = [
  { name: 'package.json', path: 'package.json', pattern: /"version":\s*"([^"]+)"/ },
  { name: 'tauri.conf.json', path: 'src-tauri/tauri.conf.json', pattern: /"version":\s*"([^"]+)"/ },
  { name: 'Cargo.toml', path: 'src-tauri/Cargo.toml', pattern: /^version\s*=\s*"([^"]+)"/m },
  { name: 'App.tsx', path: 'src/App.tsx', pattern: /v(\d+\.\d+\.\d+)/ }
];

let directoryHandle = null;
let currentVersions = {};
let fileContents = {};

document.getElementById('pickFolderBtn').addEventListener('click', pickDirectory);
document.getElementById('previewBtn').addEventListener('click', showPreview);
document.getElementById('bumpBtn').addEventListener('click', bumpVersions);

async function pickDirectory() {
  try {
    directoryHandle = await window.showDirectoryPicker();
    document.getElementById('folderName').textContent = directoryHandle.name;
    await loadFiles();
  } catch (err) {
    showStatus('Fehler beim Öffnen des Ordners: ' + err.message, 'error');
  }
}

async function loadFiles() {
  currentVersions = {};
  fileContents = {};
  let allMatch = true;
  let firstVersion = null;

  for (const target of TARGET_FILES) {
    try {
      const fileHandle = await directoryHandle.getFileHandle(target.path);
      const file = await fileHandle.getFile();
      const content = await file.text();
      fileContents[target.path] = content;

      const match = content.match(target.pattern);
      if (match) {
        const version = match[1];
        currentVersions[target.name] = version;
        if (firstVersion === null) firstVersion = version;
        if (version !== firstVersion) allMatch = false;
      } else {
        currentVersions[target.name] = 'NICHT GEFUNDEN';
        allMatch = false;
      }
    } catch (err) {
      currentVersions[target.name] = 'FEHLER: ' + err.message;
      allMatch = false;
    }
  }

  const versionSection = document.getElementById('versionSection');
  versionSection.style.display = 'block';
  document.getElementById('currentVersion').textContent = firstVersion || '-';

  if (!allMatch && firstVersion) {
    showStatus('Warnung: Versionen sind nicht konsistent über alle Dateien!', 'error');
  }
}

function showStatus(msg, type) {
  const el = document.getElementById('statusMessage');
  el.textContent = msg;
  el.className = type;
}
```

- [x] **Step 2: Commit**

```bash
git add version-bumper.html
git commit -m "feat: add directory picker and file reading"
```

---

### Task 4: Version Validation + Preview

**Files:**
- Modify: `version-bumper.html` (add to `<script>`)

- [x] **Step 1: Add semver validation and preview rendering**

Add these functions to the `<script>` in `version-bumper.html`:

```javascript
function isValidSemver(version) {
  return /^\d+\.\d+\.\d+$/.test(version);
}

document.getElementById('newVersion').addEventListener('input', (e) => {
  const val = e.target.value.trim();
  document.getElementById('bumpBtn').disabled = !isValidSemver(val);
});

function showPreview() {
  const newVersion = document.getElementById('newVersion').value.trim();
  if (!isValidSemver(newVersion)) {
    showStatus('Bitte gültige Version eingeben (z.B. 0.3.0)', 'error');
    return;
  }

  const previewSection = document.getElementById('previewSection');
  const previewContent = document.getElementById('previewContent');
  previewContent.innerHTML = '';

  for (const target of TARGET_FILES) {
    const oldContent = fileContents[target.path];
    const newContent = oldContent.replace(target.pattern, (match, p1) => {
      return match.replace(p1, newVersion);
    });

    const div = document.createElement('div');
    div.className = 'preview-file';
    div.innerHTML = '<h3>' + target.name + '</h3>';

    const oldLines = oldContent.split('\n');
    const newLines = newContent.split('\n');
    const maxLines = Math.max(oldLines.length, newLines.length);

    for (let i = 0; i < maxLines; i++) {
      const oldLine = oldLines[i] || '';
      const newLine = newLines[i] || '';
      if (oldLine !== newLine) {
        if (oldLine) {
          const removeSpan = document.createElement('span');
          removeSpan.className = 'diff-line diff-remove';
          removeSpan.textContent = '- ' + oldLine;
          div.appendChild(removeSpan);
        }
        if (newLine) {
          const addSpan = document.createElement('span');
          addSpan.className = 'diff-line diff-add';
          addSpan.textContent = '+ ' + newLine;
          div.appendChild(addSpan);
        }
      }
    }

    previewContent.appendChild(div);
  }

  previewSection.style.display = 'block';
  previewSection.scrollIntoView({ behavior: 'smooth' });

  const optionsSection = document.getElementById('optionsSection');
  optionsSection.style.display = 'block';
  updateCommands(newVersion);
}
```

- [x] **Step 2: Commit**

```bash
git add version-bumper.html
git commit -m "feat: add version validation and preview diff"
```

---

### Task 5: Write Files Back + Git/Release Commands

**Files:**
- Modify: `version-bumper.html` (add to `<script>`)

- [x] **Step 1: Implement file writing and command generation**

Add these functions to the `<script>` in `version-bumper.html`:

```javascript
async function bumpVersions() {
  const newVersion = document.getElementById('newVersion').value.trim();
  if (!isValidSemver(newVersion)) {
    showStatus('Ungültige Version!', 'error');
    return;
  }

  try {
    for (const target of TARGET_FILES) {
      const fileHandle = await directoryHandle.getFileHandle(target.path, { writable: true });
      const writable = await fileHandle.createWritable();
      const newContent = fileContents[target.path].replace(target.pattern, (match, p1) => {
        return match.replace(p1, newVersion);
      });
      await writable.write(newContent);
      await writable.close();
    }

    showStatus('Version erfolgreich auf ' + newVersion + ' gebumpet!', 'success');
    await loadFiles();
  } catch (err) {
    showStatus('Fehler beim Schreiben: ' + err.message, 'error');
  }
}

function updateCommands(newVersion) {
  const doCommit = document.getElementById('gitCommitCheck').checked;
  const doRelease = document.getElementById('buildReleaseCheck').checked;
  const commandOutput = document.getElementById('commandOutput');

  if (!doCommit && !doRelease) {
    commandOutput.style.display = 'none';
    return;
  }

  let commands = [];
  if (doCommit) {
    commands.push('git add -A');
    commands.push('git commit -m "chore: bump version to ' + newVersion + '"');
  }
  if (doRelease) {
    if (doCommit) {
      commands.push('');
    }
    commands.push('npm run release');
    if (doCommit) {
      commands.push('# oder für GitHub-Publish:');
      commands.push('npm run release:publish');
    }
  }

  document.getElementById('commandBlock').textContent = commands.join('\n');
  commandOutput.style.display = 'block';
}

document.getElementById('gitCommitCheck').addEventListener('change', () => {
  const newVersion = document.getElementById('newVersion').value.trim();
  if (isValidSemver(newVersion)) updateCommands(newVersion);
});

document.getElementById('buildReleaseCheck').addEventListener('change', () => {
  const newVersion = document.getElementById('newVersion').value.trim();
  if (isValidSemver(newVersion)) updateCommands(newVersion);
});

document.getElementById('copyCommandsBtn').addEventListener('click', () => {
  const text = document.getElementById('commandBlock').textContent;
  navigator.clipboard.writeText(text).then(() => {
    showStatus('Commands in Zwischenablage kopiert!', 'success');
  });
});
```

- [x] **Step 2: Commit**

```bash
git add version-bumper.html
git commit -m "feat: add file writing and git/release command generation"
```

---

### Task 6: Fallback for Non-Chromium Browsers

**Files:**
- Modify: `version-bumper.html` (add to `<script>`)

- [x] **Step 1: Implement upload/download fallback**

Add this to the end of the `<script>` in `version-bumper.html`:

```javascript
if (!('showDirectoryPicker' in window)) {
  document.getElementById('pickFolderBtn').style.display = 'none';
  document.getElementById('fallbackSection').style.display = 'block';

  document.getElementById('fallbackFiles').addEventListener('change', async (e) => {
    const files = Array.from(e.target.files);
    fileContents = {};
    currentVersions = {};
    let allMatch = true;
    let firstVersion = null;

    for (const file of files) {
      const content = await file.text();
      for (const target of TARGET_FILES) {
        if (file.name === target.name || file.name === target.path.split('/').pop()) {
          fileContents[target.path] = content;
          const match = content.match(target.pattern);
          if (match) {
            const version = match[1];
            currentVersions[target.name] = version;
            if (firstVersion === null) firstVersion = version;
            if (version !== firstVersion) allMatch = false;
          } else {
            currentVersions[target.name] = 'NICHT GEFUNDEN';
            allMatch = false;
          }
        }
      }
    }

    document.getElementById('versionSection').style.display = 'block';
    document.getElementById('currentVersion').textContent = firstVersion || '-';

    if (!allMatch && firstVersion) {
      showStatus('Warnung: Versionen sind nicht konsistent!', 'error');
    }
  });
}
```

- [x] **Step 2: Commit**

```bash
git add version-bumper.html
git commit -m "feat: add fallback for non-Chromium browsers"
```

---

### Task 7: Final Integration Test

**Files:**
- Modify: `version-bumper.html` (manual test)

- [x] **Step 1: Manual test in Chrome**

1. Open `version-bumper.html` in Chrome
2. Click "Ordner auswählen" and select the project root folder
3. Verify current version `0.2.1` is shown
4. Enter `0.3.0` and click "Preview"
5. Verify preview shows correct diffs for all 4 files
6. Click "Bump!"
7. Verify all 4 files on disk are updated to `0.3.0`
8. Check git commit and release checkboxes
9. Verify generated commands are correct and copy button works

- [x] **Step 2: Rollback test versions back to 0.2.1**

```bash
git checkout -- package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src/App.tsx
```

- [x] **Step 3: Commit final version**

```bash
git add version-bumper.html
git commit -m "feat: complete version-bumper tool with git/release support"
```

---

### Plan Self-Review

**Spec coverage:**
- ✅ 4 files version bump — Task 3 (read), Task 5 (write)
- ✅ Version validation (semver) — Task 4
- ✅ Preview with diffs — Task 4
- ✅ Git commit option — Task 5
- ✅ Build/release option — Task 5
- ✅ Fallback for non-Chromium — Task 6
- ✅ Standalone HTML file — all tasks

**Placeholder scan:** No TBD/TODO placeholders. All code is complete.

**Type consistency:** TARGET_FILES array is defined once in Task 3 and reused in Tasks 4-6. Version regex patterns are consistent with actual file formats verified during exploration.
