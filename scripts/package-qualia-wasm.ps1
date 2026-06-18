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
    # +simd128 for SIMD kernels. The LLM inference call tree uses large zero-copy
    # stack buffers (prefill_chunk ~640 KB + MAX_STACK_GEMM scratch in the dispatch
    # kernels) sized for native's 8 MB stack; wasm's default 1 MB stack overflows
    # and traps "memory access out of bounds" mid-inference. Give wasm an 8 MB
    # stack to match, and keep memory growable to the wasm32 4 GB ceiling so the
    # multi-hundred-MB GGUF copy in initialize_webgpu_engine can still grow.
    $env:RUSTFLAGS = "-C target-feature=+simd128 -C link-arg=-zstack-size=8388608 -C link-arg=--max-memory=4294967296"
    wasm-pack build --target web --out-dir pkg-qualia --release -- --no-default-features --features portal,wasm-llm,wasm-logic,wasm-scientific
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
$ver = "0.0.18"
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