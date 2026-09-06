//! Cold, bounded in-memory installed-app registry.

use super::compat::{engine_too_new, engine_too_old};
use super::error::AppRegistryError;
use super::fixture::{poet_fixture_manifest, POET_APP_ID};
use super::probe::probe_identity;
use super::record::{
    AppInspectView, AppRecordState, InstalledAppRecord, PermissionIntentSummary,
};
use crate::q42::app_manifest::{PermissionIntent, PortableAppManifest};

/// Maximum installed / quarantined slots retained in memory.
pub const MAX_INSTALLED_APPS: usize = 32;

/// Default engine version string used for compatibility checks.
///
/// Keep in sync with the workspace release line when bumping; comparison is
/// the simple dotted numeric helper in [`super::compat`].
pub const DEFAULT_ENGINE_VERSION: &str = "0.0.36";

/// Internal slot payload (record + inspect snapshot).
#[derive(Debug, Clone)]
struct Slot {
    record: InstalledAppRecord,
    author_name: String,
    author_did: String,
    permission_intents: Vec<PermissionIntent>,
    entry_ids: Vec<String>,
    min_engine_version: String,
    max_engine_version: String,
    required_features: Vec<String>,
}

/// Bounded installed-app registry (cold construction / inspection only).
///
/// # Security
///
/// - Does **not** execute, launch, or load application code.
/// - `inspect` / `get` / `list` are read-only and do not mutate state.
/// - Malformed packages are quarantined when identity is recoverable; otherwise rejected.
pub struct AppRegistry {
    slots: Vec<Option<Slot>>,
    engine_version: String,
}

impl Default for AppRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AppRegistry {
    /// Empty registry with [`DEFAULT_ENGINE_VERSION`].
    pub fn new() -> Self {
        Self::with_engine_version(DEFAULT_ENGINE_VERSION)
    }

    /// Empty registry with an explicit engine version for compatibility checks.
    pub fn with_engine_version(engine_version: impl Into<String>) -> Self {
        Self {
            slots: (0..MAX_INSTALLED_APPS).map(|_| None).collect(),
            engine_version: engine_version.into(),
        }
    }

    /// Seed a registry that already contains the bundled POET fixture as Active.
    ///
    /// Fixture is in-memory only — not a claim that package bytes exist on disk.
    pub fn with_bundled_poet_fixture() -> Self {
        let mut reg = Self::new();
        let manifest = poet_fixture_manifest();
        reg.register_manifest(&manifest)
            .expect("poet fixture must register");
        debug_assert_eq!(reg.get(POET_APP_ID).map(|r| r.app_id.as_str()), Some(POET_APP_ID));
        reg
    }

    pub fn engine_version(&self) -> &str {
        &self.engine_version
    }

    pub fn capacity(&self) -> usize {
        MAX_INSTALLED_APPS
    }

    pub fn len(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Register a decoded/validated manifest (no external package digest check).
    ///
    /// Integrity is treated as ok unless compatibility marks the slot incompatible.
    pub fn register_manifest(
        &mut self,
        manifest: &PortableAppManifest,
    ) -> Result<&InstalledAppRecord, AppRegistryError> {
        manifest.validate()?;
        let integrity_ok = true;
        let state = self.admission_state(manifest, integrity_ok, None);
        self.insert_from_manifest(manifest, integrity_ok, state)
    }

    /// Decode bytes, optionally verify `package_digest` against the integrity block.
    ///
    /// - Decode/validate failure → quarantine when identity recoverable, else reject.
    /// - Digest mismatch → quarantine (identity from decoded or probed fields).
    pub fn register_manifest_bytes(
        &mut self,
        bytes: &[u8],
        package_digest: Option<&[u8; 32]>,
    ) -> Result<&InstalledAppRecord, AppRegistryError> {
        match PortableAppManifest::decode(bytes) {
            Ok(manifest) => {
                let mut integrity_ok = true;
                let mut quarantine_reason: Option<String> = None;
                if let Some(digest) = package_digest {
                    if manifest.verify_package_digest(digest).is_err() {
                        integrity_ok = false;
                        quarantine_reason =
                            Some("package digest mismatch vs Integrity.package_sha256".into());
                    }
                }
                let state = self.admission_state(&manifest, integrity_ok, quarantine_reason);
                self.insert_from_manifest(&manifest, integrity_ok, state)
            }
            Err(err) => self.quarantine_or_reject(bytes, &err.to_string()),
        }
    }

    /// Read-only list of records in slot order (occupied slots only).
    pub fn list(&self) -> Vec<&InstalledAppRecord> {
        self.slots
            .iter()
            .filter_map(|s| s.as_ref().map(|slot| &slot.record))
            .collect()
    }

    /// Look up by `app_id` (exact match).
    pub fn get(&self, app_id: &str) -> Option<&InstalledAppRecord> {
        self.find_slot(app_id).map(|i| &self.slots[i].as_ref().unwrap().record)
    }

    /// Read-only inspect view. Does not mutate the registry and does not launch.
    pub fn inspect(&self, app_id: &str) -> Option<AppInspectView> {
        let idx = self.find_slot(app_id)?;
        let slot = self.slots[idx].as_ref()?;
        Some(AppInspectView {
            app_id: slot.record.app_id.clone(),
            version: slot.record.version.clone(),
            author_name: slot.author_name.clone(),
            author_did: slot.author_did.clone(),
            digest: slot.record.digest,
            integrity_ok: slot.record.integrity_ok,
            state: slot.record.state.clone(),
            permission_intents: slot.permission_intents.clone(),
            permission_summary: slot.record.permission_intents.clone(),
            entry_ids: slot.entry_ids.clone(),
            min_engine_version: slot.min_engine_version.clone(),
            max_engine_version: slot.max_engine_version.clone(),
            required_features: slot.required_features.clone(),
        })
    }

    fn admission_state(
        &self,
        manifest: &PortableAppManifest,
        integrity_ok: bool,
        quarantine_reason: Option<String>,
    ) -> AppRecordState {
        if let Some(reason) = quarantine_reason {
            return AppRecordState::Quarantined { reason };
        }
        if !integrity_ok {
            return AppRecordState::Quarantined {
                reason: "integrity check failed".into(),
            };
        }
        let min = &manifest.compatibility.min_engine_version;
        let max = &manifest.compatibility.max_engine_version;
        if engine_too_old(min, &self.engine_version) || engine_too_new(max, &self.engine_version)
        {
            return AppRecordState::Incompatible;
        }
        AppRecordState::Active
    }

    fn insert_from_manifest(
        &mut self,
        manifest: &PortableAppManifest,
        integrity_ok: bool,
        state: AppRecordState,
    ) -> Result<&InstalledAppRecord, AppRegistryError> {
        let app_id = manifest.identity.app_id.trim();
        if app_id.is_empty() {
            return Err(AppRegistryError::MissingAppId);
        }
        let record = InstalledAppRecord {
            app_id: manifest.identity.app_id.clone(),
            version: manifest.identity.version.clone(),
            digest: manifest.integrity.package_sha256,
            permission_intents: PermissionIntentSummary::from_intents(&manifest.permission_intents),
            integrity_ok,
            state,
        };
        let slot = Slot {
            record,
            author_name: manifest.identity.author.name.clone(),
            author_did: manifest.identity.author.did.clone(),
            permission_intents: manifest.permission_intents.clone(),
            entry_ids: manifest
                .entries
                .iter()
                .map(|e| e.entry_id.clone())
                .collect(),
            min_engine_version: manifest.compatibility.min_engine_version.clone(),
            max_engine_version: manifest.compatibility.max_engine_version.clone(),
            required_features: manifest.compatibility.required_features.clone(),
        };
        self.upsert(slot)
    }

    fn quarantine_or_reject(
        &mut self,
        bytes: &[u8],
        reason: &str,
    ) -> Result<&InstalledAppRecord, AppRegistryError> {
        let Some(recovered) = probe_identity(bytes) else {
            return Err(AppRegistryError::RejectedMalformed);
        };
        let record = InstalledAppRecord {
            app_id: recovered.app_id.clone(),
            version: recovered.version.clone(),
            digest: [0u8; 32],
            permission_intents: PermissionIntentSummary::default(),
            integrity_ok: false,
            state: AppRecordState::Quarantined {
                reason: reason.to_string(),
            },
        };
        let slot = Slot {
            record,
            author_name: String::new(),
            author_did: String::new(),
            permission_intents: Vec::new(),
            entry_ids: Vec::new(),
            min_engine_version: String::new(),
            max_engine_version: String::new(),
            required_features: Vec::new(),
        };
        self.upsert(slot)
    }

    fn upsert(&mut self, slot: Slot) -> Result<&InstalledAppRecord, AppRegistryError> {
        if let Some(idx) = self.find_slot(&slot.record.app_id) {
            self.slots[idx] = Some(slot);
            return Ok(&self.slots[idx].as_ref().unwrap().record);
        }
        let free = self
            .slots
            .iter()
            .position(|s| s.is_none())
            .ok_or(AppRegistryError::RegistryFull)?;
        self.slots[free] = Some(slot);
        Ok(&self.slots[free].as_ref().unwrap().record)
    }

    fn find_slot(&self, app_id: &str) -> Option<usize> {
        self.slots.iter().position(|s| {
            s.as_ref()
                .map(|slot| slot.record.app_id == app_id)
                .unwrap_or(false)
        })
    }
}
