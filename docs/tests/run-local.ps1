# Run QualiaDB browser test suites locally (headless + optional HTTP server).
# Usage: .\docs\tests\run-local.ps1 [-Serve] [-Port 8765] [-SkipWasm]

param(
    [switch]$Serve,
    [int]$Port = 8765,
    [switch]$SkipWasm
)

$ErrorActionPreference = "Stop"
$TestsDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Root = (Resolve-Path (Join-Path $TestsDir "..\..")).Path
Set-Location $Root

Write-Host "=== QualiaDB test runner (0.0.8-dev) ===" -ForegroundColor Cyan
Write-Host ""

Write-Host "[1/2] Logic suites (199 tests expected)..." -ForegroundColor Yellow
node docs/tests/run-headless.mjs --mode logic
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

if (-not $SkipWasm) {
    Write-Host ""
    Write-Host "[2/2] WASM suites (352 tests expected)..." -ForegroundColor Yellow
    node docs/tests/run-headless.mjs --mode wasm
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

Write-Host ""
Write-Host "Headless checks passed." -ForegroundColor Green

if ($Serve) {
    Write-Host ""
    Write-Host "Serving docs/ at http://localhost:$Port/tests/index.html" -ForegroundColor Cyan
    Write-Host "Press Ctrl+C to stop." -ForegroundColor DarkGray
    Set-Location (Join-Path $Root "docs")
    python -m http.server $Port
}
