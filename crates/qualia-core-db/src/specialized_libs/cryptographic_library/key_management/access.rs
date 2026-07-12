// Part of the cryptographic_library::key_management module (split per CLAUDE.md
// §11 — pure code motion, no behaviour change).
//
// Key access control: per-key access policies, permission checks (with optional
// time/IP context), and the retention-bounded access audit log.
use super::*;

/// Key access control
pub struct KeyAccessControl {
    access_policies: HashMap<String, AccessPolicy>,
    authentication_methods: Vec<AuthenticationMethod>,
    audit_log: AccessAuditLog,
}

/// Access policies
#[derive(Debug, Clone)]
pub struct AccessPolicy {
    pub policy_id: String,
    pub key_id: String,
    pub allowed_operations: Vec<KeyOperation>,
    pub required_auth: Vec<AuthenticationMethod>,
    pub time_restrictions: TimeRestrictions,
    pub ip_restrictions: Vec<String>,
}

/// Key operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KeyOperation {
    Read,
    Write,
    Delete,
    Sign,
    Verify,
    Encrypt,
    Decrypt,
    Derive,
    Rotate,
    Export,
    Import,
}

/// Authentication methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthenticationMethod {
    Password,
    Biometric,
    HardwareToken,
    MultiFactor,
    Certificate,
    ZeroKnowledge,
}

/// Time restrictions
#[derive(Debug, Clone)]
pub struct TimeRestrictions {
    pub allowed_hours: Vec<u8>,
    pub allowed_days: Vec<u8>,
    pub start_date: Option<u64>,
    pub end_date: Option<u64>,
}

/// Access audit log
pub struct AccessAuditLog {
    entries: Vec<AccessLogEntry>,
    retention_policy: RetentionPolicy,
}

/// Access log entry
#[derive(Debug, Clone)]
pub struct AccessLogEntry {
    pub entry_id: String,
    pub timestamp: u64,
    pub key_id: String,
    pub operation: KeyOperation,
    pub user_id: String,
    pub ip_address: String,
    pub success: bool,
    pub error_message: Option<String>,
}

impl KeyAccessControl {
    pub fn new() -> Self {
        Self {
            access_policies: HashMap::new(),
            authentication_methods: vec![AuthenticationMethod::MultiFactor],
            audit_log: AccessAuditLog::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    /// Register an access policy for a key.
    pub fn add_policy(&mut self, policy: AccessPolicy) {
        self.access_policies
            .insert(policy.policy_id.clone(), policy);
    }

    /// Check whether a given operation is permitted on a key.
    /// Returns true if a policy exists that explicitly allows the operation.
    /// Deny-by-default: no matching policy means denial.
    pub fn check_permission(&self, key_id: &str, operation: KeyOperation) -> bool {
        self.access_policies
            .values()
            .any(|p| p.key_id == key_id && p.allowed_operations.contains(&operation))
    }

    /// Check permission with full context (time restrictions, IP).
    pub fn check_permission_with_context(
        &self,
        key_id: &str,
        operation: KeyOperation,
        current_hour: u8,
        current_day: u8,
        ip_address: &str,
    ) -> bool {
        self.access_policies.values().any(|p| {
            if p.key_id != key_id || !p.allowed_operations.contains(&operation) {
                return false;
            }
            // Check time restrictions
            if !p.time_restrictions.allowed_hours.is_empty()
                && !p.time_restrictions.allowed_hours.contains(&current_hour)
            {
                return false;
            }
            if !p.time_restrictions.allowed_days.is_empty()
                && !p.time_restrictions.allowed_days.contains(&current_day)
            {
                return false;
            }
            // Check IP restrictions
            if !p.ip_restrictions.is_empty() && !p.ip_restrictions.iter().any(|ip| ip == ip_address)
            {
                return false;
            }
            true
        })
    }

    /// Record an access attempt in the audit log.
    pub fn log_access(
        &mut self,
        key_id: &str,
        operation: KeyOperation,
        user_id: &str,
        success: bool,
    ) {
        self.audit_log
            .log_entry(key_id, operation, user_id, success);
    }

    /// Number of registered policies.
    pub fn policy_count(&self) -> usize {
        self.access_policies.len()
    }

    /// Get a reference to the audit log.
    pub fn audit_log(&self) -> &AccessAuditLog {
        &self.audit_log
    }

    /// Get the list of configured authentication methods.
    pub fn authentication_methods(&self) -> &[AuthenticationMethod] {
        &self.authentication_methods
    }

    /// Add an authentication method if not already present.
    pub fn add_authentication_method(&mut self, method: AuthenticationMethod) {
        if !self.authentication_methods.contains(&method) {
            self.authentication_methods.push(method);
        }
    }

    /// Check whether a given authentication method is supported.
    pub fn supports_authentication_method(&self, method: &AuthenticationMethod) -> bool {
        self.authentication_methods.contains(method)
    }
}

impl AccessAuditLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            retention_policy: RetentionPolicy {
                retention_days: 365,
                auto_delete: true,
                archive_before_delete: true,
            },
        }
    }

    /// Record a key access event. Called after every key read/write/sign/verify operation.
    pub fn log_entry(
        &mut self,
        key_id: &str,
        operation: KeyOperation,
        user_id: &str,
        success: bool,
    ) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let entry = AccessLogEntry {
            entry_id: format!("acc_{}_{}", timestamp, self.entries.len()),
            timestamp,
            key_id: key_id.to_string(),
            operation,
            user_id: user_id.to_string(),
            ip_address: String::new(),
            success,
            error_message: if success {
                None
            } else {
                Some("operation failed".to_string())
            },
        };
        self.entries.push(entry);
        // Enforce retention: drop entries older than retention_days
        let cutoff =
            timestamp.saturating_sub((self.retention_policy.retention_days as u64) * 86400);
        self.entries.retain(|e| e.timestamp >= cutoff);
    }

    /// Number of logged entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Iterate over entries (newest first).
    pub fn entries(&self) -> &[AccessLogEntry] {
        &self.entries
    }
}
