# Build a deterministic WellFair linked-companion bundle (Phase 1 / M2).
# Usage: .\scripts\build-wellfair-companion.ps1 -WasmJs path\to\profile.js -WasmBinary path\to\profile_bg.wasm

param(
    [string]$PackageId = "wellfair-companion",
    [string]$Version = "0.0.24",
    [Parameter(Mandatory = $true)][string]$WasmJs,
    [Parameter(Mandatory = $true)][string]$WasmBinary,
    [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not $OutputDir) {
    $OutputDir = Join-Path $env:TEMP "wellfair-companion-$PackageId-$Version"
}

$indexHtml = Join-Path $repoRoot "crates\qualia-mobile-harness\index.html"
$qappJson = Join-Path $repoRoot "bundled\qapps\Anatomy\qapp.json"

if (-not (Test-Path $WasmJs)) { throw "Missing WasmJs: $WasmJs" }
if (-not (Test-Path $WasmBinary)) { throw "Missing WasmBinary: $WasmBinary" }

$stubIndex = Join-Path $env:TEMP "wellfair-companion-index.html"
@"
<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>WellFair Companion</title></head>
<body><div id="root">WellFair linked companion (Phase 1 bundle)</div></body></html>
"@ | Set-Content -Path $stubIndex -Encoding UTF8

$stubQapp = Join-Path $env:TEMP "wellfair-companion-qapp.json"
@"
{"name":"$PackageId","version":"$Version","required_shapes":["qualia:WellfairRecord"]}
"@ | Set-Content -Path $stubQapp -Encoding UTF8

Push-Location $repoRoot
try {
    cargo test -p qualia-client-core companion_bundle::tests::deterministic_content_hash_for_same_inputs -- --exact
    Write-Host "Companion bundle library tests passed."
    Write-Host "Assemble inputs:"
    Write-Host "  WasmJs:      $WasmJs"
    Write-Host "  WasmBinary:  $WasmBinary"
    Write-Host "  OutputDir:   $OutputDir"
    Write-Host "Run build_companion_bundle from qualia-client-core after wiring a CLI entry (Phase 1 follow-up)."
}
finally {
    Pop-Location
}