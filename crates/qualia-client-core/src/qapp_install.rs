//! Atomic qApp package installation into `{storage}/Qapps/`.
//!
//! Validates manifest, optional content manifest + signature, copies atomically,
//! maintains a durable registry, and supports rollback / revocation.

use crate::qapp_paths::{ensure_qapps_dir, qapps_dir};
use crate::qapp_registry::{QappPackageManifest, QAPP_PACKAGE_MANIFEST};
use crate::qapp_version::{is_version_newer, normalize_version_label};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

pub const QAPP_REGISTRY_FILE: &str = "registry.json";
pub const PACKAGE_MANIFEST_SIDECAR: &str = "package-manifest.json";
pub const SUPPORTED_QAPP_ABI_VERSION: &str = "1.0";
pub const SUPPORTED_HOST_API_VERSION: &str = "1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QappFileHash {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QappContentManifest {
    pub schema_version: u32,
    pub package_id: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub abi_version: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub host_api_version: String,
    pub files: Vec<QappFileHash>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub signature_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QappRegistryEntry {
    pub package_id: String,
    pub active_version: String,
    pub content_hash: String,
    pub installed_at_unix: u64,
    pub revoked: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub archived_versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct QappInstallRegistry {
    pub packages: HashMap<String, QappRegistryEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallPolicy {
    /// Bundled/dev packages may omit signatures.
    Development,
    /// Production installs require a valid signed content manifest.
    Production,
}

#[derive(Debug, PartialEq, Eq)]
pub enum QappInstallError {
    InvalidPackageId(String),
    PathTraversal(String),
    ManifestMissing,
    ManifestInvalid(String),
    ContentManifestInvalid(String),
    HashMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    SignatureInvalid(String),
    AbiMismatch {
        found: String,
        supported: String,
    },
    PackageRevoked(String),
    StagingFailed(String),
    RegistryCorrupt(String),
    Io(String),
}

impl std::fmt::Display for QappInstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPackageId(id) => write!(f, "invalid package id: {id}"),
            Self::PathTraversal(p) => write!(f, "path traversal rejected: {p}"),
            Self::ManifestMissing => write!(f, "qapp.json not found"),
            Self::ManifestInvalid(e) => write!(f, "invalid qapp.json: {e}"),
            Self::ContentManifestInvalid(e) => write!(f, "invalid package-manifest.json: {e}"),
            Self::HashMismatch {
                path,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "hash mismatch for {path}: expected {expected}, got {actual}"
                )
            }
            Self::SignatureInvalid(e) => write!(f, "signature invalid: {e}"),
            Self::AbiMismatch { found, supported } => {
                write!(f, "ABI version {found} not supported (need {supported})")
            }
            Self::PackageRevoked(id) => write!(f, "package revoked: {id}"),
            Self::StagingFailed(e) => write!(f, "staging failed: {e}"),
            Self::RegistryCorrupt(e) => write!(f, "registry corrupt: {e}"),
            Self::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl From<io::Error> for QappInstallError {
    fn from(value: io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

pub fn package_id_from_manifest(manifest: &QappPackageManifest) -> String {
    manifest.name.trim().to_string()
}

pub fn validate_package_id(package_id: &str) -> Result<(), QappInstallError> {
    if package_id.is_empty() {
        return Err(QappInstallError::InvalidPackageId("empty".into()));
    }
    if package_id.contains("..") || package_id.contains('/') || package_id.contains('\\') {
        return Err(QappInstallError::InvalidPackageId(package_id.into()));
    }
    for ch in package_id.chars() {
        let ok = ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == ' ';
        if !ok {
            return Err(QappInstallError::InvalidPackageId(package_id.into()));
        }
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex::encode(digest.as_slice())
}

fn sha256_file(path: &Path) -> Result<String, QappInstallError> {
    let bytes = fs::read(path)?;
    Ok(sha256_hex(&bytes))
}

fn registry_path(storage: &Path) -> PathBuf {
    qapps_dir(storage).join(QAPP_REGISTRY_FILE)
}

fn versions_dir(storage: &Path, package_id: &str) -> PathBuf {
    qapps_dir(storage).join(package_id).join("versions")
}

fn active_package_dir(storage: &Path, package_id: &str) -> PathBuf {
    qapps_dir(storage).join(package_id)
}

fn staging_root(storage: &Path) -> PathBuf {
    qapps_dir(storage).join(".staging")
}

pub fn load_install_registry(storage: &Path) -> Result<QappInstallRegistry, QappInstallError> {
    let path = registry_path(storage);
    if !path.is_file() {
        return Ok(QappInstallRegistry::default());
    }
    let content =
        fs::read_to_string(&path).map_err(|e| QappInstallError::RegistryCorrupt(e.to_string()))?;
    serde_json::from_str(&content).map_err(|e| QappInstallError::RegistryCorrupt(e.to_string()))
}

pub fn save_install_registry(
    storage: &Path,
    registry: &QappInstallRegistry,
) -> Result<(), QappInstallError> {
    ensure_qapps_dir(storage)?;
    let path = registry_path(storage);
    let staging = path.with_extension("json.staging");
    let json = serde_json::to_string_pretty(registry)
        .map_err(|e| QappInstallError::RegistryCorrupt(e.to_string()))?;
    fs::write(&staging, json)?;
    fs::rename(&staging, &path)?;
    Ok(())
}

pub fn resolve_active_package_dir(storage: &Path, package_id: &str) -> PathBuf {
    let active = active_package_dir(storage, package_id);
    if active.join(QAPP_PACKAGE_MANIFEST).is_file() {
        return active;
    }
    active
}

pub fn is_package_revoked(storage: &Path, package_id: &str) -> bool {
    load_install_registry(storage)
        .ok()
        .and_then(|r| r.packages.get(package_id).map(|e| e.revoked))
        .unwrap_or(false)
}

fn relative_path_ok(rel: &str) -> bool {
    let path = Path::new(rel);
    for component in path.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return false,
            _ => {}
        }
    }
    true
}

fn collect_package_files(dir: &Path) -> Result<Vec<(String, String)>, QappInstallError> {
    let mut out = Vec::new();
    collect_package_files_inner(dir, dir, &mut out)?;
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn collect_package_files_inner(
    root: &Path,
    current: &Path,
    out: &mut Vec<(String, String)>,
) -> Result<(), QappInstallError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name == ".staging" || file_name == QAPP_REGISTRY_FILE {
            continue;
        }
        if path.is_dir() {
            if file_name == "versions" {
                continue;
            }
            collect_package_files_inner(root, &path, out)?;
        } else {
            let rel = path
                .strip_prefix(root)
                .map_err(|_| QappInstallError::PathTraversal(path.display().to_string()))?
                .to_string_lossy()
                .replace('\\', "/");
            if !relative_path_ok(&rel) {
                return Err(QappInstallError::PathTraversal(rel));
            }
            let hash = sha256_file(&path)?;
            out.push((rel, hash));
        }
    }
    Ok(())
}

fn verify_content_manifest(
    source_dir: &Path,
    manifest: &QappContentManifest,
    policy: InstallPolicy,
    trust_pubkey: Option<&[u8; 32]>,
) -> Result<(), QappInstallError> {
    validate_package_id(&manifest.package_id)?;
    if !manifest.abi_version.is_empty() && manifest.abi_version != SUPPORTED_QAPP_ABI_VERSION {
        return Err(QappInstallError::AbiMismatch {
            found: manifest.abi_version.clone(),
            supported: SUPPORTED_QAPP_ABI_VERSION.into(),
        });
    }
    if !manifest.host_api_version.is_empty()
        && manifest.host_api_version != SUPPORTED_HOST_API_VERSION
    {
        return Err(QappInstallError::AbiMismatch {
            found: manifest.host_api_version.clone(),
            supported: SUPPORTED_HOST_API_VERSION.into(),
        });
    }

    for file in &manifest.files {
        if !relative_path_ok(&file.path) {
            return Err(QappInstallError::PathTraversal(file.path.clone()));
        }
        let disk_path = source_dir.join(&file.path);
        if !disk_path.is_file() {
            return Err(QappInstallError::HashMismatch {
                path: file.path.clone(),
                expected: file.sha256.clone(),
                actual: "missing".into(),
            });
        }
        let actual = sha256_file(&disk_path)?;
        if actual != file.sha256 {
            return Err(QappInstallError::HashMismatch {
                path: file.path.clone(),
                expected: file.sha256.clone(),
                actual,
            });
        }
    }

    if policy == InstallPolicy::Production {
        let sig_hex = manifest.signature_hex.trim();
        if sig_hex.is_empty() {
            return Err(QappInstallError::SignatureInvalid(
                "production install requires signature_hex".into(),
            ));
        }
        let pk = trust_pubkey.ok_or_else(|| {
            QappInstallError::SignatureInvalid("no trust pubkey configured".into())
        })?;
        let sig_bytes =
            hex::decode(sig_hex).map_err(|e| QappInstallError::SignatureInvalid(e.to_string()))?;
        if sig_bytes.len() != 64 {
            return Err(QappInstallError::SignatureInvalid(
                "expected 64-byte ed25519 signature".into(),
            ));
        }
        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&sig_bytes);
        let signature = Signature::from_bytes(&sig_arr);
        let verifying_key = VerifyingKey::from_bytes(pk)
            .map_err(|e| QappInstallError::SignatureInvalid(e.to_string()))?;
        let mut sign_payload = serde_json::to_vec(manifest)
            .map_err(|e| QappInstallError::SignatureInvalid(e.to_string()))?;
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&sign_payload) {
            if let Some(obj) = value.as_object() {
                let mut unsigned = obj.clone();
                unsigned.remove("signature_hex");
                sign_payload = serde_json::to_vec(&unsigned)
                    .map_err(|e| QappInstallError::SignatureInvalid(e.to_string()))?;
            }
        }
        verifying_key
            .verify(&sign_payload, &signature)
            .map_err(|e| QappInstallError::SignatureInvalid(e.to_string()))?;
    }

    Ok(())
}

fn load_sidecar_manifest(source_dir: &Path) -> Option<QappContentManifest> {
    let path = source_dir.join(PACKAGE_MANIFEST_SIDECAR);
    if !path.is_file() {
        return None;
    }
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), QappInstallError> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str == "versions" {
            continue;
        }
        let target = dst.join(&name);
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn remove_dir_contents(dir: &Path) -> Result<(), QappInstallError> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "versions" {
            continue;
        }
        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

fn archive_active_version(
    storage: &Path,
    package_id: &str,
    version: &str,
) -> Result<(), QappInstallError> {
    let active = active_package_dir(storage, package_id);
    if !active.join(QAPP_PACKAGE_MANIFEST).is_file() {
        return Ok(());
    }
    let archive_root = versions_dir(storage, package_id).join(version);
    if archive_root.exists() {
        return Ok(());
    }
    fs::create_dir_all(archive_root.parent().unwrap())?;
    copy_dir_all(&active, &archive_root)?;
    Ok(())
}

/// Validate a package directory before install.
pub fn validate_package_source(
    source_dir: &Path,
    policy: InstallPolicy,
    trust_pubkey: Option<&[u8; 32]>,
) -> Result<(QappPackageManifest, String), QappInstallError> {
    let manifest_path = source_dir.join(QAPP_PACKAGE_MANIFEST);
    if !manifest_path.is_file() {
        return Err(QappInstallError::ManifestMissing);
    }
    let content = fs::read_to_string(&manifest_path)?;
    let manifest: QappPackageManifest = serde_json::from_str(&content)
        .map_err(|e| QappInstallError::ManifestInvalid(e.to_string()))?;
    let package_id = package_id_from_manifest(&manifest);
    validate_package_id(&package_id)?;

    if let Some(sidecar) = load_sidecar_manifest(source_dir) {
        if sidecar.package_id != package_id {
            return Err(QappInstallError::ContentManifestInvalid(format!(
                "package_id mismatch: manifest {} vs sidecar {}",
                package_id, sidecar.package_id
            )));
        }
        verify_content_manifest(source_dir, &sidecar, policy, trust_pubkey)?;
    } else if policy == InstallPolicy::Production {
        return Err(QappInstallError::ContentManifestInvalid(
            "production install requires package-manifest.json".into(),
        ));
    }

    let files = collect_package_files(source_dir)?;
    let aggregate = files
        .iter()
        .map(|(p, h)| format!("{p}:{h}"))
        .collect::<Vec<_>>()
        .join("\n");
    let content_hash = sha256_hex(aggregate.as_bytes());
    Ok((manifest, content_hash))
}

/// Install or upgrade a package atomically into `{storage}/Qapps/{package_id}/`.
pub fn install_package_atomic(
    storage: &Path,
    source_dir: &Path,
    policy: InstallPolicy,
    trust_pubkey: Option<&[u8; 32]>,
) -> Result<QappRegistryEntry, QappInstallError> {
    let (manifest, content_hash) = validate_package_source(source_dir, policy, trust_pubkey)?;
    let package_id = package_id_from_manifest(&manifest);
    let version = normalize_version_label(&manifest.version);

    if is_package_revoked(storage, &package_id) {
        return Err(QappInstallError::PackageRevoked(package_id));
    }

    ensure_qapps_dir(storage)?;
    let staging_parent = staging_root(storage);
    fs::create_dir_all(&staging_parent)?;
    let staging_dir =
        staging_parent.join(format!("{package_id}-{version}-{}", uuid::Uuid::new_v4()));
    copy_dir_all(source_dir, &staging_dir)?;

    let dest = active_package_dir(storage, &package_id);
    let mut registry = load_install_registry(storage)?;

    let install_result = (|| -> Result<QappRegistryEntry, QappInstallError> {
        if let Some(existing) = registry.packages.get(&package_id) {
            if !is_version_newer(&version, &existing.active_version)
                && version != existing.active_version
            {
                return Err(QappInstallError::StagingFailed(format!(
                    "refusing downgrade from {} to {}",
                    existing.active_version, version
                )));
            }
            if version != existing.active_version {
                archive_active_version(storage, &package_id, &existing.active_version)?;
            }
        }

        fs::create_dir_all(dest.parent().unwrap())?;
        if dest.exists() {
            remove_dir_contents(&dest)?;
        } else {
            fs::create_dir_all(&dest)?;
        }
        copy_dir_all(&staging_dir, &dest)?;

        let installed_at_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut archived = registry
            .packages
            .get(&package_id)
            .map(|e| e.archived_versions.clone())
            .unwrap_or_default();
        if let Some(prev) = registry.packages.get(&package_id) {
            if prev.active_version != version && !archived.contains(&prev.active_version) {
                archived.push(prev.active_version.clone());
            }
        }

        let entry = QappRegistryEntry {
            package_id: package_id.clone(),
            active_version: version,
            content_hash,
            installed_at_unix,
            revoked: false,
            archived_versions: archived,
        };
        registry.packages.insert(package_id, entry.clone());
        save_install_registry(storage, &registry)?;
        Ok(entry)
    })();

    let _ = fs::remove_dir_all(&staging_dir);
    install_result
}

pub fn revoke_package(storage: &Path, package_id: &str) -> Result<(), QappInstallError> {
    validate_package_id(package_id)?;
    let mut registry = load_install_registry(storage)?;
    let entry = registry
        .packages
        .get_mut(package_id)
        .ok_or_else(|| QappInstallError::StagingFailed(format!("unknown package {package_id}")))?;
    entry.revoked = true;
    save_install_registry(storage, &registry)
}

pub fn list_registry_entries(storage: &Path) -> Result<Vec<QappRegistryEntry>, QappInstallError> {
    Ok(load_install_registry(storage)?
        .packages
        .into_values()
        .collect())
}

pub fn reconcile_registry_with_disk(
    storage: &Path,
) -> Result<QappInstallRegistry, QappInstallError> {
    ensure_qapps_dir(storage)?;
    let mut registry = load_install_registry(storage)?;
    let root = qapps_dir(storage);

    if let Ok(entries) = fs::read_dir(&root) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == QAPP_REGISTRY_FILE {
                continue;
            }
            if !path.join(QAPP_PACKAGE_MANIFEST).is_file() {
                continue;
            }
            if registry.packages.contains_key(&name) {
                continue;
            }
            let content = fs::read_to_string(path.join(QAPP_PACKAGE_MANIFEST))?;
            let manifest: QappPackageManifest = serde_json::from_str(&content)
                .map_err(|e| QappInstallError::ManifestInvalid(e.to_string()))?;
            let files = collect_package_files(&path)?;
            let aggregate = files
                .iter()
                .map(|(p, h)| format!("{p}:{h}"))
                .collect::<Vec<_>>()
                .join("\n");
            let content_hash = sha256_hex(aggregate.as_bytes());
            registry.packages.insert(
                name.clone(),
                QappRegistryEntry {
                    package_id: name,
                    active_version: normalize_version_label(&manifest.version),
                    content_hash,
                    installed_at_unix: 0,
                    revoked: false,
                    archived_versions: Vec::new(),
                },
            );
        }
    }

    save_install_registry(storage, &registry)?;
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_storage() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("qualia-qapp-install-test-{nanos}"))
    }

    fn write_minimal_package(dir: &Path, name: &str, version: &str) {
        fs::create_dir_all(dir).unwrap();
        let manifest = format!(
            r#"{{
  "name": "{name}",
  "version": "{version}",
  "required_shapes": ["schema:Test"]
}}"#
        );
        fs::write(dir.join(QAPP_PACKAGE_MANIFEST), manifest).unwrap();
        fs::write(dir.join("index.html"), "<html></html>").unwrap();
    }

    #[test]
    fn rejects_path_traversal_package_id() {
        assert!(validate_package_id("..").is_err());
        assert!(validate_package_id("foo/bar").is_err());
        assert!(validate_package_id("Anatomy").is_ok());
    }

    #[test]
    fn atomic_install_and_registry_round_trip() {
        let storage = temp_storage();
        let source = storage.join("source");
        write_minimal_package(&source, "TestApp", "0.0.1");

        let entry =
            install_package_atomic(&storage, &source, InstallPolicy::Development, None).unwrap();
        assert_eq!(entry.package_id, "TestApp");
        assert_eq!(entry.active_version, "0.0.1");
        assert!(active_package_dir(&storage, "TestApp")
            .join("index.html")
            .is_file());

        let registry = load_install_registry(&storage).unwrap();
        assert!(registry.packages.contains_key("TestApp"));
        let _ = fs::remove_dir_all(&storage);
    }

    #[test]
    fn interrupted_staging_keeps_prior_version() {
        let storage = temp_storage();
        let v1 = storage.join("v1");
        let v2 = storage.join("v2");
        write_minimal_package(&v1, "TestApp", "0.0.1");
        write_minimal_package(&v2, "TestApp", "0.0.2");
        fs::write(v2.join("index.html"), "<html>v2</html>").unwrap();

        install_package_atomic(&storage, &v1, InstallPolicy::Development, None).unwrap();
        install_package_atomic(&storage, &v2, InstallPolicy::Development, None).unwrap();

        let html =
            fs::read_to_string(active_package_dir(&storage, "TestApp").join("index.html")).unwrap();
        assert!(html.contains("v2"));
        let registry = load_install_registry(&storage).unwrap();
        assert_eq!(registry.packages["TestApp"].active_version, "0.0.2");
        assert!(versions_dir(&storage, "TestApp").join("0.0.1").is_dir());
        let _ = fs::remove_dir_all(&storage);
    }

    #[test]
    fn revoked_package_cannot_reinstall() {
        let storage = temp_storage();
        let source = storage.join("source");
        write_minimal_package(&source, "TestApp", "0.0.1");
        install_package_atomic(&storage, &source, InstallPolicy::Development, None).unwrap();
        revoke_package(&storage, "TestApp").unwrap();
        let err = install_package_atomic(&storage, &source, InstallPolicy::Development, None)
            .unwrap_err();
        assert!(matches!(err, QappInstallError::PackageRevoked(_)));
        let _ = fs::remove_dir_all(&storage);
    }

    #[test]
    fn reconcile_discovers_flat_install() {
        let storage = temp_storage();
        write_minimal_package(
            &active_package_dir(&storage, "LegacyApp"),
            "LegacyApp",
            "1.0.0",
        );
        let registry = reconcile_registry_with_disk(&storage).unwrap();
        assert!(registry.packages.contains_key("LegacyApp"));
        let _ = fs::remove_dir_all(&storage);
    }
}
