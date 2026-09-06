//! Portable application manifest v1 (APP-02) + projection adapters (APP-03).
//!
//! Cold-construction schema for app identity, entry projections, required
//! capabilities/assets, state schema, permission intents, presentation hints,
//! compatibility, integrity digests, and update channel.
//!
//! [`project_manifest`] resolves one manifest into host-neutral Poet manifold,
//! Poet container, focused mini-app, and Desktop launch descriptors without a
//! projection-specific private database or divergent permission model.
//!
//! # Security invariants
//!
//! - Unknown manifest versions fail closed.
//! - Unknown permission wire values fail closed.
//! - Permission intents that exceed a host grant ceiling fail closed.
//! - Package path fields reject `..`, absolute escapes, and URL schemes.
//! - **Presentation hints are inert for authority** — they never grant
//!   permissions; see [`authority_from_presentation_hints`].
//! - Projection adapters authorize once and copy the same IDs / permission
//!   outcome into every descriptor.

mod codec;
mod error;
mod manifest;
mod paths;
mod permissions;
mod project;

pub use codec::APP_MANIFEST_MAGIC;
pub use error::AppManifestError;
pub use manifest::{
    sha256_into, sha256_of, AppAuthor, AppIdentity, Compatibility, EntryProjection, Integrity,
    PortableAppManifest, ProjectionKind, RequiredAsset, RequiredCapability, StateSchema,
    UpdateChannel, APP_MANIFEST_VERSION, MAX_ASSETS, MAX_CAPABILITIES, MAX_ENTRIES, MAX_FEATURES,
    MAX_HINTS, MAX_MANIFEST_BYTES, MAX_PERMISSIONS,
};
pub use paths::validate_package_relative_path;
pub use permissions::{
    authority_from_presentation_hints, check_permission_intents, PermissionGrant, PermissionIntent,
    PermissionKind, PresentationHint,
};
pub use project::{
    project_manifest, project_manifest_kinds, DesktopLaunchDescriptor, FocusedMiniAppProjection,
    PoetContainerProjection, PoetManifoldProjection, ProjectedApp, SharedProjectionIds,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_digest() -> [u8; 32] {
        sha256_of(b"portable-app-package-fixture-v1")
    }

    fn sample_manifest() -> PortableAppManifest {
        PortableAppManifest {
            identity: AppIdentity {
                app_id: "did:q42:app:health.proof".into(),
                version: "1.0.0".into(),
                author: AppAuthor {
                    name: "Web Civics".into(),
                    did: "did:q42:author:webcivics".into(),
                },
            },
            entries: vec![
                EntryProjection {
                    projection: ProjectionKind::Manifold,
                    entry_id: "health.overview".into(),
                    relative_path: "entries/overview.json".into(),
                },
                EntryProjection {
                    projection: ProjectionKind::FocusedMiniApp,
                    entry_id: "health.focused".into(),
                    relative_path: "entries/focused.json".into(),
                },
            ],
            required_capabilities: vec![RequiredCapability {
                id: "qualia.graph.query".into(),
                min_version: "0.0.36".into(),
            }],
            required_assets: vec![RequiredAsset {
                asset_id: "did:q42:asset:chebi:261".into(),
                expected_sha256: sha256_of(b"chebi-rel261"),
            }],
            state_schema: StateSchema {
                schema_id: "q42:HealthAppStateShape".into(),
                schema_version: "1".into(),
            },
            permission_intents: vec![
                PermissionIntent {
                    kind: PermissionKind::ReadLocalState,
                    scope: "*".into(),
                    optional: false,
                },
                PermissionIntent {
                    kind: PermissionKind::ReadLocalAsset,
                    scope: "did:q42:asset:chebi:261".into(),
                    optional: false,
                },
            ],
            presentation_hints: vec![PresentationHint {
                key: "theme".into(),
                value: "health-calm".into(),
            }],
            compatibility: Compatibility {
                min_engine_version: "0.0.36".into(),
                max_engine_version: String::new(),
                required_features: vec!["q42-assets".into()],
            },
            integrity: Integrity {
                package_sha256: sample_digest(),
            },
            update_channel: UpdateChannel {
                channel_id: "stable".into(),
                relative_feed: "update/channel.json".into(),
            },
        }
    }

    #[test]
    fn round_trip_preserves_manifest() {
        let original = sample_manifest();
        let bytes = original.encode().unwrap();
        let decoded = PortableAppManifest::decode(&bytes).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn deterministic_hash_is_stable() {
        let a = sample_manifest().manifest_digest().unwrap();
        let b = sample_manifest().manifest_digest().unwrap();
        assert_eq!(a, b);
        assert_ne!(a, [0u8; 32]);
        // Re-encode yields identical bytes → identical digest.
        let bytes_a = sample_manifest().encode().unwrap();
        let bytes_b = sample_manifest().encode().unwrap();
        assert_eq!(bytes_a, bytes_b);
        assert_eq!(sha256_of(&bytes_a), a);
    }

    #[test]
    fn malformed_magic_rejected() {
        let mut bytes = sample_manifest().encode().unwrap();
        bytes[0] = b'X';
        assert_eq!(
            PortableAppManifest::decode(&bytes),
            Err(AppManifestError::InvalidMagic)
        );
    }

    #[test]
    fn truncated_bytes_rejected() {
        let bytes = sample_manifest().encode().unwrap();
        assert_eq!(
            PortableAppManifest::decode(&bytes[..12]),
            Err(AppManifestError::Truncated)
        );
    }

    #[test]
    fn unknown_version_fails_closed() {
        let mut bytes = sample_manifest().encode().unwrap();
        // version u16 LE at offset 8
        bytes[8] = 99;
        bytes[9] = 0;
        assert_eq!(
            PortableAppManifest::decode(&bytes),
            Err(AppManifestError::UnsupportedVersion)
        );
    }

    #[test]
    fn unknown_permission_on_wire_fails_closed() {
        let mut crafted = sample_manifest();
        crafted.permission_intents = vec![PermissionIntent {
            kind: PermissionKind::ReadLocalState,
            scope: "*".into(),
            optional: false,
        }];
        let wire = crafted.encode().unwrap();
        // Patch the permission kind byte to 0xFF. Projection pads can look like
        // kind=1/optional=0, so confirm by decode error kind.
        let mut found = false;
        for i in 0..wire.len().saturating_sub(1) {
            if wire[i] == PermissionKind::ReadLocalState as u8 && wire[i + 1] == 0 {
                let mut candidate = wire.clone();
                candidate[i] = 0xFF;
                if PortableAppManifest::decode(&candidate)
                    == Err(AppManifestError::UnknownPermission)
                {
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "failed to locate permission kind byte");
    }

    #[test]
    fn permission_escalation_fails_closed() {
        let manifest = sample_manifest();
        let grant = PermissionGrant {
            allowed: vec![PermissionKind::ReadLocalState],
        };
        // Manifest also requires ReadLocalAsset → escalation.
        assert_eq!(
            check_permission_intents(&manifest.permission_intents, &grant),
            Err(AppManifestError::PermissionEscalation)
        );
        let full = PermissionGrant {
            allowed: vec![
                PermissionKind::ReadLocalState,
                PermissionKind::ReadLocalAsset,
            ],
        };
        assert!(check_permission_intents(&manifest.permission_intents, &full).is_ok());

        // NetworkEgress above a read-only grant.
        let escalate = [PermissionIntent {
            kind: PermissionKind::NetworkEgress,
            scope: "*".into(),
            optional: false,
        }];
        assert_eq!(
            check_permission_intents(&escalate, &full),
            Err(AppManifestError::PermissionEscalation)
        );
    }

    #[test]
    fn presentation_hints_are_inert_for_authority() {
        let mut manifest = sample_manifest();
        manifest.presentation_hints.push(PresentationHint {
            key: "grant".into(),
            value: "network_egress,admin,shell".into(),
        });
        manifest.presentation_hints.push(PresentationHint {
            key: "permission".into(),
            value: PermissionKind::NetworkEgress.as_str().into(),
        });
        let from_hints = authority_from_presentation_hints(&manifest.presentation_hints);
        assert!(
            from_hints.allowed.is_empty(),
            "hints must never produce a grant"
        );
        // Even with malicious-looking hints, required intents still need a real grant.
        assert_eq!(
            check_permission_intents(&manifest.permission_intents, &from_hints),
            Err(AppManifestError::PermissionEscalation)
        );
        // Round-trip still preserves hints as data only.
        let decoded = PortableAppManifest::decode(&manifest.encode().unwrap()).unwrap();
        assert_eq!(decoded.presentation_hints, manifest.presentation_hints);
    }

    #[test]
    fn path_traversal_in_entry_fails() {
        let mut manifest = sample_manifest();
        manifest.entries[0].relative_path = "../escape.bin".into();
        assert_eq!(
            manifest.validate(),
            Err(AppManifestError::PathTraversal)
        );
        assert_eq!(manifest.encode(), Err(AppManifestError::PathTraversal));
    }

    #[test]
    fn absolute_update_feed_fails() {
        let mut manifest = sample_manifest();
        manifest.update_channel.relative_feed = "/etc/passwd".into();
        assert_eq!(manifest.validate(), Err(AppManifestError::AbsolutePath));
    }

    #[test]
    fn missing_identity_fails() {
        let mut manifest = sample_manifest();
        manifest.identity.app_id.clear();
        assert_eq!(manifest.validate(), Err(AppManifestError::MissingAppId));
    }
}
