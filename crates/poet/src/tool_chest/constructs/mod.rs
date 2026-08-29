//! Bundled construct catalogue. Academic stubs stay Library Software until seeded.

use super::core::construct::{ConstructSeed, ConstructSource};

fn construct(
    id: &str,
    label: &str,
    description: &str,
    icon: &str,
    honesty: &str,
    default_manifold: &str,
    manifold_ids: &[&str],
    source: ConstructSource,
    required_shapes: &[&str],
) -> ConstructSeed {
    ConstructSeed {
        id: id.into(),
        label: label.into(),
        description: description.into(),
        icon: icon.into(),
        observer: String::new(),
        honesty: honesty.into(),
        default_manifold: default_manifold.into(),
        manifold_ids: manifold_ids.iter().map(|id| (*id).to_string()).collect(),
        library_uri: format!("urn:poet:construct:{id}"),
        required_shapes: required_shapes.iter().map(|id| (*id).to_string()).collect(),
        source,
    }
}

/// Default POET shell — every seeded manifold (backward compatible pager).
pub fn poet_construct() -> ConstructSeed {
    construct(
        "poet",
        "POET",
        "Default observer-scope: every seeded lens (research, health, anatomy, studio, knowledge, rights, social).",
        "poet",
        "live",
        "research",
        &[
            "research",
            "media",
            "social",
            "communications",
            "knowledge",
            "ontology",
            "projects",
            "rights",
            "sanctuary",
            "health",
            "studio",
            "datasets",
            "settings",
            "devices",
            "vibe",
            "anatomy",
        ],
        ConstructSource::Bundled,
        &[],
    )
}

/// Health composition — Anatomy is a manifold on this construct, not a construct.
pub fn health_construct() -> ConstructSeed {
    construct(
        "health",
        "Health",
        "Observer's health-scope. Anatomy is a lens (manifold) on this construct, not a construct and not a nested EHR.",
        "health",
        "partial",
        "health",
        &["health", "anatomy"],
        ConstructSource::Bundled,
        &[
            "schema:HealthCondition",
            "qualia:AnatomicalStructure",
            "qualia:ConsentRecord",
        ],
    )
}

pub fn research_lab_construct() -> ConstructSeed {
    construct(
        "research-lab",
        "Research lab",
        "GIS, library, Domain Lab (clinical/chemistry/physics/bioinformatics), knowledge graph.",
        "flask",
        "live",
        "research",
        &["research", "knowledge", "ontology", "datasets"],
        ConstructSource::Bundled,
        &[],
    )
}

pub fn studio_construct() -> ConstructSeed {
    construct(
        "studio",
        "Studio",
        "Dual Studio, Scene session, Audio session — absorbed into POET, not a nested DAW.",
        "studio",
        "live",
        "studio",
        &["studio", "media"],
        ConstructSource::Bundled,
        &[],
    )
}

pub fn rights_construct() -> ConstructSeed {
    construct(
        "rights",
        "Rights",
        "Agreements, deontic norms, Hohfeld, obligations. Natural persons are rdfs:Class.",
        "scale",
        "live",
        "rights",
        &["rights", "projects", "sanctuary"],
        ConstructSource::Bundled,
        &["COP-R4"],
    )
}

/// Shared delivery work — a social manifold family. The construct is still
/// this observer's holding of that work, not a group-mind.
pub fn projects_construct() -> ConstructSeed {
    construct(
        "projects",
        "Projects",
        "Social manifolds for many people: members, presence, discussion, commons, obligations. The construct is yours; the lens is shared work.",
        "projects",
        "live",
        "projects",
        &["projects", "social", "communications", "rights"],
        ConstructSource::Bundled,
        &["schema:Project", "qualia:ContributionRecord"],
    )
}

pub fn knowledge_construct() -> ConstructSeed {
    construct(
        "knowledge",
        "Knowledge",
        "SPARQL, ontology, N3, SHACL/ShEx. Graph authoring; persons are not owl:Thing.",
        "graph",
        "live",
        "knowledge",
        &["knowledge", "ontology"],
        ConstructSource::Bundled,
        &[],
    )
}

/// Catalogue stubs — former academic QApps without a manifold seed.
pub fn stub_constructs() -> Vec<ConstructSeed> {
    [
        (
            "african-american-studies",
            "African American Studies",
            "social-sciences",
        ),
        ("anthropology", "Anthropology", "social-sciences"),
        ("philosophy", "Philosophy", "humanities"),
        ("linguistics", "Linguistics", "humanities"),
        ("legal-studies", "Legal Studies", "applied-liberal-arts"),
        ("bioethics", "Bioethics", "applied-liberal-arts"),
    ]
    .into_iter()
    .map(|(id, label, category)| ConstructSeed {
        id: id.into(),
        label: label.into(),
        description: format!(
            "Library Software stub ({category}). No manifold seed — not a pager tab."
        ),
        icon: "book".into(),
        observer: String::new(),
        honesty: "stub".into(),
        default_manifold: String::new(),
        manifold_ids: vec![],
        library_uri: format!("urn:poet:construct:{id}"),
        required_shapes: vec![],
        source: ConstructSource::Stub,
    })
    .collect()
}

/// Openable (non-stub) constructs first, then honesty-labelled stubs.
pub fn all_constructs() -> Vec<ConstructSeed> {
    let mut out = vec![
        poet_construct(),
        health_construct(),
        research_lab_construct(),
        studio_construct(),
        rights_construct(),
        knowledge_construct(),
        projects_construct(),
    ];
    out.extend(stub_constructs());
    out
}

pub fn construct_by_id(id: &str) -> Option<ConstructSeed> {
    all_constructs().into_iter().find(|c| c.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_constructs_have_manifolds_stubs_do_not() {
        let all = all_constructs();
        assert!(all.len() >= 6);
        let poet = construct_by_id("poet").unwrap();
        assert!(poet.contains_manifold("health"));
        assert_eq!(poet.honesty, "live");
        assert!(construct_by_id("anatomy").is_none());
        let health = construct_by_id("health").unwrap();
        assert_eq!(health.source, ConstructSource::Bundled);
        assert!(health.contains_manifold("anatomy"));
        let projects = construct_by_id("projects").unwrap();
        assert!(projects.contains_manifold("projects"));
        assert!(projects.contains_manifold("social"));
        assert!(health.contains_manifold("health"));
        let stub = construct_by_id("philosophy").unwrap();
        assert_eq!(stub.source, ConstructSource::Stub);
        assert!(stub.manifold_ids.is_empty());
        assert_eq!(stub.honesty, "stub");
    }

    #[test]
    fn no_stub_is_labelled_live() {
        for seed in stub_constructs() {
            assert_ne!(seed.honesty, "live", "{}", seed.id);
        }
    }
}
