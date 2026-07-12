// Part of the cryptographic_library::key_management module (split per CLAUDE.md
// §11 — pure code motion, no behaviour change).
//
// Key rotation: per-algorithm rotation policies, the rotation schedule/queue,
// the retention-bounded rotation history, and the key-rotation operation.
use super::*;

/// Key rotator
pub struct KeyRotator {
    rotation_policies: HashMap<KeyAlgorithm, RotationPolicy>,
    rotation_schedule: RotationSchedule,
    rotation_history: RotationHistory,
}

/// Rotation policies
#[derive(Debug, Clone)]
pub struct RotationPolicy {
    pub policy_id: String,
    pub algorithm: KeyAlgorithm,
    pub rotation_interval: u64,
    pub grace_period: u64,
    pub automatic_rotation: bool,
    pub notification_settings: NotificationSettings,
}

/// Notification settings
#[derive(Debug, Clone)]
pub struct NotificationSettings {
    pub notify_before_rotation: bool,
    pub notification_days: u32,
    pub notification_channels: Vec<NotificationChannel>,
}

/// Notification channels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email,
    SMS,
    Webhook,
    Slack,
    Custom(String),
}

/// Rotation schedule
pub struct RotationSchedule {
    pub scheduled_rotations: Vec<ScheduledRotation>,
    pub rotation_queue: Vec<QueuedRotation>,
    pub completed_rotations: Vec<CompletedRotation>,
}

/// Scheduled rotation
#[derive(Debug, Clone)]
pub struct ScheduledRotation {
    pub rotation_id: String,
    pub key_id: String,
    pub scheduled_time: u64,
    pub rotation_type: RotationType,
}

/// Rotation types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RotationType {
    Automatic,
    Manual,
    Emergency,
    Compliance,
}

/// Queued rotation
#[derive(Debug, Clone)]
pub struct QueuedRotation {
    pub rotation_id: String,
    pub key_id: String,
    pub queued_at: u64,
    pub priority: RotationPriority,
}

/// Rotation priorities
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RotationPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Completed rotation
#[derive(Debug, Clone)]
pub struct CompletedRotation {
    pub rotation_id: String,
    pub key_id: String,
    pub old_key_id: String,
    pub new_key_id: String,
    pub completed_at: u64,
    pub success: bool,
}

/// Rotation history
pub struct RotationHistory {
    entries: Vec<RotationHistoryEntry>,
    retention_policy: RetentionPolicy,
}

/// Rotation history entry
#[derive(Debug, Clone)]
pub struct RotationHistoryEntry {
    pub entry_id: String,
    pub timestamp: u64,
    pub key_id: String,
    pub rotation_type: RotationType,
    pub success: bool,
    pub error_message: Option<String>,
}

impl KeyRotator {
    pub fn new() -> Self {
        Self {
            rotation_policies: HashMap::new(),
            rotation_schedule: RotationSchedule::new(),
            rotation_history: RotationHistory::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        // Initialize rotation policies
        self.rotation_policies.insert(
            KeyAlgorithm::MLDSA,
            RotationPolicy::new(KeyAlgorithm::MLDSA),
        );
        self.rotation_policies
            .insert(KeyAlgorithm::AES, RotationPolicy::new(KeyAlgorithm::AES));
        Ok(())
    }

    pub fn rotate_key(&mut self, old_key: &Key) -> Result<Key, CryptographicError> {
        let new_key_id = format!(
            "{}_rotated_{}",
            old_key.key_id,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        let mut new_key_data = rand::random::<[u8; 32]>().to_vec();
        new_key_data.resize(old_key.key_data.len(), 0);
        let new_key = Key {
            key_id: new_key_id.clone(),
            key_type: old_key.key_type.clone(),
            key_algorithm: old_key.key_algorithm.clone(),
            key_data: new_key_data,
            metadata: KeyMetadata {
                key_id: new_key_id,
                key_type: old_key.key_type.clone(),
                key_algorithm: old_key.key_algorithm.clone(),
                key_size: old_key.metadata.key_size,
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                expires_at: 0,
                last_used: 0,
                usage_count: 0,
                security_level: old_key.metadata.security_level.clone(),
                access_level: old_key.metadata.access_level.clone(),
            },
        };
        Ok(new_key)
    }

    /// Get a reference to the rotation schedule.
    pub fn rotation_schedule(&self) -> &RotationSchedule {
        &self.rotation_schedule
    }

    /// Get a mutable reference to the rotation schedule.
    pub fn rotation_schedule_mut(&mut self) -> &mut RotationSchedule {
        &mut self.rotation_schedule
    }

    /// Get a reference to the rotation history.
    pub fn rotation_history(&self) -> &RotationHistory {
        &self.rotation_history
    }

    /// Get a mutable reference to the rotation history.
    pub fn rotation_history_mut(&mut self) -> &mut RotationHistory {
        &mut self.rotation_history
    }
}

impl RotationPolicy {
    pub fn new(algorithm: KeyAlgorithm) -> Self {
        Self {
            policy_id: format!("rotation_policy_{:?}", algorithm),
            algorithm,
            rotation_interval: 86400 * 90, // 90 days
            grace_period: 86400 * 7,       // 7 days
            automatic_rotation: true,
            notification_settings: NotificationSettings {
                notify_before_rotation: true,
                notification_days: 7,
                notification_channels: vec![NotificationChannel::Email],
            },
        }
    }
}

impl NotificationSettings {
    pub fn new() -> Self {
        Self {
            notify_before_rotation: true,
            notification_days: 7,
            notification_channels: vec![NotificationChannel::Email],
        }
    }
}

impl RotationSchedule {
    pub fn new() -> Self {
        Self {
            scheduled_rotations: Vec::new(),
            rotation_queue: Vec::new(),
            completed_rotations: Vec::new(),
        }
    }
}

impl RotationHistory {
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

    /// Record a rotation in the history, enforcing retention.
    pub fn add_entry(&mut self, entry: RotationHistoryEntry) {
        let cutoff = entry
            .timestamp
            .saturating_sub((self.retention_policy.retention_days as u64) * 86400);
        self.entries.retain(|e| e.timestamp >= cutoff);
        self.entries.push(entry);
    }

    /// Number of recorded rotation history entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Iterate over history entries.
    pub fn entries(&self) -> &[RotationHistoryEntry] {
        &self.entries
    }

    /// Get the retention policy for rotation history.
    pub fn retention_policy(&self) -> &RetentionPolicy {
        &self.retention_policy
    }
}
