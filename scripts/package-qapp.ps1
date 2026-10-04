# Package an application directory into an installable .qpkg bundle with package-manifest.json
param(
    [Parameter(Mandatory=$true)]
    [string]$AppDir,
    [string]$OutDir = "",
    [switch]$Sign,
    [string]$PrivateKeyHex
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($OutDir)) {
    $OutDir = Join-Path (Get-Location) "dist\qapps"
} elseif (-not [System.IO.Path]::IsPathRooted($OutDir)) {
    $OutDir = [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $OutDir))
}

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

if (-not (Test-Path $AppDir)) {
    throw "Application directory not found: $AppDir"
}
$AppDir = (Resolve-Path $AppDir).Path

$manifestPath = Join-Path $AppDir "qapp.json"
if (-not (Test-Path $manifestPath)) {
    throw "qapp.json manifest missing in $AppDir"
}

$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
$pkgName = $manifest.name
$pkgVersion = $manifest.version
$pkgId = if ($manifest.x_qualia -and $manifest.x_qualia.app_id) { $manifest.x_qualia.app_id } else { $pkgName }

Write-Host "Packaging QApp: $pkgId (v$pkgVersion) from $AppDir..."

# Collect files and calculate sha256
$files = Get-ChildItem -Path $AppDir -Recurse -File | Where-Object {
    $_.FullName -notmatch '[\\/](\.git|\.staging|dist|target|pkg-poet|\.qpkg)[\\/]' -and
    $_.Name -ne "package-manifest.json" -and
    $_.Extension -ne ".qpkg"
}

$fileHashes = @()
foreach ($file in $files) {
    $relPath = $file.FullName.Substring($AppDir.Length).TrimStart('\', '/').Replace('\', '/')
    $hash = (Get-FileHash -Path $file.FullName -Algorithm SHA256).Hash.ToLower()
    $fileHashes += [PSCustomObject]@{
        path = $relPath
        sha256 = $hash
    }
}

$sidecar = [PSCustomObject]@{
    schema_version = 1
    package_id = $pkgId
    version = $pkgVersion
    abi_version = "1.0"
    host_api_version = "1"
    files = $fileHashes
    signature_hex = ""
}

$sidecarPath = Join-Path $AppDir "package-manifest.json"
$sidecar | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $sidecarPath -Encoding UTF8
Write-Host "Generated content manifest: $sidecarPath"

# Create output bundle
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$bundleName = "$pkgId-v$pkgVersion.qpkg"
$bundlePath = Join-Path $OutDir $bundleName

if (Test-Path $bundlePath) {
    Remove-Item -Force $bundlePath
}

# Zip package files into .qpkg archive
Write-Host "Creating archive: $bundlePath..."
$zip = [System.IO.Compression.ZipFile]::Open($bundlePath, [System.IO.Compression.ZipArchiveMode]::Create)
try {
    foreach ($item in $fileHashes) {
        $sourceFile = Join-Path $AppDir $item.path.Replace('/', '\')
        [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $sourceFile, $item.path) | Out-Null
    }
    # Add sidecar
    [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $sidecarPath, "package-manifest.json") | Out-Null
} finally {
    $zip.Dispose()
}

$bundleSize = (Get-Item $bundlePath).Length
Write-Host "Successfully built $bundleName ($([math]::Round($bundleSize / 1KB, 1)) KB)" -ForegroundColor Green
Write-Host "Bundle location: $bundlePath"
