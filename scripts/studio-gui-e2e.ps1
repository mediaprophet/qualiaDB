# Run Playwright GUI E2E against the settings portal (requires desktop app open on :8080).
param(
    [int]$Port = 8080,
    [string]$BindAddress = "127.0.0.1",
    [switch]$Headed
)

$ErrorActionPreference = "Stop"
$env:QUALIA_PORTAL_URL = "http://${BindAddress}:${Port}"
$e2eDir = Join-Path $PSScriptRoot "studio-gui-e2e"

if (-not (Test-Path $e2eDir)) {
    throw "Missing $e2eDir"
}

Push-Location $e2eDir
try {
    if (-not (Test-Path "node_modules")) {
        Write-Host "Installing Playwright dependencies..."
        npm install
        npx playwright install chromium
    }
    if ($Headed) {
        npm run test:headed
    } else {
        npm run test
    }
} finally {
    Pop-Location
}