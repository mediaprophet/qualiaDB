# Download princeton.q42 from GitHub Release assets (replaces Git LFS).
param(
    [string]$Tag,
    [string]$Repo = $env:QUALIA_GITHUB_REPO
)

Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force -ErrorAction SilentlyContinue | Out-Null
$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $RepoRoot

if (-not $Repo) { $Repo = 'mediaprophet/qualiaDB' }

$VersionFile = Join-Path $RepoRoot 'docs\data\wordnet\VERSION'
if (-not $Tag -and (Test-Path $VersionFile)) {
    $Tag = 'v' + (Get-Content $VersionFile -Raw).Trim()
}
if (-not $Tag) { $Tag = 'v0.0.16' }

$OutDir = Join-Path $RepoRoot 'docs\data\wordnet'
$Canonical = Join-Path $OutDir 'princeton.q42'
$Playground = Join-Path $RepoRoot 'docs\playground\wordnet.q42'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

Write-Host "=== Fetch Princeton WordNet from GitHub Release ==="
Write-Host "  Repo : $Repo"
Write-Host "  Tag  : $Tag"
Write-Host "  Out  : $Canonical"
Write-Host ""

$url = "https://github.com/$Repo/releases/download/$Tag/princeton.q42"
try {
    Invoke-WebRequest -Uri $url -OutFile $Canonical -UseBasicParsing
} catch {
    $gh = Get-Command gh -ErrorAction SilentlyContinue
    if ($gh -and ($env:GH_TOKEN -or $env:GITHUB_TOKEN)) {
        & gh release download $Tag --repo $Repo --pattern 'princeton.q42' --dir $OutDir --clobber
    } else {
        throw $_
    }
}

if (-not (Test-Path $Canonical)) {
    throw "princeton.q42 not found after download"
}

Copy-Item $Canonical $Playground -Force
Get-Item $Canonical, $Playground | Format-Table Name, Length -AutoSize