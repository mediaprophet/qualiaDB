//! Manifest: declarative description of a toolbox plugin.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)
//!
//! A manifest is a CBOR-LD file that describes a toolbox plugin — its
//! id, label, icon, ontology prefix, tool-chains, and tools. The
//! tool-chest registry reads manifests at startup to discover and
//! load toolboxes.
//!
//! # WASM compatibility
//!
//! All types are `#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]`
//! with no platform-specific dependencies.

use super::tool::ToolKind;

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

/// A declarative description of a toolbox plugin.
///
/// Loaded from `manifest.cbor` in each toolbox directory. The registry
/// uses this to discover toolboxes without loading the full Rust code.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    /// Toolbox id — e.g. `social`.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Icon identifier.
    pub icon: String,
    /// Ontology prefix.
    pub ontology_prefix: String,
    /// Short description.
    pub description: String,
    /// Whether this toolbox is enabled by default.
    #[serde(default = "default_true")]
    pub enabled_by_default: bool,
    /// Tool-chains declared in this toolbox.
    pub chains: Vec<ManifestChain>,
    /// Required capability scopes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_capabilities: Vec<String>,
}

fn default_true() -> bool {
    true
}

/// A tool-chain declared in a manifest.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ManifestChain {
    /// Chain id.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Icon identifier.
    pub icon: String,
    /// Short description.
    pub description: String,
    /// Tools in this chain.
    pub tools: Vec<ManifestTool>,
}

/// A tool declared in a manifest.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ManifestTool {
    /// Tool id.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Icon identifier.
    pub icon: String,
    /// Tool interaction kind.
    pub kind: ToolKind,
    /// Capability scope required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_scope: Option<String>,
    /// Short description.
    pub description: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_serialisation() {
        let m = Manifest {
            id: "social".into(),
            label: "Social".into(),
            icon: "users".into(),
            ontology_prefix: "soc".into(),
            description: "Social toolbox".into(),
            enabled_by_default: true,
            chains: vec![ManifestChain {
                id: "social:connections".into(),
                label: "Connections".into(),
                icon: "link".into(),
                description: "Connection requests".into(),
                tools: vec![ManifestTool {
                    id: "social:send_connection_request".into(),
                    label: "Send Connection Request".into(),
                    icon: "send".into(),
                    kind: ToolKind::RunAction,
                    capability_scope: Some("graph:mutate".into()),
                    description: "Send a ZKP-verified connection request.".into(),
                }],
            }],
            required_capabilities: vec!["graph:read".into()],
        };

        let cbor = ciborium::to_vec(&m).expect("cbor encode");
        let decoded: Manifest = ciborium::from_reader(&cbor[..]).expect("cbor decode");
        assert_eq!(decoded.id, "social");
        assert_eq!(decoded.chains.len(), 1);
        assert_eq!(decoded.chains[0].tools.len(), 1);
    }
}
