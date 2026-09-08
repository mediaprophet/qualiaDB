# Build unified Qualia WASM portal from qualia-core-db and publish to docs/pkg/qualia.
param(
    [string]$CrateDir = "$PSScriptRoot\..\crates\qualia-core-db",
    [string]$DocsPkg = "$PSScriptRoot\..\docs\pkg\qualia",
    [string]$DesktopPortalPkg = "$PSScriptRoot\..\crates\webizen-desktop\static\portal\pkg\qualia"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $CrateDir)) {
    Write-Error "qualia-core-db crate not found at $CrateDir"
}

if (-not (Get-Command wasm-pack -ErrorAction SilentlyContinue)) {
    Write-Error "wasm-pack not found. Install: cargo install wasm-pack"
}

$crateManifest = Join-Path $CrateDir "Cargo.toml"
$manifestText = Get-Content -LiteralPath $crateManifest -Raw
if ($manifestText -notmatch '(?ms)^\[package\]\s*.*?^version\s*=\s*"([^"]+)"') {
    throw "Could not read the qualia-core-db package version from $crateManifest"
}
$packageVersion = $Matches[1]

Push-Location $CrateDir
try {
    # Full WASM-safe portal (logic + science + WebGPU viewport). GitHub Pages / release-wasm
    # sanity cap is 16 MiB raw / 4 MiB gzip — not a slim viewport budget. +simd128 for the
    # SIMD kernels. The browser LLM ships in the wasm-full *playground* bundle (docs/playground)
    # — not the portal — which is where the 8 MB stack / 4 GB max-memory link-args belong.
    # Ontology MCP stays the tight 640 KiB / 200 KiB product.
    $env:RUSTFLAGS = "-C target-feature=+simd128"
    cmd.exe /d /s /c "wasm-pack build --target web --out-dir pkg-qualia --release -- --no-default-features --features portal 2>&1"
    $wasmPackExitCode = $LASTEXITCODE
    if ($wasmPackExitCode -ne 0) {
        throw "wasm-pack portal build failed with exit code $wasmPackExitCode"
    }
} finally {
    Pop-Location
}

$src = Join-Path $CrateDir "pkg-qualia"
if (-not (Test-Path $src)) {
    Write-Error "Build output missing: $src"
}

New-Item -ItemType Directory -Force -Path $DocsPkg | Out-Null
Copy-Item -Path (Join-Path $src "*") -Destination $DocsPkg -Recurse -Force

# Publish canonical qualia.* names from qualia_core_db build only (never legacy qualia_wasm_*).
$coreAliases = @{
    "qualia_core_db.js"           = "qualia.js"
    "qualia_core_db.d.ts"         = "qualia.d.ts"
    "qualia_core_db_bg.wasm"      = "qualia_bg.wasm"
    "qualia_core_db_bg.wasm.d.ts" = "qualia_bg.wasm.d.ts"
}
foreach ($kv in $coreAliases.GetEnumerator()) {
    $from = Join-Path $DocsPkg $kv.Key
    if (-not (Test-Path $from)) {
        Write-Error "Missing wasm-pack artifact: $($kv.Key)"
    }
    Copy-Item $from (Join-Path $DocsPkg $kv.Value) -Force
}

$qualiaJs = Join-Path $DocsPkg "qualia.js"
if (Test-Path $qualiaJs) {
    $js = Get-Content $qualiaJs -Raw
    $js = $js -replace 'qualia_core_db_bg\.wasm', 'qualia_bg.wasm'
    $js = $js -replace 'qualia_wasm_bg\.wasm', 'qualia_bg.wasm'
    Set-Content $qualiaJs $js -NoNewline
    Write-Host "Patched qualia.js wasm import -> qualia_bg.wasm"
}

# Publish friendly package.json for GitHub Pages / Jekyll
@{
    name = "qualia-portal"
    type = "module"
    version = $packageVersion
    main = "qualia.js"
    types = "qualia.d.ts"
    files = @("qualia.js", "qualia_bg.wasm", "qualia.d.ts", "qualia_bg.wasm.d.ts", "LICENSE")
} | ConvertTo-Json | Set-Content (Join-Path $DocsPkg "package.json") -Encoding UTF8

$wasmCheck = Join-Path $PSScriptRoot "..\docs\tests\wasm-size-check.mjs"
$portalWasm = Join-Path $DocsPkg "qualia_bg.wasm"
if (Get-Command node -ErrorAction SilentlyContinue) {
    & node $wasmCheck $portalWasm 16777216 4194304
    if ($LASTEXITCODE -ne 0) {
        throw "portal WASM exceeded the GitHub Pages 16 MiB / 4 MiB sanity cap"
    }
} else {
    Write-Host "node not found; skip wasm-size-check (CI still enforces the 16 MiB / 4 MiB cap)"
}

Write-Host "Qualia WASM portal v$packageVersion built from qualia-core-db -> $DocsPkg"

if ($DesktopPortalPkg -and (Test-Path $DocsPkg)) {
    New-Item -ItemType Directory -Force -Path $DesktopPortalPkg | Out-Null
    Copy-Item -Path (Join-Path $DocsPkg "*") -Destination $DesktopPortalPkg -Recurse -Force
    Write-Host "Synced portal WASM -> $DesktopPortalPkg"
}

$sync = Join-Path $PSScriptRoot "sync-portal-design-kit.ps1"
if (Test-Path $sync) {
    & $sync
}
