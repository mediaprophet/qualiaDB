#Requires -Version 5.1
<#
.SYNOPSIS
  Fetch PermissiveReady vision weights into vendor/vision/ subdirs.

.DESCRIPTION
  Does NOT claim commercial licences. MIT/Apache zoo weights only.
  Skips assets with empty source_urls (manual place). Training corpora not fetched.

.EXAMPLE
  .\vendor\vision\download.ps1
  .\vendor\vision\download.ps1 -Assets yunet,sface,mediapipe_face_landmarker
#>
param(
    [string[]]$Assets = @(),
    [switch]$Force
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$ManifestPath = Join-Path $Root "MANIFEST.json"
if (-not (Test-Path $ManifestPath)) {
    throw "Missing MANIFEST.json at $ManifestPath"
}

$manifest = Get-Content -Raw -Path $ManifestPath | ConvertFrom-Json
$wanted = if ($Assets.Count -gt 0) { $Assets } else { $manifest.assets | ForEach-Object { $_.id } }

function Get-FileSha256([string]$Path) {
    if (-not (Test-Path $Path)) { return $null }
    return (Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLowerInvariant()
}

function Download-Asset($asset) {
    $dir = Join-Path $Root $asset.rel_dir
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    $dest = Join-Path $dir $asset.filename
    $meta = Join-Path $dir "FETCH.json"

    if ((Test-Path $dest) -and -not $Force) {
        Write-Host "[skip] $($asset.id) already present: $dest"
        return $true
    }

    if ($asset.manual_place -eq $true -or -not $asset.source_urls -or $asset.source_urls.Count -eq 0) {
        Write-Host "[manual] $($asset.id): place $($asset.filename) into $dir (licence $($asset.licence))"
        return $false
    }

    $ok = $false
    foreach ($url in $asset.source_urls) {
        Write-Host "[get] $($asset.id) <- $url"
        try {
            Invoke-WebRequest -Uri $url -OutFile $dest -UseBasicParsing
            if ((Test-Path $dest) -and ((Get-Item $dest).Length -gt 1024)) {
                $ok = $true
                break
            }
        } catch {
            Write-Warning "  failed: $($_.Exception.Message)"
            if (Test-Path $dest) { Remove-Item $dest -Force }
        }
    }

    if (-not $ok) {
        Write-Warning "[fail] $($asset.id)"
        return $false
    }

    $sha = Get-FileSha256 $dest
    $fetch = [ordered]@{
        id           = $asset.id
        filename     = $asset.filename
        licence      = $asset.licence
        licence_tag  = $asset.licence_tag
        sha256       = $sha
        bytes        = (Get-Item $dest).Length
        fetched_unix = [int][double]::Parse((Get-Date -UFormat %s))
        note         = "PermissiveReady weight; not a commercial-licence gate"
    }
    ($fetch | ConvertTo-Json) | Set-Content -Path $meta -Encoding utf8
    Write-Host "[ok] $($asset.id) sha256=$sha bytes=$($fetch.bytes)"
    return $true
}

$got = 0
$miss = 0
foreach ($id in $wanted) {
    $asset = $manifest.assets | Where-Object { $_.id -eq $id } | Select-Object -First 1
    if (-not $asset) {
        Write-Warning "Unknown asset id: $id"
        $miss++
        continue
    }
    if (Download-Asset $asset) { $got++ } else { $miss++ }
}

Write-Host ""
Write-Host "Done. present=$got  missing_or_manual=$miss"
Write-Host "Adapters still required (AdapterMissing) until swarm wires ONNX/TFLite loaders."
exit 0
