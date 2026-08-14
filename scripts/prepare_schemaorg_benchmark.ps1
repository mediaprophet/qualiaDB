[CmdletBinding()]
param(
    [string]$Release = "30.0",
    [string]$Variant = "current-https",
    [string]$DataRoot = "data/schemaorg"
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$releaseDir = Join-Path $repoRoot (Join-Path $DataRoot $Release)
$docsReleaseDir = Join-Path $repoRoot (Join-Path "docs/data/schemaorg" $Release)
$baseName = "schemaorg-$Variant"
$ntPath = Join-Path $releaseDir "$baseName.nt"
$q42Path = Join-Path $releaseDir "$baseName.q42"
$rawUrl = "https://raw.githubusercontent.com/schemaorg/schemaorg/main/data/releases/$Release/$baseName.nt"

New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null
New-Item -ItemType Directory -Force -Path $docsReleaseDir | Out-Null

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

Write-Host "Ingesting N-Triples into unified v3 .q42 (embedded lexicon, no sidecars)..." -ForegroundColor Yellow
Push-Location $repoRoot
cargo run --release -p qualia-cli -- import $ntPath $q42Path
Pop-Location

if (Test-Path $ntPath) {
    $ntMb = [math]::Round((Get-Item $ntPath).Length / 1MB, 2)
    Write-Host "  NT size    : $ntMb MB (N-Triples, all RDF engines)" -ForegroundColor DarkGray
}
if (Test-Path $q42Path) {
    $qMb = [math]::Round((Get-Item $q42Path).Length / 1MB, 2)
    Write-Host "  .q42 size  : $qMb MB (v3 unified volume — lex + bidx + LZ4 blocks embedded)" -ForegroundColor DarkGray
}

Write-Host "Syncing benchmark artifacts into docs/data for GitHub Pages and local site testing..." -ForegroundColor Yellow
Copy-Item -Force $ntPath $docsReleaseDir
Copy-Item -Force $q42Path $docsReleaseDir
Remove-Item -Force (Join-Path $releaseDir "$baseName.q42.lex") -ErrorAction SilentlyContinue
Remove-Item -Force (Join-Path $releaseDir "$baseName.q42.bidx") -ErrorAction SilentlyContinue
Remove-Item -Force (Join-Path $docsReleaseDir "$baseName.q42.lex") -ErrorAction SilentlyContinue
Remove-Item -Force (Join-Path $docsReleaseDir "$baseName.q42.bidx") -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "Next steps:" -ForegroundColor Green
Write-Host "  1. Build q42 bench helper: cargo build --release -p qualia-cli"
Write-Host "  2. Run comparative harness with the Schema.org profile:"
Write-Host "     python benchmarks/harness.py --all --dataset-profile schemaorg-30-current-https --output docs/comparative_benchmark_results.schemaorg-30-current-https.json"
Write-Host "  3. Open docs/comparative_benchmarks.html and select the Schema.org 30.0 profile."
