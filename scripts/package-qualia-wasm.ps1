# Build unified Qualia WASM portal from qualia-core-db and publish to docs/pkg/qualia.
param(
    [string]$CrateDir = "$PSScriptRoot\..\crates\qualia-core-db",
    [string]$DocsPkg = "$PSScriptRoot\..\docs\pkg\qualia",
    [string]$DesktopPortalPkg = ""
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

# Generate the canonical deployment names directly. The shared packager pins the
# Rust toolchain, locks dependencies, stages the complete generated set, and owns cleanup.
& (Join-Path $PSScriptRoot "package-qualia-wasm-full.ps1") `
    -CrateDir $CrateDir -OutputDir $DocsPkg -Features portal -OutputName qualia -FlagProfile Portal

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

$poetSync = Join-Path $PSScriptRoot "package-poet-wasm.ps1"
if (Test-Path $poetSync) {
    & $poetSync
}
