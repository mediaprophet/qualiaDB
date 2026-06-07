[CmdletBinding()]
param(
    [string]$Release = "30.0",
    [string]$Variant = "current-https",
    [string]$DataRoot = "data/schemaorg",
    [switch]$Compress
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$releaseDir = Join-Path $repoRoot (Join-Path $DataRoot $Release)
$baseName = "schemaorg-$Variant"
$ntPath = Join-Path $releaseDir "$baseName.nt"
$q42Base = Join-Path $releaseDir $baseName
$q42Path = "$q42Base.q42"
$compressedQ42Path = "$q42Base.c.q42"
$rawUrl = "https://raw.githubusercontent.com/schemaorg/schemaorg/main/data/releases/$Release/$baseName.nt"

New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null

Write-Host "Schema.org benchmark preparation" -ForegroundColor Cyan
Write-Host "  Source URL : $rawUrl"
Write-Host "  NT output  : $ntPath"
Write-Host "  Q42 output : $q42Path"

if (-not (Test-Path $ntPath)) {
    Write-Host "Downloading Schema.org release file..." -ForegroundColor Yellow
    Invoke-WebRequest -UseBasicParsing -Uri $rawUrl -OutFile $ntPath
} else {
    Write-Host "NT source already present, reusing local file." -ForegroundColor DarkGray
}

Write-Host "Ingesting N-Triples into native .q42..." -ForegroundColor Yellow
cargo run --release -p qualia-cli --bin qualia-cli -- ingest --input $ntPath --output $q42Base

if ($Compress.IsPresent) {
    Write-Host "Compressing .q42 artifact for browser/native transport..." -ForegroundColor Yellow
    cargo run --release -p qualia-cli --bin qualia-cli -- compress --input $q42Path --output $compressedQ42Path
}

if (Test-Path $ntPath) {
    $ntMb = [math]::Round((Get-Item $ntPath).Length / 1MB, 2)
    Write-Host "  NT size    : $ntMb MB (N-Triples, all RDF engines)" -ForegroundColor DarkGray
}
if (Test-Path $q42Path) {
    $qMb = [math]::Round((Get-Item $q42Path).Length / 1MB, 2)
    Write-Host "  .q42 size  : $qMb MB (Qualia native SuperBlocks)" -ForegroundColor DarkGray
}
if (Test-Path $compressedQ42Path) {
    $cMb = [math]::Round((Get-Item $compressedQ42Path).Length / 1MB, 2)
    Write-Host "  .c.q42 size: $cMb MB (LZ4 distribution artifact)" -ForegroundColor DarkGray
}

Write-Host ""
Write-Host "Next steps:" -ForegroundColor Green
Write-Host "  1. Build q42 bench helper: cargo build --release -p qualia-cli"
Write-Host "  2. Run comparative harness with the Schema.org profile:"
Write-Host "     python benchmarks/harness.py --all --dataset-profile schemaorg-30-current-https --output docs/comparative_benchmark_results.schemaorg-30-current-https-q42.json"
Write-Host "  3. Open docs/comparative_benchmarks.html and select the Schema.org 30.0 profile."
