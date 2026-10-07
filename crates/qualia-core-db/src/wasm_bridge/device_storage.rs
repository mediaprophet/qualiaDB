//! Device storage + backup-folder policy for Web Civics / mobile installs.
//!
//! Primary durable store on browser/PWA/mobile WebView is OPFS (see
//! [`crate::wasm_storage`]). A **user-chosen backup folder** (File System Access
//! API on Chromium/Android WebView, or a host-provided Documents path on
//! Flutter/Tauri) holds export snapshots so OPFS wipe / site-data clear is
//! recoverable.
//!
//! This module is **policy + receipt verification** (no Window dependency).
//! Host adapters (`docs/js/webcivics-device-storage.js`) perform the actual
//! pickers, OPFS I/O, and IndexedDB handle persistence.

use serde::{Deserialize, Serialize};

/// OPFS directory name under `navigator.storage.getDirectory()`.
pub const WEBCIVICS_OPFS_DIR: &str = "webcivics";
/// Manifest file inside the OPFS vault.
pub const VAULT_MANIFEST_NAME: &str = "vault-manifest.v1.json";
/// IndexedDB database id used by the JS host to remember the backup folder handle.
pub const BACKUP_IDB_NAME: &str = "webcivics-device-storage-v1";
pub const BACKUP_IDB_STORE: &str = "handles";
pub const BACKUP_IDB_KEY: &str = "backup_dir_handle";
/// Suggested subdirectory name inside the user-picked backup folder.
pub const BACKUP_SUBDIR: &str = "webcivics-backups";
/// Minimum free bytes before the host should refuse a large ingest.
pub const MIN_PRIMARY_HEADROOM_BYTES: u64 = 8 * 1024 * 1024;
/// Soft target: keep at least this many dated backup snapshots.
pub const DEFAULT_BACKUP_RETENTION: u32 = 5;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStoragePolicy {
    pub profile: &'static str,
    pub engine_version: &'static str,
    pub primary: PrimaryStorePolicy,
    pub backup: BackupStorePolicy,
    pub recovery: RecoveryPolicy,
    pub capabilities: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PrimaryStorePolicy {
    pub kind: &'static str,
    pub opfs_directory: &'static str,
    pub manifest_name: &'static str,
    pub persist_permission: bool,
    pub block_size_bytes: u32,
    pub min_headroom_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackupStorePolicy {
    pub kind: &'static str,
    pub idb_name: &'static str,
    pub idb_store: &'static str,
    pub idb_key: &'static str,
    pub suggested_subdir: &'static str,
    pub retention_count: u32,
    pub formats: Vec<&'static str>,
    /// Hosts without `showDirectoryPicker` (many iOS PWAs) must use download/share.
    pub fallback_when_no_picker: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryPolicy {
    pub restore_requires_manifest: bool,
    pub verify_content_hash: bool,
    pub primary_wipe_does_not_erase_backup: bool,
}

/// Canonical policy document for Civics / mobile hosts.
pub fn device_storage_policy() -> DeviceStoragePolicy {
    DeviceStoragePolicy {
        profile: "webcivics-device-storage",
        engine_version: "0.0.39",
        primary: PrimaryStorePolicy {
            kind: "opfs",
            opfs_directory: WEBCIVICS_OPFS_DIR,
            manifest_name: VAULT_MANIFEST_NAME,
            persist_permission: true,
            block_size_bytes: 40_960,
            min_headroom_bytes: MIN_PRIMARY_HEADROOM_BYTES,
        },
        backup: BackupStorePolicy {
            kind: "user-directory-or-host-path",
            idb_name: BACKUP_IDB_NAME,
            idb_store: BACKUP_IDB_STORE,
            idb_key: BACKUP_IDB_KEY,
            suggested_subdir: BACKUP_SUBDIR,
            retention_count: DEFAULT_BACKUP_RETENTION,
            formats: vec![
                "application/vnd.web-civics.vault-backup+json",
                "application/ld+json",
                "text/turtle",
            ],
            fallback_when_no_picker: "download-blob-or-share-sheet",
        },
        recovery: RecoveryPolicy {
            restore_requires_manifest: true,
            verify_content_hash: true,
            primary_wipe_does_not_erase_backup: true,
        },
        capabilities: vec![
            "device-storage-policy",
            "opfs-primary-vault",
            "backup-folder-link",
            "backup-export-restore",
        ],
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StoragePlanInput {
    pub quota_bytes: u64,
    pub usage_bytes: u64,
    pub backup_folder_linked: bool,
    pub last_backup_unix: Option<u64>,
    pub now_unix: u64,
    /// Seconds after which a backup is considered stale (default 7 days).
    #[serde(default = "default_stale_secs")]
    pub stale_after_secs: u64,
}

fn default_stale_secs() -> u64 {
    7 * 24 * 3600
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoragePlan {
    pub available_bytes: u64,
    pub primary_ok: bool,
    pub recommend_persist: bool,
    pub recommend_link_backup_folder: bool,
    pub recommend_run_backup: bool,
    pub warnings: Vec<&'static str>,
    pub engine_version: &'static str,
}

/// Decide whether the host should persist, link a backup folder, or run a backup.
pub fn plan_device_storage(input: &StoragePlanInput) -> StoragePlan {
    let available = input.quota_bytes.saturating_sub(input.usage_bytes);
    let mut warnings = Vec::new();
    let primary_ok = available >= MIN_PRIMARY_HEADROOM_BYTES;
    if !primary_ok {
        warnings.push("primary_headroom_low");
    }
    let recommend_link = !input.backup_folder_linked;
    if recommend_link {
        warnings.push("backup_folder_not_linked");
    }
    let stale = match input.last_backup_unix {
        Some(t) => input.now_unix.saturating_sub(t) > input.stale_after_secs,
        None => true,
    };
    let recommend_run = input.backup_folder_linked && stale;
    if recommend_run {
        warnings.push("backup_stale_or_missing");
    }
    StoragePlan {
        available_bytes: available,
        primary_ok,
        recommend_persist: true,
        recommend_link_backup_folder: recommend_link,
        recommend_run_backup: recommend_run,
        warnings,
        engine_version: "0.0.39",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackupManifest {
    pub schema: String,
    pub created_unix: u64,
    pub content_sha256: String,
    pub byte_length: u64,
    pub primary_kind: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackupVerifyResult {
    pub ok: bool,
    pub errors: Vec<&'static str>,
    pub engine_version: &'static str,
}

/// Fail-closed checks for a backup snapshot manifest (host supplies SHA-256 hex).
pub fn verify_backup_manifest(m: &BackupManifest, payload_sha256_hex: &str) -> BackupVerifyResult {
    let mut errors = Vec::new();
    if m.schema != "webcivics.vault-backup.v1" {
        errors.push("schema_mismatch");
    }
    if m.content_sha256.len() != 64 || !m.content_sha256.chars().all(|c| c.is_ascii_hexdigit()) {
        errors.push("content_hash_malformed");
    }
    if !payload_sha256_hex.eq_ignore_ascii_case(&m.content_sha256) {
        errors.push("content_hash_mismatch");
    }
    if m.byte_length == 0 {
        errors.push("empty_payload");
    }
    if m.primary_kind != "opfs" && m.primary_kind != "host-path" {
        errors.push("unknown_primary_kind");
    }
    BackupVerifyResult {
        ok: errors.is_empty(),
        errors,
        engine_version: "0.0.39",
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm_exports {
    use super::*;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub fn device_storage_policy_wasm() -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&device_storage_policy())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen]
    pub fn plan_device_storage_wasm(val: JsValue) -> Result<JsValue, JsValue> {
        let input: StoragePlanInput = serde_wasm_bindgen::from_value(val)?;
        serde_wasm_bindgen::to_value(&plan_device_storage(&input))
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen]
    pub fn verify_backup_manifest_wasm(
        manifest: JsValue,
        payload_sha256_hex: &str,
    ) -> Result<JsValue, JsValue> {
        let m: BackupManifest = serde_wasm_bindgen::from_value(manifest)?;
        serde_wasm_bindgen::to_value(&verify_backup_manifest(&m, payload_sha256_hex))
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm_exports::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_names_opfs_and_backup_subdir() {
        let p = device_storage_policy();
        assert_eq!(p.primary.opfs_directory, "webcivics");
        assert_eq!(p.backup.suggested_subdir, "webcivics-backups");
        assert!(p.recovery.primary_wipe_does_not_erase_backup);
    }

    #[test]
    fn plan_flags_missing_backup_and_low_headroom() {
        let plan = plan_device_storage(&StoragePlanInput {
            quota_bytes: 10_000_000,
            usage_bytes: 9_500_000,
            backup_folder_linked: false,
            last_backup_unix: None,
            now_unix: 1_000_000,
            stale_after_secs: 100,
        });
        assert!(!plan.primary_ok);
        assert!(plan.recommend_link_backup_folder);
        assert!(!plan.recommend_run_backup); // cannot run until linked
        assert!(plan.warnings.contains(&"backup_folder_not_linked"));
    }

    #[test]
    fn plan_recommends_stale_backup_when_linked() {
        let plan = plan_device_storage(&StoragePlanInput {
            quota_bytes: 100_000_000,
            usage_bytes: 1_000,
            backup_folder_linked: true,
            last_backup_unix: Some(1),
            now_unix: 1_000_000,
            stale_after_secs: 60,
        });
        assert!(plan.primary_ok);
        assert!(plan.recommend_run_backup);
    }

    #[test]
    fn verify_rejects_hash_mismatch() {
        let m = BackupManifest {
            schema: "webcivics.vault-backup.v1".into(),
            created_unix: 1,
            content_sha256: "a".repeat(64),
            byte_length: 12,
            primary_kind: "opfs".into(),
            notes: None,
        };
        let r = verify_backup_manifest(&m, &"b".repeat(64));
        assert!(!r.ok);
        assert!(r.errors.contains(&"content_hash_mismatch"));
    }

    #[test]
    fn verify_accepts_matching_manifest() {
        let hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let m = BackupManifest {
            schema: "webcivics.vault-backup.v1".into(),
            created_unix: 1,
            content_sha256: hash.into(),
            byte_length: 12,
            primary_kind: "opfs".into(),
            notes: Some("test".into()),
        };
        let r = verify_backup_manifest(&m, hash);
        assert!(r.ok);
        assert!(r.errors.is_empty());
    }
}
