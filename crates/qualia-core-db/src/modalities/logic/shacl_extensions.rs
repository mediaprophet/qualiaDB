//! SHACL Extensions for Logging, System Tray, and Enhanced Settings
//!
//! This module extends the core SHACL compiler with constraints for the new
//! client-side functionality including comprehensive logging, system tray
//! integration, and enhanced configuration settings.

use crate::webizen::SlgOpcode;

// ── Logging System Constraints ─────────────────────────────────────────────

/// `q42:LogConfiguration` — validates logging system configuration
#[derive(Debug, Clone)]
pub struct LogConfiguration {
    pub max_memory_logs: u32,
    pub max_disk_logs: u32,
    pub flush_interval_ms: u32,
}

/// `q42:LogLevel` — validates log level enumeration
#[derive(Debug, Clone)]
pub struct LogLevel {
    pub allowed_levels: Vec<String>, // ["DEBUG", "INFO", "WARN", "ERROR", "CRITICAL"]
}

/// `q42:LogEntry` — validates individual log entry structure
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub require_timestamp: bool,
    pub require_context: bool,
    pub max_message_length: u32,
    pub allow_data_payload: bool,
}

/// `q42:LogRetention` — validates log retention policies
#[derive(Debug, Clone)]
pub struct LogRetention {
    pub max_age_days: u32,
    pub max_size_mb: u32,
    pub backup_before_cleanup: bool,
}

/// `q42:LogExportFormat` — validates log export format constraints
#[derive(Debug, Clone)]
pub struct LogExportFormat {
    pub allowed_formats: Vec<String>, // ["json", "txt", "csv"]
    pub max_export_size_mb: u32,
}

// ── System Tray Constraints ─────────────────────────────────────────────────

/// `q42:SystemTrayConfiguration` — validates system tray menu configuration
#[derive(Debug, Clone)]
pub struct SystemTrayConfiguration {
    pub max_menu_items: u8,
    pub require_separator_logic: bool,
    pub allow_nested_menus: bool,
}

/// `q42:TrayMenuItem` — validates individual system tray menu items
#[derive(Debug, Clone)]
pub struct TrayMenuItem {
    pub require_label: bool,
    pub require_action: bool,
    pub max_label_length: u32,
    pub allow_icons: bool,
}

/// `q42:TrayStatusIndicator` — validates system tray status indicators
#[derive(Debug, Clone)]
pub struct TrayStatusIndicator {
    pub allow_live_updates: bool,
    pub update_interval_ms: u32,
    pub max_status_length: u32,
}

/// `q42:TrayAction` — validates system tray action handlers
#[derive(Debug, Clone)]
pub struct TrayAction {
    pub require_window_context: bool,
    pub allow_async_actions: bool,
    pub timeout_ms: u32,
}

// ── Enhanced Settings Constraints ─────────────────────────────────────────────

/// `q42:StorageConfiguration` — validates storage path and quota settings
#[derive(Debug, Clone)]
pub struct StorageConfiguration {
    pub min_quota_gb: u32,
    pub max_quota_gb: u32,
    pub require_absolute_path: bool,
    pub allowed_path_patterns: Vec<String>,
}

/// `q42:NetworkConfiguration` — validates daemon network settings
#[derive(Debug, Clone)]
pub struct NetworkConfiguration {
    pub allowed_ports: Vec<u16>,
    pub require_port_conflict_check: bool,
    pub default_host: String,
}

/// `q42:TaxRecipientConfiguration` — validates ILP tax recipient configuration
#[derive(Debug, Clone)]
pub struct TaxRecipientConfiguration {
    pub require_ilp_address: bool,
    pub min_share_percent: u8,
    pub max_share_percent: u8,
    pub allow_nym_routing: bool,
    pub total_share_validation: bool,
}

/// `q42:SecurityConfiguration` — validates security-related settings
#[derive(Debug, Clone)]
pub struct SecurityConfiguration {
    pub require_encryption: bool,
    pub allowed_cipher_suites: Vec<String>,
    pub min_key_length_bits: u16,
}

// ── Opcode Generation for New Constraints ─────────────────────────────────────

impl LogConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_memory_logs as f64),
            SlgOpcode::CheckMaxInclusive(self.max_disk_logs as f64),
            SlgOpcode::CheckMinInclusive(self.flush_interval_ms as f64),
        ]
    }
}

impl LogLevel {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        // Validate that log level is in allowed set
        self.allowed_levels.iter().map(|level| {
            SlgOpcode::CheckHasValue(crate::q_hash(level))
        }).collect()
    }
}

impl LogEntry {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        let mut opcodes = Vec::new();
        if self.require_timestamp {
            opcodes.push(SlgOpcode::CheckMinCount(1));
        }
        if self.require_context {
            opcodes.push(SlgOpcode::CheckMinCount(1));
        }
        if self.max_message_length > 0 {
            opcodes.push(SlgOpcode::CheckMaxLength(self.max_message_length));
        }
        if !self.allow_data_payload {
            opcodes.push(SlgOpcode::CheckMaxCount(3)); // timestamp, level, message only
        }
        opcodes
    }
}

impl LogRetention {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_age_days as f64),
            SlgOpcode::CheckMaxInclusive(self.max_size_mb as f64),
        ]
    }
}

impl LogExportFormat {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        let mut opcodes = vec![
            SlgOpcode::CheckMaxInclusive(self.max_export_size_mb as f64),
        ];
        // Add format validation
        for format in &self.allowed_formats {
            opcodes.push(SlgOpcode::CheckHasValue(crate::q_hash(format)));
        }
        opcodes
    }
}

impl SystemTrayConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_menu_items as f64),
        ]
    }
}

impl TrayMenuItem {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        let mut opcodes = Vec::new();
        if self.require_label {
            opcodes.push(SlgOpcode::CheckMinCount(1));
        }
        if self.require_action {
            opcodes.push(SlgOpcode::CheckMinCount(1));
        }
        if self.max_label_length > 0 {
            opcodes.push(SlgOpcode::CheckMaxLength(self.max_label_length));
        }
        opcodes
    }
}

impl TrayStatusIndicator {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxLength(self.max_status_length),
            SlgOpcode::CheckMinInclusive(self.update_interval_ms as f64),
        ]
    }
}

impl TrayAction {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.timeout_ms as f64),
        ]
    }
}

impl StorageConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        let mut opcodes = vec![
            SlgOpcode::CheckMinInclusive(self.min_quota_gb as f64),
            SlgOpcode::CheckMaxInclusive(self.max_quota_gb as f64),
        ];
        if self.require_absolute_path {
            opcodes.push(SlgOpcode::CheckMinCount(1));
        }
        opcodes
    }
}

impl NetworkConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        let mut opcodes = Vec::new();
        for port in &self.allowed_ports {
            opcodes.push(SlgOpcode::CheckHasValue(*port as u64));
        }
        opcodes
    }
}

impl TaxRecipientConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMinInclusive(self.min_share_percent as f64),
            SlgOpcode::CheckMaxInclusive(self.max_share_percent as f64),
        ]
    }
}

impl SecurityConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMinInclusive(self.min_key_length_bits as f64),
        ]
    }
}

// ── SHACL TTL Vocabulary Extensions ─────────────────────────────────────────────

/// Returns the SHACL TTL vocabulary extensions for the new constraints
pub fn get_shacl_extensions_ttl() -> &'static str {
    r#"
@prefix q42: <https://qualia.network/q42#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

# ── Logging System Constraints ─────────────────────────────────────────────

q42:LogConfiguration a sh:NodeShape ;
    sh:property [
        sh:path q42:maxMemoryLogs ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 100000 ;
        sh:message "Maximum memory logs must be between 1 and 100000" ;
    ] ;
    sh:property [
        sh:path q42:maxDiskLogs ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 1000000 ;
        sh:message "Maximum disk logs must be between 1 and 1000000" ;
    ] ;
    sh:property [
        sh:path q42:flushIntervalMs ;
        sh:datatype xsd:integer ;
        sh:minInclusive 100 ;
        sh:maxInclusive 60000 ;
        sh:message "Flush interval must be between 100ms and 60000ms" ;
    ] .

q42:LogLevel a sh:NodeShape ;
    sh:property [
        sh:path q42:allowedLevels ;
        sh:in ("DEBUG" "INFO" "WARN" "ERROR" "CRITICAL") ;
        sh:message "Log level must be one of: DEBUG, INFO, WARN, ERROR, CRITICAL" ;
    ] .

q42:LogEntry a sh:NodeShape ;
    sh:property [
        sh:path q42:timestamp ;
        sh:datatype xsd:dateTime ;
        sh:minCount 1 ;
        sh:message "Log entry must have a timestamp" ;
    ] ;
    sh:property [
        sh:path q42:level ;
        sh:datatype xsd:string ;
        sh:minCount 1 ;
        sh:message "Log entry must have a level" ;
    ] ;
    sh:property [
        sh:path q42:message ;
        sh:datatype xsd:string ;
        sh:maxLength 10000 ;
        sh:message "Log message must not exceed 10000 characters" ;
    ] .

q42:LogRetention a sh:NodeShape ;
    sh:property [
        sh:path q42:maxAgeDays ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 365 ;
        sh:message "Log retention age must be between 1 and 365 days" ;
    ] ;
    sh:property [
        sh:path q42:maxSizeMb ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 10240 ;
        sh:message "Log retention size must be between 1MB and 10GB" ;
    ] .

# ── System Tray Constraints ─────────────────────────────────────────────────

q42:SystemTrayConfiguration a sh:NodeShape ;
    sh:property [
        sh:path q42:maxMenuItems ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 20 ;
        sh:message "System tray can have at most 20 menu items" ;
    ] .

q42:TrayMenuItem a sh:NodeShape ;
    sh:property [
        sh:path q42:label ;
        sh:datatype xsd:string ;
        sh:minCount 1 ;
        sh:maxLength 50 ;
        sh:message "Tray menu item must have a label (max 50 characters)" ;
    ] ;
    sh:property [
        sh:path q42:action ;
        sh:datatype xsd:string ;
        sh:minCount 1 ;
        sh:message "Tray menu item must have an action" ;
    ] .

q42:TrayStatusIndicator a sh:NodeShape ;
    sh:property [
        sh:path q42:statusText ;
        sh:datatype xsd:string ;
        sh:maxLength 100 ;
        sh:message "Status text must not exceed 100 characters" ;
    ] ;
    sh:property [
        sh:path q42:updateIntervalMs ;
        sh:datatype xsd:integer ;
        sh:minInclusive 100 ;
        sh:maxInclusive 10000 ;
        sh:message "Update interval must be between 100ms and 10000ms" ;
    ] .

# ── Enhanced Settings Constraints ─────────────────────────────────────────────

q42:StorageConfiguration a sh:NodeShape ;
    sh:property [
        sh:path q42:storageQuotaGb ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 1000 ;
        sh:message "Storage quota must be between 1GB and 1000GB" ;
    ] ;
    sh:property [
        sh:path q42:storagePath ;
        sh:datatype xsd:string ;
        sh:minCount 1 ;
        sh:pattern "^/" ;
        sh:message "Storage path must be an absolute path" ;
    ] .

q42:NetworkConfiguration a sh:NodeShape ;
    sh:property [
        sh:path q42:daemonPort ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1024 ;
        sh:maxInclusive 65535 ;
        sh:message "Daemon port must be between 1024 and 65535" ;
    ] ;
    sh:property [
        sh:path q42:daemonHost ;
        sh:datatype xsd:string ;
        sh:pattern "^(localhost|127\\.0\\.0\\.1|0\\.0\\.0\\.0)$" ;
        sh:message "Daemon host must be localhost, 127.0.0.1, or 0.0.0.0" ;
    ] .

q42:TaxRecipientConfiguration a sh:NodeShape ;
    sh:property [
        sh:path q42:sharePercent ;
        sh:datatype xsd:integer ;
        sh:minInclusive 0 ;
        sh:maxInclusive 100 ;
        sh:message "Share percent must be between 0 and 100" ;
    ] ;
    sh:property [
        sh:path q42:ilpAddress ;
        sh:datatype xsd:string ;
        sh:minCount 1 ;
        sh:pattern "^\\$ilp\\." ;
        sh:message "ILP address must start with $ilp." ;
    ] .

q42:SecurityConfiguration a sh:NodeShape ;
    sh:property [
        sh:path q42:minKeyLengthBits ;
        sh:datatype xsd:integer ;
        sh:minInclusive 128 ;
        sh:maxInclusive 4096 ;
        sh:message "Key length must be between 128 and 4096 bits" ;
    ] ;
    sh:property [
        sh:path q42:requireEncryption ;
        sh:datatype xsd:boolean ;
        sh:message "Require encryption must be a boolean" ;
    ] .
"#
}
