# Package QualiaDB Flutter desktop for Windows (portable folder + CI dist output).
# Usage: .\scripts\package-flutter-windows.ps1 [-OutDir path] [-SkipBuild]

param(
    [string]$OutDir = "",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
# robocopy uses exit code 1 for successful copy; do not treat as a native-command error.
$PSNativeCommandUseErrorActionPreference = $false
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)

function Invoke-RobocopyChecked {
    param(
        [Parameter(Mandatory = $true)][string]$Source,
        [Parameter(Mandatory = $true)][string]$Destination,
        [switch]$Mirror,
        [switch]$Recurse
    )
    $args = @($Source, $Destination)
    if ($Mirror) { $args += "/MIR" }
    elseif ($Recurse) { $args += "/E" }
    $args += "/NFL", "/NDL", "/NJH", "/NJS", "/nc", "/ns", "/np"
    robocopy @args | Out-Null
    if ($LASTEXITCODE -ge 8) { throw "robocopy failed with exit code $LASTEXITCODE" }
    $global:LASTEXITCODE = 0
}
if (-not $OutDir) {
    $OutDir = Join-Path $Root "dist\qualia-flutter-windows-x64"
}
$Flutter = Join-Path $Root "crates\qualia-flutter"
$Build = Join-Path $Flutter "build\windows\x64\runner\Release"
$RustDll = Join-Path $Root "target\release\qualia_flutter_rust.dll"
$DirectMl = Join-Path $Root "vendor\directml\bin\x64-win\DirectML.dll"

if (-not $SkipBuild) {
    Write-Host "Building Flutter Windows release..."
    Push-Location $Flutter
    flutter pub get | Out-Null
    flutter build windows --release
    if ($LASTEXITCODE -ne 0) { throw "Flutter build failed with exit code $LASTEXITCODE" }
    Pop-Location

    Write-Host "Building Rust FFI (release)..."
    cargo build --release -p qualia_flutter_rust
    if ($LASTEXITCODE -ne 0) { throw "Rust build failed with exit code $LASTEXITCODE" }
}

if (-not (Test-Path $Build)) { throw "Flutter build output missing: $Build" }
if (-not (Test-Path $RustDll)) { throw "Rust DLL missing: $RustDll" }

Write-Host "Staging portable bundle to $OutDir ..."
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
Invoke-RobocopyChecked -Source $Build -Destination $OutDir -Mirror

Copy-Item $RustDll $OutDir -Force
if (Test-Path $DirectMl) {
    Copy-Item $DirectMl $OutDir -Force
    Write-Host "Bundled DirectML.dll"
}

# Optional: ship WebView2 Fixed Version runtime next to the exe (no separate install).
# Place Microsoft's fixed runtime under vendor\webview2\ (must contain msedgewebview2.exe).
$WebView2Vendor = Join-Path $Root "vendor\webview2"
if (Test-Path $WebView2Vendor) {
    $Dest = Join-Path $OutDir "WebView2Runtime"
    New-Item -ItemType Directory -Force -Path $Dest | Out-Null
    Invoke-RobocopyChecked -Source $WebView2Vendor -Destination $Dest -Recurse
    Write-Host "Bundled WebView2 Fixed Version runtime"
}

Write-Host "Bundled qualia_flutter_rust.dll"

& (Join-Path $Root "scripts\copy-bundled-qapps.ps1") -OutDir $OutDir
& (Join-Path $Root "scripts\copy-bundled-resources.ps1") -OutDir $OutDir

Write-Host "Done. Portable bundle: $OutDir\qualia_flutter.exe"
$global:LASTEXITCODE = 0
exit 0
