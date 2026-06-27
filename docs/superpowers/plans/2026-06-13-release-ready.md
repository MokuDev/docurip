# docurip Release-Ready Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable NSIS installer bundling and auto-update via GitHub Releases so docurip can be distributed as a Windows desktop app.

**Architecture:** Configure Tauri v2's built-in NSIS bundler, add the updater plugin for GitHub Releases-based auto-update, create a local release script, and add a frontend update check hook.

**Tech Stack:** Tauri v2, NSIS (via Tauri), tauri-plugin-updater, React 19, TypeScript, PowerShell

> **Status (verifiziert gegen v0.3.3):** ✅ Alle Tasks technisch umgesetzt. NSIS-Bundle aktiv (`tauri.conf.json:38`), `tauri-plugin-updater = "2"` in Cargo (Zeile 17), `@tauri-apps/plugin-updater` in package.json, Plugin registriert in `lib.rs:25`, `updater:default` in capabilities, pubkey gesetzt, `useUpdater.ts` mit Cache (siehe FIX-PLAN Task 9) und `scripts/release.ps1` vorhanden.
>
> **Abweichung Task 5:** `authors` in `Cargo.toml` ist `["moku"]` statt `["Docurip Contributors"]` — bewusste Entscheidung des Maintainers.

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `src-tauri/tauri.conf.json` | Modify | Enable bundle, NSIS config, updater config |
| `src-tauri/Cargo.toml` | Modify | Add updater dependency, fix authors |
| `package.json` | Modify | Add updater npm package, add release script |
| `src-tauri/src/lib.rs:20-24` | Modify | Register updater plugin |
| `src-tauri/capabilities/default.json` | Modify | Add updater permission |
| `src/hooks/useUpdater.ts` | Create | Updater check hook |
| `src/App.tsx:1-4,20-21` | Modify | Integrate updater hook |
| `scripts/release.ps1` | Create | Local release build + optional GitHub publish |

---

### Task 1: Enable NSIS Bundling

**Files:**
- Modify: `src-tauri/tauri.conf.json`

- [x] **Step 1: Update bundle config**

Replace the `"bundle"` section in `src-tauri/tauri.conf.json`:

```json
"bundle": {
  "active": true,
  "targets": ["nsis"],
  "icon": [
    "icons/32x32.png",
    "icons/128x128.png",
    "icons/128x128@2x.png",
    "icons/icon.icns",
    "icons/icon.ico"
  ],
  "windows": {
    "nsis": {
      "installMode": "both",
      "displayLanguageSelector": false
    }
  }
}
```

- [x] **Step 2: Verify config is valid JSON**

Run: `node -e "JSON.parse(require('fs').readFileSync('src-tauri/tauri.conf.json','utf8'))"`
Expected: No output (no parse error)

- [x] **Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "feat: enable NSIS bundling for Windows installer"
```

---

### Task 2: Add Updater Plugin

**Files:**
- Modify: `src-tauri/Cargo.toml:11-34`
- Modify: `package.json:12-22`
- Modify: `src-tauri/src/lib.rs:20-24`
- Modify: `src-tauri/capabilities/default.json:5-15`
- Modify: `src-tauri/tauri.conf.json`

- [x] **Step 1: Add Rust dependency**

Add to `[dependencies]` in `src-tauri/Cargo.toml` (after `tauri-plugin-store`):

```toml
tauri-plugin-updater = "2"
```

- [x] **Step 2: Add npm dependency**

Run: `npm install @tauri-apps/plugin-updater`

Expected: `@tauri-apps/plugin-updater` added to `dependencies` in `package.json`

- [x] **Step 3: Register plugin in lib.rs**

In `src-tauri/src/lib.rs`, add after line 24 (`.plugin(tauri_plugin_store::Builder::default().build())`):

```rust
        .plugin(tauri_plugin_updater::Builder::new().build())
```

The builder chain becomes:

```rust
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
```

- [x] **Step 4: Add updater permission**

Replace `src-tauri/capabilities/default.json`:

```json
{
  "identifier": "default",
  "description": "Default capabilities for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "shell:allow-open",
    "fs:default",
    "dialog:default",
    "store:default",
    "fs:allow-read-file",
    "fs:allow-write-file",
    "fs:allow-read-dir",
    "fs:allow-mkdir",
    "updater:default"
  ]
}
```

- [x] **Step 5: Add updater config to tauri.conf.json**

Add a `"plugins"` key at the top level of `src-tauri/tauri.conf.json` (after `"bundle"`):

```json
"plugins": {
  "updater": {
    "endpoints": [
      "https://github.com/moku-org/docurip/releases/latest/download/latest.json"
    ],
    "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEUwNEZDMzJCQkQyQ0FGMDgKUldTY0JrMHdFZ1Z2Yi93YXJTbGM2ZmR4SXVrVkRUMm9Fa0pDUzdUa1dRTitqZUQ4cTJpSDNMUzAK"
  }
}
```

> **NOTE:** The `pubkey` placeholder must be replaced in Task 3 with a real generated key. The `endpoints` URL should be updated to match the actual GitHub repo owner/name.

- [x] **Step 6: Verify Rust compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles successfully (may show warnings, no errors)

- [x] **Step 7: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock package.json package-lock.json src-tauri/src/lib.rs src-tauri/capabilities/default.json src-tauri/tauri.conf.json
git commit -m "feat: add tauri-plugin-updater for auto-update via GitHub Releases"
```

---

### Task 3: Generate Updater Signing Keypair

**Files:**
- Modify: `src-tauri/tauri.conf.json` (pubkey field)

- [x] **Step 1: Generate keypair**

Run: `npx tauri signer generate -w ~/.tauri/docurip.key`

This outputs a public key to stdout. Copy it.

- [x] **Step 2: Update pubkey in tauri.conf.json**

Replace the placeholder `pubkey` value in `src-tauri/tauri.conf.json` → `plugins.updater.pubkey` with the generated public key from Step 1.

- [x] **Step 3: Verify config is valid**

Run: `node -e "JSON.parse(require('fs').readFileSync('src-tauri/tauri.conf.json','utf8'))"`
Expected: No output

- [x] **Step 4: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "chore: set real updater signing public key"
```

---

### Task 4: Frontend Update Check Hook

**Files:**
- Create: `src/hooks/useUpdater.ts`
- Modify: `src/App.tsx`

- [x] **Step 1: Create useUpdater hook**

Create `src/hooks/useUpdater.ts`:

```typescript
import { useEffect, useState } from 'react';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

interface UpdateInfo {
  version: string;
  body: string;
}

export function useUpdater() {
  const [updateAvailable, setUpdateAvailable] = useState<UpdateInfo | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function checkForUpdate() {
      try {
        const update = await check();
        if (update && !cancelled) {
          setUpdateAvailable({
            version: update.version,
            body: update.body ?? '',
          });
        }
      } catch (err) {
        if (!cancelled) {
          console.warn('Update check failed:', err);
          setError(String(err));
        }
      }
    }

    checkForUpdate();
    return () => { cancelled = true; };
  }, []);

  const installUpdate = async () => {
    setDownloading(true);
    try {
      const update = await check();
      if (update) {
        await update.downloadAndInstall();
        await relaunch();
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setDownloading(false);
    }
  };

  return { updateAvailable, downloading, error, installUpdate, dismiss: () => setUpdateAvailable(null) };
}
```

- [x] **Step 2: Add @tauri-apps/plugin-process dependency**

Run: `npm install @tauri-apps/plugin-process`

- [x] **Step 3: Integrate hook in App.tsx**

Add import at top of `src/App.tsx`:

```typescript
import { useUpdater } from './hooks/useUpdater';
```

Add hook call inside `App()` function, after the `useCrawlEvents` call (line 24):

```typescript
const { updateAvailable, downloading, installUpdate, dismiss } = useUpdater();
```

Add update banner inside the JSX, after `<TopStatusBar />` (after line 35):

```tsx
{updateAvailable && (
  <div className="bg-accentGreen/10 border-b border-accentGreen/20 px-4 py-2 flex items-center justify-between text-sm">
    <span className="text-ghost">
      Update available: <strong className="text-accentGreen">v{updateAvailable.version}</strong>
    </span>
    <div className="flex items-center space-x-2">
      <button
        onClick={installUpdate}
        disabled={downloading}
        className="px-3 py-1 bg-accentGreen hover:bg-brightGreen text-deepVoid font-semibold rounded text-xs transition-all disabled:opacity-50"
      >
        {downloading ? 'Downloading...' : 'Install & Restart'}
      </button>
      <button
        onClick={dismiss}
        className="px-2 py-1 text-charcoal hover:text-ghost text-xs transition-colors"
      >
        Dismiss
      </button>
    </div>
  </div>
)}
```

- [x] **Step 4: Verify TypeScript compiles**

Run: `npm run build`
Expected: No TypeScript errors

- [x] **Step 5: Commit**

```bash
git add src/hooks/useUpdater.ts src/App.tsx package.json package-lock.json
git commit -m "feat: add auto-update check on app startup"
```

---

### Task 5: Metadata Cleanup

**Files:**
- Modify: `src-tauri/Cargo.toml:5`

- [x] **Step 1: Fix authors field**

In `src-tauri/Cargo.toml`, change line 5:

```toml
authors = ["Docurip Contributors"]
```

- [x] **Step 2: Verify Rust compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles successfully

- [x] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "chore: update Cargo.toml authors field"
```

---

### Task 6: Release Script

**Files:**
- Create: `scripts/release.ps1`

- [x] **Step 1: Create release script**

Create `scripts/release.ps1`:

```powershell
<#
.SYNOPSIS
    Build and optionally publish a docurip release.
.DESCRIPTION
    Validates version consistency, runs tauri build, and optionally creates a GitHub Release.
.PARAMETER Publish
    If set, creates a GitHub Release with gh CLI and uploads the installer.
.EXAMPLE
    .\scripts\release.ps1
    .\scripts\release.ps1 -Publish
#>

param(
    [switch]$Publish
)

$ErrorActionPreference = "Stop"

function Get-VersionFromJson($path, $key) {
    $json = Get-Content $path -Raw | ConvertFrom-Json
    return $json.$key
}

function Get-VersionFromToml($path) {
    $content = Get-Content $path -Raw
    if ($content -match 'version\s*=\s*"([^"]+)"') {
        return $matches[1]
    }
    throw "Could not parse version from $path"
}

Write-Host "=== docurip release ===" -ForegroundColor Cyan

# Step 1: Validate version consistency
Write-Host "`n[1/4] Validating version consistency..." -ForegroundColor Yellow

$npmVersion = Get-VersionFromJson "package.json" "version"
$tauriVersion = Get-VersionFromJson "src-tauri/tauri.conf.json" "version"
$cargoVersion = Get-VersionFromToml "src-tauri/Cargo.toml"

Write-Host "  package.json:       $npmVersion"
Write-Host "  tauri.conf.json:    $tauriVersion"
Write-Host "  Cargo.toml:         $cargoVersion"

if ($npmVersion -ne $tauriVersion -or $npmVersion -ne $cargoVersion) {
    Write-Host "`nERROR: Version mismatch! All three files must have the same version." -ForegroundColor Red
    exit 1
}

$version = $npmVersion
Write-Host "  All versions match: $version" -ForegroundColor Green

# Step 2: Build
Write-Host "`n[2/4] Building installer..." -ForegroundColor Yellow

npm run tauri build
if ($LASTEXITCODE -ne 0) {
    Write-Host "`nERROR: Build failed!" -ForegroundColor Red
    exit 1
}

# Step 3: Find output
Write-Host "`n[3/4] Locating installer..." -ForegroundColor Yellow

$nsisDir = "src-tauri/target/release/bundle/nsis"
$setupExe = Get-ChildItem -Path $nsisDir -Filter "*.exe" | Select-Object -First 1

if (-not $setupExe) {
    Write-Host "`nERROR: No .exe found in $nsisDir" -ForegroundColor Red
    exit 1
}

Write-Host "  Installer: $($setupExe.FullName)" -ForegroundColor Green
Write-Host "  Size:      $([math]::Round($setupExe.Length / 1MB, 2)) MB"

# Step 4: Publish (optional)
if ($Publish) {
    Write-Host "`n[4/4] Creating GitHub Release v$version..." -ForegroundColor Yellow

    $tagName = "v$version"
    $releaseNotes = "Release $tagName"

    gh release create $tagName $setupExe.FullName --title "docurip $tagName" --notes $releaseNotes
    if ($LASTEXITCODE -ne 0) {
        Write-Host "`nERROR: GitHub Release creation failed!" -ForegroundColor Red
        exit 1
    }

    Write-Host "  Published: $tagName" -ForegroundColor Green
} else {
    Write-Host "`n[4/4] Skipping publish (use -Publish to create GitHub Release)" -ForegroundColor DarkGray
}

Write-Host "`n=== Done ===" -ForegroundColor Cyan
```

- [x] **Step 2: Verify script syntax**

Run: `pwsh -NoProfile -Command "& { Get-Content scripts/release.ps1 | Out-Null }"`
Expected: No errors

- [x] **Step 3: Add npm release script**

Add to `scripts` in `package.json`:

```json
"release": "pwsh -NoProfile -File scripts/release.ps1",
"release:publish": "pwsh -NoProfile -File scripts/release.ps1 -Publish"
```

- [x] **Step 4: Commit**

```bash
git add scripts/release.ps1 package.json
git commit -m "feat: add local release script with optional GitHub publish"
```

---

### Task 7: Full Build Verification

**Files:** None (verification only)

- [x] **Step 1: Verify Rust compiles**

Run: `cd src-tauri && cargo check`
Expected: No errors

- [x] **Step 2: Verify TypeScript compiles**

Run: `npm run build`
Expected: No errors

- [x] **Step 3: Run existing tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass

- [x] **Step 4: Full tauri build (produces installer)**

Run: `npm run tauri build`
Expected: Build succeeds, installer at `src-tauri/target/release/bundle/nsis/docurip_0.2.0_x64-setup.exe`

- [x] **Step 5: Verify installer exists and has reasonable size**

Run: `Get-ChildItem src-tauri/target/release/bundle/nsis/*.exe | Select-Object Name, @{N='SizeMB';E={[math]::Round($_.Length/1MB,2)}}`
Expected: File exists, size 2-10 MB

- [x] **Step 6: Commit any remaining changes**

```bash
git add -A
git commit -m "chore: release-ready v0.2.0"
```
