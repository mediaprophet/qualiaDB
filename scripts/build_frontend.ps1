# build_frontend.ps1
# Script to build webizen-studio as WASM and stage it for the daemon

$ErrorActionPreference = "Stop"

Write-Host "Ensuring dioxus-cli is installed..."
if (!(Get-Command "dx" -ErrorAction SilentlyContinue)) {
    Write-Host "Installing dioxus-cli..."
    cargo install dioxus-cli --version 0.7.9 --locked
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

if (-not (Test-Path (Join-Path $public "index.html"))) {
    throw "Dioxus output is missing index.html: $public"
}

New-Item -ItemType Directory -Path $assets -Force | Out-Null
Get-ChildItem -LiteralPath $assets -Filter "webizen-studio*" -File |
    Remove-Item -Force
Copy-Item -LiteralPath (Join-Path $public "index.html") -Destination (Join-Path $dist "index.html") -Force
Copy-Item -Path (Join-Path $public "assets\*") -Destination $assets -Force

Write-Host "Build complete. Staged fresh desktop assets in crates/webizen-studio/dist."
