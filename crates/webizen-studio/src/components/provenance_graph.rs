//! W3C PROV-O & Jury-Safe Four-Plane Provenance Graph Explorer.

#![allow(non_snake_case)]

use aether::prelude::*;

use crate::components::honesty_chip::{HonestyChip, HonestyLevel};
use crate::components::qapp_engine::invoke_json;
use crate::components::settings::identity_plane::{
    IdentifierFabricPlane, DID_Q42_QRC_LABEL, VC_ORIGIN_INTEGRITY_DISCLAIMER,
};

pub const TRI_SOURCE_ETALON_BADGE: &str = "Etalon Oracle";
pub const TRI_SOURCE_QDNF_BADGE: &str = "QDNF Network";
pub const TRI_SOURCE_QUALIADB_BADGE: &str = "QualiaDB .q42 v4";

pub const TRI_SOURCE_BADGES: [&str; 3] = [
    TRI_SOURCE_ETALON_BADGE,
    TRI_SOURCE_QDNF_BADGE,
    TRI_SOURCE_QUALIADB_BADGE,
];

pub const FLAG_PERMISSIVE_COMMONS: u16 = 0x0020;
pub const FLAG_SANCTUARY: u16 = 0x0040;

/// Publication lock state for `.q42` volumes and provenance graphs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationLockState {
    /// `FLAG_SANCTUARY` (`0x0040`) — fail-closed against public distribution or magnets.
    SanctuaryLocked,
    /// `FLAG_PERMISSIVE_COMMONS` (`0x0020`) without Sanctuary — eligible for public commons.
    PermissiveCommons,
    /// Unclassified or mixed state — defaults to fail-closed.
    UnclassifiedFailClosed,
}

impl PublicationLockState {
    pub const fn badge_label(self) -> &'static str {
        match self {
            Self::SanctuaryLocked => "FLAG_SANCTUARY (0x0040) — Public Distribution Locked",
            Self::PermissiveCommons => "FLAG_PERMISSIVE_COMMONS (0x0020) — Commons Permissive",
            Self::UnclassifiedFailClosed => "Unclassified — Public Distribution Fail-Closed",
        }
    }

    pub const fn allows_public_distribution(self) -> bool {
        matches!(self, Self::PermissiveCommons)
    }
}

/// Evaluates `.q42` header flags and publication class, enforcing `FLAG_SANCTUARY` precedence
/// even if a caller also sets `FLAG_PERMISSIVE_COMMONS`.
pub fn resolve_publication_lock_state(flags: u16, publication_class: &str) -> PublicationLockState {
    let lower = publication_class.to_ascii_lowercase();
    let has_sanctuary = (flags & FLAG_SANCTUARY) != 0
        || lower.contains("sanctuary")
        || lower.contains("medical")
        || lower.contains("bilateral")
        || lower.contains("clinical")
        || lower.contains("guardianship");
    if has_sanctuary {
        return PublicationLockState::SanctuaryLocked;
    }
    let has_commons = (flags & FLAG_PERMISSIVE_COMMONS) != 0 || lower.contains("commons");
    if has_commons {
        PublicationLockState::PermissiveCommons
    } else {
        PublicationLockState::UnclassifiedFailClosed
    }
}

/// Classifies a provenance node into one of the four unmerged Identifier Fabric planes.
/// Never promotes `did:q42:`, `did:qi:`, `did:webizen:`, or `urn:omni:` into `NaturalAgent`.
pub fn classify_provenance_node_plane(id: &str, role_hint: &str) -> IdentifierFabricPlane {
    let lower_id = id.to_ascii_lowercase();
    if lower_id.starts_with("did:q42:")
        || lower_id.starts_with("<did:q42:")
        || lower_id.starts_with("urn:omni:")
        || lower_id.starts_with("dni:")
        || role_hint == "SpatiotemporalHandle"
    {
        IdentifierFabricPlane::SpatiotemporalHandle
    } else if lower_id.starts_with("did:qi:")
        || lower_id.starts_with("did:webizen:")
        || lower_id.starts_with("did:")
        || lower_id.contains("sensor")
        || lower_id.contains("wallet")
        || role_hint == "Instrument"
    {
        IdentifierFabricPlane::Instrument
    } else if role_hint == "NaturalAgent"
        || lower_id.starts_with("urn:qualia:person:")
        || lower_id.starts_with("urn:qualia:natural-agent:")
    {
        IdentifierFabricPlane::NaturalAgent
    } else {
        IdentifierFabricPlane::ClaimOpinion
    }
}

#[component]
/// Aether `ProvenanceGraph` component.
pub fn ProvenanceGraph() -> Element {
    let mut nodes = use_signal(Vec::<(String, String)>::new);
    let mut edges = use_signal(Vec::<(String, String, String)>::new);
    let mut lock_flags = use_signal(|| FLAG_SANCTUARY);

    use_effect(move || {
        spawn(async move {
            let query = "SELECT ?s ?p ?o WHERE { ?s ?p ?o . FILTER(?p = <urn:prov:wasGeneratedBy> || ?p = <urn:prov:wasAssociatedWith>) }";
            let args = serde_json::json!({"query": query});
            if let Ok(res) = invoke_json("execute_sparql_query", args).await {
                if let Ok(triples) = serde_json::from_value::<Vec<(String, String, String)>>(res) {
                    let mut n = Vec::new();
                    for t in triples.iter() {
                        let s_plane = classify_provenance_node_plane(&t.0, "ClaimOpinion")
                            .code()
                            .to_string();
                        if !n.contains(&(t.0.clone(), s_plane.clone())) {
                            n.push((t.0.clone(), s_plane));
                        }
                        let o_plane = classify_provenance_node_plane(&t.2, "Instrument")
                            .code()
                            .to_string();
                        if !n.contains(&(t.2.clone(), o_plane.clone())) {
                            n.push((t.2.clone(), o_plane));
                        }
                    }
                    nodes.set(n);
                    edges.set(triples);
                }
            }
        });
    });

    let current_lock = resolve_publication_lock_state(lock_flags(), "");

    rsx! {
        div { style: "height: 100vh; display: flex; flex-direction: column; background: #18181b; color: #fafafa; font-family: sans-serif;",
            div { style: "padding: 16px; border-bottom: 1px solid #27272a; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 12px;",
                div { style: "display: flex; align-items: center; gap: 10px; flex-wrap: wrap;",
                    h2 { style: "margin: 0; font-size: 18px; color: #e4e4e7;", "W3C PROV-O & Four-Plane Provenance Explorer" }
                    HonestyChip { level: HonestyLevel::Ready, detail: "Tri-Source Enumerated".to_string() }
                    HonestyChip { level: HonestyLevel::Planned, detail: "3D force layout".to_string() }
                }
                div { style: "display: flex; gap: 8px; flex-wrap: wrap;",
                    button {
                        style: "background: #27272a; border: 1px solid #3f3f46; color: white; padding: 6px 12px; border-radius: 6px; font-size: 12px; cursor: pointer;",
                        onclick: move |_| lock_flags.set(FLAG_SANCTUARY),
                        "Sanctuary Lock (0x0040)"
                    }
                    button {
                        style: "background: #27272a; border: 1px solid #3f3f46; color: white; padding: 6px 12px; border-radius: 6px; font-size: 12px; cursor: pointer;",
                        onclick: move |_| lock_flags.set(FLAG_PERMISSIVE_COMMONS),
                        "Permissive Commons (0x0020)"
                    }
                }
            }

            // Tri-Source Evaluation & Publication Lock Strip
            div {
                "data-tri-source": "true",
                style: "padding: 10px 16px; border-bottom: 1px solid #27272a; background: #121215; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 10px; font-size: 12px;",
                div { style: "display: flex; align-items: center; gap: 8px; flex-wrap: wrap;",
                    span { style: "color: #a1a1aa; font-weight: 700; text-transform: uppercase; font-size: 11px;", "Tri-Source Evaluation:" }
                    for badge in TRI_SOURCE_BADGES.iter() {
                        span {
                            key: "{badge}",
                            style: "padding: 3px 9px; border-radius: 999px; border: 1px solid rgba(56,189,248,.4); background: rgba(12,74,110,.35); color: #7dd3fc; font-weight: 700;",
                            "{badge}"
                        }
                    }
                }
                div {
                    "data-publication-lock": "{current_lock.badge_label()}",
                    style: if current_lock.allows_public_distribution() {
                        "padding: 3px 10px; border-radius: 999px; border: 1px solid rgba(74,222,128,.45); background: rgba(6,78,59,.35); color: #86efac; font-weight: 700;"
                    } else {
                        "padding: 3px 10px; border-radius: 999px; border: 1px solid rgba(251,113,133,.45); background: rgba(136,19,55,.35); color: #fda4af; font-weight: 700;"
                    },
                    "{current_lock.badge_label()}"
                }
            }

            // Four-Plane Jury Legend Strip
            div { style: "padding: 8px 16px; border-bottom: 1px solid #27272a; background: #18181b; display: flex; gap: 12px; flex-wrap: wrap; font-size: 11px; color: #d4d4d8;",
                for plane in IdentifierFabricPlane::ALL.iter() {
                    span {
                        key: "{plane.code()}",
                        style: format!("padding: 2px 8px; border-left: 3px solid {}; background: #27272a; border-radius: 4px;", plane.border_color()),
                        "{plane.title()}"
                    }
                }
                span { style: "color: #fde68a;", "VC: {VC_ORIGIN_INTEGRITY_DISCLAIMER}" }
                span { style: "color: #67e8f9;", "did:q42: {DID_Q42_QRC_LABEL}" }
            }

            div { style: "flex: 1; position: relative; overflow: hidden; background: radial-gradient(circle, #27272a 1px, transparent 1px); background-size: 20px 20px;",
                for (i, (id, typ)) in nodes.read().iter().enumerate() {
                    div {
                        key: "{id}",
                        style: "position: absolute; top: {20 + i * 15}%; left: {30 + i * 15}%; background: #0284c7; padding: 10px; border-radius: 8px; box-shadow: 0 4px 6px rgba(0,0,0,0.5); z-index: 10;",
                        div { style: "font-size: 12px; opacity: 0.85;", "Plane: {typ}" }
                        div { style: "font-weight: bold;", "{id}" }
                        if id.starts_with("did:q42:") {
                            div { style: "font-size: 10px; color: #e0f2fe; margin-top: 4px;", "{DID_Q42_QRC_LABEL}" }
                        }
                    }
                }
                if nodes.read().is_empty() {
                    div { style: "position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); color: #71717a;",
                        "No provenance data found ({edges.read().len()} edges)."
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tri_source_badges_and_sanctuary_precedence_are_enforced() {
        assert_eq!(
            TRI_SOURCE_BADGES,
            ["Etalon Oracle", "QDNF Network", "QualiaDB .q42 v4"]
        );
        let sanctuary = resolve_publication_lock_state(FLAG_SANCTUARY, "permissive_commons");
        assert_eq!(sanctuary, PublicationLockState::SanctuaryLocked);
        assert!(!sanctuary.allows_public_distribution());

        let both_flags =
            resolve_publication_lock_state(FLAG_SANCTUARY | FLAG_PERMISSIVE_COMMONS, "commons");
        assert_eq!(both_flags, PublicationLockState::SanctuaryLocked);
        assert!(!both_flags.allows_public_distribution());

        let commons = resolve_publication_lock_state(FLAG_PERMISSIVE_COMMONS, "permissive_commons");
        assert_eq!(commons, PublicationLockState::PermissiveCommons);
        assert!(commons.allows_public_distribution());
    }

    #[test]
    fn provenance_node_classification_never_collapses_qrc_or_did_into_natural_agent() {
        assert_eq!(
            classify_provenance_node_plane("did:q42:800000000000002a", "NaturalAgent"),
            IdentifierFabricPlane::SpatiotemporalHandle
        );
        assert_eq!(
            classify_provenance_node_plane("urn:omni:v1:physical:sol:earth", "Entity"),
            IdentifierFabricPlane::SpatiotemporalHandle
        );
        assert_eq!(
            classify_provenance_node_plane("did:qi:1234567890abcdef", "NaturalAgent"),
            IdentifierFabricPlane::Instrument
        );
        assert_eq!(
            classify_provenance_node_plane("urn:qualia:person:alice", "NaturalAgent"),
            IdentifierFabricPlane::NaturalAgent
        );
    }
}
