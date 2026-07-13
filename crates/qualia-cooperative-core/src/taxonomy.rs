//! Extensible taxonomy primitives — the base for the `AuthorityType` and `GuardianshipRole`
//! vocabularies (see `docs/plans/adr-authority-attestation-guardianship-model.md`).
//!
//! Design constraint (Timothy, 2026-07-03): **must be extensible**. Terms are therefore *data*,
//! not a closed Rust enum — a `Taxonomy` is an open registry of `TaxonomyTerm`s keyed by a stable
//! URI id (which `q_hash`es cleanly and can be exported to / loaded from a cooperative ontology).
//! New terms can be added at load-time (ontology) or runtime without changing these types. Each
//! term carries an attribute bag so ABAC axioms and jurisdiction facets can grow without a schema
//! migration. Well-known terms also get `const` ids so code references them without magic strings.

use serde::{Deserialize, Serialize};

/// A stable term identifier — a URI, e.g. `urn:qualia:guardianship-role:welfare:healthcare-proxy`.
/// Kept as `String` (not an enum) so the vocabulary is open/extensible.
pub type TermId = String;

/// Selfhood vs personhood (Timothy): *selfhood* is inherent to the person and non-delegable by
/// default (genome, biometrics, reproductive autonomy); *personhood* is the socio-legal
/// relationship between an agent and societal structures, which may be delegated under a role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Sphere {
    /// Inherent to the person — never shared/delegated unless a role explicitly, narrowly permits it.
    Selfhood,
    /// Socio-legal — delegable under an appropriate role.
    #[default]
    Personhood,
}

/// A single term in some taxonomy: a stable id, a human label, an optional grouping category
/// (a coarser term id), and an open attribute bag for ABAC / ontological axioms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxonomyTerm {
    pub id: TermId,
    pub label: String,
    /// Grouping/parent term id (a facet or category). `None` for top-level facets/categories.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<TermId>,
    #[serde(default)]
    pub description: String,
    /// Extensible axioms: `("sphere","selfhood")`, `("domains","health,welfare")`,
    /// `("facet","temporal")`, jurisdiction hints, etc. Attribute keys are open by design.
    #[serde(default)]
    pub attributes: Vec<(String, String)>,
}

impl TaxonomyTerm {
    pub fn new(id: impl Into<TermId>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            category: None,
            description: String::new(),
            attributes: Vec::new(),
        }
    }

    pub fn in_category(mut self, category: impl Into<TermId>) -> Self {
        self.category = Some(category.into());
        self
    }

    pub fn described(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_attr(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push((key.into(), value.into()));
        self
    }

    /// First attribute value for `key`, if any.
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// A comma-separated attribute parsed into its members (e.g. `domains = "health,welfare"`).
    pub fn attr_list(&self, key: &str) -> Vec<String> {
        self.attr(key)
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The sphere this term operates in (attribute `sphere`), defaulting to Personhood.
    pub fn sphere(&self) -> Sphere {
        match self.attr("sphere") {
            Some("selfhood") => Sphere::Selfhood,
            _ => Sphere::Personhood,
        }
    }
}

/// An open registry of terms. Both `AuthorityType` and `GuardianshipRole` are `Taxonomy`s.
/// Extensible: `insert` adds or replaces a term by id; nothing here is closed at compile time.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Taxonomy {
    terms: Vec<TaxonomyTerm>,
}

impl Taxonomy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_terms(terms: impl IntoIterator<Item = TaxonomyTerm>) -> Self {
        let mut t = Self::new();
        for term in terms {
            t.insert(term);
        }
        t
    }

    /// Add or replace a term (idempotent by id). This is the extensibility point — a bundled
    /// ontology, a jurisdiction pack, or a user extension can register additional terms.
    pub fn insert(&mut self, term: TaxonomyTerm) {
        if let Some(existing) = self.terms.iter_mut().find(|t| t.id == term.id) {
            *existing = term;
        } else {
            self.terms.push(term);
        }
    }

    /// Merge another taxonomy's terms into this one (later terms win on id collision).
    pub fn extend_with(&mut self, other: &Taxonomy) {
        for term in &other.terms {
            self.insert(term.clone());
        }
    }

    pub fn get(&self, id: &str) -> Option<&TaxonomyTerm> {
        self.terms.iter().find(|t| t.id == id)
    }

    pub fn contains(&self, id: &str) -> bool {
        self.get(id).is_some()
    }

    /// All direct members of a category (by category term id).
    pub fn in_category(&self, category_id: &str) -> Vec<&TaxonomyTerm> {
        self.terms
            .iter()
            .filter(|t| t.category.as_deref() == Some(category_id))
            .collect()
    }

    pub fn all(&self) -> &[TaxonomyTerm] {
        &self.terms
    }

    pub fn len(&self) -> usize {
        self.terms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_get_and_replace_by_id() {
        let mut tax = Taxonomy::new();
        tax.insert(TaxonomyTerm::new("urn:x:a", "A").with_attr("v", "1"));
        assert_eq!(tax.get("urn:x:a").unwrap().attr("v"), Some("1"));
        // Idempotent replace.
        tax.insert(TaxonomyTerm::new("urn:x:a", "A2").with_attr("v", "2"));
        assert_eq!(tax.len(), 1);
        assert_eq!(tax.get("urn:x:a").unwrap().label, "A2");
        assert_eq!(tax.get("urn:x:a").unwrap().attr("v"), Some("2"));
    }

    #[test]
    fn category_membership() {
        let tax = Taxonomy::from_terms([
            TaxonomyTerm::new("urn:cat", "Cat"),
            TaxonomyTerm::new("urn:x:1", "One").in_category("urn:cat"),
            TaxonomyTerm::new("urn:x:2", "Two").in_category("urn:cat"),
            TaxonomyTerm::new("urn:y:1", "Other").in_category("urn:othercat"),
        ]);
        let members = tax.in_category("urn:cat");
        assert_eq!(members.len(), 2);
    }

    #[test]
    fn extensible_at_runtime_with_custom_term() {
        // A user / ontology pack adds a term the core never hard-coded.
        let mut tax = Taxonomy::new();
        tax.insert(
            TaxonomyTerm::new("urn:custom:role:beekeeping-steward", "Beekeeping Steward")
                .in_category("urn:qualia:guardianship-role:socio-economic")
                .with_attr("domains", "custodial,welfare")
                .with_attr("sphere", "personhood"),
        );
        assert!(tax.contains("urn:custom:role:beekeeping-steward"));
        assert_eq!(
            tax.get("urn:custom:role:beekeeping-steward").unwrap().sphere(),
            Sphere::Personhood
        );
    }

    #[test]
    fn attr_list_parses_comma_members() {
        let t = TaxonomyTerm::new("urn:x", "X").with_attr("domains", "health, welfare ,legal");
        assert_eq!(t.attr_list("domains"), vec!["health", "welfare", "legal"]);
    }

    #[test]
    fn sphere_defaults_to_personhood() {
        assert_eq!(TaxonomyTerm::new("urn:x", "X").sphere(), Sphere::Personhood);
        assert_eq!(
            TaxonomyTerm::new("urn:x", "X").with_attr("sphere", "selfhood").sphere(),
            Sphere::Selfhood
        );
    }
}
