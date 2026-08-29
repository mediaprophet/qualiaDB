//! Vibe-host execution capability contract.
//!
//! This module describes *where* a Vibe capability runs; it does not change
//! the language or silently substitute a different computation.  In
//! particular, the detached WASM graph is an isolated snapshot, not a claim
//! that a persistent native graph has been reached.

/// Versioned identifier emitted by the loopback negotiation endpoint.
pub const BRIDGE_PROTOCOL: &str = "qualia-vibe-bridge/1";

/// Execution route for a host capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityRoute {
    /// Runs wholly inside the loaded WASM module.
    StandaloneWasm,
    /// Runs in WASM, but against an isolated, non-persistent snapshot.
    StandaloneSnapshot,
    /// Needs the native engine through an authenticated local adapter.
    NativeBridge,
    /// Uses an in-process native engine directly.
    NativeDirect,
}

impl CapabilityRoute {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StandaloneWasm => "standalone-wasm",
            Self::StandaloneSnapshot => "standalone-snapshot",
            Self::NativeBridge => "native-bridge",
            Self::NativeDirect => "native-direct",
        }
    }
}

/// Meaning promised by a route. This is deliberately separate from speed or
/// availability: an isolated graph snapshot is useful, but it is not a
/// persistent graph transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticGuarantee {
    Exact,
    IsolatedSnapshot,
    Unavailable,
}

impl SemanticGuarantee {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::IsolatedSnapshot => "isolated-snapshot",
            Self::Unavailable => "unavailable",
        }
    }
}

/// A static capability boundary shared by catalogue metadata, the daemon
/// negotiation document, and tools-side diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityBoundary {
    pub standalone_route: CapabilityRoute,
    pub standalone_semantics: SemanticGuarantee,
    pub native_route: CapabilityRoute,
    pub native_semantics: SemanticGuarantee,
}

const EXACT_WASM_BINDINGS: &[&str] = &[
    "math.abs",
    "math.min",
    "math.max",
    "rdf.triple",
    "rdf.reify",
    "quin.statement",
    "capability.resolve",
];

const SNAPSHOT_BINDINGS: &[&str] = &[
    "graph.read",
    "graph.write",
    "aura.validate",
    "pulse.publish",
];

const NATIVE_BRIDGE_BINDINGS: &[&str] = &[
    "GraphDatabase.sparql",
    "Inference.load_model",
    "Inference.run_transformer",
];

/// Resolve a capability boundary without allocating. Unknown capabilities are
/// intentionally unavailable until an owning host declares them.
pub const fn boundary_for(id: &str) -> CapabilityBoundary {
    if slice_contains(EXACT_WASM_BINDINGS, id) {
        return CapabilityBoundary {
            standalone_route: CapabilityRoute::StandaloneWasm,
            standalone_semantics: SemanticGuarantee::Exact,
            native_route: CapabilityRoute::NativeDirect,
            native_semantics: SemanticGuarantee::Exact,
        };
    }
    if slice_contains(SNAPSHOT_BINDINGS, id) {
        return CapabilityBoundary {
            standalone_route: CapabilityRoute::StandaloneSnapshot,
            standalone_semantics: SemanticGuarantee::IsolatedSnapshot,
            native_route: CapabilityRoute::NativeDirect,
            native_semantics: SemanticGuarantee::Exact,
        };
    }
    if slice_contains(NATIVE_BRIDGE_BINDINGS, id) {
        return CapabilityBoundary {
            standalone_route: CapabilityRoute::NativeBridge,
            standalone_semantics: SemanticGuarantee::Unavailable,
            native_route: CapabilityRoute::NativeDirect,
            native_semantics: SemanticGuarantee::Exact,
        };
    }
    CapabilityBoundary {
        standalone_route: CapabilityRoute::NativeBridge,
        standalone_semantics: SemanticGuarantee::Unavailable,
        native_route: CapabilityRoute::NativeDirect,
        native_semantics: SemanticGuarantee::Exact,
    }
}

const fn slice_contains(haystack: &[&str], needle: &str) -> bool {
    let mut index = 0;
    while index < haystack.len() {
        if str_eq(haystack[index], needle) {
            return true;
        }
        index += 1;
    }
    false
}

const fn str_eq(left: &str, right: &str) -> bool {
    let left_bytes = left.as_bytes();
    let right_bytes = right.as_bytes();
    if left_bytes.len() != right_bytes.len() {
        return false;
    }
    let mut index = 0;
    while index < left_bytes.len() {
        if left_bytes[index] != right_bytes[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// The JSON-shaped, allocation-permitted cold-path view of the contract.
pub fn negotiation_document(native_available: bool) -> serde_json::Value {
    let capabilities: Vec<serde_json::Value> = crate::poet_host::invoke::ids::ALL_BOUND
        .iter()
        .map(|id| capability_json(id, native_available))
        .collect();

    serde_json::json!({
        "protocol": BRIDGE_PROTOCOL,
        "execution_host": "vibe-host",
        "profile": crate::wasm_capabilities::compiled_profile(),
        "backend": {
            "kind": "browser-loopback-http",
            "available": native_available,
            "engine_version": crate::ENGINE_VERSION,
        },
        "capabilities": capabilities,
        "security": {
            "token_header": "X-Qualia-Token",
            "origin_checked": true,
            "private_network_access": true,
            "user_gesture_required_by_client": true,
        }
    })
}

/// One capability entry for both machine and human tooling.
pub fn capability_json(id: &str, native_available: bool) -> serde_json::Value {
    let boundary = boundary_for(id);
    let schema = crate::poet_host::invoke::coverage::schema_for(id);
    let (route, semantics, available) = if native_available {
        (boundary.native_route, boundary.native_semantics, true)
    } else {
        (
            boundary.standalone_route,
            boundary.standalone_semantics,
            boundary.standalone_semantics != SemanticGuarantee::Unavailable,
        )
    };
    serde_json::json!({
        "id": id,
        "mode": route.as_str(),
        "semantics": semantics.as_str(),
        "available": available,
        "requires_native": boundary.standalone_semantics == SemanticGuarantee::Unavailable,
        "transport": if route == CapabilityRoute::NativeBridge { "browser-loopback-http" } else { "none" },
        "family": schema.map(|value| value.family).unwrap_or("unclassified"),
        "honesty": schema.map(|value| value.honesty).unwrap_or("unclassified"),
        "effect_class": schema.map(|value| value.effect_class).unwrap_or("unknown"),
        "arg_schema": schema
            .and_then(|value| serde_json::from_str::<serde_json::Value>(value.arg_shape).ok())
            .unwrap_or(serde_json::Value::Null),
        "return_schema": schema
            .and_then(|value| serde_json::from_str::<serde_json::Value>(value.return_shape).ok())
            .unwrap_or(serde_json::Value::Null),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_write_is_never_described_as_persistent_in_standalone_wasm() {
        let graph = capability_json("graph.write", false);
        assert_eq!(graph["mode"], "standalone-snapshot");
        assert_eq!(graph["semantics"], "isolated-snapshot");
    }

    #[test]
    fn native_only_capability_is_explicit_when_daemon_is_absent() {
        let model = capability_json("Inference.load_model", false);
        assert_eq!(model["mode"], "native-bridge");
        assert_eq!(model["available"], false);
        assert_eq!(model["requires_native"], true);
    }

    #[test]
    fn negotiation_uses_versioned_protocol_and_vibe_host_name() {
        let document = negotiation_document(true);
        assert_eq!(document["protocol"], BRIDGE_PROTOCOL);
        assert_eq!(document["execution_host"], "vibe-host");
        let capabilities = document["capabilities"].as_array().unwrap();
        assert_eq!(
            capabilities.len(),
            crate::poet_host::invoke::ids::ALL_BOUND.len()
        );
        assert!(capabilities
            .iter()
            .any(|entry| entry["id"] == "Audio.oscillator"));
        assert!(capabilities
            .iter()
            .any(|entry| entry["id"] == "Scene.render"));
    }
}
