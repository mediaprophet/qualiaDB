# Offline desktop bundles: Shoelace (studio) + Qualia WASM portal (settings design-studio).
param(
    [switch]$SkipWasmBuild
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot

& (Join-Path $Root "scripts\bundle-shoelace.ps1")

$PortalPkg = Join-Path $Root "crates\webizen-desktop\static\portal\pkg\qualia"
$WasmMarker = Join-Path $PortalPkg "qualia_bg.wasm"

if ($SkipWasmBuild -and (Test-Path $WasmMarker)) {
    Write-Host "Portal WASM present at $PortalPkg - skipping wasm-pack build (-SkipWasmBuild)."
    exit 0
}

$PackageScript = Join-Path $Root "scripts\package-qualia-wasm.ps1"
$DocsPkg = Join-Path $Root "docs\pkg\qualia"

if (Test-Path $PackageScript) {
    & $PackageScript -DocsPkg $DocsPkg -DesktopPortalPkg $PortalPkg
} else {
    Write-Warning "package-qualia-wasm.ps1 not found; portal Live mode may be unavailable offline."
}