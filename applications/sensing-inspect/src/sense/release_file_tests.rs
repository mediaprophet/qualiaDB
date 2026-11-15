//! Source-only fixtures for the public, combined signature/content boundary.
use super::*;
use ed25519_dalek::{Signer, SigningKey};
use std::path::PathBuf;

// Cover GNU package admission here; manifest-level tests retain the MSVC cases.
const TARGET: &str = "x86_64-pc-windows-gnu";
const VERSION: &str = "0.1.0";
// Fixed tiny fixture payloads plus a 16KiB manifest bound fixture production.
const FIXTURE_BUDGET: usize = 32_768;

struct PackageFixture {
    _directory: tempfile::TempDir,
    root: PathBuf,
    signature: PathBuf,
    public_key: PathBuf,
    key: SigningKey,
    manifest: serde_json::Value,
}

impl PackageFixture {
    fn new() -> Self {
        let directory = tempfile::Builder::new()
            .prefix("qualia-release-fixture-")
            .tempdir()
            .unwrap();
        let root = directory.path().join("package");
        fs::create_dir(&root).unwrap();
        let mut entries = Vec::new();
        for name in FILES {
            // These bytes are inert fixtures; no executable is invoked.
            let data = if name == ".qualia-package-owner" {
                &[0x51; 32][..]
            } else {
                b"fixture"
            };
            fs::write(root.join(name), data).unwrap();
            entries.push(serde_json::json!({"path": name, "bytes": data.len(),
                "sha256": hex::encode(Sha256::digest(data))}));
        }
        let key = SigningKey::from_bytes(&[11; 32]);
        let public_key = directory.path().join("trusted-key.bin");
        let signature = directory.path().join("signature.bin");
        fs::write(&public_key, key.verifying_key().to_bytes()).unwrap();
        let fixture = Self {
            root,
            signature,
            public_key,
            key,
            _directory: directory,
            manifest: serde_json::json!({
                "schema": "qualia.sensing-inspect.package.v1", "source_version": VERSION,
                "target_declaration": TARGET, "entrypoint": "qualia-sensing-inspect.exe",
                "supplied_binary_qualified": false, "build_reproduced": false,
                "integrity_is_authenticity": false, "files": entries
            }),
        };
        fixture.write_manifest(&serde_json::to_vec(&fixture.manifest).unwrap());
        fixture
    }

    fn write_manifest(&self, bytes: &[u8]) {
        assert!(bytes.len() <= MAX_MANIFEST_BYTES);
        fs::write(self.root.join("package.json"), bytes).unwrap();
        fs::write(
            &self.signature,
            self.key
                .sign(&release_signing_digest(bytes).unwrap())
                .to_bytes(),
        )
        .unwrap();
    }

    fn verify(&self, budget: usize) -> Result<serde_json::Value> {
        verify_release(
            &self.root,
            &self.signature,
            &self.public_key,
            TARGET,
            VERSION,
            budget,
        )
    }
}

#[test]
fn public_verifier_binds_signature_to_all_package_bytes_with_exact_aggregate_budget() {
    let fixture = PackageFixture::new();
    let report = fixture.verify(FIXTURE_BUDGET).unwrap();
    assert_eq!(report["release_signature_matches_caller_pin"], true);
    assert_eq!(report["package_contents_match_signed_manifest"], true);
    assert_eq!(report["execution_authorized"], false);
    let exact = report["encoded_bytes"].as_u64().unwrap() as usize;
    assert!(fixture.verify(exact).is_ok());
    assert!(fixture.verify(exact - 1).is_err());
    // Equal-length replacement exercises hashing rather than length rejection.
    fs::write(fixture.root.join("qualia-sensing-inspect.exe"), b"changed").unwrap();
    assert!(fixture.verify(FIXTURE_BUDGET).is_err());
}

#[test]
fn public_verifier_rejects_unlisted_missing_and_nonregular_package_files() {
    let fixture = PackageFixture::new();
    let extra = fixture.root.join("unexpected.bin");
    fs::write(&extra, b"x").unwrap();
    assert!(fixture.verify(FIXTURE_BUDGET).is_err());
    fs::remove_file(extra).unwrap();
    let payload = fixture.root.join("README.md");
    fs::remove_file(&payload).unwrap();
    assert!(fixture.verify(FIXTURE_BUDGET).is_err());
    fs::create_dir(payload).unwrap();
    assert!(fixture.verify(FIXTURE_BUDGET).is_err());
}

#[test]
fn public_verifier_rejects_truncated_key_and_signature_files() {
    let fixture = PackageFixture::new();
    fs::write(&fixture.public_key, [0; 31]).unwrap();
    assert!(fixture.verify(FIXTURE_BUDGET).is_err());
    fs::write(&fixture.public_key, fixture.key.verifying_key().to_bytes()).unwrap();
    fs::write(&fixture.signature, [0; 63]).unwrap();
    assert!(fixture.verify(FIXTURE_BUDGET).is_err());
}

#[test]
fn public_verifier_rejects_signed_duplicate_unknown_properties_and_invalid_file_contracts() {
    let mut fixture = PackageFixture::new();
    let original = serde_json::to_vec(&fixture.manifest).unwrap();
    for property in [
        br#""schema":"qualia.sensing-inspect.package.v1","#.as_slice(),
        br#""unknown":false,"#.as_slice(),
    ] {
        let mut bytes = vec![b'{'];
        bytes.extend_from_slice(property);
        bytes.extend_from_slice(&original[1..]);
        fixture.write_manifest(&bytes);
        assert!(fixture.verify(FIXTURE_BUDGET).is_err());
    }
    let text = std::str::from_utf8(&original).unwrap();
    for prefix in [
        r#""path":".qualia-package-owner","path":"#,
        r#""unknown":false,"path":"#,
    ] {
        let changed = text.replacen(r#""path":"#, prefix, 1);
        fixture.write_manifest(changed.as_bytes());
        assert!(fixture.verify(FIXTURE_BUDGET).is_err());
    }
    fixture.manifest["files"][0]["bytes"] = 31.into();
    fixture.write_manifest(&serde_json::to_vec(&fixture.manifest).unwrap());
    assert!(fixture.verify(FIXTURE_BUDGET).is_err());
    fixture.manifest["files"][0]["bytes"] = 32.into();
    fixture.manifest["files"][1]["bytes"] = 0.into();
    fixture.write_manifest(&serde_json::to_vec(&fixture.manifest).unwrap());
    assert!(fixture.verify(FIXTURE_BUDGET).is_err());
}

#[test]
fn package_fixture_cleans_up_after_success_failure_and_unwind() {
    let success_path = {
        let fixture = PackageFixture::new();
        fixture.verify(FIXTURE_BUDGET).unwrap();
        fixture._directory.path().to_path_buf()
    };
    assert!(!success_path.exists());
    let failure_path = {
        let fixture = PackageFixture::new();
        assert!(fixture.verify(1).is_err());
        fixture._directory.path().to_path_buf()
    };
    assert!(!failure_path.exists());
    let mut unwind_path = None;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let fixture = PackageFixture::new();
        unwind_path = Some(fixture._directory.path().to_path_buf());
        panic!("exercise fixture cleanup during unwind");
    }));
    assert!(result.is_err());
    assert!(!unwind_path.unwrap().exists());
}
