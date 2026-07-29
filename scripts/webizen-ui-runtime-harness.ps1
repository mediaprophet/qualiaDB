# Webizen UI runtime harness checklist (U6-C)
# Compile-verified gates + printed manual steps for Local stream dogfood.
# Usage:
#   pwsh -File scripts/webizen-ui-runtime-harness.ps1
#   pwsh -File scripts/webizen-ui-runtime-harness.ps1 -SkipTests
#   pwsh -File scripts/webizen-ui-runtime-harness.ps1 -SkipCheck

param(
    [switch]$SkipCheck,
    [switch]$SkipTests
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path (Join-Path $Root "Cargo.toml"))) {
    $Root = $PSScriptRoot
    if (-not (Test-Path (Join-Path $Root "Cargo.toml"))) {
        Write-Error "Run from repo root or scripts/; Cargo.toml not found."
    }
}
Set-Location $Root
Write-Host "=== Webizen UI Runtime Harness (U6-C) ===" -ForegroundColor Cyan
Write-Host "Root: $Root"
Write-Host "Date: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
Write-Host ""

$results = @()

function Add-Result([string]$Name, [string]$Status, [string]$Detail = "") {
    $script:results += [pscustomobject]@{ Name = $Name; Status = $Status; Detail = $Detail }
    $color = switch ($Status) {
        "PASS" { "Green" }
        "FAIL" { "Red" }
        "BLOCKED" { "Yellow" }
        default { "Gray" }
    }
    Write-Host ("[{0}] {1} {2}" -f $Status, $Name, $Detail) -ForegroundColor $color
}

if (-not $SkipCheck) {
    Write-Host "`n-- cargo check -p webizen-desktop -p webizen-studio --" -ForegroundColor DarkCyan
    cargo check -p webizen-desktop -p webizen-studio
    if ($LASTEXITCODE -eq 0) {
        Add-Result "cargo check desktop+studio" "PASS"
    } else {
        Add-Result "cargo check desktop+studio" "FAIL" "exit $LASTEXITCODE"
    }
} else {
    Add-Result "cargo check desktop+studio" "SKIP"
}

if (-not $SkipTests) {
    Write-Host "`n-- unit: command_palette + virtualized_list + conduct_banner --" -ForegroundColor DarkCyan
    cargo test -p webizen-studio --bin webizen-studio command_palette virtualized_list conduct_banner -- --nocapture
    if ($LASTEXITCODE -eq 0) {
        Add-Result "studio unit (palette/virt/conduct)" "PASS"
    } else {
        Add-Result "studio unit (palette/virt/conduct)" "FAIL" "exit $LASTEXITCODE"
    }
} else {
    Add-Result "studio unit (palette/virt/conduct)" "SKIP"
}

# Docs present
$docs = @(
    "docs/manuals/webizen-ui-event-catalogue.md",
    "docs/manuals/webizen-ui-architecture.md",
    "docs/manuals/talk-and-agents-ui.md",
    "scripts/webizen-ui-runtime-harness.md",
    "crates/webizen-studio/src/components/command_palette.rs",
    "crates/webizen-desktop/src/shell/shell_html.rs"
)
foreach ($d in $docs) {
    if (Test-Path (Join-Path $Root $d)) {
        Add-Result "artifact $d" "PASS"
    } else {
        Add-Result "artifact $d" "FAIL" "missing"
    }
}

Write-Host "`n=== Manual steps (not auto-run) ===" -ForegroundColor Cyan
Write-Host @"
1. Launch desktop (cargo run -p webizen-desktop or packaged exe).
2. Ctrl+K → open palette → visit Talk, Browser, 10D, Settings, Library.
3. Settings → backend Local (not Ollama as default excellence path).
4. Activate GGUF if available → Talk prompt → assert chat-token stream + chat-done.
5. Without model: mark stream steps BLOCKED; do not claim e2e success.
6. Paste result template into docs/plans/webizen-ui-PROGRESS-LOG.md

Full notes: scripts/webizen-ui-runtime-harness.md
"@

Write-Host "`n=== Summary ===" -ForegroundColor Cyan
$results | Format-Table -AutoSize
$fail = ($results | Where-Object { $_.Status -eq "FAIL" }).Count
if ($fail -gt 0) {
    Write-Host "Harness compile/doc gate: FAIL ($fail)" -ForegroundColor Red
    exit 1
}
Write-Host "Harness compile/doc gate: PASS (runtime stream still needs model + manual dogfood)" -ForegroundColor Green
exit 0
