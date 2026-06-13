<#
.SYNOPSIS
    Build and optionally publish a docurip release.
.DESCRIPTION
    Validates version consistency, runs tauri build, generates latest.json, and optionally creates a GitHub Release in MokuDev/docurip.
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

# Step 0: Ensure signing key is available
$keyPath = Join-Path $env:USERPROFILE ".tauri\docurip.key"
if (Test-Path $keyPath) {
    if (-not $env:TAURI_SIGNING_PRIVATE_KEY) {
        $env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $keyPath -Raw
        Write-Host "  Loaded signing key from $keyPath" -ForegroundColor Green
    } else {
        Write-Host "  Signing key already set via env var" -ForegroundColor Green
    }
    if (-not $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) {
        $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
    }
} else {
    Write-Host "`nWARNING: Signing key not found at $keyPath" -ForegroundColor Yellow
    Write-Host "  Build will proceed but .sig file will NOT be generated." -ForegroundColor Yellow
    Write-Host "  Auto-updater will not work without signed releases." -ForegroundColor Yellow
}

# Step 1: Validate version consistency
Write-Host "`n[1/5] Validating version consistency..." -ForegroundColor Yellow

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
Write-Host "`n[2/5] Building installer..." -ForegroundColor Yellow

npm run tauri build
if ($LASTEXITCODE -ne 0) {
    Write-Host "`nERROR: Build failed!" -ForegroundColor Red
    exit 1
}

# Step 2b: Sign the installer (env var may not survive npm subprocess chain)
Write-Host "  Signing installer..." -ForegroundColor Yellow
$nsisDir = "src-tauri/target/release/bundle/nsis"
$newExe = Get-ChildItem -Path $nsisDir -Filter "*_x64-setup.exe" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if ($newExe -and $env:TAURI_SIGNING_PRIVATE_KEY) {
    $sigPath = "$($newExe.FullName).sig"
    if (-not (Test-Path $sigPath)) {
        npx tauri signer sign $newExe.FullName
        if ($LASTEXITCODE -ne 0) {
            Write-Host "`nERROR: Signing failed!" -ForegroundColor Red
            exit 1
        }
        Write-Host "  Signed: $($newExe.Name).sig" -ForegroundColor Green
    } else {
        Write-Host "  Signature already exists" -ForegroundColor Green
    }
}

# Step 3: Find output
Write-Host "`n[3/5] Locating installer..." -ForegroundColor Yellow

$nsisDir = "src-tauri/target/release/bundle/nsis"
$setupExe = Get-ChildItem -Path $nsisDir -Filter "*_x64-setup.exe" | Sort-Object LastWriteTime -Descending | Select-Object -First 1

if (-not $setupExe) {
    Write-Host "`nERROR: No .exe found in $nsisDir" -ForegroundColor Red
    exit 1
}

Write-Host "  Installer: $($setupExe.FullName)" -ForegroundColor Green
Write-Host "  Size:      $([math]::Round($setupExe.Length / 1MB, 2)) MB"

if ($setupExe.Name -notmatch $version) {
    Write-Host "`nWARNING: Installer filename does not contain version $version" -ForegroundColor Yellow
    Write-Host "  Expected: docurip_${version}_x64-setup.exe" -ForegroundColor Yellow
    Write-Host "  Found:    $($setupExe.Name)" -ForegroundColor Yellow
}

# Step 4: Publish (optional)
if ($Publish) {
    $releaseRepo = "MokuDev/docurip"
    $tagName = "v$version"

    Write-Host "`n[4/5] Generating latest.json for auto-updater..." -ForegroundColor Yellow

    $sigFile = "$($setupExe.FullName).sig"
    if (-not (Test-Path $sigFile)) {
        Write-Host "`nERROR: Signature file not found: $sigFile" -ForegroundColor Red
        Write-Host "  Set TAURI_SIGNING_PRIVATE_KEY before building to enable signing." -ForegroundColor Red
        exit 1
    }

    $signature = (Get-Content $sigFile -Raw).Trim()
    $exeName = $setupExe.Name
    $downloadUrl = "https://github.com/$releaseRepo/releases/download/$tagName/$exeName"
    $pubDate = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")

    $latestJson = @{
        version  = $version
        notes    = "docurip $tagName"
        pub_date = $pubDate
        platforms = @{
            "windows-x86_64" = @{
                signature = $signature
                url       = $downloadUrl
            }
        }
    } | ConvertTo-Json -Depth 4

    $latestJsonPath = Join-Path $nsisDir "latest.json"
    Set-Content -Path $latestJsonPath -Value $latestJson -Encoding UTF8
    Write-Host "  latest.json created" -ForegroundColor Green

    Write-Host "`n[5/5] Creating GitHub Release v$version in $releaseRepo..." -ForegroundColor Yellow

    gh release create $tagName $setupExe.FullName $latestJsonPath --repo $releaseRepo --title "docurip $tagName" --notes "Release $tagName"
    if ($LASTEXITCODE -ne 0) {
        Write-Host "`nERROR: GitHub Release creation failed!" -ForegroundColor Red
        exit 1
    }

    Write-Host "  Published: $tagName to $releaseRepo" -ForegroundColor Green
} else {
    Write-Host "`n[4/5] Skipping publish (use -Publish to create GitHub Release)" -ForegroundColor DarkGray
    Write-Host "  NOTE: Set TAURI_SIGNING_PRIVATE_KEY env var before using -Publish" -ForegroundColor DarkGray
}

Write-Host "`n=== Done ===" -ForegroundColor Cyan
