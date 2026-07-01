//! Storage path helpers for installed Qualia qapps.

use crate::qapp_registry::{QAPPS_DIR, QAPP_PACKAGE_MANIFEST};
use std::path::{Path, PathBuf};

/// `{storage}/Qapps/`
pub fn qapps_dir(storage_path: impl AsRef<Path>) -> PathBuf {
    storage_path.as_ref().join(QAPPS_DIR)
}

/// Ensure `{storage}/Qapps/` exists.
pub fn ensure_qapps_dir(storage_path: impl AsRef<Path>) -> std::io::Result<PathBuf> {
    let dir = qapps_dir(storage_path);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Active install directory for a package id (flat layout under `{storage}/Qapps/{id}/`).
pub fn resolve_active_package_dir(storage_path: impl AsRef<Path>, package_id: &str) -> PathBuf {
    crate::qapp_install::resolve_active_package_dir(storage_path.as_ref(), package_id)
}

/// `{package}/qapp.json` when present.
pub fn resolve_package_manifest_path(qapp_dir: &Path) -> Option<PathBuf> {
    let path = qapp_dir.join(QAPP_PACKAGE_MANIFEST);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}
