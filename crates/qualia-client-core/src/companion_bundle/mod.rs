//! Deterministic WellFair companion WASM/PWA bundle builder (Workstream 3, M2).
//!
//! Consumes a prebuilt signed WASM artifact and emits a reproducible package tree.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const BUNDLE_SCHEMA_VERSION: u32 = 1;
pub const COMPANION_PROFILE_NAME: &str = "wellfair-linked-companion";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BundleFileEntry {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompanionBundleManifest {
    pub schema_version: u32,
    pub package_id: String,
    pub version: String,
    pub profile: String,
    pub host_api_version: String,
    pub abi_version: String,
    pub content_hash: String,
    pub files: Vec<BundleFileEntry>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub signature_hex: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BundleBuildError {
    MissingInput(String),
    Io(String),
    InvalidManifest(String),
}

impl std::fmt::Display for BundleBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingInput(p) => write!(f, "missing input: {p}"),
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::InvalidManifest(e) => write!(f, "invalid manifest: {e}"),
        }
    }
}

impl From<io::Error> for BundleBuildError {
    fn from(value: io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes).as_slice())
}

/// Walk `root` deterministically and hash every file (sorted paths).
pub fn hash_bundle_tree(root: &Path) -> Result<(String, Vec<BundleFileEntry>), BundleBuildError> {
    let mut entries = Vec::new();
    collect_files(root, root, &mut entries)?;
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    let aggregate = entries
        .iter()
        .map(|e| format!("{}:{}:{}", e.path, e.sha256, e.size_bytes))
        .collect::<Vec<_>>()
        .join("\n");
    let content_hash = sha256_hex(aggregate.as_bytes());
    Ok((content_hash, entries))
}

fn collect_files(
    root: &Path,
    current: &Path,
    out: &mut Vec<BundleFileEntry>,
) -> Result<(), BundleBuildError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, out)?;
        } else {
            let rel = path
                .strip_prefix(root)
                .map_err(|_| BundleBuildError::InvalidManifest(path.display().to_string()))?
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = fs::read(&path)?;
            out.push(BundleFileEntry {
                path: rel,
                sha256: sha256_hex(&bytes),
                size_bytes: bytes.len() as u64,
            });
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct CompanionBundleInput {
    pub package_id: String,
    pub version: String,
    pub wasm_js: PathBuf,
    pub wasm_binary: PathBuf,
    pub index_html: PathBuf,
    pub qapp_json: PathBuf,
}

/// Assemble bundle directory with deterministic ordering and manifest emission.
pub fn build_companion_bundle(
    output_dir: &Path,
    input: &CompanionBundleInput,
) -> Result<CompanionBundleManifest, BundleBuildError> {
    for required in [
        &input.wasm_js,
        &input.wasm_binary,
        &input.index_html,
        &input.qapp_json,
    ] {
        if !required.is_file() {
            return Err(BundleBuildError::MissingInput(
                required.display().to_string(),
            ));
        }
    }

    if output_dir.exists() {
        fs::remove_dir_all(output_dir)?;
    }
    fs::create_dir_all(output_dir.join("wasm"))?;
    fs::create_dir_all(output_dir.join("assets"))?;

    fs::copy(&input.qapp_json, output_dir.join("qapp.json"))?;
    fs::copy(&input.index_html, output_dir.join("index.html"))?;
    fs::copy(
        &input.wasm_js,
        output_dir
            .join("wasm")
            .join(input.wasm_js.file_name().unwrap()),
    )?;
    fs::copy(
        &input.wasm_binary,
        output_dir
            .join("wasm")
            .join(input.wasm_binary.file_name().unwrap()),
    )?;

    let (content_hash, files) = hash_bundle_tree(output_dir)?;
    let manifest = CompanionBundleManifest {
        schema_version: BUNDLE_SCHEMA_VERSION,
        package_id: input.package_id.clone(),
        version: input.version.clone(),
        profile: COMPANION_PROFILE_NAME.to_string(),
        host_api_version: crate::qapp_install::SUPPORTED_HOST_API_VERSION.to_string(),
        abi_version: crate::qapp_install::SUPPORTED_QAPP_ABI_VERSION.to_string(),
        content_hash: content_hash.clone(),
        files,
        signature_hex: String::new(),
    };

    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| BundleBuildError::InvalidManifest(e.to_string()))?;
    fs::write(output_dir.join("package-manifest.json"), &json)?;
    let cbor = serde_json::to_vec(&manifest)
        .map_err(|e| BundleBuildError::InvalidManifest(e.to_string()))?;
    fs::write(output_dir.join("package-manifest.cbor"), cbor)?;

    Ok(manifest)
}

/// Verify an on-disk bundle matches its embedded manifest hashes.
pub fn verify_companion_bundle(
    bundle_dir: &Path,
) -> Result<CompanionBundleManifest, BundleBuildError> {
    let manifest_path = bundle_dir.join("package-manifest.json");
    if !manifest_path.is_file() {
        return Err(BundleBuildError::MissingInput(
            "package-manifest.json".into(),
        ));
    }
    let content = fs::read_to_string(&manifest_path)?;
    let manifest: CompanionBundleManifest = serde_json::from_str(&content)
        .map_err(|e| BundleBuildError::InvalidManifest(e.to_string()))?;

    let mut on_disk: BTreeMap<String, BundleFileEntry> = BTreeMap::new();
    let (_, scanned) = hash_bundle_tree(bundle_dir)?;
    for entry in scanned {
        if entry.path == "package-manifest.json" || entry.path == "package-manifest.cbor" {
            continue;
        }
        on_disk.insert(entry.path.clone(), entry);
    }

    for expected in &manifest.files {
        if expected.path == "package-manifest.json" || expected.path == "package-manifest.cbor" {
            continue;
        }
        let actual = on_disk
            .get(&expected.path)
            .ok_or_else(|| BundleBuildError::MissingInput(expected.path.clone()))?;
        if actual.sha256 != expected.sha256 || actual.size_bytes != expected.size_bytes {
            return Err(BundleBuildError::InvalidManifest(format!(
                "hash mismatch for {}",
                expected.path
            )));
        }
    }

    let mut payload_entries: Vec<BundleFileEntry> = manifest.files.clone();
    payload_entries.sort_by(|a, b| a.path.cmp(&b.path));
    let aggregate = payload_entries
        .iter()
        .map(|e| format!("{}:{}:{}", e.path, e.sha256, e.size_bytes))
        .collect::<Vec<_>>()
        .join("\n");
    let recomputed = sha256_hex(aggregate.as_bytes());
    if recomputed != manifest.content_hash {
        return Err(BundleBuildError::InvalidManifest(
            "content_hash does not match bundle tree".into(),
        ));
    }

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("qualia-bundle-{label}-{nanos}"))
    }

    #[test]
    fn deterministic_content_hash_for_same_inputs() {
        let root = temp_dir("build");
        let out_a = root.join("out-a");
        let out_b = root.join("out-b");
        let src = root.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(
            src.join("qapp.json"),
            r#"{"name":"wellfair","version":"0.0.24","required_shapes":[]}"#,
        )
        .unwrap();
        fs::write(src.join("index.html"), "<html>wellfair</html>").unwrap();
        fs::write(src.join("profile.js"), "export default {}").unwrap();
        fs::write(src.join("profile_bg.wasm"), b"\0asm").unwrap();

        let input = CompanionBundleInput {
            package_id: "wellfair-companion".into(),
            version: "0.0.24".into(),
            wasm_js: src.join("profile.js"),
            wasm_binary: src.join("profile_bg.wasm"),
            index_html: src.join("index.html"),
            qapp_json: src.join("qapp.json"),
        };

        let m_a = build_companion_bundle(&out_a, &input).unwrap();
        let m_b = build_companion_bundle(&out_b, &input).unwrap();
        assert_eq!(m_a.content_hash, m_b.content_hash);
        verify_companion_bundle(&out_a).unwrap();
        let _ = fs::remove_dir_all(&root);
    }
}
