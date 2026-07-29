//! WP2 — Package & Publish: turn a `QappManifest` into an installable PWA bundle on disk.
//!
//! Reuses [`qualia_cooperative_core::qapp_package::generate_pwa`] (the P0 foundation) and writes
//! the generated bundle to a caller-chosen directory, path-traversal-safe. Producing the bytes is
//! WP2; **serving** them over a secure origin so a phone can install is a later stage (P1, see the
//! companion-PWA plan). This module is the authoring → artifact bridge.

#![cfg(not(target_arch = "wasm32"))]

use std::path::{Component, Path};

use qualia_cooperative_core::qapp_package::{
    generate_pwa, Capability, IconRef, PwaContent, QappKind, QappManifest, WasmRef,
};

/// Parse a kind string to a [`QappKind`] (extensible: unknown → `Custom`).
pub fn parse_kind(s: &str) -> QappKind {
    match s.trim().to_ascii_lowercase().as_str() {
        "cooperative" => QappKind::Cooperative,
        "health" => QappKind::Health,
        "journal" => QappKind::Journal,
        "directory" => QappKind::Directory,
        "" => QappKind::Custom("custom".to_string()),
        other => QappKind::Custom(other.to_string()),
    }
}

/// Parse a capability token to a [`Capability`] (extensible: unknown → `Custom`).
pub fn parse_capability(s: &str) -> Capability {
    match s.trim().to_ascii_lowercase().as_str() {
        "read_records" | "readrecords" => Capability::ReadRecords,
        "write_records" | "writerecords" => Capability::WriteRecords,
        "sync" => Capability::Sync,
        "blob_store" | "blobstore" => Capability::BlobStore,
        "notifications" => Capability::Notifications,
        "camera" => Capability::Camera,
        other => Capability::Custom(other.to_string()),
    }
}

/// Build a [`QappManifest`] from discrete author-supplied fields. The wasm bundle is referenced by
/// filename only (its hash/size are filled by the build stage, P3); an empty `wasm_filename`
/// defaults to `app.wasm`.
pub fn build_manifest(
    id: &str,
    name: &str,
    kind: &str,
    description: &str,
    capabilities_csv: &str,
    wasm_filename: &str,
) -> QappManifest {
    let wasm = if wasm_filename.trim().is_empty() {
        "app.wasm".to_string()
    } else {
        wasm_filename.trim().to_string()
    };
    let mut manifest = QappManifest::new(id, name)
        .with_kind(parse_kind(kind))
        .with_description(description)
        .with_entry(WasmRef {
            path: wasm,
            sha256_hex: String::new(),
            size_bytes: 0,
        })
        // A PWA needs an icon to be installable; reference one the author drops alongside the
        // bundle (like the wasm entry). Without it, manifest validation fails.
        .with_icon(IconRef {
            src: "icon-512.png".to_string(),
            sizes: "512x512".to_string(),
            purpose: "any".to_string(),
        });
    for cap in capabilities_csv
        .split(',')
        .map(|c| c.trim())
        .filter(|c| !c.is_empty())
    {
        manifest = manifest.with_capability(parse_capability(cap));
    }
    manifest
}

/// A bundle-relative path is safe iff it is relative and contains no `..`/root components.
fn is_safe_relative(path: &str) -> bool {
    let p = Path::new(path);
    !p.is_absolute()
        && p.components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

/// Generate the PWA bundle for `manifest` and write every file under `target_dir`. Returns the list
/// of written (bundle-relative) paths. Rejects an invalid manifest and any unsafe bundle path.
pub fn write_pwa_bundle(target_dir: &Path, manifest: &QappManifest) -> Result<Vec<String>, String> {
    if let Err(problems) = manifest.validate() {
        return Err(format!("Invalid manifest: {}", problems.join("; ")));
    }
    let bundle = generate_pwa(manifest);
    let mut written = Vec::new();
    for file in &bundle.files {
        if !is_safe_relative(&file.path) {
            return Err(format!("Unsafe bundle path rejected: {}", file.path));
        }
        let dest = target_dir.join(&file.path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        match &file.content {
            PwaContent::Text(s) => {
                std::fs::write(&dest, s.as_bytes()).map_err(|e| e.to_string())?
            }
            PwaContent::Bytes(b) => std::fs::write(&dest, b).map_err(|e| e.to_string())?,
        }
        written.push(file.path.clone());
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn build_manifest_maps_kind_and_capabilities() {
        let m = build_manifest(
            "coop.qualia.journal",
            "Journal",
            "journal",
            "a private journal",
            "read_records, write_records, sync",
            "journal.wasm",
        );
        assert_eq!(m.kind, QappKind::Journal);
        assert_eq!(m.capabilities.len(), 3);
        assert!(m.capabilities.contains(&Capability::WriteRecords));
        assert_eq!(m.entry_wasm.path, "journal.wasm");
        // Unknown kind → Custom (extensible).
        assert_eq!(
            build_manifest("x.y", "Z", "bespoke-thing", "", "", "").kind,
            QappKind::Custom("bespoke-thing".to_string())
        );
    }

    #[test]
    fn write_bundle_emits_installable_scaffold() {
        let dir = tempdir().unwrap();
        let manifest = build_manifest(
            "coop.qualia.demo",
            "Demo",
            "cooperative",
            "demo qapp",
            "read_records",
            "app.wasm",
        );
        let written = write_pwa_bundle(dir.path(), &manifest).unwrap();
        for expected in ["manifest.webmanifest", "sw.js", "index.html"] {
            assert!(
                written.iter().any(|p| p == expected),
                "missing {expected} in {written:?}"
            );
            assert!(dir.path().join(expected).exists(), "{expected} not on disk");
        }
        // The manifest is a real Web App Manifest.
        let webmanifest = std::fs::read_to_string(dir.path().join("manifest.webmanifest")).unwrap();
        assert!(webmanifest.contains("\"display\""));
        assert!(webmanifest.contains("Demo"));
    }

    #[test]
    fn rejects_unsafe_paths() {
        assert!(is_safe_relative("index.html"));
        assert!(is_safe_relative("icons/app.png"));
        assert!(!is_safe_relative("../escape"));
        assert!(!is_safe_relative("/etc/passwd"));
    }

    #[test]
    fn invalid_manifest_is_rejected() {
        let dir = tempdir().unwrap();
        // Empty id → validation failure (no dot / empty).
        let bad = build_manifest("", "", "journal", "", "", "");
        assert!(write_pwa_bundle(dir.path(), &bad).is_err());
    }
}
