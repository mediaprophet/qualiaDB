#!/usr/bin/env pwsh
# Comprehensive YAGO Test Suite for QualiaDB
# Tests Q42 converter, SPARQL, SPARQL-Star, and performance benchmarks

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

# Configuration
$CLI_PATH = "C:\Projects\qualiaDB\target\release\qualia-cli.exe"
$YAGO_DIR = "C:\Projects\qualiaDB\local\ontology\yago"
$TEST_DIR = "C:\Projects\qualiaDB\yago-tests"
$RESULTS_DIR = "$TEST_DIR\results"
$LOG_DIR = "$TEST_DIR\logs"

# Create directories
New-Item -ItemType Directory -Force -Path $RESULTS_DIR | Out-Null
New-Item -ItemType Directory -Force -Path $LOG_DIR | Out-Null

# Test files configuration
$TEST_FILES = @{
    "schema" = @{
        "source" = "$YAGO_DIR\yago-schema.ttl"
        "size" = "small"
        "description" = "YAGO schema definitions (36KB)"
    }
    "taxonomy" = @{
        "source" = "$YAGO_DIR\yago-taxonomy.ttl"
        "size" = "medium"
        "description" = "YAGO taxonomy hierarchy (13MB)"
    }
    "meta_facts" = @{
        "source" = "$YAGO_DIR\yago-meta-facts.ntx"
        "size" = "large"
        "description" = "YAGO meta facts (1GB)"
    }
}

# SPARQL test queries
$SPARQL_QUERIES = @{
    "basic_select" = @"
SELECT ?subject ?predicate ?object
WHERE {
  ?subject ?predicate ?object .
}
LIMIT 10
"@
    "count_entities" = @"
SELECT (COUNT(?entity) AS ?count)
WHERE {
  ?entity a ?type .
}
"@
    "filter_by_type" = @"
SELECT ?entity ?label
WHERE {
  ?entity a ?type .
  ?entity rdfs:label ?label .
  FILTER(?type = <http://schema.org/Person>)
}
LIMIT 100
"@
    "property_path" = @"
SELECT ?entity ?parent ?grandparent
WHERE {
  ?entity rdfs:subClassOf ?parent .
  ?parent rdfs:subClassOf ?grandparent .
}
LIMIT 50
"@
}

# SPARQL-Star test queries
$SPARQL_STAR_QUERIES = @{
    "annotated_statement" = @"
SELECT ?s ?p ?o
WHERE {
  << ?s ?p ?o >> prov:wasGeneratedBy ?activity .
}
LIMIT 10
"@
    "nested_statement" = @"
SELECT ?entity ?type
WHERE {
  ?entity a ?type .
  << ?entity a ?type >> ex:confidence ?confidence .
  FILTER(?confidence > 0.9)
}
LIMIT 50
"@
}

function Log-Message {
    param([string]$Message, [string]$Level = "INFO")
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $logEntry = "[$timestamp] [$Level] $Message"
    Write-Host $logEntry
    Add-Content -Path "$LOG_DIR\test-run.log" -Value $logEntry
}

function Measure-Command-Time {
    param([scriptblock]$ScriptBlock)
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        $result = & $ScriptBlock
        $stopwatch.Stop()
        return @{
            "Result" = $result
            "Time" = $stopwatch.Elapsed
            "Success" = $true
        }
    } catch {
        $stopwatch.Stop()
        return @{
            "Result" = $null
            "Time" = $stopwatch.Elapsed
            "Success" = $false
            "Error" = $_.Exception.Message
        }
    }
}

function Test-Q42-Conversion {
    param([string]$TestName, [string]$SourceFile, [string]$Size)
    
    Log-Message "Starting Q42 conversion test: $TestName" "INFO"
    
    $outputFile = "$RESULTS_DIR\$TestName.q42"
    $testResult = Measure-Command-Time {
        & $CLI_PATH ingest $SourceFile $outputFile
    }
    
    $result = @{
        "TestName" = $TestName
        "Size" = $Size
        "SourceFile" = $SourceFile
        "OutputFile" = $outputFile
        "Success" = $testResult.Success
        "Time" = $testResult.Time
        "Error" = $testResult.Error
    }
    
    if ($testResult.Success) {
        # Get file sizes
        if (Test-Path $SourceFile) {
            $sourceSize = (Get-Item $SourceFile).Length
            $result.SourceSize = $sourceSize
        }
        if (Test-Path $outputFile) {
            $outputSize = (Get-Item $outputFile).Length
            $result.OutputSize = $outputSize
            $result.CompressionRatio = [math]::Round($outputSize / $sourceSize, 4)
        }
        Log-Message "Q42 conversion completed in $($testResult.Time.TotalSeconds.ToString('F2'))s" "SUCCESS"
    } else {
        Log-Message "Q42 conversion failed: $($testResult.Error)" "ERROR"
    }
    
    return $result
}

function Test-SPARQL-Query {
    param([string]$TestName, [string]$QueryFile, [string]$DataFile)
    
    Log-Message "Starting SPARQL query test: $TestName" "INFO"
    
    $testResult = Measure-Command-Time {
        & $CLI_PATH query $DataFile --query $QueryFile
    }
    
    $result = @{
        "TestName" = $TestName
        "DataFile" = $DataFile
        "QueryFile" = $QueryFile
        "Success" = $testResult.Success
        "Time" = $testResult.Time
        "Error" = $testResult.Error
    }
    
    if ($testResult.Success) {
        Log-Message "SPARQL query completed in $($testResult.Time.TotalSeconds.ToString('F2'))s" "SUCCESS"
    } else {
        Log-Message "SPARQL query failed: $($testResult.Error)" "ERROR"
    }
    
    return $result
}

function Test-SPARQL-Star-Query {
    param([string]$TestName, [string]$QueryFile, [string]$DataFile)
    
    Log-Message "Starting SPARQL-Star query test: $TestName" "INFO"
    
    $testResult = Measure-Command-Time {
        & $CLI_PATH query $DataFile --query $QueryFile --star
    }
    
    $result = @{
        "TestName" = $TestName
        "DataFile" = $DataFile
        "QueryFile" = $QueryFile
        "Success" = $testResult.Success
        "Time" = $testResult.Time
        "Error" = $testResult.Error
    }
    
    if ($testResult.Success) {
        Log-Message "SPARQL-Star query completed in $($testResult.Time.TotalSeconds.ToString('F2'))s" "SUCCESS"
    } else {
        Log-Message "SPARQL-Star query failed: $($testResult.Error)" "ERROR"
    }
    
    return $result
}

function Run-Benchmark-Suite {
    param([string]$DataFile)
    
    Log-Message "Running benchmark suite on $DataFile" "INFO"
    
    $benchmarkResult = Measure-Command-Time {
        & $CLI_PATH benchmark --suite full $DataFile
    }
    
    $result = @{
        "DataFile" = $DataFile
        "Success" = $benchmarkResult.Success
        "Time" = $benchmarkResult.Time
        "Error" = $benchmarkResult.Error
    }
    
    if ($benchmarkResult.Success) {
        Log-Message "Benchmark suite completed in $($benchmarkResult.Time.TotalSeconds.ToString('F2'))s" "SUCCESS"
    } else {
        Log-Message "Benchmark suite failed: $($benchmarkResult.Error)" "ERROR"
    }
    
    return $result
}

function Generate-Report {
    param([array]$Results)
    
    $reportPath = "$RESULTS_DIR\test-report-$(Get-Date -Format 'yyyyMMdd-HHmmss').json"
    $Results | ConvertTo-Json -Depth 10 | Out-File -FilePath $reportPath
    
    # Generate human-readable report
    $textReportPath = "$RESULTS_DIR\test-report-$(Get-Date -Format 'yyyyMMdd-HHmmss').txt"
    
    $report = @"
================================================================================
QUALIADB YAGO TEST SUITE REPORT
Generated: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')
================================================================================

"@
    
    foreach ($result in $Results) {
        $report += @"
--------------------------------------------------------------------------------
TEST: $($result.TestName)
--------------------------------------------------------------------------------
Status: $(if ($result.Success) { "PASSED" } else { "FAILED" })
Duration: $($result.Time.TotalSeconds.ToString('F2')) seconds
"@
        
        if ($result.SourceSize) {
            $report += "Source Size: $([math]::Round($result.SourceSize / 1MB, 2)) MB`n"
        }
        if ($result.OutputSize) {
            $report += "Output Size: $([math]::Round($result.OutputSize / 1MB, 2)) MB`n"
        }
        if ($result.CompressionRatio) {
            $report += "Compression Ratio: $($result.CompressionRatio.ToString('P2'))`n"
        }
        if ($result.Error) {
            $report += "Error: $($result.Error)`n"
        }
        $report += "`n"
    }
    
    $report | Out-File -FilePath $textReportPath
    Log-Message "Reports generated: $reportPath and $textReportPath" "INFO"
}

# Main execution
Log-Message "Starting YAGO Test Suite" "INFO"
Log-Message "CLI Path: $CLI_PATH" "INFO"
Log-Message "YAGO Directory: $YAGO_DIR" "INFO"

$allResults = @()

# Test 1: Q42 Converter with small file
if (Test-Path $TEST_FILES.schema.source) {
    Log-Message "Test 1: Q42 Converter - Small file (schema)" "INFO"
    $result = Test-Q42-Conversion -TestName "yago-schema" -SourceFile $TEST_FILES.schema.source -Size "small"
    $allResults += $result
} else {
    Log-Message "Schema file not found, skipping" "WARN"
}

# Test 2: Q42 Converter with medium file
if (Test-Path $TEST_FILES.taxonomy.source) {
    Log-Message "Test 2: Q42 Converter - Medium file (taxonomy)" "INFO"
    $result = Test-Q42-Conversion -TestName "yago-taxonomy" -SourceFile $TEST_FILES.taxonomy.source -Size "medium"
    $allResults += $result
} else {
    Log-Message "Taxonomy file not found, skipping" "WARN"
}

# Test 3: Q42 Converter with large file (optional - may take long time)
if (Test-Path $TEST_FILES.meta_facts.source) {
    $response = Read-Host "Do you want to test with large meta-facts file (1GB)? This may take several minutes. (y/n)"
    if ($response -eq 'y' -or $response -eq 'Y') {
        Log-Message "Test 3: Q42 Converter - Large file (meta-facts)" "INFO"
        $result = Test-Q42-Conversion -TestName "yago-meta-facts" -SourceFile $TEST_FILES.meta_facts.source -Size "large"
        $allResults += $result
    } else {
        Log-Message "Large file test skipped by user" "INFO"
    }
} else {
    Log-Message "Meta-facts file not found, skipping" "WARN"
}

# Test 4: SPARQL Queries (on converted files)
$convertedFiles = @()
foreach ($result in $allResults) {
    if ($result.Success -and (Test-Path $result.OutputFile)) {
        $convertedFiles += $result.OutputFile
    }
}

if ($convertedFiles.Count -gt 0) {
    $testDataFile = $convertedFiles[0] # Use smallest converted file for query tests
    Log-Message "Test 4: SPARQL Queries on $testDataFile" "INFO"
    
    foreach ($queryName in $SPARQL_QUERIES.Keys) {
        $queryFile = "$RESULTS_DIR\query-$queryName.rq"
        $SPARQL_QUERIES[$queryName] | Out-File -FilePath $queryFile
        
        $result = Test-SPARQL-Query -TestName "sparql-$queryName" -QueryFile $queryFile -DataFile $testDataFile
        $allResults += $result
    }
} else {
    Log-Message "No converted files available for SPARQL testing" "WARN"
}

# Test 5: SPARQL-Star Queries
if ($convertedFiles.Count -gt 0) {
    $testDataFile = $convertedFiles[0]
    Log-Message "Test 5: SPARQL-Star Queries on $testDataFile" "INFO"
    
    foreach ($queryName in $SPARQL_STAR_QUERIES.Keys) {
        $queryFile = "$RESULTS_DIR\star-query-$queryName.rq"
        $SPARQL_STAR_QUERIES[$queryName] | Out-File -FilePath $queryFile
        
        $result = Test-SPARQL-Star-Query -TestName "sparql-star-$queryName" -QueryFile $queryFile -DataFile $testDataFile
        $allResults += $result
    }
} else {
    Log-Message "No converted files available for SPARQL-Star testing" "WARN"
}

# Test 6: Benchmark Suite
if ($convertedFiles.Count -gt 0) {
    $testDataFile = $convertedFiles[0]
    Log-Message "Test 6: Benchmark Suite on $testDataFile" "INFO"
    
    $result = Run-Benchmark-Suite -DataFile $testDataFile
    $result.TestName = "benchmark-suite"
    $allResults += $result
} else {
    Log-Message "No converted files available for benchmark testing" "WARN"
}

# Generate reports
Generate-Report -Results $allResults

Log-Message "YAGO Test Suite completed" "INFO"
Log-Message "Results saved to: $RESULTS_DIR" "INFO"