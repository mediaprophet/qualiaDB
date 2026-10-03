# Build standalone Poet WASM application and publish to docs/poet-app and static/portal/poet-app.
param(
    [string]$CrateDir = "$PSScriptRoot\..\crates\poet",
    [string]$DocsPkg = "$PSScriptRoot\..\docs\poet-app",
    [string]$DesktopPortalPkg = "$PSScriptRoot\..\crates\webizen-desktop\static\portal\poet-app",
    [switch]$Build
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $CrateDir)) {
    Write-Error "poet crate not found at $CrateDir"
}

$crateManifest = Join-Path $CrateDir "Cargo.toml"
$manifestText = Get-Content -LiteralPath $crateManifest -Raw
if ($manifestText -notmatch '(?ms)^\[package\]\s*.*?^version\s*=\s*"([^"]+)"') {
    throw "Could not read the poet package version from $crateManifest"
}
$packageVersion = $Matches[1]

Write-Host "Packaging Poet WASM v$packageVersion..."

New-Item -ItemType Directory -Force -Path $DocsPkg | Out-Null
New-Item -ItemType Directory -Force -Path $DesktopPortalPkg | Out-Null

if ($Build) {
    if (Get-Command trunk -ErrorAction SilentlyContinue) {
        Push-Location $CrateDir
        try {
            Write-Host "Building Poet via trunk..."
            cmd.exe /c "trunk build --release"
            $dist = Join-Path $CrateDir "dist"
            if (Test-Path $dist) {
                Copy-Item -Path (Join-Path $dist "*") -Destination $DocsPkg -Recurse -Force
            }
        } finally {
            Pop-Location
        }
    } elseif (Get-Command wasm-pack -ErrorAction SilentlyContinue) {
        Push-Location $CrateDir
        try {
            Write-Host "Building Poet via wasm-pack..."
            cmd.exe /c "wasm-pack build --target web --out-dir pkg-poet --release"
            $pkg = Join-Path $CrateDir "pkg-poet"
            if (Test-Path $pkg) {
                Copy-Item -Path (Join-Path $pkg "*") -Destination $DocsPkg -Recurse -Force
            }
        } catch {
            Write-Warning "wasm-pack compile failed; preserving existing verified bundle in ${DocsPkg}: $_"
        } finally {
            Pop-Location
        }
    }
} else {
    Write-Host "Syncing verified Poet WASM bundle from $DocsPkg"
}

# Sync verified assets to desktop portal
if (Test-Path (Join-Path $DocsPkg "index.html")) {
    Copy-Item -Path (Join-Path $DocsPkg "*") -Destination $DesktopPortalPkg -Recurse -Force
    Write-Host "Successfully synced Poet WASM bundle to $DesktopPortalPkg"
} else {
    Write-Error "Poet WASM index.html missing from $DocsPkg"
}

Write-Host "Poet WASM distribution ready."
