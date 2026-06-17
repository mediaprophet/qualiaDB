# Build unified Qualia WASM portal from qualia-core-db and publish to docs/pkg/qualia.
param(
    [string]$CrateDir = "$PSScriptRoot\..\crates\qualia-core-db",
    [string]$DocsPkg = "$PSScriptRoot\..\docs\pkg\qualia"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $CrateDir)) {
    Write-Error "qualia-core-db crate not found at $CrateDir"
}

if (-not (Get-Command wasm-pack -ErrorAction SilentlyContinue)) {
    Write-Error "wasm-pack not found. Install: cargo install wasm-pack"
}

Push-Location $CrateDir
try {
    $env:RUSTFLAGS = "-C target-feature=+simd128"
    wasm-pack build --target web --out-dir pkg-qualia --release --no-default-features -- --features portal
} finally {
    Pop-Location
}

$src = Join-Path $CrateDir "pkg-qualia"
if (-not (Test-Path $src)) {
    Write-Error "Build output missing: $src"
}

New-Item -ItemType Directory -Force -Path $DocsPkg | Out-Null
Copy-Item -Path (Join-Path $src "*") -Destination $DocsPkg -Recurse -Force

$map = @{
    "qualia_core_db.js"          = "qualia.js"
    "qualia_core_db_bg.wasm"     = "qualia_bg.wasm"
    "qualia_core_db.d.ts"          = "qualia.d.ts"
    "qualia_core_db_bg.wasm.d.ts"  = "qualia_bg.wasm.d.ts"
    "qualia_wasm.js"               = "qualia.js"
    "qualia_wasm_bg.wasm"          = "qualia_bg.wasm"
    "qualia_wasm.d.ts"             = "qualia.d.ts"
    "qualia_wasm_bg.wasm.d.ts"     = "qualia_bg.wasm.d.ts"
}
foreach ($kv in $map.GetEnumerator()) {
    $from = Join-Path $DocsPkg $kv.Key
    if (Test-Path $from) {
        Copy-Item $from (Join-Path $DocsPkg $kv.Value) -Force
    }
}

# Publish friendly package.json for GitHub Pages / Jekyll
$ver = "0.0.17"
@{
    name = "qualia-portal"
    type = "module"
    version = $ver
    main = "qualia.js"
    types = "qualia.d.ts"
    files = @("qualia.js", "qualia_bg.wasm", "qualia.d.ts", "qualia_bg.wasm.d.ts")
} | ConvertTo-Json | Set-Content (Join-Path $DocsPkg "package.json") -Encoding UTF8

Write-Host "Qualia WASM portal v$ver built from qualia-core-db -> $DocsPkg"

$sync = Join-Path $PSScriptRoot "sync-portal-design-kit.ps1"
if (Test-Path $sync) {
    & $sync
}