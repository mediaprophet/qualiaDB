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
    pub fn from_class(c: EffectClass) -> Effect {
        match c {
            EffectClass::Pure => Effect::Pure,
            EffectClass::Hot => Effect::Hot,
            EffectClass::Cold => Effect::Cold,
            EffectClass::Async => Effect::Async,
            EffectClass::External => Effect::External,
        }
    }

    pub fn join(self, other: Effect) -> Effect {
        self.max(other)
    }
}

/// Binding id → minimum effect required to call it.
pub fn binding_effect(path: &str) -> Effect {
    match path {
        p if p.starts_with("math.") => Effect::Pure,
        p if p.starts_with("rdf.") => Effect::Pure,
        "quin.statement" | "receipt_empty" | "capability.resolve" => Effect::Pure,
        "graph.snapshot" | "graph.query" => Effect::Pure,
        "aura.validate" => Effect::Pure,
        "graph.stage" | "graph.commit" | "pulse.publish" | "capability.invoke" | "time.unix" => {
            Effect::External
        }
        _ => Effect::Pure,
    }
}

pub fn capability_for(path: &str) -> Option<&'static str> {
    match path {
        "graph.query" | "graph.snapshot" => Some("graph.read"),
        "graph.stage" | "graph.commit" => Some("graph.write"),
        "aura.validate" => Some("aura.validate"),
        "pulse.publish" => Some("pulse.publish"),
        "capability.invoke" => Some("capability.invoke"),
        _ => None,
    }
}
