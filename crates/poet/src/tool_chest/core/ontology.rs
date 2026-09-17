//! Ontology loader: loads pre-compiled CBOR-LD ontology files at startup.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)
//!
//! The runtime never parses N3 — it loads pre-compiled CBOR-LD ontology
//! files. Each ontology module registers its prefix, classes, and
//! properties via this loader.
//!
//! # WASM compatibility
//!
//! All types are `#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]`
//! with no platform-specific dependencies.

use core::fmt;

// ---------------------------------------------------------------------------
// OntologyModule
// ---------------------------------------------------------------------------

/// A loaded ontology module — a compiled CBOR-LD ontology file.
///
/// Each module carries its prefix IRI, a list of class definitions,
/// and a list of property definitions. The runtime uses these to
/// validate VibeScript payloads and to drive UI generation.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct OntologyModule {
    /// The short prefix used in N3 authoring — e.g. `soc`, `set`, `comm`.
    pub prefix: String,
    /// The full IRI for the prefix — e.g. `https://qualiadb.org/schema/ui/social#`.
    pub iri: String,
    /// Human-readable label for the module.
    pub label: String,
    /// Path to the compiled CBOR-LD file (relative to the ontology directory).
    pub cbor_path: String,
    /// Classes defined in this module.
    pub classes: Vec<OntologyClass>,
    /// Properties defined in this module.
    pub properties: Vec<OntologyProperty>,
    /// Other prefixes this module imports.
    pub imports: Vec<String>,
}

/// A class definition from an ontology module.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct OntologyClass {
    /// Local name within the module — e.g. `SocialEdge`, `ConnectionRequest`.
    pub local_name: String,
    /// Full IRI — e.g. `https://qualiadb.org/schema/ui/social#SocialEdge`.
    pub iri: String,
    /// Human-readable label.
    pub label: String,
    /// Comment / description.
    pub comment: String,
    /// Superclass IRIs (if any).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub super_classes: Vec<String>,
}

/// A property definition from an ontology module.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct OntologyProperty {
    /// Local name — e.g. `edgeDuration`, `crStatus`.
    pub local_name: String,
    /// Full IRI.
    pub iri: String,
    /// Human-readable label.
    pub label: String,
    /// Comment / description.
    pub comment: String,
    /// Domain class IRI (the class this property applies to).
    pub domain: String,
    /// Range class IRI or XSD type (the value type).
    pub range: String,
}

impl OntologyModule {
    /// Look up a class by local name.
    pub fn class(&self, local_name: &str) -> Option<&OntologyClass> {
        self.classes.iter().find(|c| c.local_name == local_name)
    }

    /// Look up a property by local name.
    pub fn property(&self, local_name: &str) -> Option<&OntologyProperty> {
        self.properties.iter().find(|p| p.local_name == local_name)
    }
}

impl fmt::Display for OntologyModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OntologyModule({}:{}, {} classes, {} properties)",
            self.prefix,
            self.label,
            self.classes.len(),
            self.properties.len()
        )
    }
}

// ---------------------------------------------------------------------------
// OntologyRegistry
// ---------------------------------------------------------------------------

/// Registry of all loaded ontology modules.
///
/// The tool-chest initialises this at startup by loading CBOR-LD
/// ontology files. Modules are keyed by prefix for fast lookup.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct OntologyRegistry {
    modules: Vec<OntologyModule>,
}

impl OntologyRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    /// Register a module.
    pub fn register(&mut self, module: OntologyModule) {
        self.modules.push(module);
    }

    /// Look up a module by prefix.
    pub fn module(&self, prefix: &str) -> Option<&OntologyModule> {
        self.modules.iter().find(|m| m.prefix == prefix)
    }

    /// Look up a module mutably by prefix.
    pub fn module_mut(&mut self, prefix: &str) -> Option<&mut OntologyModule> {
        self.modules.iter_mut().find(|m| m.prefix == prefix)
    }

    /// All registered modules.
    pub fn modules(&self) -> &[OntologyModule] {
        &self.modules
    }

    /// Resolve a prefixed name (e.g. `soc:SocialEdge`) to a full IRI.
    pub fn resolve(&self, prefixed: &str) -> Option<String> {
        let (prefix, local) = prefixed.split_once(':')?;
        let module = self.module(prefix)?;
        module
            .class(local)
            .map(|c| c.iri.clone())
            .or_else(|| module.property(local).map(|p| p.iri.clone()))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_module() -> OntologyModule {
        OntologyModule {
            prefix: "soc".into(),
            iri: "https://qualiadb.org/schema/ui/social#".into(),
            label: "Social".into(),
            cbor_path: "ontologies/social.cbor".into(),
            classes: vec![OntologyClass {
                local_name: "SocialEdge".into(),
                iri: "https://qualiadb.org/schema/ui/social#SocialEdge".into(),
                label: "Social Edge".into(),
                comment: "A relationship between two entities.".into(),
                super_classes: vec![],
            }],
            properties: vec![OntologyProperty {
                local_name: "edgeDuration".into(),
                iri: "https://qualiadb.org/schema/ui/social#edgeDuration".into(),
                label: "edge duration".into(),
                comment: "How long the relationship has existed.".into(),
                domain: "https://qualiadb.org/schema/ui/social#SocialEdge".into(),
                range: "https://qualiadb.org/schema/ui/social#DurationCategory".into(),
            }],
            imports: vec!["agency".into(), "obligations".into()],
        }
    }

    #[test]
    fn module_lookup() {
        let m = sample_module();
        assert!(m.class("SocialEdge").is_some());
        assert!(m.class("Nonexistent").is_none());
        assert!(m.property("edgeDuration").is_some());
    }

    #[test]
    fn registry_resolve() {
        let mut reg = OntologyRegistry::new();
        reg.register(sample_module());
        let iri = reg.resolve("soc:SocialEdge");
        assert_eq!(
            iri.as_deref(),
            Some("https://qualiadb.org/schema/ui/social#SocialEdge")
        );
        assert!(reg.resolve("soc:Nonexistent").is_none());
        assert!(reg.resolve("unknown:Foo").is_none());
    }
}
