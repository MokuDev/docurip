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
