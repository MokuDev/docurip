# docurip — Release & Update Workflow

Schritt-für-Schritt-Anleitung für Releases und Updates.

## Repositories

| Repo | Sichtbar | Zweck |
|------|----------|-------|
| `MokuDev/docurip-src` | **privat** | Quellcode |
| `MokuDev/docurip` | **öffentlich** | Installer + `latest.json` für Auto-Updater |

Der Updater in der App greift nur auf `MokuDev/docurip` zu — der Source-Code bleibt privat.

---

## Voraussetzungen

| Tool | Zweck |
|------|-------|
| Node.js + npm | Frontend-Build |
| Rust + Cargo | Backend-Build |
| PowerShell 7 | Release-Script |
| GitHub CLI (`gh`) | Release-Publishing |
| Tauri CLI | `npx tauri` oder global installiert |

Signierungs-Schlüssel existiert unter:

```
~/.tauri/docurip.key     (leeres Passwort)
```

Das Release-Script lädt den Key automatisch — kein manuelles Setzen von Env-Variablen nötig.

---

## Release erstellen

### 1. Version bumpen (3 Dateien + App.tsx)

| Datei | Feld | Beispiel |
|-------|------|----------|
| `package.json` | `"version"` | `"0.3.0"` |
| `src-tauri/tauri.conf.json` | `"version"` | `"0.3.0"` |
| `src-tauri/Cargo.toml` | `version` | `"0.3.0"` |
| `src/App.tsx` | Footer `vX.Y.Z` | `v0.3.0` |

### 2. Committen

```powershell
git add -A
git commit -m "chore: bump version to 0.3.0"
```

### 3. Build + Publish

```powershell
npm run release:publish
```

Das Script macht automatisch:
1. Signing-Key aus `~/.tauri/docurip.key` laden (falls nicht als Env-Var gesetzt)
2. Version-Konsistenz-Check (3 Dateien)
3. `npm run tauri build` (produziert NSIS-Installer)
4. Signiert den Installer via `npx tauri signer sign` (erzeugt `.sig`-Datei)
5. Generiert `latest.json` aus der `.sig`-Datei
6. Erstellt GitHub Release in **`MokuDev/docurip`** mit Installer + `latest.json`

### 4. Release verifizieren

- [ ] Release sichtbar: `https://github.com/MokuDev/docurip/releases`
- [ ] `latest.json` als Asset vorhanden
- [ ] Installer-Datei (.exe) als Asset vorhanden
- [ ] Endpoint erreichbar: `https://github.com/MokuDev/docurip/releases/latest/download/latest.json`

Fertig. Nutzer bekommen beim nächsten App-Start den Update-Banner.

---

## Nur lokaler Build (ohne Publish)

```powershell
npm run release
```

Installer liegt dann unter:

```
src-tauri\target\release\bundle\nsis\docurip_X.Y.Z_x64-setup.exe
```

---

## Schnellreferenz

| Aktion | Command |
|--------|---------|
| Nur builden | `npm run release` |
| Builden + Publish | `npm run release:publish` |
| Dev-Server | `npm run tauri dev` |
| Rust prüfen | `cargo check` (in `src-tauri/`) |
| Tests | `cargo test` (in `src-tauri/`) |
| Frontend builden | `npm run build` |

---

## Troubleshooting

### "Version mismatch" beim Build

Alle drei Dateien müssen exakt gleiche Version haben. Prüfe `package.json`, `tauri.conf.json` und `Cargo.toml`.

### Keine `.sig` Datei nach Build

Das Script signiert automatisch nach dem Build. Falls es trotzdem fehlt:
- Prüfe ob `~/.tauri/docurip.key` existiert (348 Bytes)
- Manuell signieren: `npx tauri signer sign <pfad-zur-exe>`

### Updater zeigt kein Update an

- `latest.json` muss im Release-Repo als Asset liegen
- Endpoint in `tauri.conf.json` → `plugins.updater.endpoints` muss erreichbar sein (`MokuDev/docurip`)
- Version in `latest.json` muss höher sein als die installierte Version
- Public Key in `tauri.conf.json` muss zum Private Key (`~/.tauri/docurip.key`) passen

### `gh release create` schlägt fehl

- `gh auth status` prüfen
- Sicherstellen, dass `MokuDev/docurip` existiert und Push-Rechte bestehen

### NSIS-Installer wird nicht gefunden

Das Script sucht nach `*_x64-setup.exe` in `src-tauri/target/release/bundle/nsis/` (neueste zuerst). Wenn das Verzeichnis leer ist, hat der Build fehlgeschlagen — Build-Logs prüfen.
