//! Bundled POET fixture manifest (test / seed only — not on-disk package bytes).

use crate::q42::app_manifest::{
    sha256_of, AppAuthor, AppIdentity, Compatibility, EntryProjection, Integrity,
    PermissionIntent, PermissionKind, PortableAppManifest, PresentationHint, ProjectionKind,
    RequiredCapability, StateSchema, UpdateChannel,
};

/// Canonical bundled POET app id for registry seeding.
pub const POET_APP_ID: &str = "did:q42:app:poet";

/// Build a valid in-memory POET portable manifest fixture.
///
/// This does **not** claim real package bytes exist on disk; the integrity
/// digest is a fixture hash over a well-known label string.
pub fn poet_fixture_manifest() -> PortableAppManifest {
    let package_label = b"poet-bundled-fixture-package-v0";
    PortableAppManifest {
        identity: AppIdentity {
            app_id: POET_APP_ID.into(),
            version: "0.0.36".into(),
            author: AppAuthor {
                name: "Web Civics".into(),
                did: "did:q42:author:webcivics".into(),
            },
        },
        entries: vec![EntryProjection {
            projection: ProjectionKind::Manifold,
            entry_id: "poet.home".into(),
            relative_path: "entries/home.json".into(),
        }],
        required_capabilities: vec![RequiredCapability {
            id: "qualia.graph.query".into(),
            min_version: "0.0.36".into(),
        }],
        required_assets: vec![],
        state_schema: StateSchema {
            schema_id: "q42:PoetAppStateShape".into(),
            schema_version: "1".into(),
        },
        permission_intents: vec![
            PermissionIntent {
                kind: PermissionKind::ReadLocalState,
                scope: "*".into(),
                optional: false,
            },
            PermissionIntent {
                kind: PermissionKind::IdentityRead,
                scope: "self".into(),
                optional: true,
            },
        ],
        presentation_hints: vec![PresentationHint {
            key: "theme".into(),
            value: "poet-default".into(),
        }],
        compatibility: Compatibility {
            min_engine_version: "0.0.36".into(),
            max_engine_version: String::new(),
            required_features: vec!["q42-apps".into()],
        },
        integrity: Integrity {
            package_sha256: sha256_of(package_label),
        },
        update_channel: UpdateChannel {
            channel_id: "none".into(),
            relative_feed: String::new(),
        },
    }
}
