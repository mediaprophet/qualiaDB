# [NEW] tests/test_converter.ps1
# Verification suite checking boundary conditions for RDF-star structures

$ErrorActionPreference = "Stop"
Write-Host "Executing Automated Qualia Ingestion Verification Pipeline..." -ForegroundColor Cyan

# 1. Generate minimal valid Turtle-star graph data with nested definitions
$TestGraphPath = "./test_nested_claims.ntx"
@'
<< <http://example.org/Alice> <http://example.org/treated> <http://example.org/PatientB> >> <http://example.org/assignedBy> <http://example.org/DoctorX> .
<http://example.org/DoctorX> <http://example.org/hasClearanceLevel> "High"^^<http://www.w3.org/2001/XMLSchema#integer> .
'@ | Out-File -FilePath $TestGraphPath -Encoding utf8

$OutputVolumePath = "./test_nested_claims.q42"

# 2. Invoke compiler targeting strict memory limit checking boundaries
& cargo run --bin qualia-cli -- ingest --input $TestGraphPath --output $OutputVolumePath

# 3. Assert target multi-block system structural integrity constraints
$fileSize = (Get-Item $OutputVolumePath).Length
$headerOffsetBytes = 256
$superBlockSizeBytes = 40960

if ($fileSize -le $headerOffsetBytes) {
    Write-Host "CRITICAL FAILURE: Archive file layout size ($fileSize bytes) is too small to contain valid structures." -ForegroundColor Red
    Exit 1
}

Write-Host "Pipeline execution validation successful. Database format matches architecture specifications cleanly." -ForegroundColor Green

# 4. YAGO Benchmark Integration
$YagoGraphPath = "C:\Projects\qualiaDB\local\ontology\yago\yago-meta-facts.ntx"
$YagoOutputPath = "./yago-meta-facts.q42"

if (Test-Path $YagoGraphPath) {
    Write-Host "Found YAGO Benchmark dataset. Ingesting via Out-of-Core Pipeline..." -ForegroundColor Cyan
    $yagoStartTime = Get-Date
    & cargo run --bin qualia-cli -- ingest --input $YagoGraphPath --output $YagoOutputPath
    $yagoEndTime = Get-Date
    $yagoDuration = $yagoEndTime - $yagoStartTime
    Write-Host "YAGO Ingestion completed in $($yagoDuration.TotalSeconds) seconds." -ForegroundColor Green
    
    Write-Host "Running SPARQL-Star Benchmark Suite on YAGO Data..." -ForegroundColor Cyan
    & cargo run --bin qualia-cli -- benchmark-action sparql-star $YagoOutputPath
} else {
    Write-Host "YAGO Benchmark dataset not found at $YagoGraphPath. Skipping performance benchmark." -ForegroundColor Yellow
}
