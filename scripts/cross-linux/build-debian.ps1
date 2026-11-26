# Webizen / QualiaDB Debian native cross-build helper.
# PLAN-WEBIZEN-DEBIAN-NATIVE-PROFILES-2026-11-26 WP-3
#
# Copyright (c) 2026 Timothy Charles Holborn
# License: Attribution-NonCommercial-NoDerivatives 4.0 International (CC BY-NC-ND 4.0)
#
# Extends scripts/cross-linux/build-linux.ps1 with -Profile and -Target.
# Always uses an isolated --target-dir (never C:\github\synth\target).

[CmdletBinding()]
param(
    [ValidateSet('host', 'desktop', 'subsystem', 'mobile', 'iot-headless', 'iot-display')]
    [string]$Profile = 'host',

    [ValidateSet('x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu', 'armv7-unknown-linux-gnueabihf')]
    [string]$Target = 'x86_64-unknown-linux-gnu',

    [string]$Crate = 'qualia-cli',

    [string]$TargetDir = ''
)

$ErrorActionPreference = 'Stop'

$crossDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$env:PATH = "$crossDir;$env:PATH"

$repoRoot = (Resolve-Path (Join-Path $crossDir '..\..')).Path
if ([string]::IsNullOrWhiteSpace($TargetDir)) {
    $TargetDir = Join-Path $repoRoot 'target-debian-profiles'
}

$forbiddenTargets = @(
    (Join-Path 'C:\github\synth' 'target'),
    (Join-Path 'C:\github\synth' 'target-studio')
)
foreach ($forbidden in $forbiddenTargets) {
    $resolved = $TargetDir
    try { $resolved = (Resolve-Path -LiteralPath $TargetDir -ErrorAction SilentlyContinue).Path } catch { }
    if ($resolved -and ($resolved -eq $forbidden -or $resolved.StartsWith($forbidden + [IO.Path]::DirectorySeparatorChar))) {
        throw "Refusing to build into live Synth target directory: $resolved"
    }
}

$cargoProfile = switch ($Profile) {
    'host' { 'webizen-host' }
    'desktop' { 'webizen-desktop' }
    'subsystem' { 'webizen-subsystem' }
    'mobile' { 'webizen-mobile' }
    'iot-headless' { 'webizen-iot-headless' }
    'iot-display' { 'webizen-iot-display' }
}

switch ($Target) {
    'x86_64-unknown-linux-gnu' {
        if (Test-Path (Join-Path $crossDir 'x86_64-linux-gnu-gcc.exe')) {
            $env:CC_x86_64_unknown_linux_gnu = 'x86_64-linux-gnu-gcc'
            $env:AR_x86_64_unknown_linux_gnu = 'x86_64-linux-gnu-ar'
            $env:CFLAGS_x86_64_unknown_linux_gnu = '--target=x86_64-unknown-linux-gnu'
        }
    }
}

$tomlPath = Join-Path $repoRoot 'Cargo.toml'
$tomlText = Get-Content -LiteralPath $tomlPath -Raw
$useNamedProfile = $tomlText -match ("\[profile\." + [regex]::Escape($cargoProfile) + "\]")

$cargoArgs = @(
    'build',
    '-p', $Crate,
    '--target', $Target,
    '--target-dir', $TargetDir
)
if ($useNamedProfile) {
    $cargoArgs += @('--profile', $cargoProfile)
} else {
    $cargoArgs += '--release'
    Write-Host "Workspace Cargo.toml does not yet define [profile.$cargoProfile]; using --release (WP-4 pending)."
}

if ($Profile -eq 'iot-headless') {
    $cargoArgs += '--no-default-features'
}

$env:WEBIZEN_NATIVE_PROFILE = $cargoProfile
Write-Host "Building $Crate profile=$cargoProfile target=$Target isolated target-dir=$TargetDir"
& cargo @cargoArgs
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$profileOutDir = if ($useNamedProfile) { $cargoProfile } else { 'release' }
$outPath = Join-Path $TargetDir (Join-Path $Target (Join-Path $profileOutDir $Crate))
Write-Host "Linux binary should be in $outPath"
