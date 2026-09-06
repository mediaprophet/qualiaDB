//! Installed-app registry and read-only inspection (WD-02).
//!
//! Cold, bounded in-memory registry of portable application manifests.
//! Registration decodes/validates, checks optional package digests, and
//! evaluates simple engine compatibility. Malformed packages are quarantined
//! when identity is recoverable; otherwise rejected. **Inspection never
//! launches or executes an app.**
//!
//! POET is the first bundled fixture via [`AppRegistry::with_bundled_poet_fixture`].

mod compat;
mod error;
mod fixture;
mod probe;
mod record;
mod registry;

pub use error::AppRegistryError;
pub use fixture::{poet_fixture_manifest, POET_APP_ID};
pub use record::{AppInspectView, AppRecordState, InstalledAppRecord, PermissionIntentSummary};
pub use registry::{AppRegistry, DEFAULT_ENGINE_VERSION, MAX_INSTALLED_APPS};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42::app_manifest::{
        sha256_of, AppAuthor, AppIdentity, Compatibility, EntryProjection, Integrity,
        PermissionIntent, PermissionKind, PortableAppManifest, ProjectionKind, RequiredCapability,
        StateSchema, UpdateChannel, APP_MANIFEST_MAGIC,
    };

    fn sample_digest() -> [u8; 32] {
        sha256_of(b"registry-test-package-v1")
    }

    fn sample_manifest(app_id: &str) -> PortableAppManifest {
        PortableAppManifest {
            identity: AppIdentity {
                app_id: app_id.into(),
                version: "1.0.0".into(),
                author: AppAuthor {
                    name: "Test Author".into(),
                    did: "did:q42:author:test".into(),
                },
            },
            entries: vec![EntryProjection {
                projection: ProjectionKind::Manifold,
                entry_id: "main".into(),
                relative_path: "entries/main.json".into(),
            }],
            required_capabilities: vec![RequiredCapability {
                id: "qualia.graph.query".into(),
                min_version: "0.0.36".into(),
            }],
            required_assets: vec![],
            state_schema: StateSchema {
                schema_id: "q42:TestState".into(),
                schema_version: "1".into(),
            },
            permission_intents: vec![PermissionIntent {
                kind: PermissionKind::ReadLocalState,
                scope: "*".into(),
                optional: false,
            }],
            presentation_hints: vec![],
            compatibility: Compatibility {
                min_engine_version: "0.0.36".into(),
                max_engine_version: String::new(),
                required_features: vec![],
            },
            integrity: Integrity {
                package_sha256: sample_digest(),
            },
            update_channel: UpdateChannel {
                channel_id: "none".into(),
                relative_feed: String::new(),
            },
        }
    }

    #[test]
    fn register_ok() {
        let mut reg = AppRegistry::new();
        let m = sample_manifest("did:q42:app:health.proof");
        let rec = reg.register_manifest(&m).unwrap();
        assert_eq!(rec.app_id, "did:q42:app:health.proof");
        assert!(rec.integrity_ok);
        assert_eq!(rec.state, AppRecordState::Active);
        assert_eq!(reg.len(), 1);
        assert!(reg.get("did:q42:app:health.proof").is_some());
    }

    #[test]
    fn malformed_quarantined_when_identity_recoverable() {
        let mut reg = AppRegistry::new();
        let mut bytes = sample_manifest("did:q42:app:broken").encode().unwrap();
        // Trailing garbage keeps identity probeable but fails closed on decode.
        bytes.push(0xFF);
        let rec = reg
            .register_manifest_bytes(&bytes, None)
            .expect("quarantine slot");
        assert_eq!(rec.app_id, "did:q42:app:broken");
        assert!(rec.state.is_quarantined());
        assert!(!rec.integrity_ok);
    }

    #[test]
    fn malformed_rejected_when_identity_unrecoverable() {
        let mut reg = AppRegistry::new();
        let garbage = b"not-a-manifest";
        assert_eq!(
            reg.register_manifest_bytes(garbage, None),
            Err(AppRegistryError::RejectedMalformed)
        );
        assert!(reg.is_empty());
    }

    #[test]
    fn digest_mismatch_quarantines() {
        let mut reg = AppRegistry::new();
        let m = sample_manifest("did:q42:app:digest");
        let bytes = m.encode().unwrap();
        let wrong = sha256_of(b"wrong-package-bytes");
        let rec = reg.register_manifest_bytes(&bytes, Some(&wrong)).unwrap();
        assert!(rec.state.is_quarantined());
        assert!(!rec.integrity_ok);
        assert_eq!(rec.digest, sample_digest());
    }

    #[test]
    fn digest_match_active() {
        let mut reg = AppRegistry::new();
        let m = sample_manifest("did:q42:app:ok");
        let bytes = m.encode().unwrap();
        let digest = sample_digest();
        let rec = reg.register_manifest_bytes(&bytes, Some(&digest)).unwrap();
        assert_eq!(rec.state, AppRecordState::Active);
        assert!(rec.integrity_ok);
    }

    #[test]
    fn incompatible_when_min_engine_too_new() {
        let mut reg = AppRegistry::with_engine_version("0.0.36");
        let mut m = sample_manifest("did:q42:app:future");
        m.compatibility.min_engine_version = "0.0.99".into();
        let rec = reg.register_manifest(&m).unwrap();
        assert_eq!(rec.state, AppRecordState::Incompatible);
        assert!(rec.integrity_ok);
    }

    #[test]
    fn list_bounded_at_capacity() {
        let mut reg = AppRegistry::new();
        for i in 0..MAX_INSTALLED_APPS {
            let id = format!("did:q42:app:slot{i}");
            reg.register_manifest(&sample_manifest(&id)).unwrap();
        }
        assert_eq!(reg.len(), MAX_INSTALLED_APPS);
        assert_eq!(reg.list().len(), MAX_INSTALLED_APPS);
        let overflow = sample_manifest("did:q42:app:overflow");
        assert_eq!(
            reg.register_manifest(&overflow),
            Err(AppRegistryError::RegistryFull)
        );
        // Upsert same id still works without growing.
        reg.register_manifest(&sample_manifest("did:q42:app:slot0"))
            .unwrap();
        assert_eq!(reg.len(), MAX_INSTALLED_APPS);
    }

    #[test]
    fn inspect_does_not_mutate() {
        let mut reg = AppRegistry::new();
        let m = sample_manifest("did:q42:app:inspect");
        reg.register_manifest(&m).unwrap();
        let before = reg.len();
        let view = reg.inspect("did:q42:app:inspect").unwrap();
        assert_eq!(view.app_id, "did:q42:app:inspect");
        assert_eq!(view.permission_intents.len(), 1);
        assert_eq!(view.entry_ids, vec!["main".to_string()]);
        assert_eq!(view.state, AppRecordState::Active);
        // Re-inspect and list — length unchanged, state unchanged.
        let _ = reg.inspect("did:q42:app:inspect");
        assert_eq!(reg.len(), before);
        assert_eq!(
            reg.get("did:q42:app:inspect").unwrap().state,
            AppRecordState::Active
        );
        // inspect is &self — no launch side effects; magic constant still unused here
        // as a reminder that registry never opens APP_MANIFEST_MAGIC as code.
        let _ = APP_MANIFEST_MAGIC;
    }

    #[test]
    fn poet_fixture_present() {
        let reg = AppRegistry::with_bundled_poet_fixture();
        let poet = reg.get(POET_APP_ID).expect("poet bundled");
        assert_eq!(poet.app_id, POET_APP_ID);
        assert_eq!(poet.state, AppRecordState::Active);
        assert!(poet.integrity_ok);
        let view = reg.inspect(POET_APP_ID).unwrap();
        assert!(view
            .permission_summary
            .kinds
            .contains(&"read_local_state"));
        // First occupied slot is POET.
        assert_eq!(reg.list()[0].app_id, POET_APP_ID);
    }
}
