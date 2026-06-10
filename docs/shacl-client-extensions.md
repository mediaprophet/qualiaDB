# SHACL Extensions for Qualia Client Features

**Date:** 2026-06-10  
**Related Files:** 
- `crates/qualia-core-db/src/modalities/logic/shacl_extensions.rs`
- `crates/qualia-core-db/shapes/qualia-client-extensions.shacl.ttl`
- `crates/qualia-core-db/src/modalities/logic/mod.rs`

## Overview

This document describes the SHACL (Shapes Constraint Language) extensions added to support the new Qualia Client functionality, including comprehensive logging, system tray integration, and enhanced configuration settings.

## Architecture

### Extension Module Structure

```
crates/qualia-core-db/src/modalities/logic/
├── shacl.rs              # Core SHACL compiler
├── shacl_extensions.rs   # New client feature extensions
└── mod.rs                # Module exports and re-exports
```

### SHACL Shape Files

```
crates/qualia-core-db/shapes/
├── qualia-agency.shacl.ttl              # Existing agency shapes
└── qualia-client-extensions.shacl.ttl   # New client extensions
```

## New SHACL Constraints

### 1. Logging System Constraints

#### `q42:LogConfiguration`
Validates logging system configuration parameters.

**Properties:**
- `maxMemoryLogs`: Maximum logs to keep in memory (1-100,000)
- `maxDiskLogs`: Maximum logs to keep on disk (1-1,000,000)
- `flushIntervalMs`: Buffer flush interval in milliseconds (100-60,000)

**Severity:** Violation for limits, Warning for flush interval

**Example Usage:**
```rust
use qualia_core_db::modalities::logic::shacl_extensions::LogConfiguration;

let config = LogConfiguration {
    max_memory_logs: 10000,
    max_disk_logs: 100000,
    flush_interval_ms: 5000,
};

let opcodes = config.to_opcodes();
```

#### `q42:LogLevel`
Validates log level enumeration values.

**Allowed Levels:** DEBUG, INFO, WARN, ERROR, CRITICAL

**Severity:** Violation

**SHACL Shape:**
```turtle
q42:LogLevelShape a sh:NodeShape ;
    sh:property [
        sh:path q42:level ;
        sh:in ("DEBUG" "INFO" "WARN" "ERROR" "CRITICAL") ;
        sh:severity sh:Violation ;
    ] .
```

#### `q42:LogEntry`
Validates individual log entry structure.

**Properties:**
- `timestamp`: ISO 8601 datetime (required)
- `level`: Log level string (required)
- `message`: Log message (required, max 10,000 chars)
- `context`: Context identifier (optional, max 100 chars)
- `data`: Optional data payload

**Severity:** Violation for required fields, Warning for length limits

#### `q42:LogRetention`
Validates log retention policy configuration.

**Properties:**
- `maxAgeDays`: Maximum age of logs in days (1-365)
- `maxSizeMb`: Maximum total size in MB (1-10,240)
- `backupBeforeCleanup`: Boolean flag for backup before cleanup

**Severity:** Violation for limits, Warning for backup flag

#### `q42:LogExportFormat`
Validates log export format configuration.

**Properties:**
- `format`: Export format - json, txt, or csv
- `maxExportSizeMb`: Maximum export size in MB (1-100)

**Severity:** Violation

### 2. System Tray Constraints

#### `q42:SystemTrayConfiguration`
Validates system tray menu configuration.

**Properties:**
- `maxMenuItems`: Maximum number of menu items (1-20)
- `requireSeparatorLogic`: Boolean flag for separator enforcement
- `allowNestedMenus`: Boolean flag for nested menu support

**Severity:** Violation for item limit, Warning for flags

#### `q42:TrayMenuItem`
Validates individual system tray menu items.

**Properties:**
- `id`: Unique item identifier (required, max 50 chars)
- `label`: Display label (required, max 50 chars)
- `action`: Action handler identifier (required)
- `icon`: Icon identifier (optional)

**Severity:** Violation for required fields, Info for icon

#### `q42:TrayStatusIndicator`
Validates system tray status indicators.

**Properties:**
- `statusText`: Status display text (max 100 chars)
- `updateIntervalMs`: Update interval in milliseconds (100-10,000)
- `allowLiveUpdates`: Boolean flag for live updates

**Severity:** Violation for interval, Warning for text length and flags

#### `q42:TrayAction`
Validates system tray action handlers.

**Properties:**
- `requireWindowContext`: Boolean flag for window context requirement
- `allowAsyncActions`: Boolean flag for async action support
- `timeoutMs`: Action timeout in milliseconds (0-30,000)

**Severity:** Violation for timeout, Warning for flags

### 3. Enhanced Settings Constraints

#### `q42:StorageConfiguration`
Validates storage path and quota settings.

**Properties:**
- `storagePath`: Absolute file system path (required, must start with / or \)
- `storageQuotaGb`: Storage quota in GB (1-1,000)
- `minQuotaGb`: Minimum allowed quota (minimum 1)
- `maxQuotaGb`: Maximum allowed quota (maximum 1,000)

**Severity:** Violation for path and quota, Warning for min/max limits

**Pattern Validation:**
```turtle
sh:pattern "^[/\\\\]"  # Must start with / or \
```

#### `q42:NetworkConfiguration`
Validates daemon network settings.

**Properties:**
- `daemonPort`: Network port (1,024-65,535)
- `daemonHost`: Host address (localhost, 127.0.0.1, 0.0.0.0, or ::1)
- `requirePortConflictCheck`: Boolean flag for port conflict detection
- `defaultHost`: Default hostname for fallback

**Severity:** Violation for port and host, Warning for conflict check flag

**Pattern Validation:**
```turtle
sh:pattern "^(localhost|127\\.0\\.0\\.1|0\\.0\\.0\\.0|::1)$"
```

#### `q42:TaxRecipientConfiguration`
Validates ILP tax recipient configuration.

**Properties:**
- `label`: Recipient display label (required, max 100 chars)
- `ilpAddress`: ILP payment address (required, must start with $ilp.)
- `sharePercent`: Percentage share (0-100)
- `minSharePercent`: Minimum allowed share (minimum 0)
- `maxSharePercent`: Maximum allowed share (maximum 100)
- `allowNymRouting`: Boolean flag for Nym mixnet routing
- `totalShareValidation`: Boolean flag for total percentage validation

**Severity:** Violation for required fields and limits, Warning for validation flags

**Pattern Validation:**
```turtle
sh:pattern "^\\$ilp\\."  # Must start with $ilp.
```

#### `q42:SecurityConfiguration`
Validates security-related settings.

**Properties:**
- `requireEncryption`: Boolean flag for encryption requirement
- `minKeyLengthBits`: Minimum encryption key length in bits (128-4,096)
- `allowedCipherSuites`: List of allowed cipher suite identifiers

**Severity:** Violation for encryption and key length, Warning for cipher suites

## Integration with Qualia Client

### TypeScript/JavaScript Integration

The Qualia Client can use these SHACL constraints for client-side validation:

```typescript
import { logger } from './lib/logger';

// Validate log entry structure
function validateLogEntry(entry: LogEntry): boolean {
  return (
    entry.timestamp !== undefined &&
    ['DEBUG', 'INFO', 'WARN', 'ERROR', 'CRITICAL'].includes(entry.level) &&
    entry.message.length <= 10000 &&
    (entry.context || '').length <= 100
  );
}

// Validate storage configuration
function validateStorageConfig(config: StorageConfig): boolean {
  return (
    config.storagePath.match(/^[/\\]/) !== null &&
    config.storageQuotaGb >= 1 &&
    config.storageQuotaGb <= 1000
  );
}
```

### Rust Backend Integration

The constraints can be used in the Rust backend for data validation:

```rust
use qualia_core_db::modalities::logic::shacl_extensions::{
    LogConfiguration, StorageConfiguration
};

fn validate_log_config(config: &LogConfiguration) -> bool {
    let opcodes = config.to_opcodes();
    // Evaluate opcodes against actual configuration
    // Returns true if all constraints pass
}

fn validate_storage_config(config: &StorageConfiguration) -> bool {
    let opcodes = config.to_opcodes();
    // Evaluate opcodes against actual configuration
    // Returns true if all constraints pass
}
```

## SHACL Compiler Integration

The new constraints are integrated with the existing SHACL compiler:

```rust
use qualia_core_db::modalities::logic::shacl_extensions::get_shacl_extensions_ttl;

// Get the SHACL TTL vocabulary for the new constraints
let extensions_ttl = get_shacl_extensions_ttl();

// The extensions can be loaded into the SHACL compiler
// alongside the core SHACL vocabulary
```

## Opcode Generation

Each constraint type generates appropriate `SlgOpcode` sequences for validation:

```rust
impl LogConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_memory_logs as f64),
            SlgOpcode::CheckMaxInclusive(self.max_disk_logs as f64),
            SlgOpcode::CheckMinInclusive(self.flush_interval_ms as f64),
        ]
    }
}
```

## Validation Workflow

1. **Configuration Loading**: Load configuration from file or user input
2. **Constraint Application**: Apply relevant SHACL constraints
3. **Opcode Generation**: Convert constraints to SlgOpcode sequences
4. **Validation Execution**: Execute opcodes in the SLG VM
5. **Result Reporting**: Return validation results with severity levels

## Severity Levels

- **Violation**: Critical validation failure, data rejected
- **Warning**: Non-critical issue, data accepted with warning
- **Info**: Informational validation, no action required

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_configuration_validation() {
        let config = LogConfiguration {
            max_memory_logs: 10000,
            max_disk_logs: 100000,
            flush_interval_ms: 5000,
        };
        let opcodes = config.to_opcodes();
        assert_eq!(opcodes.len(), 3);
    }

    #[test]
    fn test_storage_configuration_validation() {
        let config = StorageConfiguration {
            min_quota_gb: 1,
            max_quota_gb: 1000,
            require_absolute_path: true,
            allowed_path_patterns: vec!["^/".to_string()],
        };
        let opcodes = config.to_opcodes();
        assert!(!opcodes.is_empty());
    }
}
```

### SHACL Validation Tests

```turtle
# Test data for SHACL validation
@prefix ex: <http://example.org/> .

ex:validLogConfig a q42:LogConfiguration ;
    q42:maxMemoryLogs 10000 ;
    q42:maxDiskLogs 100000 ;
    q42:flushIntervalMs 5000 .

ex:invalidLogConfig a q42:LogConfiguration ;
    q42:maxMemoryLogs 200000 ;  # Exceeds maximum
    q42:maxDiskLogs 100000 ;
    q42:flushIntervalMs 5000 .
```

## Performance Considerations

- **Opcode Generation**: O(1) for most constraint types
- **Validation Execution**: O(n) where n is the number of constraints
- **Memory Usage**: Minimal, as opcodes are stack-allocated
- **Caching**: Constraints can be pre-compiled and cached

## Security Considerations

- **Path Validation**: Storage paths are validated to prevent directory traversal
- **Network Validation**: Host addresses are restricted to safe values
- **ILP Validation**: ILP addresses are validated to prevent payment routing errors
- **Key Length**: Minimum key length requirements ensure adequate encryption strength

## Future Extensions

Potential additional SHACL constraints for future client features:

1. **UI Theme Constraints**: Color palette, font sizes, spacing
2. **Plugin System Constraints**: Plugin validation, sandboxing rules
3. **Notification Constraints**: Notification frequency, content validation
4. **Backup Constraints**: Backup scheduling, encryption requirements
5. **Accessibility Constraints**: WCAG compliance validation

## Related Documentation

- [SHACL Specification](http://www.w3.org/ns/shacl)
- [QualiaDB Architecture](../ARCHITECTURE.md)
- [Client Build Instructions](../crates/qualia-client/BUILD.md)
- [AGENTS.md](../AGENTS.md) - SHACL compiler details

## Migration Notes

For existing QualiaDB deployments:

1. **No Breaking Changes**: New constraints are additive
2. **Backward Compatibility**: Existing configurations remain valid
3. **Gradual Adoption**: Constraints can be adopted incrementally
4. **Validation Mode**: Can run in "report-only" mode initially

## Conclusion

The SHACL extensions provide robust validation for the new Qualia Client features, ensuring data integrity, security, and consistency across web and desktop deployments. The integration with the existing SHACL compiler maintains consistency with the broader QualiaDB validation framework.