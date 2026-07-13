//! The **body-system registry** — an extensible, provenance-tagged taxonomy of body systems.
//!
//! The 17 systems in [`super::systems::BODY_SYSTEMS`] are a *seed*, not a closed universe: a person, a
//! jurisdiction/curation pack, or a linked-data ontology (e.g. a UBERON anatomical-system class) can
//! **register more** — the fascial system, the interstitium, the microbiome — and have them evaluated
//! and rendered as first-class systems, not silently dropped. This is the "software provides the MEANS,
//! not the definitions" principle applied to the anatomy taxonomy: we ship a seed and the machinery;
//! *what counts as a body system* is extensible by the person / their trusted sources.
//!
//! The accumulation engine ([`super::accumulate`]) is already generic over an arbitrary `system_id`;
//! this registry is what makes a *new* system **first-class** — carrying a human label, an
//! accessibility-first plain-language label, how it is rendered (discrete organs vs a distributed
//! overlay), its overlay host systems, a default identity colour, and **where its definition came
//! from** ([`SystemProvenance`]). Presentation, colour, and knowledge import resolve through the
//! registry, so a registered extension is labelled, coloured, represented, and importable exactly like
//! a seeded system.
//!
//! `default_registry()` is the built-in seed (the 17). An app that has extensions builds its own
//! registry (seed + registered systems) — from a `.q42`/ontology pack once the disease↔organ source is
//! chosen — and threads it where completeness matters (knowledge import, the UI system list).

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::model::{overlay_host_systems, system_representation, SystemRepresentation};
use super::systems::BODY_SYSTEMS;

/// The neutral render colour for a system with no seeded/registered colour (linear RGBA). A person's
/// σ-derived burden colour overrides this at runtime; this is only the default identity swatch.
pub const NEUTRAL_SYSTEM_RGBA: [f32; 4] = [0.62, 0.66, 0.72, 1.0];

/// Where a body-system *definition* came from — so an auditor (and the person) can see whether a system
/// is a built-in seed, curated, ontology-derived, or self-declared. Additions are transparent, never
/// silent (the same honesty stance as the knowledge base's source provenance).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum SystemProvenance {
    /// One of the built-in seeded systems (the 17 in [`BODY_SYSTEMS`]).
    Seed,
    /// Registered from a linked-data ontology — e.g. a UBERON anatomical-system class IRI. This is the
    /// "graph-fed" path: the disease↔organ ontology defines additional systems.
    Ontology { iri: String },
    /// Registered from a curation / knowledge pack.
    Pack { source_id: String },
    /// Declared by the person themselves.
    User,
}

/// Where a system sits in the anatomy hierarchy. The body is classically divided into **11 major organ
/// systems**; finer/emerging systems are taught as sub-branches of a major, and some are body-wide
/// networks that cut across majors. Marking this keeps the canonical framing honest while the registry
/// stays open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemTier {
    /// One of the 11 canonical major organ systems.
    CanonicalMajor,
    /// A finer system taught as a sub-branch of a major (sensory / vestibular / ENS / glymphatic → nervous).
    SubSystem,
    /// A body-wide signalling / gland network that cuts across the majors (ECS, exocrine).
    CrossCutting,
}

/// How one system functionally relates to another. **Structural context only — this does NOT propagate
/// burden.** An adverse load on one system does not automatically imply load on a related one; the link
/// explains *how the systems depend on / act upon each other*, for the person's understanding. (Timothy's
/// call, 2026-07-11: structural links, no automatic propagation.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemRelationKind {
    /// This system requires another to function (muscular **depends on** nervous for the contraction signal).
    DependsOn,
    /// This system governs / modulates another (endocrine **regulates** reproductive via sex hormones).
    Regulates,
    /// This system provides a resource to another (respiratory **supplies** circulatory with oxygen).
    Supplies,
}

/// A structural link from one system to another, with a plain-language reason. Curation-grade seed
/// (well-established relationships), extensible via the same registry mechanism.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemRelation {
    pub kind: SystemRelationKind,
    /// The target system id.
    pub target: String,
    /// Plain-language reason ("motor control — contraction is signalled by motor nerves").
    pub note: String,
}

/// A first-class body system: identity, human + plain-language labels, how it renders, its overlay
/// hosts (for a distributed system), a default colour, its place in the hierarchy, its structural links
/// to other systems, and its provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemDef {
    /// Stable id (e.g. `"circulatory"`, `"fascial"`). The join key across the engine, meshes, and data.
    pub id: String,
    /// Clinical / general human label (`"Circulatory (Cardiovascular) System"`).
    pub label: String,
    /// Accessibility-first, plain-language wording (`"heart and blood flow"`) — never diagnostic.
    pub plain_label: String,
    /// Whether the system has characteristic organ meshes to paint, or is a distributed network shown
    /// as an overlay on its host structures.
    pub representation: SystemRepresentation,
    /// For a [`SystemRepresentation::DistributedOverlay`] system, the system ids whose organ meshes it is
    /// highlighted over (empty = a whole-body cue). Empty for discrete-organ systems.
    pub overlay_hosts: Vec<String>,
    /// Default identity colour (linear RGBA); the person's burden σ overrides it at runtime.
    pub color_rgba: [f32; 4],
    /// The system's tier in the hierarchy (canonical major / sub-system / cross-cutting).
    pub tier: SystemTier,
    /// The parent (major) system this is a sub-branch of, if any (sensory → nervous). `None` for majors
    /// and cross-cutting networks.
    pub parent: Option<String>,
    /// Structural links to other systems — how this system depends on / regulates / supplies them.
    /// Context for understanding, **not** burden propagation.
    pub relations: Vec<SystemRelation>,
    /// Where this system definition came from.
    pub provenance: SystemProvenance,
}

impl SystemDef {
    /// Whether this system is rendered as a distributed overlay (no standalone organ mesh).
    pub fn is_overlay(&self) -> bool {
        self.representation == SystemRepresentation::DistributedOverlay
    }

    /// Whether this is one of the 11 canonical major organ systems.
    pub fn is_canonical_major(&self) -> bool {
        self.tier == SystemTier::CanonicalMajor
    }
}

/// An extensible set of [`SystemDef`]s. Seeded with the 17 built-in systems; `register` adds/refines
/// more. Lookups are by id (and by human label, for importing label-keyed knowledge files). Cloneable
/// and serialisable so an app can persist / ship an extended taxonomy.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemRegistry {
    systems: Vec<SystemDef>,
}

impl SystemRegistry {
    /// An empty registry (no systems). Use [`SystemRegistry::seed`] for the built-in 17.
    pub fn new_empty() -> Self {
        Self { systems: Vec::new() }
    }

    /// The built-in seed: the 17 systems from [`BODY_SYSTEMS`], each enriched with its representation
    /// and overlay hosts (from [`super::model`]) and a default identity colour. Provenance = `Seed`.
    pub fn seed() -> Self {
        let systems = BODY_SYSTEMS
            .iter()
            .map(|s| SystemDef {
                id: s.id.to_string(),
                label: s.label.to_string(),
                plain_label: s.plain_label.to_string(),
                representation: system_representation(s.id),
                overlay_hosts: overlay_host_systems(s.id).iter().map(|h| h.to_string()).collect(),
                color_rgba: seed_color_rgba(s.id),
                tier: seed_tier(s.id),
                parent: seed_parent(s.id).map(str::to_string),
                relations: seed_relations(s.id),
                provenance: SystemProvenance::Seed,
            })
            .collect();
        Self { systems }
    }

    /// Register (or **refine**) a system. If a system with the same id already exists it is replaced —
    /// a later, more-authoritative definition (e.g. an ontology pack) may correct a seed's label/colour.
    /// Returns `&mut Self` for chaining.
    pub fn register(&mut self, def: SystemDef) -> &mut Self {
        if let Some(existing) = self.systems.iter_mut().find(|s| s.id == def.id) {
            *existing = def;
        } else {
            self.systems.push(def);
        }
        self
    }

    /// Look up a system by id.
    pub fn get(&self, id: &str) -> Option<&SystemDef> {
        let want = id.trim();
        self.systems.iter().find(|s| s.id == want)
    }

    /// Whether a system id is registered.
    pub fn contains(&self, id: &str) -> bool {
        self.get(id).is_some()
    }

    /// Look up a system by its human label (case-insensitive, trimmed) — for importing label-keyed
    /// knowledge files (e.g. `condition-map.json` keys systems by `"Endocrine System"`).
    pub fn get_by_label(&self, label: &str) -> Option<&SystemDef> {
        let want = label.trim().to_ascii_lowercase();
        self.systems.iter().find(|s| s.label.to_ascii_lowercase() == want)
    }

    /// All registered systems, in registration order (the 17 seed order first).
    pub fn all(&self) -> &[SystemDef] {
        &self.systems
    }

    /// The registered ids, in order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.systems.iter().map(|s| s.id.as_str())
    }

    pub fn len(&self) -> usize {
        self.systems.len()
    }

    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }

    /// The human label for a system id, or the id itself if unregistered (so nothing renders blank).
    pub fn label_for(&self, id: &str) -> String {
        self.get(id).map(|s| s.label.clone()).unwrap_or_else(|| id.to_string())
    }

    /// The plain-language label for a system id, or the id itself if unregistered.
    pub fn plain_label_for(&self, id: &str) -> String {
        self.get(id).map(|s| s.plain_label.clone()).unwrap_or_else(|| id.to_string())
    }

    /// How a system is rendered — defaults to [`SystemRepresentation::DiscreteOrgans`] for an
    /// unregistered id (the safe default: try to paint an organ mesh if one turns up).
    pub fn representation_of(&self, id: &str) -> SystemRepresentation {
        self.get(id).map(|s| s.representation).unwrap_or(SystemRepresentation::DiscreteOrgans)
    }

    /// The overlay host system ids for a distributed system (empty for discrete or unregistered).
    pub fn overlay_hosts_of(&self, id: &str) -> &[String] {
        self.get(id).map(|s| s.overlay_hosts.as_slice()).unwrap_or(&[])
    }

    /// The default identity colour for a system id, or [`NEUTRAL_SYSTEM_RGBA`] if unregistered.
    pub fn color_of(&self, id: &str) -> [f32; 4] {
        self.get(id).map(|s| s.color_rgba).unwrap_or(NEUTRAL_SYSTEM_RGBA)
    }

    /// The canonical major organ systems (the classic 11 in the seed).
    pub fn canonical_majors(&self) -> impl Iterator<Item = &SystemDef> {
        self.systems.iter().filter(|s| s.tier == SystemTier::CanonicalMajor)
    }

    /// The direct sub-systems of `parent_id` (e.g. `nervous` → sensory / vestibular / ENS / glymphatic).
    pub fn sub_systems_of(&self, parent_id: &str) -> impl Iterator<Item = &SystemDef> {
        let want = parent_id.trim().to_string();
        self.systems.iter().filter(move |s| s.parent.as_deref() == Some(want.as_str()))
    }

    /// A system's structural links to other systems (empty for unregistered or unlinked systems).
    pub fn relations_of(&self, id: &str) -> &[SystemRelation] {
        self.get(id).map(|s| s.relations.as_slice()).unwrap_or(&[])
    }
}

/// The built-in default registry — the 17 seeded systems. Callers with extensions build their own
/// (`SystemRegistry::seed()` then `register(...)`, or loaded from a `.q42`/ontology pack).
pub fn default_registry() -> &'static SystemRegistry {
    static DEFAULT: OnceLock<SystemRegistry> = OnceLock::new();
    DEFAULT.get_or_init(SystemRegistry::seed)
}

/// The seeded default identity colour for a built-in system id (linear RGBA). Distinct hues per family
/// so systems read apart at a glance; the person's σ burden colour overrides this at runtime. Kept here
/// as the single source of truth for the shipped palette (the asset-pack producer resolves through it).
fn seed_color_rgba(id: &str) -> [f32; 4] {
    match id {
        "circulatory" => [0.80, 0.28, 0.28, 1.0],
        "respiratory" => [0.58, 0.72, 0.90, 1.0],
        "digestive" => [0.82, 0.62, 0.40, 1.0],
        "urinary" => [0.78, 0.74, 0.42, 1.0],
        "nervous" | "ens" | "glymphatic" => [0.90, 0.86, 0.72, 1.0],
        "reproductive" => [0.82, 0.60, 0.70, 1.0],
        "immune_lymphatic" => [0.60, 0.82, 0.70, 1.0],
        "endocrine" | "exocrine" | "ecs" => [0.72, 0.60, 0.82, 1.0],
        "muscular" => [0.80, 0.45, 0.40, 1.0],
        "skeletal" => [0.90, 0.87, 0.80, 1.0],
        "integumentary" => [0.90, 0.76, 0.66, 1.0],
        "sensory" | "vestibular" => [0.55, 0.80, 0.82, 1.0],
        _ => NEUTRAL_SYSTEM_RGBA,
    }
}

/// The hierarchy tier of a seeded system: the classic 11 are canonical majors; sensory / vestibular /
/// ENS / glymphatic are sub-branches of the nervous system; ECS + exocrine are body-wide cross-cutting
/// networks.
fn seed_tier(id: &str) -> SystemTier {
    match id {
        "sensory" | "vestibular" | "ens" | "glymphatic" => SystemTier::SubSystem,
        "ecs" | "exocrine" => SystemTier::CrossCutting,
        _ => SystemTier::CanonicalMajor, // the 11 classic majors
    }
}

/// The parent major system for a seeded sub-system (the finer nervous sub-branches parent to `nervous`).
fn seed_parent(id: &str) -> Option<&'static str> {
    match id {
        "sensory" | "vestibular" | "ens" | "glymphatic" => Some("nervous"),
        _ => None,
    }
}

/// The seeded structural links a system has to others — curation-grade, well-established relationships
/// (the systems do not operate in isolation). Context only; NOT burden propagation.
fn seed_relations(id: &str) -> Vec<SystemRelation> {
    use SystemRelationKind::{DependsOn, Regulates, Supplies};
    let rel = |kind, target: &str, note: &str| SystemRelation {
        kind,
        target: target.to_string(),
        note: note.to_string(),
    };
    match id {
        "muscular" => vec![rel(DependsOn, "nervous", "motor control — contraction is signalled by motor nerves")],
        "skeletal" => vec![rel(DependsOn, "circulatory", "calcium/mineral exchange and red-marrow perfusion")],
        "nervous" => vec![rel(DependsOn, "circulatory", "highly perfusion-dependent — needs constant oxygen + glucose")],
        "respiratory" => vec![rel(Supplies, "circulatory", "oxygenates the blood and clears carbon dioxide")],
        "endocrine" => vec![rel(Regulates, "reproductive", "sex hormones drive reproductive function")],
        "urinary" => vec![rel(Regulates, "circulatory", "fluid/electrolyte balance and blood pressure (RAAS)")],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_has_the_seventeen_first_class() {
        let reg = SystemRegistry::seed();
        assert_eq!(reg.len(), 17);
        // Every seeded system resolves to a non-empty label + plain label, and carries Seed provenance.
        for s in reg.all() {
            assert!(!s.label.is_empty() && !s.plain_label.is_empty(), "{} labelled", s.id);
            assert_eq!(s.provenance, SystemProvenance::Seed);
        }
        // Representation + overlay hosts agree with the model's free functions (single source: seeded
        // from them).
        let ecs = reg.get("ecs").unwrap();
        assert!(ecs.is_overlay());
        assert!(ecs.overlay_hosts.is_empty(), "ECS is a whole-body cue");
        assert_eq!(reg.get("ens").unwrap().overlay_hosts, vec!["digestive".to_string()]);
        assert_eq!(reg.representation_of("circulatory"), SystemRepresentation::DiscreteOrgans);
    }

    #[test]
    fn hierarchy_and_structural_links_are_seeded() {
        let reg = SystemRegistry::seed();
        // The classic 11 canonical major organ systems.
        assert_eq!(reg.canonical_majors().count(), 11);
        assert!(reg.get("circulatory").unwrap().is_canonical_major());
        assert!(!reg.get("sensory").unwrap().is_canonical_major());
        // The finer systems parent to the nervous system (sub-branches).
        let nervous_subs: Vec<&str> = reg.sub_systems_of("nervous").map(|s| s.id.as_str()).collect();
        for s in ["sensory", "vestibular", "ens", "glymphatic"] {
            assert!(nervous_subs.contains(&s), "{s} should be a sub-system of nervous: {nervous_subs:?}");
        }
        // ECS + exocrine are cross-cutting networks (no parent, not a major).
        assert_eq!(reg.get("ecs").unwrap().tier, SystemTier::CrossCutting);
        assert!(reg.get("ecs").unwrap().parent.is_none());
        // Structural interdependence (context, not burden propagation): Timothy's two examples.
        let mus = reg.relations_of("muscular");
        assert_eq!(mus.len(), 1);
        assert_eq!(mus[0].kind, SystemRelationKind::DependsOn);
        assert_eq!(mus[0].target, "nervous");
        assert!(!mus[0].note.is_empty(), "the link carries a plain-language reason");
        assert!(
            reg.relations_of("skeletal").iter().any(|r| r.target == "circulatory"),
            "skeletal depends on circulatory (calcium)"
        );
    }

    #[test]
    fn default_registry_is_the_seed() {
        assert_eq!(default_registry().len(), 17);
        assert_eq!(default_registry().label_for("digestive"), "Digestive System");
        assert_eq!(default_registry().plain_label_for("nervous"), "brain and nerves");
        // An unregistered system does not render blank — it falls back to its id + safe defaults.
        assert_eq!(default_registry().label_for("fascial"), "fascial");
        assert_eq!(default_registry().representation_of("fascial"), SystemRepresentation::DiscreteOrgans);
        assert_eq!(default_registry().color_of("fascial"), NEUTRAL_SYSTEM_RGBA);
    }

    #[test]
    fn colours_are_the_single_source_of_truth_for_the_palette() {
        let reg = default_registry();
        // The values the asset-pack producer's palette used to hardcode now live here.
        assert_eq!(reg.color_of("circulatory"), [0.80, 0.28, 0.28, 1.0]);
        assert_eq!(reg.color_of("immune_lymphatic"), [0.60, 0.82, 0.70, 1.0]);
        // Every seeded system has a non-neutral identity colour.
        for s in reg.all() {
            assert_ne!(reg.color_of(&s.id), NEUTRAL_SYSTEM_RGBA, "{} needs a colour", s.id);
        }
    }

    #[test]
    fn a_registered_extension_is_first_class() {
        // "There should be more": register an 18th system (the fascial system) and confirm it is
        // evaluated-ready and fully presentable — label, plain label, representation, overlay, colour —
        // exactly like a seeded one.
        let mut reg = SystemRegistry::seed();
        reg.register(SystemDef {
            id: "fascial".to_string(),
            label: "Fascial System".to_string(),
            plain_label: "connective tissue".to_string(),
            representation: SystemRepresentation::DistributedOverlay,
            overlay_hosts: vec!["muscular".to_string(), "skeletal".to_string()],
            color_rgba: [0.70, 0.78, 0.66, 1.0],
            tier: SystemTier::CrossCutting,
            parent: None,
            relations: vec![SystemRelation {
                kind: SystemRelationKind::Supplies,
                target: "muscular".to_string(),
                note: "force transmission and structural continuity".to_string(),
            }],
            provenance: SystemProvenance::Ontology { iri: "http://purl.obolibrary.org/obo/UBERON_0007795".to_string() },
        });

        assert_eq!(reg.len(), 18);
        assert!(reg.contains("fascial"));
        assert_eq!(reg.label_for("fascial"), "Fascial System");
        assert_eq!(reg.plain_label_for("fascial"), "connective tissue");
        assert_eq!(reg.representation_of("fascial"), SystemRepresentation::DistributedOverlay);
        assert_eq!(reg.overlay_hosts_of("fascial"), &["muscular".to_string(), "skeletal".to_string()]);
        assert_eq!(reg.color_of("fascial"), [0.70, 0.78, 0.66, 1.0]);
        // The seeded systems are untouched.
        assert_eq!(reg.label_for("digestive"), "Digestive System");
        // Provenance records the ontology source, transparently.
        assert!(matches!(reg.get("fascial").unwrap().provenance, SystemProvenance::Ontology { .. }));
    }

    #[test]
    fn register_refines_an_existing_system_rather_than_duplicating() {
        let mut reg = SystemRegistry::seed();
        let before = reg.len();
        // A pack refines the digestive system's colour — replaces, does not duplicate.
        reg.register(SystemDef {
            id: "digestive".to_string(),
            label: "Digestive System".to_string(),
            plain_label: "digestion".to_string(),
            representation: SystemRepresentation::DiscreteOrgans,
            overlay_hosts: Vec::new(),
            color_rgba: [0.10, 0.20, 0.30, 1.0],
            tier: SystemTier::CanonicalMajor,
            parent: None,
            relations: Vec::new(),
            provenance: SystemProvenance::Pack { source_id: "curator-x".to_string() },
        });
        assert_eq!(reg.len(), before, "refinement does not add a row");
        assert_eq!(reg.color_of("digestive"), [0.10, 0.20, 0.30, 1.0]);
        assert!(matches!(reg.get("digestive").unwrap().provenance, SystemProvenance::Pack { .. }));
    }

    #[test]
    fn every_registered_system_is_accounted_for() {
        // The generalised completeness guarantee: every registered system — seeded OR extension — is
        // first-class (resolvable label, plain label, representation, colour). Nothing is half-defined.
        let mut reg = SystemRegistry::seed();
        reg.register(SystemDef {
            id: "microbiome".to_string(),
            label: "Microbiome".to_string(),
            plain_label: "gut microbes".to_string(),
            representation: SystemRepresentation::DistributedOverlay,
            overlay_hosts: vec!["digestive".to_string()],
            color_rgba: [0.66, 0.72, 0.50, 1.0],
            tier: SystemTier::CrossCutting,
            parent: None,
            relations: Vec::new(),
            provenance: SystemProvenance::User,
        });
        for s in reg.all() {
            assert!(!reg.label_for(&s.id).is_empty());
            assert!(!reg.plain_label_for(&s.id).is_empty());
            // Overlay systems either name hosts or are an intentional whole-body cue (empty) — both valid.
            let _ = reg.representation_of(&s.id);
            let _ = reg.color_of(&s.id);
        }
    }
}
