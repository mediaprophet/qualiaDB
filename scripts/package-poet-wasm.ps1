param(
    [string]$CrateDir = "",
    [string]$DocsPkg = "$PSScriptRoot\..\docs\poet-app",
    [string]$DesktopPortalPkg = "",
    [switch]$Build
)

$ErrorActionPreference = "Stop"

if (-not $CrateDir) {
    if (Test-Path "$PSScriptRoot\..\..\poet\wasm") {
        $CrateDir = "$PSScriptRoot\..\..\poet\wasm"
    } elseif (Test-Path "$PSScriptRoot\..\..\poet") {
        $CrateDir = "$PSScriptRoot\..\..\poet"
    } elseif (Test-Path "$PSScriptRoot\..\crates\poet") {
        $CrateDir = "$PSScriptRoot\..\crates\poet"
    }
}

if (-not (Test-Path $CrateDir)) {
    if (Test-Path (Join-Path $DocsPkg "index.html")) {
        Write-Host "Poet crate source not found; verified bundle already present in $DocsPkg"
        if ($DesktopPortalPkg) {
            New-Item -ItemType Directory -Force -Path $DesktopPortalPkg | Out-Null
            Copy-Item -Path (Join-Path $DocsPkg "*") -Destination $DesktopPortalPkg -Recurse -Force
            Write-Host "Synced Poet bundle -> $DesktopPortalPkg"
        }
        exit 0
    }
    Write-Host "Poet source and DocsPkg not found; skip poet wasm packaging"
    exit 0
}

$crateManifest = Join-Path $CrateDir "Cargo.toml"
$manifestText = Get-Content -LiteralPath $crateManifest -Raw
if ($manifestText -notmatch '(?ms)^\[package\]\s*.*?^version\s*=\s*"([^"]+)"') {
    throw "Could not read the poet package version from $crateManifest"
}
$packageVersion = $Matches[1]

Write-Host "Packaging Poet WASM v$packageVersion..."

New-Item -ItemType Directory -Force -Path $DocsPkg | Out-Null
if ($DesktopPortalPkg) {
    New-Item -ItemType Directory -Force -Path $DesktopPortalPkg | Out-Null
}

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
if ($DesktopPortalPkg) {
    if (Test-Path (Join-Path $DocsPkg "index.html")) {
        Copy-Item -Path (Join-Path $DocsPkg "*") -Destination $DesktopPortalPkg -Recurse -Force
        Write-Host "Successfully synced Poet WASM bundle to $DesktopPortalPkg"
    } else {
        Write-Error "Poet WASM index.html missing from $DocsPkg"
    }
}

Write-Host "Poet WASM distribution ready."
