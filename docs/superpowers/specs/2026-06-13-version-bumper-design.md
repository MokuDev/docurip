# Version-Bumper Tool — Design Spec

**Datum:** 2026-06-13  
**Status:** Entwurf

## Überblick

Eigenständiges Tool zum gleichzeitigen Bumpen der Version in allen 4 Dateien des docurip-Projekts. Implementiert als einzelne HTML-Datei (`version-bumper.html`) mit eingebettetem Vanilla JS/CSS. Keine Build-Schritte, kein Framework, einfach im Browser öffnen.

## Ziel

- Ein Klick öffnet das Tool im Browser
- Nutzer wählt Projektordner aus
- Tool zeigt aktuelle Version aus allen 4 Dateien
- Nutzer gibt neue Version ein
- Tool validiert Format (semver: `MAJOR.MINOR.PATCH`)
- Preview zeigt Änderungen pro Datei
- Bei Bestätigung werden alle 4 Dateien im Originalordner überschrieben

## Dateien die geändert werden

| Datei | Pfad im Projekt | Feld |
|-------|-----------------|------|
| `package.json` | `/package.json` | `version` |
| `tauri.conf.json` | `/src-tauri/tauri.conf.json` | `version` |
| `Cargo.toml` | `/src-tauri/Cargo.toml` | `version` |
| `App.tsx` | `/src/App.tsx` | Footer `vX.Y.Z` |

## Technologie

- **Vanilla HTML/CSS/JS** — keine Dependencies, keine Build-Schritte
- **File System Access API** (`showDirectoryPicker`, `FileSystemDirectoryHandle`) zum Lesen/Schreiben im Projektordner
- **Fallback** für Firefox/Safari: `<input type="file" multiple>` + Download-Buttons für jede Datei

## UI-Layout

```
┌─────────────────────────────────────────┐
│  🚀 docurip Version Bumper              │
├─────────────────────────────────────────┤
│                                         │
│  [📁 Projektordner wählen]              │
│                                         │
│  Aktuelle Version: 0.2.1               │
│                                         │
│  Neue Version: [0.3.0        ]          │
│                                         │
│  [🔍 Preview]  [✅ Bump!]               │
│                                         │
├─────────────────────────────────────────┤
│  Preview-Bereich:                       │
│  ┌─ package.json ──────────────────┐   │
│  │ - "version": "0.2.1"           │   │
│  │ + "version": "0.3.0"           │   │
│  └─────────────────────────────────┘   │
│  ┌─ tauri.conf.json ───────────────┐   │
│  │ - "version": "0.2.1"           │   │
│  │ + "version": "0.3.0"           │   │
│  └─────────────────────────────────┘   │
│  ... (alle 4 Dateien)                   │
└─────────────────────────────────────────┘
```

## Validierung

- Semver-Format prüfen: `^\d+\.\d+\.\d+$`
- Prüfen dass alle 4 Dateien vor dem Bump die gleiche Version haben
- Warnung wenn Versionen nicht übereinstimmen

## Fehlerbehandlung

- Datei nicht gefunden → Fehlermeldung mit Dateinamen
- Keine Schreibrechte → Fehlermeldung
- API nicht verfügbar → Fallback-UI aktivieren (Upload/Download)

## Abhängigkeiten

- Keine — standalone HTML-Datei

## Im Scope (erweitert)

1. **Version bumpen** — Alle 4 Dateien aktualisieren
2. **Git-Commit** — Optional: `git add` + `git commit` mit Message `chore: bump version to X.Y.Z`
3. **Build/Release** — Optional: `npm run release` (lokal) oder `npm run release:publish` (mit GitHub-Publish)

## Nicht im Scope

- Git-Commit automatisch erstellen (Nutzer macht das selbst)
- Build/Release ausführen
- Changelog aktualisieren
