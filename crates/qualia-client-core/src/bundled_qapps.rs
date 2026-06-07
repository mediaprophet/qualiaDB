//! Seed qapps shipped beside the desktop executable into `{storage}/Qapps/`.
//!
//! Release bundles place packages under `{exe}/bundled/qapps/{Name}/`.
//! Dev builds resolve `bundled/qapps/{Name}/`, then gitignored `app-development/{Name}/`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::qapp_paths::{ensure_qapps_dir, qapps_dir, resolve_package_manifest_path};
use crate::qapp_registry::QAPP_PACKAGE_MANIFEST;
use crate::qapp_version::{is_version_newer, normalize_version_label};

/// Default qapps copied on first launch when absent from user storage.
pub const DEFAULT_BUNDLED_QAPPS: &[&str] = &["Anatomy"];

/// Version + update offer for an installed qapp.
#[derive(Debug, Clone, Serialize)]
pub struct QappVersionStatus {
    pub qapp_name: String,
    pub installed: bool,
    pub installed_version: Option<String>,
    pub offered_version: Option<String>,
    pub update_available: bool,
    /// `bundled` | `external` | `none`
    pub offer_source: String,
    pub message: String,
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

fn has_valid_manifest(dir: &Path) -> bool {
    resolve_package_manifest_path(dir).is_some_and(|p| p.is_file())
}

/// Read `version` from a qapp package directory's manifest.
pub fn read_qapp_version_from_dir(qapp_dir: &Path) -> Option<String> {
    let manifest_path = resolve_package_manifest_path(qapp_dir)?;
    let content = fs::read_to_string(manifest_path).ok()?;
    let manifest: crate::qapp_registry::QappPackageManifest = serde_json::from_str(&content).ok()?;
    Some(normalize_version_label(&manifest.version))
}

/// Resolve bundled source for a qapp directory name (e.g. `"Anatomy"`).
pub fn resolve_bundled_qapp_source(qapp_name: &str) -> Option<PathBuf> {
    if let Ok(extra) = std::env::var("QUALIA_BUNDLED_QAPPS_DIR") {
        let candidate = PathBuf::from(extra).join(qapp_name);
        if has_valid_manifest(&candidate) {
            return Some(candidate);
        }
    }

    if let Some(root) = exe_dir() {
        for rel in [
            format!("bundled/qapps/{qapp_name}"),
            format!("qapps/{qapp_name}"),
            format!("bundled/{qapp_name}"),
        ] {
            let candidate = root.join(&rel);
            if has_valid_manifest(&candidate) {
                return Some(candidate);
            }
        }
    }

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for rel in [
        format!("bundled/qapps/{qapp_name}"),
        format!("app-development/{qapp_name}"),
    ] {
        let candidate = repo_root.join(&rel);
        if has_valid_manifest(&candidate) {
            return Some(candidate);
        }
    }

    None
}

pub fn installed_qapp_version(storage_path: &Path, qapp_name: &str) -> Option<String> {
    let dir = qapps_dir(storage_path).join(qapp_name);
    if dir.join(QAPP_PACKAGE_MANIFEST).is_file() {
        read_qapp_version_from_dir(&dir)
    } else {
        None
    }
}

pub fn bundled_offered_version(qapp_name: &str) -> Option<String> {
    resolve_bundled_qapp_source(qapp_name).and_then(|p| read_qapp_version_from_dir(&p))
}

fn build_version_status(
    qapp_name: &str,
    installed_version: Option<String>,
    offered_version: Option<String>,
    offer_source: &str,
) -> QappVersionStatus {
    let installed = installed_version.is_some();
    let update_available = match (&installed_version, &offered_version) {
        (Some(inst), Some(off)) => is_version_newer(off, inst),
        (None, Some(_)) => true,
        _ => false,
    };

    let message = match (&installed_version, &offered_version, update_available) {
        (None, Some(off), true) => format!("Install qapp {qapp_name} v{off} from {offer_source}."),
        (Some(inst), Some(off), true) => {
            format!("Update available: v{inst} → v{off} ({offer_source}).")
        }
        (Some(inst), None, false) => format!("Installed v{inst}; no newer package found."),
        (Some(inst), Some(off), false) if inst == off => {
            format!("Up to date at v{inst}.")
        }
        (Some(inst), Some(off), false) => {
            format!("Installed v{inst}; bundled/external offer v{off} is not newer.")
        }
        (None, None, false) => format!("{qapp_name} is not installed and no package source was found."),
        _ => format!("{qapp_name} version status evaluated."),
    };

    QappVersionStatus {
        qapp_name: qapp_name.to_string(),
        installed,
        installed_version,
        offered_version,
        update_available,
        offer_source: offer_source.to_string(),
        message,
    }
}

/// Compare installed copy against the bundled/desktop-shipped package.
pub fn check_bundled_qapp_update(qapp_name: &str, storage_path: &Path) -> QappVersionStatus {
    let installed_version = installed_qapp_version(storage_path, qapp_name);
    let offered_version = bundled_offered_version(qapp_name);
    build_version_status(
        qapp_name,
        installed_version,
        offered_version,
        "bundled",
    )
}

/// Compare installed copy against an external directory (manual install / dev tree).
pub fn check_qapp_update_from_source(
    qapp_name: &str,
    storage_path: &Path,
    source_dir: &Path,
) -> Result<QappVersionStatus, String> {
    if !has_valid_manifest(source_dir) {
        return Err(format!(
            "qapp.json not found in {}",
            source_dir.display()
        ));
    }
    let offered_version = read_qapp_version_from_dir(source_dir)
        .ok_or_else(|| "Could not read version from source qapp.json".to_string())?;
    let installed_version = installed_qapp_version(storage_path, qapp_name);
    Ok(build_version_status(
        qapp_name,
        installed_version,
        Some(offered_version),
        "external",
    ))
}

/// List update offers for installed qapps and default bundled catalog entries.
pub fn list_bundled_qapp_updates(storage_path: &Path) -> Vec<QappVersionStatus> {
    let mut names = Vec::new();
    let qapps_root = qapps_dir(storage_path);
    if let Ok(entries) = fs::read_dir(&qapps_root) {
        for entry in entries.filter_map(Result::ok) {
            if entry.path().is_dir() {
                names.push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }
    for name in DEFAULT_BUNDLED_QAPPS {
        if !names.iter().any(|n| n == *name) {
            names.push((*name).to_string());
        }
    }

    let mut reports: Vec<QappVersionStatus> = names
        .iter()
        .map(|name| check_bundled_qapp_update(name, storage_path))
        .collect();
    reports.sort_by(|a, b| a.qapp_name.cmp(&b.qapp_name));
    reports
}

/// Copy bundled qapp into `{storage}/Qapps/{name}` when not already installed.
pub fn seed_qapp_if_missing(storage_path: impl AsRef<Path>, qapp_name: &str) -> Result<bool, String> {
    let storage = storage_path.as_ref();
    let dest = qapps_dir(storage).join(qapp_name);
    if dest.join(QAPP_PACKAGE_MANIFEST).is_file() {
        return Ok(false);
    }

    let source = resolve_bundled_qapp_source(qapp_name).ok_or_else(|| {
        format!("Bundled qapp source not found for {qapp_name}")
    })?;

    if dest.exists() {
        fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
    }
    copy_dir_all(&source, &dest).map_err(|e| format!("Failed to copy {qapp_name}: {e}"))?;
    Ok(true)
}

/// Replace an installed qapp with files from `source_dir` and re-register capabilities.
pub fn upgrade_qapp_from_source(
    storage_path: &Path,
    qapp_name: &str,
    source_dir: &Path,
) -> Result<String, String> {
    if !has_valid_manifest(source_dir) {
        return Err(format!(
            "qapp.json not found in {}",
            source_dir.display()
        ));
    }

    let manifest = crate::api::load_qapp_package_from_dir(source_dir)
        .map_err(|e| format!("Invalid source manifest: {e}"))?;
    if manifest.name != qapp_name {
        return Err(format!(
            "Source manifest name '{}' does not match expected '{}'",
            manifest.name, qapp_name
        ));
    }

    let previous = installed_qapp_version(storage_path, qapp_name);
    let next = normalize_version_label(&manifest.version);
    let dest = qapps_dir(storage_path).join(qapp_name);
    let _ = ensure_qapps_dir(storage_path).map_err(|e| e.to_string())?;

    if dest.exists() {
        fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
    }
    copy_dir_all(source_dir, &dest).map_err(|e| format!("Failed to upgrade {qapp_name}: {e}"))?;

    let _ = crate::qapp_manifest::install_qapp_capabilities(&manifest);

    Ok(match previous {
        Some(old) => format!("Upgraded {qapp_name} v{old} → v{next}"),
        None => format!("Installed {qapp_name} v{next}"),
    })
}

/// Apply a bundled update when a newer version is available.
pub fn apply_bundled_qapp_update(storage_path: &Path, qapp_name: &str) -> Result<String, String> {
    let status = check_bundled_qapp_update(qapp_name, storage_path);
    if !status.update_available {
        return Err(status.message);
    }
    let source = resolve_bundled_qapp_source(qapp_name)
        .ok_or_else(|| format!("Bundled qapp source not found for {qapp_name}"))?;
    upgrade_qapp_from_source(storage_path, qapp_name, &source)
}

fn register_default_capabilities(_storage_path: &Path) {
    for name in DEFAULT_BUNDLED_QAPPS {
        if let Ok(manifest) = crate::api::load_installed_qapp_package(name) {
            let _ = crate::qapp_manifest::install_qapp_capabilities(&manifest);
        } else if let Some(src) = resolve_bundled_qapp_source(name) {
            if let Ok(manifest) = crate::api::load_qapp_package_from_dir(&src) {
                let _ = crate::qapp_manifest::install_qapp_capabilities(&manifest);
            }
        }
    }
}

/// Seed all default bundled qapps and register Anatomy capabilities when present.
pub fn seed_bundled_qapps() -> Result<Vec<String>, String> {
    let state = crate::state::APP_STATE
        .get()
        .ok_or("APP_STATE not initialized")?;
    let storage = state.config.lock().map_err(|e| e.to_string())?.storage_path.clone();
    let storage_path = PathBuf::from(&storage);
    let _ = ensure_qapps_dir(&storage_path).map_err(|e| e.to_string())?;

    let mut seeded = Vec::new();
    for name in DEFAULT_BUNDLED_QAPPS {
        match seed_qapp_if_missing(&storage_path, name) {
            Ok(true) => seeded.push((*name).to_string()),
            Ok(false) => {}
            Err(e) => eprintln!("[bundled_qapps] skip {name}: {e}"),
        }
    }

    register_default_capabilities(&storage_path);

    for offer in list_bundled_qapp_updates(&storage_path) {
        if offer.update_available {
            eprintln!(
                "[bundled_qapps] update available for {}: {}",
                offer.qapp_name, offer.message
            );
        }
    }

    Ok(seeded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracked_anatomy_source_resolves_when_present() {
        let tracked = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../bundled/qapps/Anatomy/qapp.json");
        if tracked.is_file() {
            let src = resolve_bundled_qapp_source("Anatomy");
            assert!(src.is_some(), "expected bundled Anatomy path");
        }
    }

    #[test]
    fn anatomy_manifest_version_reads_when_present() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let anatomy = root.join("bundled/qapps/Anatomy");
        if !anatomy.join("qapp.json").is_file() {
            return;
        }
        let v = read_qapp_version_from_dir(&anatomy);
        assert_eq!(v.as_deref(), Some("0.0.8"));
    }
}
