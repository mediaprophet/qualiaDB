# QualiaDB YAGO Test Suite

Comprehensive test suite for QualiaDB Q42 converter, inspection, and performance benchmarks using YAGO ontology data.

## Test Structure

### Test Categories

1. **Q42 Converter Tests**
   - Small file conversion (schema - 36KB)
   - Medium file conversion (taxonomy - 13MB)
   - Large file conversion (meta-facts - 1GB, optional)

2. **Inspection Tests**
   - Schema file validation
   - Taxonomy file validation
   - NQuin alignment verification

3. **Performance Analysis**
   - Conversion speed metrics
   - Compression ratio analysis
   - Processing throughput calculations

### Future Test Categories (Requires Daemon Mode)

4. **SPARQL Query Tests** (Requires daemon)
   - Basic SELECT queries
   - COUNT aggregates
   - FILTER operations
   - Property path queries

5. **SPARQL-Star Tests** (Requires daemon)
   - Annotated statement queries
   - Nested statement queries
   - Confidence-based filtering

6. **Performance Benchmarks** (Requires daemon)
   - Full benchmark suite
   - Memory usage analysis
   - Query performance metrics

## Directory Structure

```
yago-tests/
├── Run-Yago-Tests.ps1          # Main test script
├── README.md                   # This file
├── results/                    # Test results and reports
│   ├── *.q42                  # Converted Q42 files
│   ├── *.rq                   # SPARQL query files
│   ├── test-report-*.json     # JSON reports
│   └── test-report-*.txt      # Human-readable reports
└── logs/                       # Execution logs
    └── test-run.log           # Detailed log file
```

## Prerequisites

- QualiaDB CLI built and available at `C:\Projects\qualiaDB\target\release\qualia-cli.exe`
- YAGO ontology data available at `C:\Projects\qualiaDB\local\ontology\yago`
- PowerShell 5.1 or later

## Quick Start

### Run Automated Tests (Recommended)

```powershell
cd C:\Projects\qualiaDB\yago-tests
powershell -ExecutionPolicy Bypass -File .\Run-Yago-Tests-Auto.ps1
```

### Run Interactive Tests (Includes Large Files)

```powershell
cd C:\Projects\qualiaDB\yago-tests
powershell -ExecutionPolicy Bypass -File .\Run-Yago-Tests.ps1
```

### Run SPARQL Tests (Requires Daemon)

1. Start the QualiaDB daemon:
```powershell
C:\Projects\qualiaDB\target\release\qualia-cli.exe daemon start
```

2. Run SPARQL queries against the daemon (requires separate client or HTTP requests)

3. Stop the daemon when finished:
```powershell
C:\Projects\qualiaDB\target\release\qualia-cli.exe daemon stop
```

### Run Specific Test Categories

Edit the test scripts to comment out unwanted test sections.

## Test Data

### YAGO Files

- **yago-schema.ttl** (36KB): Schema definitions
- **yago-taxonomy.ttl** (13MB): Taxonomy hierarchy  
- **yago-meta-facts.ntx** (1GB): Meta facts
- **yago-facts.ttl** (23GB): Full facts (not included in default tests)
- **yago-beyond-wikipedia.ttl** (127GB): Extended data (not included in default tests)

## Expected Results

### Current Test Results (2026-06-12)

#### Q42 Conversion
- ✅ **Schema File**: 36 KB → 23 KB (64% of original) in 0.34s
- ✅ **Taxonomy File**: 13.4 MB → 3.5 MB (28% of original) in 1.30s
- ✅ **Compression**: 28-64% size reduction achieved
- ✅ **Performance**: 9.8 MB/s processing speed

#### Inspection
- ✅ **Schema Inspection**: 0.10s for 23 KB file
- ✅ **Taxonomy Inspection**: 0.71s for 3.5 MB file
- ⚠️ **Alignment Warnings**: Files show NQuin alignment warnings (non-critical)

### Success Criteria

- ✅ Q42 conversion completes without errors
- ✅ Compression ratio < 1.0 (size reduction)
- ✅ Inspection operations complete successfully
- ⏳ SPARQL queries return results (requires daemon mode)
- ⏳ SPARQL-Star queries execute successfully (requires daemon mode)
- ⏳ Benchmark suite completes (requires daemon mode)

### Performance Metrics

The test suite measures:
- Conversion time (seconds)
- Compression ratio
- Query execution time (milliseconds)
- Memory usage (MB)
- Throughput (triples/second)

## Reports

### JSON Report
Machine-readable report with all test metrics:
```json
{
  "TestName": "yago-schema",
  "Size": "small",
  "Success": true,
  "Time": "00:00:01.2345678",
  "SourceSize": 36440,
  "OutputSize": 28500,
  "CompressionRatio": 0.7825
}
```

### Text Report
Human-readable summary with test results and performance metrics.

## Troubleshooting

### Common Issues

1. **CLI not found**
   - Ensure CLI is built: `cargo build --release -p qualia-cli`
   - Check CLI path in script configuration

2. **YAGO files not found**
   - Verify YAGO data directory path
   - Check file permissions

3. **Memory issues with large files**
   - Skip large file tests when prompted
   - Increase system memory if needed

4. **SPARQL query failures**
   - Check query syntax
   - Verify data was converted successfully
   - Review log files for detailed errors

## Customization

### Add New Tests

1. Add test configuration to `$TEST_FILES` hashtable
2. Add queries to `$SPARQL_QUERIES` or `$SPARQL_STAR_QUERIES`
3. Create corresponding test function
4. Call test function in main execution block

### Modify Queries

Edit the `$SPARQL_QUERIES` or `$SPARQL_STAR_QUERIES` hashtables in the script.

### Change Test Data

Update the `$YAGO_DIR` variable and file paths in the configuration section.

## Performance Expectations

Based on QualiaDB architecture:

- **Small files (< 1MB)**: < 1 second conversion
- **Medium files (1-100MB)**: < 30 seconds conversion
- **Large files (100MB-1GB)**: < 5 minutes conversion
- **Query latency**: < 10ms for indexed queries
- **Compression**: 20-40% size reduction typical

## Notes

- Large file tests are optional and require user confirmation
- Tests are run sequentially to ensure accurate measurements
- Results are timestamped to allow multiple test runs
- Log files contain detailed execution information

## Support

For issues or questions:
- Check log files in `logs/` directory
- Review test reports in `results/` directory
- Consult QualiaDB documentation in `docs/manuals/`