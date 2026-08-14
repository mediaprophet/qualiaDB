//! Which anatomical **reference model** the 3D body uses, and how each organ maps to a body system.
//!
//! The model is selected from the user's declared **chromosomal basis** — their "DNA selection",
//! `XY` or `XX`. This is a *biological-substrate attribute the user declares*, **not** a gender or an
//! identity claim: it is one attribute among many, never collapsed into identity (see the
//! DID-is-identifier-not-identity stance). It selects which Visible Human reference mesh set applies
//! (the CCF / HRA `ccf-3d-reference-object-library`, CC-BY-4.0: `VH_Male` / `VH_Female`), so a person's
//! records are mapped onto anatomy that matches their body.
//!
//! `XY`/`XX` are the two Timothy named and the two the CCF library ships. The type is deliberately a
//! small closed enum rather than a free string so a caller cannot smuggle an unvalidated value in; if
//! additional karyotypes are ever curated they are an explicit, reviewed extension, not silent drift.

use serde::{Deserialize, Serialize};

/// The declared chromosomal basis — the user's "DNA selection". A biological-substrate attribute, not
/// an identity or gender claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Karyotype {
    /// XY → the male reference model.
    Xy,
    /// XX → the female reference model.
    Xx,
}

/// Which Visible Human reference model the 3D body renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnatomyModel {
    Male,
    Female,
}

impl Karyotype {
    /// The reference model this chromosomal basis selects: `XY → Male`, `XX → Female`.
    pub fn anatomy_model(self) -> AnatomyModel {
        match self {
            Karyotype::Xy => AnatomyModel::Male,
            Karyotype::Xx => AnatomyModel::Female,
        }
    }

    /// The canonical two-letter token (`"XY"` / `"XX"`).
    pub fn as_str(self) -> &'static str {
        match self {
            Karyotype::Xy => "XY",
            Karyotype::Xx => "XX",
        }
    }

    /// Parse a declared selection (case-insensitive, trimmed). `None` for anything but the two
    /// curated values — the caller decides how to handle an unrecognised declaration (fail-closed,
    /// prompt, etc.) rather than us guessing a body for someone.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "XY" => Some(Karyotype::Xy),
            "XX" => Some(Karyotype::Xx),
            _ => None,
        }
    }
}

impl AnatomyModel {
    /// The CCF / HRA asset-set directory prefix (`"VH_Male"` / `"VH_Female"`).
    pub fn asset_set(self) -> &'static str {
        match self {
            AnatomyModel::Male => "VH_Male",
            AnatomyModel::Female => "VH_Female",
        }
    }

    /// The organ-filename infix the CCF assets use (`3d-vh-`**`m`**`-lung.glb` /
    /// `3d-vh-`**`f`**`-lung.glb`).
    pub fn file_infix(self) -> &'static str {
        match self {
            AnatomyModel::Male => "m",
            AnatomyModel::Female => "f",
        }
    }

    /// Lowercase model token for manifest facts / DTOs (`"male"` / `"female"`).
    pub fn as_str(self) -> &'static str {
        match self {
            AnatomyModel::Male => "male",
            AnatomyModel::Female => "female",
        }
    }

    /// Parse a model token (case-insensitive).
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "male" => Some(AnatomyModel::Male),
            "female" => Some(AnatomyModel::Female),
            _ => None,
        }
    }
}

/// Normalize a CCF/HRA organ key to a model-agnostic base token: strip the `3d-vh-m-` / `3d-vh-f-`
/// asset prefix, the `.glb` suffix, and a trailing laterality (`-l`/`-r`/`-left`/`-right`), leaving the
/// bare organ (`"3d-vh-m-eye-l.glb"` → `"eye"`, `"blood-vasculature"` → `"blood-vasculature"`).
pub fn normalize_organ_key(raw: &str) -> String {
    let mut s = raw.trim().to_ascii_lowercase();
    for ext in [".glb", ".gltf", ".obj", ".stl"] {
        if let Some(rest) = s.strip_suffix(ext) {
            s = rest.to_string();
            break;
        }
    }
    // Strip the CCF asset prefix up to and including the sex marker `-m-`/`-f-`. The provider varies
    // (`vh`, `allen`, `sbu`, `nih`, …), so match the marker rather than a fixed prefix; organ tokens
    // never contain a dash-bounded single `m`/`f`, so the first occurrence is always the sex marker.
    let has_3d = s.starts_with("3d-");
    if let Some(pos) = s.find("-m-").or_else(|| s.find("-f-")) {
        if has_3d || pos <= 8 {
            s = s[pos + 3..].to_string();
        }
    }
    for lat in ["-left", "-right", "-l", "-r"] {
        if let Some(rest) = s.strip_suffix(lat) {
            s = rest.to_string();
            break;
        }
    }
    s
}

/// Model-agnostic organ → body-system map for the common HRA/CCF organs. The `AnatomyModel` decides
/// which organs are *present* in a body (a `VH_Male` set has `prostate`, a `VH_Female` set has
/// `uterus`); this table only says which of the 17 systems an organ belongs to, so a loaded organ mesh
/// can be coloured by that system's burden. Curated, not guessed — an unknown organ returns `None` and
/// is reported by the caller, never silently coloured.
static ORGAN_SYSTEMS: &[(&str, &str)] = &[
    // Circulatory
    ("blood-vasculature", "circulatory"),
    ("heart", "circulatory"),
    ("vasculature", "circulatory"),
    // Respiratory
    ("lung", "respiratory"),
    ("larynx", "respiratory"),
    ("trachea", "respiratory"),
    ("bronchus", "respiratory"),
    ("main-bronchus", "respiratory"),
    ("respiratory-system", "respiratory"),
    // Digestive
    ("liver", "digestive"),
    ("stomach", "digestive"),
    ("mouth", "digestive"),
    ("small-intestine", "digestive"),
    ("large-intestine", "digestive"),
    ("colon", "digestive"),
    ("pancreas", "digestive"),
    ("gallbladder", "digestive"),
    ("esophagus", "digestive"),
    // Nervous
    ("brain", "nervous"),
    ("spinal-cord", "nervous"),
    ("nerve", "nervous"),
    // Sensory
    ("eye", "sensory"),
    ("ear", "sensory"),
    ("cochlea", "sensory"),
    // Vestibular (balance — the inner-ear apparatus; distinct from hearing)
    ("inner-ear", "vestibular"),
    ("semicircular-canal", "vestibular"),
    ("vestibule", "vestibular"),
    // Urinary
    ("kidney", "urinary"),
    ("bladder", "urinary"),
    ("urinary-bladder", "urinary"),
    ("ureter", "urinary"),
    ("urethra", "urinary"),
    ("renal-pyramid", "urinary"),
    // Immune / lymphatic
    ("spleen", "immune_lymphatic"),
    ("thymus", "immune_lymphatic"),
    ("lymph-node", "immune_lymphatic"),
    ("tonsil", "immune_lymphatic"),
    ("lymphatic-system", "immune_lymphatic"),
    // Integumentary
    ("skin", "integumentary"),
    ("mammary-gland", "integumentary"),
    // Skeletal
    ("bone", "skeletal"),
    ("skeleton", "skeletal"),
    ("rib", "skeletal"),
    ("pelvis", "skeletal"),
    // Muscular
    ("muscle", "muscular"),
    ("diaphragm", "muscular"), // a muscle (primary) that is also the engine of respiration (secondary)
    // Endocrine
    ("thyroid-gland", "endocrine"),
    ("thyroid", "endocrine"),
    ("adrenal-gland", "endocrine"),
    ("pituitary-gland", "endocrine"),
    // Exocrine (duct glands; several are functionally shared with digestive/integumentary — the
    // liver and pancreas are dual-role and mapped to digestive as their primary macroscopic organ).
    ("salivary-gland", "exocrine"),
    ("parotid-gland", "exocrine"),
    ("submandibular-gland", "exocrine"),
    ("sublingual-gland", "exocrine"),
    ("lacrimal-gland", "exocrine"),
    ("sweat-gland", "exocrine"),
    ("sebaceous-gland", "exocrine"),
    // Reproductive (model-specific organs — present per the loaded model's asset set)
    ("prostate", "reproductive"),
    ("testis", "reproductive"),
    ("uterus", "reproductive"),
    ("ovary", "reproductive"),
    ("fallopian-tube", "reproductive"),
    ("vagina", "reproductive"),
    ("placenta", "reproductive"),
    ("placenta-full-term", "reproductive"),
];

/// The **primary** body-system id for an HRA/CCF organ (after [`normalize_organ_key`]), or `None` if the
/// organ is not in the curated map. An organ is a *building block that participates in several systems*
/// (the pancreas is digestive **and** endocrine); this returns its primary macroscopic home — the one
/// used for default colour/placement. Use [`system_memberships_for_organ`] for the full set.
pub fn body_system_for_organ(organ: &str) -> Option<&'static str> {
    let key = normalize_organ_key(organ);
    ORGAN_SYSTEMS
        .iter()
        .find(|(o, _)| *o == key)
        .map(|(_, sys)| *sys)
}

/// An organ's **additional** system memberships beyond its primary — the dual/multi-role structures. An
/// organ is a building block shared across systems: the pancreas secretes digestive enzymes *and* insulin
/// (endocrine); the kidney filters blood *and* makes hormones (renin/EPO/vit-D); skin is a barrier that
/// also senses and secretes. Curation-grade (well-established textbook roles), extensible. Every id here
/// is a real body system (a test asserts it). This is the data behind organ→**many** systems.
static ORGAN_SECONDARY_SYSTEMS: &[(&str, &str)] = &[
    ("pancreas", "endocrine"),     // islets of Langerhans → insulin/glucagon
    ("pancreas", "exocrine"),      // acinar cells → digestive enzymes via ducts
    ("liver", "endocrine"),        // IGF-1, angiotensinogen, thrombopoietin, hormone metabolism
    ("liver", "exocrine"),         // bile secreted via the biliary ducts
    ("kidney", "endocrine"),       // renin, erythropoietin, calcitriol activation
    ("ovary", "endocrine"),        // oestrogen / progesterone
    ("testis", "endocrine"),       // testosterone
    ("bone", "immune_lymphatic"),  // red marrow → haematopoiesis of immune cells
    ("skin", "sensory"),           // cutaneous mechano/thermo/nociceptors
    ("skin", "exocrine"),          // sweat + sebaceous glands
    ("mammary-gland", "exocrine"), // milk secreted via ducts
    ("diaphragm", "respiratory"),  // the principal muscle of ventilation
];

/// All body systems an organ participates in, as `(system_id, is_primary)` — the primary first, then any
/// additional memberships. Empty if the organ is unmapped. This is what makes an organ a shared building
/// block rather than a leaf of one system: a caller can colour by the primary (default) **or** blend across
/// all memberships, and a person's condition on any member system lights the organ.
/// Seeded primary organ → system pairs (cold authoring / coverage). Overlay systems
/// do not appear as primaries.
pub fn primary_organ_system_pairs() -> &'static [(&'static str, &'static str)] {
    ORGAN_SYSTEMS
}

/// Seeded extra memberships (an organ participates in more than its primary system).
pub fn secondary_organ_system_pairs() -> &'static [(&'static str, &'static str)] {
    ORGAN_SECONDARY_SYSTEMS
}

pub fn system_memberships_for_organ(organ: &str) -> Vec<(&'static str, bool)> {
    let key = normalize_organ_key(organ);
    let mut out: Vec<(&'static str, bool)> = Vec::new();
    if let Some((_, sys)) = ORGAN_SYSTEMS.iter().find(|(o, _)| *o == key) {
        out.push((*sys, true));
    }
    for (o, sys) in ORGAN_SECONDARY_SYSTEMS {
        if *o == key && !out.iter().any(|(s, _)| s == sys) {
            out.push((*sys, false));
        }
    }
    out
}

/// How a body system is rendered on the 3D body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemRepresentation {
    /// Has one or more characteristic organ meshes — colour them directly (the σ percept paints the mesh).
    DiscreteOrgans,
    /// No standalone organ mesh — a signalling / neural / clearance network that lives *across* other
    /// organs, rendered as a highlight/overlay on its host structures (or a whole-body cue).
    DistributedOverlay,
}

/// Classify one of the 17 systems by how it is rendered. This is an anatomical fact, not a coverage
/// gap: the **endocannabinoid** system (CB1/CB2 receptors throughout the CNS + periphery), the
/// **enteric-nervous** system (the ~500-million-neuron web lining the gut), and the **glymphatic**
/// system (astrocyte + cerebrospinal-fluid clearance in the brain) are real and first-class in the
/// taxonomy — they accumulate burden and get a σ percept — but they have no single organ to paint, so
/// they are surfaced as an overlay on their host structures. Every other system has discrete organs.
pub fn system_representation(system_id: &str) -> SystemRepresentation {
    match system_id {
        "ecs" | "ens" | "glymphatic" => SystemRepresentation::DistributedOverlay,
        _ => SystemRepresentation::DiscreteOrgans,
    }
}

/// For a [`SystemRepresentation::DistributedOverlay`] system, the discrete systems whose organ meshes
/// it should be highlighted **over** — anatomical placement hints for the overlay render. The enteric
/// nervous system lines the gut → overlay the digestive organs; the glymphatic system clears the brain
/// → overlay the nervous system; the endocannabinoid system's receptors are body-wide → an empty slice
/// meaning a **whole-body** cue rather than a localised highlight. Returns `&[]` for discrete-organ
/// systems (they paint their own mesh and have no overlay host); call it only for distributed systems.
pub fn overlay_host_systems(system_id: &str) -> &'static [&'static str] {
    match system_id {
        "ens" => &["digestive"],
        "glymphatic" => &["nervous"],
        "ecs" => &[], // receptors are everywhere → whole-body cue
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `body_system` (the systems registry validity lookup) is used only by
    // `every_mapped_system_id_is_a_real_body_system`, so it is scoped to the tests.
    use crate::anatomy::systems::body_system;

    #[test]
    fn karyotype_selects_reference_model() {
        assert_eq!(Karyotype::Xy.anatomy_model(), AnatomyModel::Male);
        assert_eq!(Karyotype::Xx.anatomy_model(), AnatomyModel::Female);
        assert_eq!(Karyotype::Xy.anatomy_model().asset_set(), "VH_Male");
        assert_eq!(Karyotype::Xx.anatomy_model().asset_set(), "VH_Female");
        assert_eq!(AnatomyModel::Male.file_infix(), "m");
        assert_eq!(AnatomyModel::Female.file_infix(), "f");
    }

    #[test]
    fn karyotype_parse_is_closed_and_case_insensitive() {
        assert_eq!(Karyotype::parse("xy"), Some(Karyotype::Xy));
        assert_eq!(Karyotype::parse("  XX "), Some(Karyotype::Xx));
        // Not a curated value → None (caller fails closed rather than guessing a body).
        assert_eq!(Karyotype::parse("XXY"), None);
        assert_eq!(Karyotype::parse(""), None);
        // Round-trip.
        assert_eq!(
            Karyotype::parse(Karyotype::Xy.as_str()),
            Some(Karyotype::Xy)
        );
    }

    #[test]
    fn organ_keys_normalize_across_asset_and_lateral_forms() {
        assert_eq!(normalize_organ_key("3d-vh-m-eye-l.glb"), "eye");
        assert_eq!(normalize_organ_key("3d-vh-f-eye-r.glb"), "eye");
        assert_eq!(normalize_organ_key("eye-left"), "eye");
        assert_eq!(
            normalize_organ_key("blood-vasculature"),
            "blood-vasculature"
        );
        assert_eq!(normalize_organ_key("  VH-M-Lung  "), "lung");
        // The provider varies across the real CCF set — strip on the sex marker, not a fixed prefix.
        assert_eq!(normalize_organ_key("3d-allen-m-brain.glb"), "brain");
        assert_eq!(
            normalize_organ_key("3d-sbu-m-large-intestine.glb"),
            "large-intestine"
        );
        assert_eq!(normalize_organ_key("3d-nih-f-lymph-node.glb"), "lymph-node");
        assert_eq!(
            normalize_organ_key("3d-vh-m-main-bronchus.glb"),
            "main-bronchus"
        );
    }

    #[test]
    fn organs_map_to_systems_and_unknowns_are_none() {
        assert_eq!(
            body_system_for_organ("3d-vh-m-lung.glb"),
            Some("respiratory")
        );
        assert_eq!(
            body_system_for_organ("blood-vasculature"),
            Some("circulatory")
        );
        assert_eq!(body_system_for_organ("eye-left"), Some("sensory"));
        assert_eq!(body_system_for_organ("kidney-r"), Some("urinary"));
        // Model-specific reproductive organs.
        assert_eq!(body_system_for_organ("prostate"), Some("reproductive"));
        assert_eq!(body_system_for_organ("uterus"), Some("reproductive"));
        // The organs the first real full-body run surfaced (multi-provider prefixes + new tokens).
        assert_eq!(
            body_system_for_organ("3d-allen-m-brain.glb"),
            Some("nervous")
        );
        assert_eq!(
            body_system_for_organ("3d-nih-m-lymph-node.glb"),
            Some("immune_lymphatic")
        );
        assert_eq!(
            body_system_for_organ("3d-vh-m-main-bronchus.glb"),
            Some("respiratory")
        );
        assert_eq!(
            body_system_for_organ("3d-vh-m-mouth.glb"),
            Some("digestive")
        );
        assert_eq!(
            body_system_for_organ("3d-vh-m-pelvis.glb"),
            Some("skeletal")
        );
        assert_eq!(
            body_system_for_organ("3d-vh-m-urinary-bladder.glb"),
            Some("urinary")
        );
        // Unknown → reported as None, never guessed.
        assert_eq!(body_system_for_organ("3d-vh-m-flux-capacitor.glb"), None);
    }

    #[test]
    fn organs_participate_in_multiple_systems() {
        use crate::anatomy::systems::body_system;
        // The pancreas is a building block of THREE systems — primary digestive, plus endocrine + exocrine.
        let m = system_memberships_for_organ("3d-vh-m-pancreas.glb");
        assert_eq!(m[0], ("digestive", true), "primary comes first");
        let ids: Vec<&str> = m.iter().map(|(s, _)| *s).collect();
        assert!(
            ids.contains(&"endocrine") && ids.contains(&"exocrine"),
            "pancreas multi-system: {ids:?}"
        );
        // The primary agrees with body_system_for_organ.
        assert_eq!(body_system_for_organ("pancreas"), Some("digestive"));
        // Skin: a barrier (primary integumentary) that also senses and secretes.
        let skin: Vec<&str> = system_memberships_for_organ("skin")
            .iter()
            .map(|(s, _)| *s)
            .collect();
        assert!(
            skin.contains(&"integumentary")
                && skin.contains(&"sensory")
                && skin.contains(&"exocrine"),
            "skin memberships: {skin:?}"
        );
        // A single-system organ has exactly one membership (the primary).
        assert_eq!(
            system_memberships_for_organ("heart"),
            vec![("circulatory", true)]
        );
        // The diaphragm is now a muscle (primary) that is also respiratory.
        let dia: Vec<&str> = system_memberships_for_organ("diaphragm")
            .iter()
            .map(|(s, _)| *s)
            .collect();
        assert_eq!(dia, vec!["muscular", "respiratory"]);
        // Every secondary membership names a real body system (no typo silently colouring nothing).
        for (organ, sys) in ORGAN_SECONDARY_SYSTEMS {
            assert!(
                body_system(sys).is_some(),
                "organ {organ} secondary → unknown system {sys}"
            );
        }
        // An unknown organ has no memberships (reported, not guessed).
        assert!(system_memberships_for_organ("flux-capacitor").is_empty());
    }

    #[test]
    fn every_mapped_system_id_is_a_real_body_system() {
        // Guards against a typo'd system id silently colouring nothing.
        for (organ, sys) in ORGAN_SYSTEMS {
            assert!(
                body_system(sys).is_some(),
                "organ {organ} maps to unknown system id {sys}"
            );
        }
    }

    #[test]
    fn every_one_of_the_17_systems_is_accounted_for() {
        // The honest completeness guarantee: no system is silently unsupported. Each of the 17 is
        // either a discrete-organ system with at least one organ in the paint map, or an explicitly
        // distributed-overlay network (ECS / ENS / glymphatic) with deliberately no standalone organ.
        use crate::anatomy::systems::BODY_SYSTEMS;
        assert_eq!(BODY_SYSTEMS.len(), 17);
        for s in BODY_SYSTEMS {
            let has_organ = ORGAN_SYSTEMS.iter().any(|(_, sys)| *sys == s.id);
            match system_representation(s.id) {
                SystemRepresentation::DiscreteOrgans => assert!(
                    has_organ,
                    "discrete-organ system {} has no organ in the paint map",
                    s.id
                ),
                SystemRepresentation::DistributedOverlay => assert!(
                    !has_organ,
                    "distributed system {} should not carry a standalone organ",
                    s.id
                ),
            }
        }
    }

    #[test]
    fn only_ecs_ens_glymphatic_are_distributed_overlays() {
        for &distributed in &["ecs", "ens", "glymphatic"] {
            assert_eq!(
                system_representation(distributed),
                SystemRepresentation::DistributedOverlay
            );
        }
        // The newly-added discrete systems now paint real organs.
        assert_eq!(body_system_for_organ("inner-ear"), Some("vestibular"));
        assert_eq!(body_system_for_organ("salivary-gland"), Some("exocrine"));
        for &discrete in &["circulatory", "vestibular", "exocrine", "reproductive"] {
            assert_eq!(
                system_representation(discrete),
                SystemRepresentation::DiscreteOrgans
            );
        }
    }

    #[test]
    fn distributed_overlays_carry_anatomical_host_hints() {
        // ENS lines the gut, glymphatic clears the brain, ECS is whole-body (empty = no local host).
        assert_eq!(overlay_host_systems("ens"), &["digestive"]);
        assert_eq!(overlay_host_systems("glymphatic"), &["nervous"]);
        assert_eq!(overlay_host_systems("ecs"), &[] as &[&str]);
        // Every host hint names a real, discrete-organ system (so the highlight has a mesh to land on).
        for &distributed in &["ens", "glymphatic", "ecs"] {
            for host in overlay_host_systems(distributed) {
                assert!(
                    body_system(host).is_some(),
                    "{distributed} host {host} unknown"
                );
                assert_eq!(
                    system_representation(host),
                    SystemRepresentation::DiscreteOrgans
                );
            }
        }
    }
}
