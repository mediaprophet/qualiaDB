$ErrorActionPreference = "Stop"

$testDir = "c:\Projects\qualiaDB\scratch\test_converter"
if (Test-Path $testDir) {
    Remove-Item -Recurse -Force $testDir
}
New-Item -ItemType Directory -Path $testDir | Out-Null

$ntxFile = Join-Path $testDir "test_nested.ntx"
$q42File = Join-Path $testDir "test_nested.q42"
$inspectOut = Join-Path $testDir "inspect.txt"

# Create a test N-Triples-star file
$ntxContent = @"
<http://example.org/alice> <http://xmlns.com/foaf/0.1/knows> <http://example.org/bob> .
<< <http://example.org/alice> <http://xmlns.com/foaf/0.1/knows> <http://example.org/bob> >> <http://example.org/certainty> "0.9" .
"@
Set-Content -Path $ntxFile -Value $ntxContent

Write-Host "Running CLI Ingest..."
& cargo run --bin qualia-cli -- ingest --input $ntxFile --output $q42File

Write-Host "Running CLI Inspect..."
& cargo run --bin qualia-cli -- inspect $q42File > $inspectOut

Write-Host "Inspect Output:"
Get-Content $inspectOut

Write-Host "Converter Test completed."
