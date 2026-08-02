# Build the browser LLM/anatomy bundle consumed by docs/online-llm-demo.html.
param(
    [string]$CrateDir = "$PSScriptRoot\..\crates\qualia-core-db",
    [string]$OutputDir = "$PSScriptRoot\..\docs\playground"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path (Join-Path $CrateDir "Cargo.toml"))) {
    throw "qualia-core-db crate not found at $CrateDir"
}
if (-not (Get-Command wasm-pack -ErrorAction SilentlyContinue)) {
    throw "wasm-pack not found. Install it with: cargo install wasm-pack"
}

$previousRustFlags = $env:RUSTFLAGS
try {
    $env:RUSTFLAGS = "-C target-feature=+simd128 -C link-arg=-zstack-size=8388608 -C link-arg=--max-memory=4294967296"
    Push-Location $CrateDir
    try {
        wasm-pack build `
            --target web `
            --release `
            --out-dir $OutputDir `
            --out-name qualia_core_db `
            --no-typescript `
            -- `
            --no-default-features `
            --features wasm-full
        if ($LASTEXITCODE -ne 0) {
            throw "wasm-pack full browser build failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
} finally {
    $env:RUSTFLAGS = $previousRustFlags
}

$glue = Join-Path $OutputDir "qualia_core_db.js"
$wasm = Join-Path $OutputDir "qualia_core_db_bg.wasm"
if (-not (Test-Path $glue) -or -not (Test-Path $wasm)) {
    throw "Full browser artifacts were not produced in $OutputDir"
}

Write-Host "Qualia full WASM bundle built with SIMD128 -> $OutputDir"
