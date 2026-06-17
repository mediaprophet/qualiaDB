# ingest_princeton_wordnet.ps1
# Ingest Princeton WordNet 3.1 RDF/XML into a unified v3 .q42 volume and wire demo paths.
#
# Requirements:
#   - cargo-built qualia-cli (release)
#   - Princeton wordnet.rdf (~500 MB RDF/XML)
#
# Usage:
#   .\scripts\ingest_princeton_wordnet.ps1
#   $env:QUALIA_PRINCETON_RDF = 'C:\path\to\wordnet.rdf'; .\scripts\ingest_princeton_wordnet.ps1
#   .\scripts\ingest_princeton_wordnet.ps1 -SkipPlayground   # data dir only

param(
    [string]$RdfPath = $env:QUALIA_PRINCETON_RDF,
    [switch]$SkipPlayground
)

# Bypass policy when invoked as a script file (CI/local automation).
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force -ErrorAction SilentlyContinue | Out-Null
$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $RepoRoot

if (-not $RdfPath) {
    $candidates = @(
        'C:\Projects\ontologies-2023\wordnet.rdf',
        (Join-Path $RepoRoot 'bundled\ontologies\wordnet\wordnet.rdf'),
        (Join-Path $RepoRoot 'data\wordnet.rdf')
    )
    foreach ($c in $candidates) {
        if (Test-Path $c) { $RdfPath = $c; break }
    }
}

if (-not $RdfPath -or -not (Test-Path $RdfPath)) {
    Write-Error @"
Princeton wordnet.rdf not found.
Set QUALIA_PRINCETON_RDF to the RDF/XML file (e.g. C:\Projects\ontologies-2023\wordnet.rdf)
"@
}

$Cli = Join-Path $RepoRoot 'target\release\qualia-cli.exe'
if (-not (Test-Path $Cli)) {
    Write-Host 'Building qualia-cli (release)...'
    cargo build --release -p qualia-cli
}

$RdfPath = (Resolve-Path $RdfPath).Path
$RdfDir = Split-Path -Parent $RdfPath
$BuiltQ42 = Join-Path $RdfDir (([IO.Path]::GetFileNameWithoutExtension($RdfPath)) + '.q42')

$DataDir = Join-Path $RepoRoot 'docs\data\wordnet'
$Canonical = Join-Path $DataDir 'princeton.q42'
$Playground = Join-Path $RepoRoot 'docs\playground\wordnet.q42'
$LocalLib = Join-Path $RepoRoot 'Local_LIbraries\wordnet\wordnet.q42'

New-Item -ItemType Directory -Force -Path (Split-Path $LocalLib) | Out-Null
New-Item -ItemType Directory -Force -Path $DataDir | Out-Null

$rdfMb = [math]::Round((Get-Item $RdfPath).Length / 1MB, 2)
Write-Host "=== Princeton WordNet → Q42 v3 ==="
Write-Host "  Source : $RdfPath ($rdfMb MB RDF/XML)"
Write-Host "  Output : $Canonical"
Write-Host ""

$sw = [Diagnostics.Stopwatch]::StartNew()
& $Cli ingest semantic $RdfPath
if ($LASTEXITCODE -ne 0) { throw "qualia-cli ingest failed with exit $LASTEXITCODE" }
$sw.Stop()

if (-not (Test-Path $BuiltQ42)) {
    throw "Expected ingest output missing: $BuiltQ42"
}

$q42 = Get-Item $BuiltQ42
$q42Mb = [math]::Round($q42.Length / 1MB, 2)
Write-Host ""
Write-Host "Ingest finished in $($sw.Elapsed.ToString('hh\:mm\:ss')) — $($q42.Length) bytes ($q42Mb MB)"

Copy-Item $BuiltQ42 $Canonical -Force
Copy-Item $BuiltQ42 $LocalLib -Force
if (-not $SkipPlayground) {
    Copy-Item $BuiltQ42 $Playground -Force
}

Write-Host ""
Write-Host 'Installed:'
Write-Host "  $Canonical"
Write-Host "  $LocalLib"
if (-not $SkipPlayground) { Write-Host "  $Playground" }
Write-Host ''
Write-Host 'Demos mount via docs/playground/vfs-manifest.json → data/wordnet/princeton.q42'
Write-Host 'Or download release asset: .\scripts\fetch_wordnet_release.ps1'