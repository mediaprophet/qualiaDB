# build_frontend.ps1
# Script to build webizen-studio as WASM and stage it for the daemon

$ErrorActionPreference = "Stop"

Write-Host "Ensuring dioxus-cli is installed..."
if (!(Get-Command "dx" -ErrorAction SilentlyContinue)) {
    Write-Host "Installing dioxus-cli..."
    cargo install dioxus-cli --version 0.7.9 --locked
}

Write-Host "Building webizen-studio..."
Push-Location "$PSScriptRoot/../crates/webizen-studio"
dx build --release

if ($LASTEXITCODE -ne 0) {
    Write-Error "Dioxus build failed."
    Pop-Location
    exit 1
}
Pop-Location

Write-Host "Build complete. The native client will serve assets from crates/webizen-studio/dist."
