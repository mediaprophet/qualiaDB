// Part of the cryptographic_library::key_management module (split per CLAUDE.md
// §11 — pure code motion, no behaviour change).
//
// The top-level key manager and the zone-backed key storage: generate / store /
// get / list keys and enforce access control on retrieval.
use super::*;

/// Key manager for secure key storage and management
pub struct KeyManager {
    pub(in crate::specialized_libs::cryptographic_library) key_storage: KeyStorage,
    pub(in crate::specialized_libs::cryptographic_library) key_generator: KeyGenerator,
    key_rotator: KeyRotator,
    key_recovery: KeyRecovery,
}

/// Key storage using ZNS for secure key storage
pub struct KeyStorage {
    zones: HashMap<String, KeyZone>,
    pub(in crate::specialized_libs::cryptographic_library) key_catalog: KeyCatalog,
    encryption_at_rest: EncryptionAtRest,
    pub(in crate::specialized_libs::cryptographic_library) access_control: KeyAccessControl,
    key_data: HashMap<String, Key>,
}

impl KeyManager {
    pub fn new() -> Self {
        Self {
            key_storage: KeyStorage::new(),
            key_generator: KeyGenerator::new(),
            key_rotator: KeyRotator::new(),
            key_recovery: KeyRecovery::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.key_storage.initialize()?;
        self.key_generator.initialize()?;
        self.key_rotator.initialize()?;
        self.key_recovery.initialize()?;
        Ok(())
    }

    pub fn generate_key_pair(
        &mut self,
        key_id: String,
        _key_type: KeyType,
        algorithm: KeyAlgorithm,
        security_level: SecurityLevel,
    ) -> Result<(Key, Key), CryptographicError> {
        // Generate private key
        let private_key = self.key_generator.generate_key(
            format!("{}_private", key_id),
            KeyType::Private,
            algorithm,
            security_level,
        )?;

        // Generate public key from private key
        let public_key = self
            .key_generator
            .derive_public_key(&private_key, format!("{}_public", key_id))?;

        Ok((private_key, public_key))
    }

    pub fn store_key(&mut self, key: Key) -> Result<(), CryptographicError> {
        self.key_storage.store_key(key)
    }

    pub fn get_key(&self, key_id: &str) -> Result<Key, CryptographicError> {
        self.key_storage.get_key(key_id)
    }

    pub fn rotate_key(&mut self, old_key: &Key) -> Result<Key, CryptographicError> {
        self.key_rotator.rotate_key(old_key)
    }

    pub fn list_keys(&self) -> Vec<String> {
        self.key_storage.list_keys()
    }

    pub fn get_key_metadata(&self, key_id: &str) -> Option<KeyMetadata> {
        self.key_storage.get_key_metadata(key_id)
    }
}

impl KeyStorage {
    pub fn new() -> Self {
        Self {
            zones: HashMap::new(),
            key_catalog: KeyCatalog::new(),
            encryption_at_rest: EncryptionAtRest::new(),
            access_control: KeyAccessControl::new(),
            key_data: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.create_zones()?;
        self.key_catalog.initialize()?;
        self.encryption_at_rest.initialize()?;
        self.access_control.initialize()?;
        Ok(())
    }

    fn create_zones(&mut self) -> Result<(), CryptographicError> {
        let zones = vec![
            ("mldsa", KeyZoneType::MLDSA),
            ("traditional", KeyZoneType::Traditional),
            ("symmetric", KeyZoneType::Symmetric),
            ("keyexchange", KeyZoneType::KeyExchange),
            ("session", KeyZoneType::Session),
            ("backup", KeyZoneType::Backup),
            ("hsm", KeyZoneType::HSM),
        ];

        for (name, zone_type) in zones {
            let zone = KeyZone {
                zone_id: name.to_string(),
                zone_type,
                capacity: 1024 * 1024 * 1024, // 1GB
                keys: HashMap::new(),
                access_pattern: AccessPattern::Frequent,
            };
            self.zones.insert(name.to_string(), zone);
        }

        Ok(())
    }

    pub fn store_key(&mut self, key: Key) -> Result<(), CryptographicError> {
        // Determine best zone for this key
        let zone_id = self.select_best_zone(&key)?;

        // Store in zone
        let zone = self
            .zones
            .get_mut(&zone_id)
            .ok_or_else(|| CryptographicError::StorageError("Zone not found".to_string()))?;

        zone.keys.insert(key.key_id.clone(), key.metadata.clone());

        // Register in catalog
        self.key_catalog.register_key(key.metadata.clone());

        // Store actual key data
        self.key_data.insert(key.key_id.clone(), key);

        Ok(())
    }

    pub fn get_key(&self, key_id: &str) -> Result<Key, CryptographicError> {
        self.key_data
            .get(key_id)
            .cloned()
            .ok_or_else(|| CryptographicError::StorageError(format!("Key not found: {}", key_id)))
    }

    /// Get a key with access control enforcement. Returns an error if the
    /// operation is not permitted by any registered access policy.
    /// Deny-by-default when policies exist but none match.
    pub fn get_key_with_access(
        &mut self,
        key_id: &str,
        operation: KeyOperation,
        user_id: &str,
    ) -> Result<Key, CryptographicError> {
        // If policies are registered, enforce them
        if self.access_control.policy_count() > 0 {
            if !self
                .access_control
                .check_permission(key_id, operation.clone())
            {
                self.access_control
                    .log_access(key_id, operation.clone(), user_id, false);
                return Err(CryptographicError::AccessDenied(format!(
                    "Access denied for operation {:?} on key {}",
                    operation, key_id
                )));
            }
        }
        let key = self.get_key(key_id)?;
        self.access_control
            .log_access(key_id, operation, user_id, true);
        Ok(key)
    }

    pub fn get_key_metadata(&self, key_id: &str) -> Option<KeyMetadata> {
        for zone in self.zones.values() {
            if let Some(metadata) = zone.keys.get(key_id) {
                return Some(metadata.clone());
            }
        }
        None
    }

    pub fn list_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        for zone in self.zones.values() {
            keys.extend(zone.keys.keys().cloned());
        }
        keys
    }

    fn select_best_zone(&self, key: &Key) -> Result<String, CryptographicError> {
        // Simple selection logic - in real implementation would be more sophisticated
        match key.key_algorithm {
            KeyAlgorithm::MLDSA => Ok("mldsa".to_string()),
            KeyAlgorithm::AES | KeyAlgorithm::ChaCha20 => Ok("symmetric".to_string()),
            _ => Ok("traditional".to_string()),
        }
    }
}
