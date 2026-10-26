//! Effect classes.

use crate::ast::EffectClass;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Effect {
    Pure = 0,
    Hot = 1,
    Cold = 2,
    Async = 3,
    External = 4,
}

impl Effect {
    /// Constructs from class.
    pub fn from_class(c: EffectClass) -> Effect {
        match c {
            EffectClass::Pure => Effect::Pure,
            EffectClass::Hot => Effect::Hot,
            EffectClass::Cold => Effect::Cold,
            EffectClass::Async => Effect::Async,
            EffectClass::External => Effect::External,
        }
    }

    /// Joins effect sets.
    pub fn join(self, other: Effect) -> Effect {
        self.max(other)
    }
}

/// Binding id → minimum effect required to call it.
pub fn binding_effect(path: &str) -> Effect {
    match path {
        p if p.starts_with("math.") || p.starts_with("rdf.") => Effect::Pure,
        p if p.starts_with("web.")
            || p.starts_with("ui.")
            || p.starts_with("form.")
            || p.starts_with("input.")
            || p.starts_with("validate.")
            || p.starts_with("selection.")
            || p.starts_with("layout.")
            || p.starts_with("cbor_ld.") =>
        {
            Effect::Pure
        }
        "vec2" | "vec3" | "vec4" | "mat3" | "mat4" | "oklch" | "Ok" | "Err" => Effect::Pure,
        "quin.statement" | "receipt_empty" | "capability.resolve" => Effect::Pure,
        "graph.snapshot" | "graph.query" => Effect::Pure,
        "aura.validate" => Effect::Pure,
        "graph.stage" | "graph.commit" | "pulse.publish" | "capability.invoke" | "time.unix"
        | "time.now" => Effect::External,
        p if p.starts_with("Animation.") => Effect::Pure,
        p if crate::catalog::is_known(p) || crate::catalog::looks_like_catalog_path(p) => {
            Effect::External
        }
        _ => Effect::Pure,
    }
}

/// Capability required for the effect.
pub fn capability_for(path: &str) -> Option<&'static str> {
    match path {
        "graph.query" | "graph.snapshot" => Some("graph.read"),
        "graph.stage" | "graph.commit" => Some("graph.write"),
        "aura.validate" => Some("aura.validate"),
        "pulse.publish" => Some("pulse.publish"),
        "capability.invoke" => Some("capability.invoke"),
        p => crate::catalog::canonical_id(p),
    }
}
