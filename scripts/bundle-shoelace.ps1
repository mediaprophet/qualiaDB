# Vendors Shoelace 2.15 cdn/ into webizen-studio dist for offline Tauri desktop.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Studio = Join-Path $Root "crates\webizen-studio"
$Tmp = Join-Path $Studio ".vendor-tmp"
$Dest = Join-Path $Studio "dist\vendor\shoelace"

New-Item -ItemType Directory -Force -Path $Tmp | Out-Null
Push-Location $Tmp
try {
    if (-not (Get-ChildItem "*.tgz" -ErrorAction SilentlyContinue)) {
        npm.cmd pack @shoelace-style/shoelace@2.15.0 | Out-Null
    }
    $tgz = Get-ChildItem "*.tgz" | Select-Object -First 1
    if (-not $tgz) { throw "npm pack did not produce a tarball" }
    if (Test-Path "package") { Remove-Item -Recurse -Force "package" }
    tar -xzf $tgz.FullName
    if (-not (Test-Path "package\cdn")) { throw "shoelace package missing cdn/" }
    if (Test-Path $Dest) { Remove-Item -Recurse -Force $Dest }
    New-Item -ItemType Directory -Force -Path (Split-Path $Dest) | Out-Null
    Copy-Item -Recurse "package\cdn" $Dest
    Write-Host "Shoelace vendored to $Dest"
}
finally {
    Pop-Location
}