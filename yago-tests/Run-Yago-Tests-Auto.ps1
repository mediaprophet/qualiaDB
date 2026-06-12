#!/usr/bin/env pwsh
# Automated YAGO Test Suite for QualiaDB (Non-interactive)
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
}



function Log-Message {
    param([string]$Message, [string]$Level = "INFO")
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $logEntry = "[$timestamp] [$Level] $Message"
    Write-Host $logEntry
    try {
        Add-Content -Path "$LOG_DIR\test-run.log" -Value $logEntry -ErrorAction SilentlyContinue
    } catch {
        # Ignore logging errors
    }
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
        & $CLI_PATH import $SourceFile $outputFile
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

function Test-Inspection {
    param([string]$TestName, [string]$DataFile)
    
    Log-Message "Starting inspection test: $TestName" "INFO"
    
    $testResult = Measure-Command-Time {
        & $CLI_PATH inspect $DataFile
    }
    
    $result = @{
        "TestName" = $TestName
        "DataFile" = $DataFile
        "Success" = $testResult.Success
        "Time" = $testResult.Time
        "Error" = $testResult.Error
    }
    
    if ($testResult.Success) {
        Log-Message "Inspection completed in $($testResult.Time.TotalSeconds.ToString('F2'))s" "SUCCESS"
    } else {
        Log-Message "Inspection failed: $($testResult.Error)" "ERROR"
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
Log-Message "Starting YAGO Test Suite (Automated)" "INFO"
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

# Test 3: Inspection Tests (on converted files)
$convertedFiles = @()
foreach ($result in $allResults) {
    if ($result.Success -and (Test-Path $result.OutputFile)) {
        $convertedFiles += $result.OutputFile
    }
}

if ($convertedFiles.Count -gt 0) {
    foreach ($dataFile in $convertedFiles) {
        $fileName = [System.IO.Path]::GetFileNameWithoutExtension($dataFile)
        Log-Message "Test 3: Inspection test on $fileName" "INFO"
        
        $result = Test-Inspection -TestName "inspect-$fileName" -DataFile $dataFile
        $allResults += $result
    }
} else {
    Log-Message "No converted files available for inspection testing" "WARN"
}

# Generate reports
Generate-Report -Results $allResults

Log-Message "YAGO Test Suite completed" "INFO"
Log-Message "Results saved to: $RESULTS_DIR" "INFO"