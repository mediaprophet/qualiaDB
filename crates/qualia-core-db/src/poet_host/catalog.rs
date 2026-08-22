//! Vibe 0.1 binding table over Qualia's existing capability registry.
//!
//! Humans and apps reach the engine through these IDs. MCP is a separate
//! bot door; it must not be the only listing.

use crate::{vibe_host::bridge, CapabilityDescriptor, CAPABILITY_DESCRIPTORS};
use std::collections::BTreeMap;
use vibe::Value;

/// A 0.1 Vibe binding and whether the live host actually implements it.
#[derive(Debug, Clone, Copy)]
pub struct VibeBinding {
    pub id: &'static str,
    pub family: &'static str,
    pub required: bool,
    pub honesty: &'static str,
}

/// Required §11 bindings plus honesty. Unbound engine families stay MCP/CLI-only until wired.
pub const VIBE_0_1: &[VibeBinding] = &[
    VibeBinding {
        id: "math.abs",
        family: "CapabilityDiscovery",
        required: true,
        honesty: "live",
    },
    VibeBinding {
        id: "math.min",
        family: "CapabilityDiscovery",
        required: true,
        honesty: "live",
    },
    VibeBinding {
        id: "math.max",
        family: "CapabilityDiscovery",
        required: true,
        honesty: "live",
    },
    VibeBinding {
        id: "rdf.triple",
        family: "GraphDatabase",
        required: true,
        honesty: "live",
    },
    VibeBinding {
        id: "rdf.reify",
        family: "GraphDatabase",
        required: true,
        honesty: "live",
    },
    VibeBinding {
        id: "quin.statement",
        family: "GraphDatabase",
        required: true,
        honesty: "live",
    },
    VibeBinding {
        id: "graph.read",
        family: "GraphDatabase",
        required: true,
        honesty: "partial",
    },
    VibeBinding {
        id: "graph.write",
        family: "GraphDatabase",
        required: true,
        honesty: "partial",
    },
    VibeBinding {
        id: "aura.validate",
        family: "SHACL",
        required: true,
        honesty: "partial",
    },
    VibeBinding {
        id: "pulse.publish",
        family: "CapabilityDiscovery",
        required: true,
        honesty: "partial",
    },
    VibeBinding {
        id: "capability.resolve",
        family: "CapabilityDiscovery",
        required: true,
        honesty: "live",
    },
    VibeBinding {
        id: "capability.invoke",
        family: "CapabilityDiscovery",
        required: false,
        honesty: "partial",
    },
    VibeBinding {
        id: "time.unix",
        family: "CapabilityDiscovery",
        required: false,
        honesty: "partial",
    },
];

/// Bindings whose honesty flips to "live" when the host snapshot is attached
/// to the live daemon graph. On WASM / detached, they stay "partial".
const DYNAMIC_LIVE_WHEN_ATTACHED: &[&str] = &[
    "graph.read",
    "graph.write",
    "aura.validate",
    "pulse.publish",
];

/// Return the effective honesty label for a binding, given attachment state.
///
/// `graph.read`, `graph.write`, `aura.validate`, and `pulse.publish` are
/// "partial" on detached/WASM hosts but "live" when attached to the daemon
/// graph (queries refresh from the live graph, commits extend it, SHACL
/// validates against it, and pulse publishes through the transport channel).
/// All other bindings keep their static label.
pub fn dynamic_honesty(id: &str, attached: bool) -> &'static str {
    if attached && DYNAMIC_LIVE_WHEN_ATTACHED.contains(&id) {
        return "live";
    }
    VIBE_0_1
        .iter()
        .find(|b| b.id == id)
        .map(|b| b.honesty)
        .unwrap_or("unbound")
}

pub fn resolve_id(id: &str) -> Value {
    resolve_id_with(id, false)
}

pub fn resolve_id_with(id: &str, attached: bool) -> Value {
    let binding = VIBE_0_1.iter().find(|b| b.id == id);
    let family = binding.map(|b| b.family);
    let desc = family.and_then(|name| CAPABILITY_DESCRIPTORS.iter().find(|d| d.name == name));
    let honesty = dynamic_honesty(id, attached);
    let mut rec = BTreeMap::new();
    rec.insert("id".into(), Value::String(id.into()));
    rec.insert("vibe_bound".into(), Value::Bool(binding.is_some()));
    rec.insert("honesty".into(), Value::String(honesty.to_string()));
    rec.insert("human_surface".into(), Value::String("poet".into()));
    rec.insert("execution_host".into(), Value::String("vibe-host".into()));
    let bridge_metadata = bridge::capability_json(id, attached);
    if let Some(mode) = bridge_metadata.get("mode").and_then(|value| value.as_str()) {
        rec.insert("execution_mode".into(), Value::String(mode.into()));
    }
    if let Some(semantics) = bridge_metadata
        .get("semantics")
        .and_then(|value| value.as_str())
    {
        rec.insert("semantics".into(), Value::String(semantics.into()));
    }
    if let Some(requires_native) = bridge_metadata
        .get("requires_native")
        .and_then(|value| value.as_bool())
    {
        rec.insert("requires_native".into(), Value::Bool(requires_native));
    }
    if let Some(b) = binding {
        rec.insert("required".into(), Value::Bool(b.required));
        rec.insert("family".into(), Value::String(b.family.into()));
    }
    if let Some(d) = desc {
        rec.insert("maturity".into(), Value::String(d.maturity.into()));
        rec.insert(
            "mcp_tools".into(),
            Value::List(
                d.mcp_tools
                    .iter()
                    .map(|t| Value::String((*t).into()))
                    .collect(),
            ),
        );
    }
    Value::Record(rec)
}

pub fn engine_families_mcp_only() -> Vec<&'static CapabilityDescriptor> {
    CAPABILITY_DESCRIPTORS
        .iter()
        .filter(|d| !VIBE_0_1.iter().any(|b| b.family == d.name) && !d.mcp_tools.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_read_resolves_as_partial_vibe_when_detached() {
        match resolve_id("graph.read") {
            Value::Record(r) => {
                assert_eq!(r.get("vibe_bound"), Some(&Value::Bool(true)));
                assert_eq!(r.get("honesty"), Some(&Value::String("partial".into())));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn graph_read_is_live_when_attached() {
        match resolve_id_with("graph.read", true) {
            Value::Record(r) => {
                assert_eq!(r.get("honesty"), Some(&Value::String("live".into())));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn pulse_publish_is_live_when_attached() {
        match resolve_id_with("pulse.publish", true) {
            Value::Record(r) => {
                assert_eq!(r.get("honesty"), Some(&Value::String("live".into())));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn capability_invoke_stays_partial_even_when_attached() {
        match resolve_id_with("capability.invoke", true) {
            Value::Record(r) => {
                assert_eq!(r.get("honesty"), Some(&Value::String("partial".into())));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn deontic_is_not_a_vibe_0_1_keyword() {
        match resolve_id("logic.deontic.evaluate") {
            Value::Record(r) => {
                assert_eq!(r.get("vibe_bound"), Some(&Value::Bool(false)));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn detached_graph_write_is_marked_as_an_isolated_snapshot() {
        match resolve_id("graph.write") {
            Value::Record(r) => {
                assert_eq!(
                    r.get("execution_mode"),
                    Some(&Value::String("standalone-snapshot".into()))
                );
                assert_eq!(
                    r.get("semantics"),
                    Some(&Value::String("isolated-snapshot".into()))
                );
            }
            other => panic!("{other:?}"),
        }
    }
}
