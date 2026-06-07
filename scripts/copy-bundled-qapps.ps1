# Copy bundled qapps (Anatomy + WASM) into a desktop dist folder.
# Usage: .\scripts\copy-bundled-qapps.ps1 -OutDir dist\qualia-flutter-windows-x64

param(
    [Parameter(Mandatory = $true)]
    [string]$OutDir
)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$AnatomySrc = Join-Path $Root "bundled\qapps\Anatomy"
if (-not (Test-Path $AnatomySrc)) {
    $AnatomySrc = Join-Path $Root "app-development\Anatomy"
}
$WasmSrc = Join-Path $Root "docs\playground"
$Dest = Join-Path $OutDir "bundled\qapps\Anatomy"

if (-not (Test-Path $AnatomySrc)) {
    Write-Warning "Anatomy source not found at $AnatomySrc — skipping bundled qapp copy."
    return
}

Write-Host "Copying Anatomy qapp to $Dest ..."
New-Item -ItemType Directory -Force -Path $Dest | Out-Null
robocopy $AnatomySrc $Dest /E /NFL /NDL /NJH /NJS /nc /ns /np | Out-Null
if ($LASTEXITCODE -ge 8) { throw "robocopy Anatomy failed with exit code $LASTEXITCODE" }
$global:LASTEXITCODE = 0

$WasmDest = Join-Path $Dest "wasm"
New-Item -ItemType Directory -Force -Path $WasmDest | Out-Null
foreach ($file in @("qualia_core_db.js", "qualia_core_db_bg.wasm")) {
    $src = Join-Path $WasmSrc $file
    if (Test-Path $src) {
        Copy-Item $src (Join-Path $WasmDest $file) -Force
        Write-Host "  bundled wasm: $file"
    } else {
        Write-Warning "  WASM artifact missing: $src (run wasm-pack build first)"
    }
}

Write-Host "Bundled qapps staged under $Dest"
$global:LASTEXITCODE = 0
