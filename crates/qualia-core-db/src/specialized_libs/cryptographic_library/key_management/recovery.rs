// Part of the cryptographic_library::key_management module (split per CLAUDE.md
// §11 — pure code motion, no behaviour change).
//
// Key recovery: configured recovery methods, recovery policies (Shamir shares /
// thresholds / time-lock), and recovery-attempt tracking with lockout.
use super::*;

/// Key recovery
pub struct KeyRecovery {
    recovery_methods: Vec<RecoveryMethod>,
    recovery_policies: RecoveryPolicies,
    recovery_attempts: RecoveryAttempts,
}

/// Recovery methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecoveryMethod {
    ShamirSecretSharing,
    EncryptedBackup,
    HardwareToken,
    BiometricRecovery,
    SocialRecovery,
    CloudBackup,
}

/// Recovery policies
pub struct RecoveryPolicies {
    pub minimum_shares: usize,
    pub total_shares: usize,
    pub recovery_threshold: f64,
    pub time_lock: u64,
    pub geo_restrictions: Vec<String>,
}

/// Recovery attempts
pub struct RecoveryAttempts {
    pub attempts: Vec<RecoveryAttempt>,
    pub lockout_policy: LockoutPolicy,
}

/// Recovery attempt
#[derive(Debug, Clone)]
pub struct RecoveryAttempt {
    pub attempt_id: String,
    pub timestamp: u64,
    pub key_id: String,
    pub method: RecoveryMethod,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Lockout policy
pub struct LockoutPolicy {
    pub max_attempts: u32,
    pub lockout_duration: u64,
    pub exponential_backoff: bool,
}

impl KeyRecovery {
    pub fn new() -> Self {
        Self {
            recovery_methods: vec![
                RecoveryMethod::ShamirSecretSharing,
                RecoveryMethod::EncryptedBackup,
            ],
            recovery_policies: RecoveryPolicies {
                minimum_shares: 3,
                total_shares: 5,
                recovery_threshold: 0.6,
                time_lock: 86400, // 24 hours
                geo_restrictions: Vec::new(),
            },
            recovery_attempts: RecoveryAttempts {
                attempts: Vec::new(),
                lockout_policy: LockoutPolicy {
                    max_attempts: 3,
                    lockout_duration: 3600, // 1 hour
                    exponential_backoff: true,
                },
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    /// Get the list of configured recovery methods.
    pub fn recovery_methods(&self) -> &[RecoveryMethod] {
        &self.recovery_methods
    }

    /// Add a recovery method if not already present.
    pub fn add_recovery_method(&mut self, method: RecoveryMethod) {
        if !self.recovery_methods.contains(&method) {
            self.recovery_methods.push(method);
        }
    }

    /// Get the recovery policies.
    pub fn recovery_policies(&self) -> &RecoveryPolicies {
        &self.recovery_policies
    }

    /// Get a mutable reference to the recovery policies.
    pub fn recovery_policies_mut(&mut self) -> &mut RecoveryPolicies {
        &mut self.recovery_policies
    }

    /// Get a reference to the recovery attempts.
    pub fn recovery_attempts(&self) -> &RecoveryAttempts {
        &self.recovery_attempts
    }

    /// Get a mutable reference to the recovery attempts.
    pub fn recovery_attempts_mut(&mut self) -> &mut RecoveryAttempts {
        &mut self.recovery_attempts
    }
}

impl RecoveryAttempts {
    pub fn new() -> Self {
        Self {
            attempts: Vec::new(),
            lockout_policy: LockoutPolicy {
                max_attempts: 3,
                lockout_duration: 3600,
                exponential_backoff: true,
            },
        }
    }
}

impl LockoutPolicy {
    pub fn new() -> Self {
        Self {
            max_attempts: 3,
            lockout_duration: 3600,
            exponential_backoff: true,
        }
    }
}
