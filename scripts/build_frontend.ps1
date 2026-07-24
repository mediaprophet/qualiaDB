# build_frontend.ps1
# Script to build webizen-studio as WASM and stage it for the daemon

$ErrorActionPreference = "Stop"

$initialStatus = git -C "$PSScriptRoot/.." status --porcelain
if ($LASTEXITCODE -ne 0) {
    throw "Could not inspect the source tree before the frontend build."
}
$sourceTreeDirty = -not [string]::IsNullOrWhiteSpace(($initialStatus -join "`n"))

Write-Host "Ensuring dioxus-cli is installed..."
if (!(Get-Command "dx" -ErrorAction SilentlyContinue)) {
    Write-Host "Installing dioxus-cli..."
    cargo install dioxus-cli --version 0.8.0-alpha.0 --locked
}

Write-Host "Building webizen-studio..."
$publicAssets = "$PSScriptRoot/../target/dx/webizen-studio/release/web/public/assets"
if (Test-Path $publicAssets) {
    Get-ChildItem -LiteralPath $publicAssets -Filter "webizen-studio*" -File |
        Remove-Item -Force
}
Push-Location "$PSScriptRoot/../crates/webizen-studio"
dx build --web --release

if ($LASTEXITCODE -ne 0) {
    Write-Error "Dioxus build failed."
    Pop-Location
    exit 1
}
Pop-Location

$public = Resolve-Path "$PSScriptRoot/../target/dx/webizen-studio/release/web/public"
$dist = Resolve-Path "$PSScriptRoot/../crates/webizen-studio/dist"
$assets = Join-Path $dist "assets"
$browserSource = Resolve-Path "$PSScriptRoot/../crates/webizen-desktop/src/browser"

if (-not (Test-Path (Join-Path $public "index.html"))) {
    throw "Dioxus output is missing index.html: $public"
}

New-Item -ItemType Directory -Path $assets -Force | Out-Null
Get-ChildItem -LiteralPath $assets -Filter "webizen-studio*" -File |
    Remove-Item -Force
Copy-Item -LiteralPath (Join-Path $public "index.html") -Destination (Join-Path $dist "index.html") -Force
Copy-Item -Path (Join-Path $public "assets\*") -Destination $assets -Force
Copy-Item -LiteralPath (Join-Path $browserSource "chrome.html") -Destination (Join-Path $dist "browser-chrome.html") -Force
Copy-Item -LiteralPath (Join-Path $browserSource "universe.html") -Destination (Join-Path $dist "chora-universe.html") -Force
$sourceRevision = (git -C "$PSScriptRoot/.." rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($sourceRevision)) {
    throw "Could not determine the source revision for the frontend build."
}
if ($sourceTreeDirty) {
    $sourceRevision = "$sourceRevision-dirty"
}
Set-Content -LiteralPath (Join-Path $dist "source-revision.txt") -Value $sourceRevision -NoNewline

Write-Host "Build complete. Staged fresh desktop assets in crates/webizen-studio/dist."
