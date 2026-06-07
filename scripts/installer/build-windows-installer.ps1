# Build Windows Inno Setup installer from staged portable bundle.
param(
    [string]$AppVersion = "0.0.7",
    [string]$StageDir = ""
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "../..")
if (-not $StageDir) {
    $StageDir = Join-Path $Root "dist\qualia-flutter-windows-x64"
}
$Iss = Join-Path $Root "scripts\installer\qualia-flutter.iss"

if (-not (Test-Path $StageDir)) {
    throw "Stage directory missing: $StageDir (run package-flutter-windows.ps1 first)"
}

$Iscc = @(
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1

if (-not $Iscc) {
    throw "Inno Setup ISCC.exe not found. Install from https://jrsoftware.org/isinfo.php or: choco install innosetup"
}

Write-Host "Building installer (version $AppVersion)..."
& $Iscc "/DAppVersion=$AppVersion" "/DStageDir=$StageDir" $Iss
if ($LASTEXITCODE -ne 0) { throw "ISCC failed with exit code $LASTEXITCODE" }

$Out = Join-Path $Root "dist\QualiaDB-Setup-$AppVersion-x64.exe"
if (-not (Test-Path $Out)) {
    throw "Expected installer not found: $Out"
}
Write-Host "Installer: $Out"
