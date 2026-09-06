//! Projection adapters (APP-03).
//!
//! Resolve one [`PortableAppManifest`] into host-neutral descriptors for Poet
//! manifold, Poet container, focused mini-app, and Webizen Desktop launch —
//! without a projection-specific private database or divergent permission model.
//!
//! # Invariants
//!
//! - Authorization is evaluated once via [`check_permission_intents`].
//! - App / capability / asset / state-schema IDs and authorized permission kinds
//!   are identical across every emitted descriptor.
//! - Presentation hints copy as display data only; they never alter the grant.

use super::error::AppManifestError;
use super::manifest::{
    PortableAppManifest, ProjectionKind, RequiredAsset, RequiredCapability, StateSchema,
};
use super::permissions::{
    check_permission_intents, PermissionGrant, PermissionKind, PresentationHint,
};

/// Shared identity + authorization outcome for every projection of one manifest.
///
/// This is the single authority snapshot; descriptors may copy fields for
/// host convenience but must not invent a second grant or private store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedProjectionIds {
    pub app_id: String,
    pub version: String,
    pub author_name: String,
    pub author_did: String,
    pub capability_ids: Vec<String>,
    pub asset_ids: Vec<String>,
    pub state_schema: StateSchema,
    /// Permission kinds that satisfied required intents under the host grant.
    pub permission_kinds: Vec<PermissionKind>,
    /// Display-only presentation hints (never authority).
    pub presentation_hints: Vec<PresentationHint>,
}

/// Poet manifold entry projection (host-neutral; not a live `ManifoldSeed`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoetManifoldProjection {
    pub entry_id: String,
    pub app_id: String,
    pub version: String,
    pub relative_path: String,
    pub capability_ids: Vec<String>,
    pub asset_ids: Vec<String>,
    pub state_schema: StateSchema,
    pub permission_kinds: Vec<PermissionKind>,
    pub presentation_hints: Vec<PresentationHint>,
}

/// Poet container placement / tool projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoetContainerProjection {
    pub entry_id: String,
    pub app_id: String,
    pub version: String,
    pub relative_path: String,
    pub capability_ids: Vec<String>,
    pub asset_ids: Vec<String>,
    pub state_schema: StateSchema,
    pub permission_kinds: Vec<PermissionKind>,
    pub presentation_hints: Vec<PresentationHint>,
}

/// Focused mini-app entry projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusedMiniAppProjection {
    pub entry_id: String,
    pub app_id: String,
    pub version: String,
    pub relative_path: String,
    pub capability_ids: Vec<String>,
    pub asset_ids: Vec<String>,
    pub state_schema: StateSchema,
    pub permission_kinds: Vec<PermissionKind>,
    pub presentation_hints: Vec<PresentationHint>,
}

/// Webizen Desktop Apps launch / inspect descriptor (no lifecycle execution).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopLaunchDescriptor {
    pub entry_id: String,
    pub app_id: String,
    pub version: String,
    pub relative_path: String,
    pub capability_ids: Vec<String>,
    pub asset_ids: Vec<String>,
    pub state_schema: StateSchema,
    pub permission_kinds: Vec<PermissionKind>,
    pub presentation_hints: Vec<PresentationHint>,
}

/// Full cold projection of one portable app under a host grant.
///
/// Absent kinds are honest empty (`None` / empty). No projection owns private
/// state or a separate permission table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedApp {
    pub shared: SharedProjectionIds,
    pub manifold: Option<PoetManifoldProjection>,
    pub container: Option<PoetContainerProjection>,
    pub focused_mini_app: Option<FocusedMiniAppProjection>,
    pub desktop_launch: Option<DesktopLaunchDescriptor>,
}

impl ProjectedApp {
    /// Number of concrete projection descriptors emitted.
    pub fn descriptor_count(&self) -> usize {
        usize::from(self.manifold.is_some())
            + usize::from(self.container.is_some())
            + usize::from(self.focused_mini_app.is_some())
            + usize::from(self.desktop_launch.is_some())
    }
}

/// Project every entry kind present on the manifest.
pub fn project_manifest(
    manifest: &PortableAppManifest,
    grant: &PermissionGrant,
) -> Result<ProjectedApp, AppManifestError> {
    project_manifest_kinds(manifest, grant, None)
}

/// Project only the requested kinds (still authorize the full intent set).
///
/// When `kinds` is `None`, every matching entry kind is projected. Permission
/// denial is identical regardless of which kinds are requested — escalation is
/// checked before filtering.
pub fn project_manifest_kinds(
    manifest: &PortableAppManifest,
    grant: &PermissionGrant,
    kinds: Option<&[ProjectionKind]>,
) -> Result<ProjectedApp, AppManifestError> {
    manifest.validate()?;
    // Fail closed before any descriptor is built — same result for any kind filter.
    check_permission_intents(&manifest.permission_intents, grant)?;

    let shared = build_shared(manifest, grant);
    let want = |kind: ProjectionKind| kinds.map(|ks| ks.contains(&kind)).unwrap_or(true);

    let mut manifold = None;
    let mut container = None;
    let mut focused_mini_app = None;
    let mut desktop_launch = None;

    for entry in &manifest.entries {
        if !want(entry.projection) {
            continue;
        }
        if entry.entry_id.trim().is_empty() || entry.relative_path.trim().is_empty() {
            return Err(AppManifestError::InvalidProjection);
        }
        match entry.projection {
            ProjectionKind::Manifold if manifold.is_none() => {
                manifold = Some(PoetManifoldProjection {
                    entry_id: entry.entry_id.clone(),
                    app_id: shared.app_id.clone(),
                    version: shared.version.clone(),
                    relative_path: entry.relative_path.clone(),
                    capability_ids: shared.capability_ids.clone(),
                    asset_ids: shared.asset_ids.clone(),
                    state_schema: shared.state_schema.clone(),
                    permission_kinds: shared.permission_kinds.clone(),
                    presentation_hints: shared.presentation_hints.clone(),
                });
            }
            ProjectionKind::Container if container.is_none() => {
                container = Some(PoetContainerProjection {
                    entry_id: entry.entry_id.clone(),
                    app_id: shared.app_id.clone(),
                    version: shared.version.clone(),
                    relative_path: entry.relative_path.clone(),
                    capability_ids: shared.capability_ids.clone(),
                    asset_ids: shared.asset_ids.clone(),
                    state_schema: shared.state_schema.clone(),
                    permission_kinds: shared.permission_kinds.clone(),
                    presentation_hints: shared.presentation_hints.clone(),
                });
            }
            ProjectionKind::FocusedMiniApp if focused_mini_app.is_none() => {
                focused_mini_app = Some(FocusedMiniAppProjection {
                    entry_id: entry.entry_id.clone(),
                    app_id: shared.app_id.clone(),
                    version: shared.version.clone(),
                    relative_path: entry.relative_path.clone(),
                    capability_ids: shared.capability_ids.clone(),
                    asset_ids: shared.asset_ids.clone(),
                    state_schema: shared.state_schema.clone(),
                    permission_kinds: shared.permission_kinds.clone(),
                    presentation_hints: shared.presentation_hints.clone(),
                });
            }
            ProjectionKind::DesktopHost if desktop_launch.is_none() => {
                desktop_launch = Some(DesktopLaunchDescriptor {
                    entry_id: entry.entry_id.clone(),
                    app_id: shared.app_id.clone(),
                    version: shared.version.clone(),
                    relative_path: entry.relative_path.clone(),
                    capability_ids: shared.capability_ids.clone(),
                    asset_ids: shared.asset_ids.clone(),
                    state_schema: shared.state_schema.clone(),
                    permission_kinds: shared.permission_kinds.clone(),
                    presentation_hints: shared.presentation_hints.clone(),
                });
            }
            // Duplicate kind: keep first; later entries of the same kind are ignored.
            _ => {}
        }
    }

    Ok(ProjectedApp {
        shared,
        manifold,
        container,
        focused_mini_app,
        desktop_launch,
    })
}

fn build_shared(manifest: &PortableAppManifest, grant: &PermissionGrant) -> SharedProjectionIds {
    let capability_ids = manifest
        .required_capabilities
        .iter()
        .map(|RequiredCapability { id, .. }| id.clone())
        .collect();
    let asset_ids = manifest
        .required_assets
        .iter()
        .map(|RequiredAsset { asset_id, .. }| asset_id.clone())
        .collect();
    // Authorized kinds = required intents that the grant allows (optional intents
    // included only when granted). Stable order follows manifest intent order.
    let mut permission_kinds = Vec::new();
    for intent in &manifest.permission_intents {
        if grant.allows(intent.kind) && !permission_kinds.contains(&intent.kind) {
            permission_kinds.push(intent.kind);
        }
    }

    SharedProjectionIds {
        app_id: manifest.identity.app_id.clone(),
        version: manifest.identity.version.clone(),
        author_name: manifest.identity.author.name.clone(),
        author_did: manifest.identity.author.did.clone(),
        capability_ids,
        asset_ids,
        state_schema: manifest.state_schema.clone(),
        permission_kinds,
        presentation_hints: manifest.presentation_hints.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42::app_manifest::manifest::{
        sha256_of, AppAuthor, AppIdentity, Compatibility, EntryProjection, Integrity,
        RequiredAsset, RequiredCapability, UpdateChannel,
    };
    use crate::q42::app_manifest::permissions::{
        authority_from_presentation_hints, PermissionIntent,
    };

    fn sample_digest() -> [u8; 32] {
        sha256_of(b"portable-app-package-fixture-v1")
    }

    fn base_manifest(entries: Vec<EntryProjection>) -> PortableAppManifest {
        PortableAppManifest {
            identity: AppIdentity {
                app_id: "did:q42:app:health.proof".into(),
                version: "1.0.0".into(),
                author: AppAuthor {
                    name: "Web Civics".into(),
                    did: "did:q42:author:webcivics".into(),
                },
            },
            entries,
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

    fn all_four_entries() -> Vec<EntryProjection> {
        vec![
            EntryProjection {
                projection: ProjectionKind::Manifold,
                entry_id: "health.overview".into(),
                relative_path: "entries/overview.json".into(),
            },
            EntryProjection {
                projection: ProjectionKind::Container,
                entry_id: "health.measure".into(),
                relative_path: "entries/measure.json".into(),
            },
            EntryProjection {
                projection: ProjectionKind::FocusedMiniApp,
                entry_id: "health.focused".into(),
                relative_path: "entries/focused.json".into(),
            },
            EntryProjection {
                projection: ProjectionKind::DesktopHost,
                entry_id: "health.desktop".into(),
                relative_path: "entries/desktop.json".into(),
            },
        ]
    }

    fn full_grant() -> PermissionGrant {
        PermissionGrant {
            allowed: vec![
                PermissionKind::ReadLocalState,
                PermissionKind::ReadLocalAsset,
            ],
        }
    }

    fn assert_shared_ids(shared: &SharedProjectionIds, app_id: &str, version: &str) {
        assert_eq!(shared.app_id, app_id);
        assert_eq!(shared.version, version);
        assert_eq!(shared.capability_ids, vec!["qualia.graph.query".to_string()]);
        assert_eq!(
            shared.asset_ids,
            vec!["did:q42:asset:chebi:261".to_string()]
        );
        assert_eq!(shared.state_schema.schema_id, "q42:HealthAppStateShape");
        assert_eq!(
            shared.permission_kinds,
            vec![
                PermissionKind::ReadLocalState,
                PermissionKind::ReadLocalAsset
            ]
        );
    }

    #[test]
    fn conformance_all_four_kinds_share_ids() {
        let manifest = base_manifest(all_four_entries());
        let projected = project_manifest(&manifest, &full_grant()).unwrap();
        assert_eq!(projected.descriptor_count(), 4);
        assert_shared_ids(
            &projected.shared,
            "did:q42:app:health.proof",
            "1.0.0",
        );

        let m = projected.manifold.as_ref().unwrap();
        let c = projected.container.as_ref().unwrap();
        let f = projected.focused_mini_app.as_ref().unwrap();
        let d = projected.desktop_launch.as_ref().unwrap();

        assert_eq!(m.entry_id, "health.overview");
        assert_eq!(c.entry_id, "health.measure");
        assert_eq!(f.entry_id, "health.focused");
        assert_eq!(d.entry_id, "health.desktop");

        for (app_id, version, caps, assets, schema, perms) in [
            (
                &m.app_id,
                &m.version,
                &m.capability_ids,
                &m.asset_ids,
                &m.state_schema,
                &m.permission_kinds,
            ),
            (
                &c.app_id,
                &c.version,
                &c.capability_ids,
                &c.asset_ids,
                &c.state_schema,
                &c.permission_kinds,
            ),
            (
                &f.app_id,
                &f.version,
                &f.capability_ids,
                &f.asset_ids,
                &f.state_schema,
                &f.permission_kinds,
            ),
            (
                &d.app_id,
                &d.version,
                &d.capability_ids,
                &d.asset_ids,
                &d.state_schema,
                &d.permission_kinds,
            ),
        ] {
            assert_eq!(app_id, &projected.shared.app_id);
            assert_eq!(version, &projected.shared.version);
            assert_eq!(caps, &projected.shared.capability_ids);
            assert_eq!(assets, &projected.shared.asset_ids);
            assert_eq!(schema, &projected.shared.state_schema);
            assert_eq!(perms, &projected.shared.permission_kinds);
        }
    }

    #[test]
    fn conformance_missing_kinds_are_honest_empty() {
        let manifest = base_manifest(vec![
            EntryProjection {
                projection: ProjectionKind::Manifold,
                entry_id: "only.manifold".into(),
                relative_path: "entries/overview.json".into(),
            },
            EntryProjection {
                projection: ProjectionKind::FocusedMiniApp,
                entry_id: "only.focused".into(),
                relative_path: "entries/focused.json".into(),
            },
        ]);
        let projected = project_manifest(&manifest, &full_grant()).unwrap();
        assert_eq!(projected.descriptor_count(), 2);
        assert!(projected.manifold.is_some());
        assert!(projected.container.is_none());
        assert!(projected.focused_mini_app.is_some());
        assert!(projected.desktop_launch.is_none());
        assert_eq!(
            projected.manifold.as_ref().unwrap().app_id,
            projected.focused_mini_app.as_ref().unwrap().app_id
        );
    }

    #[test]
    fn conformance_permission_denial_identical_across_kind_filters() {
        let manifest = base_manifest(all_four_entries());
        let short = PermissionGrant {
            allowed: vec![PermissionKind::ReadLocalState],
        };
        let deny_all = project_manifest(&manifest, &short);
        let deny_manifold = project_manifest_kinds(
            &manifest,
            &short,
            Some(&[ProjectionKind::Manifold]),
        );
        let deny_desktop = project_manifest_kinds(
            &manifest,
            &short,
            Some(&[ProjectionKind::DesktopHost]),
        );
        let deny_focused = project_manifest_kinds(
            &manifest,
            &short,
            Some(&[ProjectionKind::FocusedMiniApp]),
        );
        assert_eq!(deny_all, Err(AppManifestError::PermissionEscalation));
        assert_eq!(deny_all, deny_manifold);
        assert_eq!(deny_all, deny_desktop);
        assert_eq!(deny_all, deny_focused);
    }

    #[test]
    fn conformance_hints_cannot_escalate() {
        let mut manifest = base_manifest(all_four_entries());
        manifest.presentation_hints.push(PresentationHint {
            key: "grant".into(),
            value: "network_egress,admin".into(),
        });
        manifest.presentation_hints.push(PresentationHint {
            key: "permission".into(),
            value: PermissionKind::NetworkEgress.as_str().into(),
        });

        let from_hints = authority_from_presentation_hints(&manifest.presentation_hints);
        assert!(from_hints.allowed.is_empty());
        assert_eq!(
            project_manifest(&manifest, &from_hints),
            Err(AppManifestError::PermissionEscalation)
        );

        // With a real grant, hints appear as display data only — never as new kinds.
        let projected = project_manifest(&manifest, &full_grant()).unwrap();
        assert!(projected
            .shared
            .presentation_hints
            .iter()
            .any(|h| h.key == "grant"));
        assert!(!projected
            .shared
            .permission_kinds
            .contains(&PermissionKind::NetworkEgress));
        assert_eq!(
            projected.manifold.as_ref().unwrap().presentation_hints,
            projected.shared.presentation_hints
        );
    }

    #[test]
    fn conformance_kind_filter_emits_subset_with_same_shared_ids() {
        let manifest = base_manifest(all_four_entries());
        let projected = project_manifest_kinds(
            &manifest,
            &full_grant(),
            Some(&[ProjectionKind::Container, ProjectionKind::DesktopHost]),
        )
        .unwrap();
        assert_eq!(projected.descriptor_count(), 2);
        assert!(projected.manifold.is_none());
        assert!(projected.focused_mini_app.is_none());
        assert!(projected.container.is_some());
        assert!(projected.desktop_launch.is_some());
        assert_eq!(
            projected.container.as_ref().unwrap().permission_kinds,
            projected.desktop_launch.as_ref().unwrap().permission_kinds
        );
        assert_eq!(
            projected.container.as_ref().unwrap().app_id,
            projected.shared.app_id
        );
    }

    #[test]
    fn invalid_empty_entry_id_fails_closed_at_validate() {
        let mut manifest = base_manifest(all_four_entries());
        manifest.entries[0].entry_id.clear();
        assert_eq!(
            project_manifest(&manifest, &full_grant()),
            Err(AppManifestError::EmptyEntries)
        );
    }

    #[test]
    fn unknown_projection_fails_at_decode() {
        assert_eq!(
            ProjectionKind::from_u8(0),
            Err(AppManifestError::InvalidProjection)
        );
        assert_eq!(
            ProjectionKind::from_u8(99),
            Err(AppManifestError::InvalidProjection)
        );
    }
}
