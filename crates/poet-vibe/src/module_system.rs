//! Module system — `import <iri> as name` with a module graph (T63).
//!
//! No npm. No `vibe.toml` dependency solver in 0.x. The catalog IS
//! the module graph — modules declare their IRI, exports, and
//! dependencies, and the resolver walks the graph to find them.
//!
//! ## Design
//!
//! - **ModuleId**: An IRI that uniquely identifies a module
//!   (e.g. `https://qualiadb.org/modules/math`).
//! - **ModuleEntry**: Metadata for a module — IRI, version, exports
//!   (function names and types), dependencies (other module IRIs).
//! - **ModuleCatalog**: The module graph — a collection of ModuleEntries
//!   with dependency edges. Supports cycle detection.
//! - **ModuleResolver**: Resolves an import IRI to a ModuleEntry,
//!   checking the catalog first, then falling back to the host.
//!
//! The catalog is built at host startup from the modules the host
//! has available. Scripts import modules by IRI; the resolver checks
//! the catalog and returns the entry (or an error if not found).
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` §8.13 T63,
//! excellence-first §2.7, ecosystem §3.4.

use std::collections::BTreeMap;

/// A module identifier — an IRI that uniquely identifies a module (T63).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ModuleId(pub String);

impl ModuleId {
    pub fn new(iri: &str) -> Self {
        Self(iri.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A module export — a function or capability the module provides (T63).
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleExport {
    /// The exported name (e.g. "abs", "max", "sha256").
    pub name: String,
    /// The export kind — function, capability, or type.
    pub kind: ExportKind,
    /// Optional type signature (for machine schema).
    pub signature: Option<String>,
}

/// The kind of module export (T63).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExportKind {
    /// A function export.
    Function,
    /// A capability export.
    Capability,
    /// A type export.
    Type,
}

impl ExportKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Capability => "capability",
            Self::Type => "type",
        }
    }
}

/// A module entry — metadata for a module in the catalog (T63).
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleEntry {
    /// The module's IRI — its unique identifier.
    pub id: ModuleId,
    /// The module's version (semver-ish, e.g. "0.1.0").
    pub version: String,
    /// What the module exports.
    pub exports: Vec<ModuleExport>,
    /// What the module depends on (other module IRIs).
    pub dependencies: Vec<ModuleId>,
    /// Optional description.
    pub description: String,
    /// Whether this is a built-in module (shipped with the host).
    pub is_builtin: bool,
}

impl ModuleEntry {
    /// Create a new module entry.
    pub fn new(id: &str, version: &str) -> Self {
        Self {
            id: ModuleId::new(id),
            version: version.into(),
            exports: Vec::new(),
            dependencies: Vec::new(),
            description: String::new(),
            is_builtin: false,
        }
    }

    /// Add an export.
    pub fn with_export(mut self, export: ModuleExport) -> Self {
        self.exports.push(export);
        self
    }

    /// Add a dependency.
    pub fn with_dependency(mut self, dep: &str) -> Self {
        self.dependencies.push(ModuleId::new(dep));
        self
    }

    /// Add a description.
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.into();
        self
    }

    /// Mark as built-in.
    pub fn builtin(mut self) -> Self {
        self.is_builtin = true;
        self
    }

    /// Check if this module exports a given name.
    pub fn exports_name(&self, name: &str) -> bool {
        self.exports.iter().any(|e| e.name == name)
    }

    /// Get all function exports.
    pub fn functions(&self) -> Vec<&ModuleExport> {
        self.exports.iter().filter(|e| e.kind == ExportKind::Function).collect()
    }

    /// Get all capability exports.
    pub fn capabilities(&self) -> Vec<&ModuleExport> {
        self.exports.iter().filter(|e| e.kind == ExportKind::Capability).collect()
    }
}

/// A module catalog — the module graph (T63).
///
/// Tracks all known modules and their dependency edges. Supports
/// cycle detection and topological ordering for loading.
#[derive(Debug, Clone, Default)]
pub struct ModuleCatalog {
    modules: BTreeMap<String, ModuleEntry>,
}

impl ModuleCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a module in the catalog.
    pub fn register(&mut self, entry: ModuleEntry) -> &mut Self {
        self.modules.insert(entry.id.as_str().into(), entry);
        self
    }

    /// Get a module by IRI.
    pub fn get(&self, iri: &str) -> Option<&ModuleEntry> {
        self.modules.get(iri)
    }

    /// Get all modules.
    pub fn all(&self) -> Vec<&ModuleEntry> {
        self.modules.values().collect()
    }

    /// Get all built-in modules.
    pub fn builtins(&self) -> Vec<&ModuleEntry> {
        self.modules.values().filter(|m| m.is_builtin).collect()
    }

    /// Number of modules.
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// Is the catalog empty?
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// Check if a dependency cycle exists starting from the given module.
    /// Returns the cycle path if found, or None if no cycle.
    pub fn detect_cycle(&self, start: &str) -> Option<Vec<String>> {
        let mut visited = std::collections::HashSet::new();
        let mut path = Vec::new();
        self.dfs_cycle(start, &mut visited, &mut path)
    }

    fn dfs_cycle(
        &self,
        current: &str,
        visited: &mut std::collections::HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        if path.contains(&current.to_string()) {
            // Found a cycle — extract the cycle portion
            let cycle_start = path.iter().position(|p| p == current).unwrap();
            let mut cycle = path[cycle_start..].to_vec();
            cycle.push(current.into());
            return Some(cycle);
        }
        if visited.contains(current) {
            return None;
        }
        visited.insert(current.into());
        path.push(current.into());
        if let Some(entry) = self.modules.get(current) {
            for dep in &entry.dependencies {
                if let Some(cycle) = self.dfs_cycle(dep.as_str(), visited, path) {
                    return Some(cycle);
                }
            }
        }
        path.pop();
        None
    }

    /// Get the topological order of all modules (for loading).
    /// Returns Err with the cycle path if a cycle is detected.
    pub fn topological_order(&self) -> Result<Vec<String>, Vec<String>> {
        let mut order = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut in_progress = std::collections::HashSet::new();

        for iri in self.modules.keys() {
            if !visited.contains(iri) {
                self.topo_dfs(iri, &mut visited, &mut in_progress, &mut order)?;
            }
        }
        Ok(order)
    }

    fn topo_dfs(
        &self,
        current: &str,
        visited: &mut std::collections::HashSet<String>,
        in_progress: &mut std::collections::HashSet<String>,
        order: &mut Vec<String>,
    ) -> Result<(), Vec<String>> {
        if visited.contains(current) {
            return Ok(());
        }
        if in_progress.contains(current) {
            // Cycle detected
            return Err(vec![current.into()]);
        }
        in_progress.insert(current.into());
        if let Some(entry) = self.modules.get(current) {
            for dep in &entry.dependencies {
                self.topo_dfs(dep.as_str(), visited, in_progress, order)?;
            }
        }
        in_progress.remove(current);
        visited.insert(current.into());
        order.push(current.into());
        Ok(())
    }
}

/// A module resolver — resolves an import IRI to a module entry (T63).
pub struct ModuleResolver<'a> {
    catalog: &'a ModuleCatalog,
}

impl<'a> ModuleResolver<'a> {
    pub fn new(catalog: &'a ModuleCatalog) -> Self {
        Self { catalog }
    }

    /// Resolve an import IRI to a module entry.
    pub fn resolve(&self, iri: &str) -> Result<&ModuleEntry, String> {
        self.catalog.get(iri).ok_or_else(|| {
            format!("module not found in catalog: {iri}")
        })
    }

    /// Resolve an import with a stripped `vibe:0.1/` prefix (T64).
    pub fn resolve_import(&self, path: &str) -> Result<&ModuleEntry, String> {
        // Strip optional vibe:0.1/ prefix (T64)
        let stripped = path.strip_prefix("vibe:0.1/").unwrap_or(path);
        self.resolve(stripped)
    }

    /// Check if all dependencies of a module are in the catalog.
    pub fn check_dependencies(&self, iri: &str) -> Result<(), Vec<String>> {
        let entry = self.resolve(iri).map_err(|e| vec![e])?;
        let mut missing = Vec::new();
        for dep in &entry.dependencies {
            if self.catalog.get(dep.as_str()).is_none() {
                missing.push(dep.as_str().into());
            }
        }
        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }
}

/// Create a default catalog with built-in modules (T63).
pub fn default_catalog() -> ModuleCatalog {
    let mut catalog = ModuleCatalog::new();

    // Math module
    catalog.register(
        ModuleEntry::new("math", "0.1.0")
            .builtin()
            .with_description("Pure math functions: abs, min, max, floor, ceil, round, sqrt, sin, cos, log, exp")
            .with_export(ModuleExport {
                name: "abs".into(),
                kind: ExportKind::Function,
                signature: Some("(x: number) -> number".into()),
            })
            .with_export(ModuleExport {
                name: "max".into(),
                kind: ExportKind::Function,
                signature: Some("(a: number, b: number) -> number".into()),
            })
            .with_export(ModuleExport {
                name: "min".into(),
                kind: ExportKind::Function,
                signature: Some("(a: number, b: number) -> number".into()),
            })
            .with_export(ModuleExport {
                name: "sqrt".into(),
                kind: ExportKind::Function,
                signature: Some("(x: number) -> f64".into()),
            }),
    );

    // Time module
    catalog.register(
        ModuleEntry::new("time", "0.1.0")
            .builtin()
            .with_description("Time primitives: now, monotonic_nanos, proper_time")
            .with_export(ModuleExport {
                name: "now".into(),
                kind: ExportKind::Function,
                signature: Some("() -> Instant".into()),
            }),
    );

    // Graph module
    catalog.register(
        ModuleEntry::new("graph", "0.1.0")
            .builtin()
            .with_description("Graph operations: query, stage, commit, snapshot")
            .with_export(ModuleExport {
                name: "query".into(),
                kind: ExportKind::Function,
                signature: Some("(query: string) -> Value".into()),
            })
            .with_export(ModuleExport {
                name: "commit".into(),
                kind: ExportKind::Function,
                signature: Some("() -> Value".into()),
            }),
    );

    // Crypto module
    catalog.register(
        ModuleEntry::new("crypto", "0.1.0")
            .builtin()
            .with_description("Cryptographic operations: sha256, sha512, blake3, hkdf, aead, sign, verify")
            .with_export(ModuleExport {
                name: "sha256".into(),
                kind: ExportKind::Function,
                signature: Some("(data: bytes) -> HashResult".into()),
            }),
    );

    // ZK module
    catalog.register(
        ModuleEntry::new("zk", "0.1.0")
            .builtin()
            .with_description("Zero-knowledge proofs: threshold, range, matmul")
            .with_export(ModuleExport {
                name: "prove_threshold".into(),
                kind: ExportKind::Function,
                signature: Some("(value: f64, threshold: f64) -> ZkProof".into()),
            }),
    );

    // HID module
    catalog.register(
        ModuleEntry::new("hid", "0.1.0")
            .builtin()
            .with_description("HID events: poll, wait")
            .with_export(ModuleExport {
                name: "poll".into(),
                kind: ExportKind::Function,
                signature: Some("() -> Value".into()),
            }),
    );

    catalog
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t63_default_catalog_has_builtins() {
        let catalog = default_catalog();
        assert!(catalog.len() >= 5);
        assert!(catalog.get("math").is_some());
        assert!(catalog.get("time").is_some());
        assert!(catalog.get("graph").is_some());
        assert!(catalog.get("crypto").is_some());
        assert!(catalog.get("zk").is_some());
    }

    #[test]
    fn t63_module_entry_exports() {
        let entry = ModuleEntry::new("math", "0.1.0")
            .with_export(ModuleExport {
                name: "abs".into(),
                kind: ExportKind::Function,
                signature: None,
            });
        assert!(entry.exports_name("abs"));
        assert!(!entry.exports_name("nonexistent"));
        assert_eq!(entry.functions().len(), 1);
    }

    #[test]
    fn t63_resolver_resolves_builtin() {
        let catalog = default_catalog();
        let resolver = ModuleResolver::new(&catalog);
        let entry = resolver.resolve("math").unwrap();
        assert_eq!(entry.id.as_str(), "math");
        assert!(entry.is_builtin);
    }

    #[test]
    fn t63_resolver_strips_vibe_prefix() {
        let catalog = default_catalog();
        let resolver = ModuleResolver::new(&catalog);
        let entry = resolver.resolve_import("vibe:0.1/math").unwrap();
        assert_eq!(entry.id.as_str(), "math");
    }

    #[test]
    fn t63_resolver_fails_for_unknown() {
        let catalog = default_catalog();
        let resolver = ModuleResolver::new(&catalog);
        assert!(resolver.resolve("nonexistent").is_err());
    }

    #[test]
    fn t63_check_dependencies_ok() {
        let mut catalog = ModuleCatalog::new();
        catalog.register(ModuleEntry::new("a", "0.1.0").with_dependency("b"));
        catalog.register(ModuleEntry::new("b", "0.1.0"));
        let resolver = ModuleResolver::new(&catalog);
        assert!(resolver.check_dependencies("a").is_ok());
    }

    #[test]
    fn t63_check_dependencies_missing() {
        let mut catalog = ModuleCatalog::new();
        catalog.register(ModuleEntry::new("a", "0.1.0").with_dependency("b"));
        // b is not registered
        let resolver = ModuleResolver::new(&catalog);
        let result = resolver.check_dependencies("a");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), vec!["b"]);
    }

    #[test]
    fn t63_detect_cycle_no_cycle() {
        let mut catalog = ModuleCatalog::new();
        catalog.register(ModuleEntry::new("a", "0.1.0").with_dependency("b"));
        catalog.register(ModuleEntry::new("b", "0.1.0"));
        assert!(catalog.detect_cycle("a").is_none());
    }

    #[test]
    fn t63_detect_cycle_with_cycle() {
        let mut catalog = ModuleCatalog::new();
        catalog.register(ModuleEntry::new("a", "0.1.0").with_dependency("b"));
        catalog.register(ModuleEntry::new("b", "0.1.0").with_dependency("c"));
        catalog.register(ModuleEntry::new("c", "0.1.0").with_dependency("a"));
        let cycle = catalog.detect_cycle("a");
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert!(cycle.len() >= 3);
    }

    #[test]
    fn t63_topological_order() {
        let mut catalog = ModuleCatalog::new();
        catalog.register(ModuleEntry::new("a", "0.1.0").with_dependency("b"));
        catalog.register(ModuleEntry::new("b", "0.1.0").with_dependency("c"));
        catalog.register(ModuleEntry::new("c", "0.1.0"));
        let order = catalog.topological_order().unwrap();
        // c should come before b, b before a
        let c_pos = order.iter().position(|x| x == "c").unwrap();
        let b_pos = order.iter().position(|x| x == "b").unwrap();
        let a_pos = order.iter().position(|x| x == "a").unwrap();
        assert!(c_pos < b_pos);
        assert!(b_pos < a_pos);
    }

    #[test]
    fn t63_topological_order_with_cycle_fails() {
        let mut catalog = ModuleCatalog::new();
        catalog.register(ModuleEntry::new("a", "0.1.0").with_dependency("b"));
        catalog.register(ModuleEntry::new("b", "0.1.0").with_dependency("a"));
        assert!(catalog.topological_order().is_err());
    }

    #[test]
    fn t63_module_id_display() {
        let id = ModuleId::new("https://qualiadb.org/modules/math");
        assert_eq!(id.to_string(), "https://qualiadb.org/modules/math");
    }

    #[test]
    fn t63_export_kind_as_str() {
        assert_eq!(ExportKind::Function.as_str(), "function");
        assert_eq!(ExportKind::Capability.as_str(), "capability");
        assert_eq!(ExportKind::Type.as_str(), "type");
    }

    #[test]
    fn t63_catalog_builtins() {
        let catalog = default_catalog();
        let builtins = catalog.builtins();
        assert!(builtins.len() >= 5);
        for b in builtins {
            assert!(b.is_builtin);
        }
    }

    #[test]
    fn t63_module_entry_capabilities() {
        let entry = ModuleEntry::new("graph", "0.1.0")
            .with_export(ModuleExport {
                name: "query".into(),
                kind: ExportKind::Function,
                signature: None,
            })
            .with_export(ModuleExport {
                name: "graph.write".into(),
                kind: ExportKind::Capability,
                signature: None,
            });
        assert_eq!(entry.functions().len(), 1);
        assert_eq!(entry.capabilities().len(), 1);
    }
}
