# ==============================================================================
# Qualia-DB Public Benchmark Harness Downloader
# ==============================================================================
# This script downloads various massive public Semantic Web datasets 
# for benchmarking the zero-allocation Fractal Sharding engine.
# It places the datasets into the /data directory.

$ErrorActionPreference = "Stop"
$DataDir = Join-Path $PSScriptRoot "..\data"

if (-not (Test-Path $DataDir)) {
    New-Item -ItemType Directory -Force -Path $DataDir | Out-Null
    Write-Host "Created /data directory." -ForegroundColor Green
}

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "📥 Qualia-DB Massive Dataset Downloader" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

# 1. GeoNames RDF Dump (~2.5 GB uncompressed)
$GeoNamesUrl = "http://download.geonames.org/all-geonames-rdf.zip"
$GeoNamesZip = Join-Path $DataDir "all-geonames-rdf.zip"
if (-not (Test-Path $GeoNamesZip)) {
    Write-Host "Downloading GeoNames RDF (~2.5GB uncompressed)..."
    Invoke-WebRequest -Uri $GeoNamesUrl -OutFile $GeoNamesZip
    Write-Host "GeoNames downloaded." -ForegroundColor Green
} else {
    Write-Host "GeoNames already downloaded." -ForegroundColor Yellow
}

# 2. YAGO 4.5 Tiny (~10 GB uncompressed)
$YagoUrl = "https://huggingface.co/datasets/yago-project/yago4.5/resolve/main/yago-4.5.0-tiny-ttl.tar.gz"
$YagoTar = Join-Path $DataDir "yago-4.5.0-tiny-ttl.tar.gz"
if (-not (Test-Path $YagoTar)) {
    Write-Host "Downloading YAGO 4.5 Tiny (~10GB uncompressed)..."
    Invoke-WebRequest -Uri $YagoUrl -OutFile $YagoTar
    Write-Host "YAGO Tiny downloaded." -ForegroundColor Green
} else {
    Write-Host "YAGO Tiny already downloaded." -ForegroundColor Yellow
}

# 3. DBpedia Mapping-based Properties (~4 GB uncompressed)
$DBpediaUrl = "https://databus.dbpedia.org/dbpedia/mappings/mappingbased-objects/2022.12.01/mappingbased-objects_lang=en.ttl.bz2"
$DBpediaBz2 = Join-Path $DataDir "mappingbased-objects.ttl.bz2"
if (-not (Test-Path $DBpediaBz2)) {
    Write-Host "Downloading DBpedia Mapping Properties (~4GB uncompressed)..."
    Invoke-WebRequest -Uri $DBpediaUrl -OutFile $DBpediaBz2
    Write-Host "DBpedia Subset downloaded." -ForegroundColor Green
} else {
    Write-Host "DBpedia Subset already downloaded." -ForegroundColor Yellow
}

# 4. Framester (WebCivics Mirror)
$FramesterUrl = "https://github.com/WebCivics/framester/archive/refs/heads/master.zip"
$FramesterZip = Join-Path $DataDir "framester-master.zip"
if (-not (Test-Path $FramesterZip)) {
    Write-Host "Downloading Framester Knowledge Graph..."
    Invoke-WebRequest -Uri $FramesterUrl -OutFile $FramesterZip
    Write-Host "Framester downloaded." -ForegroundColor Green
} else {
    Write-Host "Framester already downloaded." -ForegroundColor Yellow
}

# 5. WordNet 3.1 N-Triples (ldf.fi)
$WordNetUrl = "http://ldf.fi/wordnet/data?graph=http://ldf.fi/wordnet/wn31"
$WordNetNt = Join-Path $DataDir "wordnet31.nt"
if (-not (Test-Path $WordNetNt)) {
    Write-Host "Downloading WordNet 3.1 N-Triples..."
    Invoke-WebRequest -Uri $WordNetUrl -Headers @{"Accept"="application/n-triples"} -OutFile $WordNetNt
    Write-Host "WordNet downloaded." -ForegroundColor Green
} else {
    Write-Host "WordNet already downloaded." -ForegroundColor Yellow
}

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "✅ All benchmark datasets queued in /data/" -ForegroundColor Green
Write-Host "Extract them before running the 'qualia-cli import' pipeline!" -ForegroundColor Yellow
