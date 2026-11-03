//! Independently pinned release signature plus exact package-content admission.
use super::{error, fail, local_file, Result};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

pub const MAX_MANIFEST_BYTES: usize = 16_384;
const DOMAIN: &[u8] = b"qualia:sensing-inspect:release-manifest:v1\0";
const FILES: [&str; 6] = [
    ".qualia-package-owner",
    "qualia-sensing-inspect.exe",
    "README.md",
    "LICENSE",
    "distribution.json",
    "verify-package.ps1",
];

/// Offline release authorities sign this digest using ordinary Ed25519.
/// Length/domain bind exact UTF-8 package.json bytes; this is not Ed25519ph.
pub fn release_signing_digest(manifest: &[u8]) -> Result<[u8; 32]> {
    if manifest.is_empty() || manifest.len() > MAX_MANIFEST_BYTES {
        return fail("manifest byte limit");
    }
    let mut hash = Sha256::new();
    hash.update(DOMAIN);
    hash.update((manifest.len() as u64).to_le_bytes());
    hash.update(manifest);
    Ok(hash.finalize().into())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: String,
    source_version: String,
    target_declaration: String,
    entrypoint: String,
    supplied_binary_qualified: bool,
    build_reproduced: bool,
    integrity_is_authenticity: bool,
    files: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    path: String,
    bytes: u64,
    sha256: String,
}

fn verify_manifest(
    manifest: &[u8],
    signature: &[u8; 64],
    trusted_key: &[u8; 32],
    target: &str,
    version: &str,
) -> Result<Manifest> {
    let key =
        VerifyingKey::from_bytes(trusted_key).map_err(|_| error("invalid trusted release key"))?;
    key.verify_strict(
        &release_signing_digest(manifest)?,
        &Signature::from_bytes(signature),
    )
    .map_err(|_| error("release signature does not match the pinned key"))?;
    let parsed: Manifest = serde_json::from_slice(manifest)?;
    if parsed.schema != "qualia.sensing-inspect.package.v1"
        || parsed.entrypoint != "qualia-sensing-inspect.exe"
        || parsed.source_version != version
        || parsed.target_declaration != target
        || ![
            "x86_64-pc-windows-msvc",
            "aarch64-pc-windows-msvc",
            "x86_64-pc-windows-gnu",
        ]
        .contains(&target)
        || parsed.supplied_binary_qualified
        || parsed.build_reproduced
        || parsed.integrity_is_authenticity
        || parsed.files.len() != FILES.len()
    {
        return fail("release manifest contract or expected version/target mismatch");
    }
    let mut seen = [false; FILES.len()];
    for entry in &parsed.files {
        let index = FILES
            .iter()
            .position(|name| *name == entry.path)
            .ok_or_else(|| error("unexpected manifest path"))?;
        if seen[index]
            || (entry.path == "qualia-sensing-inspect.exe" && entry.bytes == 0)
            || (entry.path == ".qualia-package-owner" && entry.bytes != 32)
            || entry.sha256.len() != 64
            || !entry
                .sha256
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return fail("duplicate path or invalid SHA-256 declaration");
        }
        seen[index] = true;
    }
    Ok(parsed)
}

/// Read-only bounded snapshot verification. Root ancestors and all input files
/// must remain protected through return. No file is executed or installed.
/// Key authority and allowed release version are independent caller decisions;
/// this function provides no key discovery, revocation or anti-rollback state.
pub fn verify_release(
    root: &Path,
    signature: &Path,
    trusted_key: &Path,
    target: &str,
    version: &str,
    byte_budget: usize,
) -> Result<serde_json::Value> {
    if byte_budget == 0
        || byte_budget > 1_073_741_824
        || version.is_empty()
        || version.len() > 64
        || target.len() > 64
    {
        return fail("invalid release verification limits");
    }
    for path in [root, signature, trusted_key] {
        local_file::validate_path(path)?;
    }
    let root_metadata = fs::symlink_metadata(root)?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return fail("regular package directory required");
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if root_metadata.file_attributes() & 0x400 != 0 {
            return fail("package directory reparse point refused");
        }
    }
    let mut names = [false; 7];
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name
            .to_str()
            .ok_or_else(|| error("non-Unicode package entry"))?;
        let index = if name == "package.json" {
            6
        } else {
            FILES
                .iter()
                .position(|expected| *expected == name)
                .ok_or_else(|| error("unexpected package entry"))?
        };
        if names[index] {
            return fail("duplicate package entry");
        }
        names[index] = true;
    }
    if names.contains(&false) {
        return fail("incomplete package");
    }
    let manifest = local_file::read(
        &root.join("package.json"),
        MAX_MANIFEST_BYTES.min(byte_budget),
    )?;
    let signature: [u8; 64] = local_file::read(signature, 64)?
        .try_into()
        .map_err(|_| error("release signature must contain exactly 64 raw bytes"))?;
    let trusted_key: [u8; 32] = local_file::read(trusted_key, 32)?
        .try_into()
        .map_err(|_| error("trusted public key must contain exactly 32 raw bytes"))?;
    let parsed = verify_manifest(&manifest, &signature, &trusted_key, target, version)?;
    let mut total = manifest.len();
    for entry in &parsed.files {
        let length = usize::try_from(entry.bytes).map_err(|_| error("file length overflow"))?;
        total = total
            .checked_add(length)
            .filter(|sum| *sum <= byte_budget)
            .ok_or_else(|| error("aggregate package byte limit"))?;
    }
    for entry in &parsed.files {
        // Names were matched against the fixed package layout before joining.
        let length = usize::try_from(entry.bytes).map_err(|_| error("file length overflow"))?;
        let data = local_file::read(&root.join(&entry.path), length.max(1))?;
        if data.len() as u64 != entry.bytes || hex::encode(Sha256::digest(&data)) != entry.sha256 {
            return fail("signed package content mismatch");
        }
    }
    Ok(serde_json::json!({
        "schema": "qualia.sensing-inspect.release-verification.v1",
        "release_signature_matches_caller_pin": true,
        "package_contents_match_signed_manifest": true,
        "pinned_public_key_sha256": hex::encode(Sha256::digest(trusted_key)),
        "signed_manifest_sha256": hex::encode(Sha256::digest(&manifest)),
        "expected_target": target, "expected_version": version, "encoded_bytes": total,
        "key_authority_independently_established": false, "release_revocation_checked": false,
        "rollback_protection_established": false, "binary_qualified": false,
        "execution_authorized": false, "snapshot_requires_protected_root": true
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn manifest() -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "schema": "qualia.sensing-inspect.package.v1", "source_version": "0.1.0",
            "target_declaration": "x86_64-pc-windows-msvc", "entrypoint": "qualia-sensing-inspect.exe",
            "supplied_binary_qualified": false, "build_reproduced": false, "integrity_is_authenticity": false,
            "files": FILES.iter().map(|name| serde_json::json!({"path": name,
                "bytes": if *name == ".qualia-package-owner" {32} else {1}, "sha256": "0".repeat(64)})).collect::<Vec<_>>()
        })).unwrap()
    }

    #[test]
    fn signed_manifest_requires_exact_bytes_caller_pin_target_and_version() {
        let bytes = manifest();
        let key = SigningKey::from_bytes(&[7; 32]);
        let signature = key
            .sign(&release_signing_digest(&bytes).unwrap())
            .to_bytes();
        let public = key.verifying_key().to_bytes();
        assert!(verify_manifest(
            &bytes,
            &signature,
            &public,
            "x86_64-pc-windows-msvc",
            "0.1.0"
        )
        .is_ok());
        assert!(verify_manifest(
            &bytes,
            &signature,
            &public,
            "aarch64-pc-windows-msvc",
            "0.1.0"
        )
        .is_err());
        assert!(verify_manifest(
            &bytes,
            &signature,
            &public,
            "x86_64-pc-windows-msvc",
            "0.0.1"
        )
        .is_err());
        let mut changed = bytes.clone();
        changed.push(b' ');
        assert!(verify_manifest(
            &changed,
            &signature,
            &public,
            "x86_64-pc-windows-msvc",
            "0.1.0"
        )
        .is_err());
        assert!(verify_manifest(
            &bytes,
            &signature,
            &SigningKey::from_bytes(&[8; 32]).verifying_key().to_bytes(),
            "x86_64-pc-windows-msvc",
            "0.1.0"
        )
        .is_err());
    }

    #[test]
    fn signed_manifest_rejects_weak_key_and_duplicate_file_paths() {
        let bytes = manifest();
        let mut weak = [0; 32];
        weak[0] = 1;
        let mut forged = [0; 64];
        forged[0] = 1;
        assert!(
            verify_manifest(&bytes, &forged, &weak, "x86_64-pc-windows-msvc", "0.1.0").is_err()
        );
        let mut parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        parsed["files"][1] = parsed["files"][0].clone();
        let changed = serde_json::to_vec(&parsed).unwrap();
        let key = SigningKey::from_bytes(&[7; 32]);
        let signature = key
            .sign(&release_signing_digest(&changed).unwrap())
            .to_bytes();
        assert!(verify_manifest(
            &changed,
            &signature,
            &key.verifying_key().to_bytes(),
            "x86_64-pc-windows-msvc",
            "0.1.0"
        )
        .is_err());
    }
}

#[cfg(test)]
#[path = "release_file_tests.rs"]
mod file_tests;
