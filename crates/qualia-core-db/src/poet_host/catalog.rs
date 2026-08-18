//! Vibe 0.1 binding table over Qualia's existing capability registry.
//!
//! Humans and apps reach the engine through these IDs. MCP is a separate
//! bot door; it must not be the only listing.

use crate::{CapabilityDescriptor, CAPABILITY_DESCRIPTORS};
use poet_vibe::Value;
use std::collections::BTreeMap;

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

pub fn resolve_id(id: &str) -> Value {
    let binding = VIBE_0_1.iter().find(|b| b.id == id);
    let family = binding.map(|b| b.family);
    let desc = family.and_then(|name| CAPABILITY_DESCRIPTORS.iter().find(|d| d.name == name));
    let mut rec = BTreeMap::new();
    rec.insert("id".into(), Value::String(id.into()));
    rec.insert("vibe_bound".into(), Value::Bool(binding.is_some()));
    rec.insert(
        "honesty".into(),
        Value::String(binding.map(|b| b.honesty.to_string()).unwrap_or_else(|| "unbound".into())),
    );
    rec.insert(
        "human_surface".into(),
        Value::String("poet".into()),
    );
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
        .filter(|d| {
            !VIBE_0_1.iter().any(|b| b.family == d.name) && !d.mcp_tools.is_empty()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_read_resolves_as_partial_vibe() {
        match resolve_id("graph.read") {
            Value::Record(r) => {
                assert_eq!(r.get("vibe_bound"), Some(&Value::Bool(true)));
                assert_eq!(
                    r.get("honesty"),
                    Some(&Value::String("partial".into()))
                );
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
}
