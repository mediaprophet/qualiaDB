//! The body-system taxonomy — mirrors `bundled/qapps/Anatomy/Knowledge/system-map.json` so the
//! native 3D view and the accumulation engine agree on system identity.

/// A body system. `id`/`label` mirror the Anatomy qapp's `system-map.json`; `plain_label` is an
/// accessibility-first, general-audience wording (plain, non-diagnostic — accessibility is core, not a
/// mode). The clinical `label` remains available behind progressive disclosure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodySystem {
    pub id: &'static str,
    pub label: &'static str,
    pub plain_label: &'static str,
}

/// The 17 seeded body systems (extensible — jurisdiction/ontology packs can add more later).
pub static BODY_SYSTEMS: &[BodySystem] = &[
    BodySystem {
        id: "circulatory",
        label: "Circulatory (Cardiovascular) System",
        plain_label: "heart and blood flow",
    },
    BodySystem {
        id: "respiratory",
        label: "Respiratory System",
        plain_label: "breathing",
    },
    BodySystem {
        id: "digestive",
        label: "Digestive System",
        plain_label: "digestion",
    },
    BodySystem {
        id: "nervous",
        label: "Nervous System",
        plain_label: "brain and nerves",
    },
    BodySystem {
        id: "muscular",
        label: "Muscular System",
        plain_label: "muscles",
    },
    BodySystem {
        id: "skeletal",
        label: "Skeletal System",
        plain_label: "bones and joints",
    },
    BodySystem {
        id: "endocrine",
        label: "Endocrine System",
        plain_label: "hormones",
    },
    BodySystem {
        id: "immune_lymphatic",
        label: "Immune / Lymphatic System",
        plain_label: "immune defences",
    },
    BodySystem {
        id: "integumentary",
        label: "Integumentary System",
        plain_label: "skin",
    },
    BodySystem {
        id: "urinary",
        label: "Urinary (Excretory) System",
        plain_label: "kidneys and fluid balance",
    },
    BodySystem {
        id: "reproductive",
        label: "Reproductive System",
        plain_label: "reproductive health",
    },
    BodySystem {
        id: "sensory",
        label: "Sensory System",
        plain_label: "senses",
    },
    BodySystem {
        id: "vestibular",
        label: "Vestibular System",
        plain_label: "balance",
    },
    BodySystem {
        id: "exocrine",
        label: "Exocrine System",
        plain_label: "glands",
    },
    BodySystem {
        id: "ecs",
        label: "Endocannabinoid System (ECS)",
        plain_label: "internal balance (ECS)",
    },
    BodySystem {
        id: "ens",
        label: "Enteric Nervous System (ENS)",
        plain_label: "gut nerves",
    },
    BodySystem {
        id: "glymphatic",
        label: "Glymphatic System",
        plain_label: "the brain's overnight cleaning",
    },
];

/// Look up a body system by id.
pub fn body_system(id: &str) -> Option<&'static BodySystem> {
    BODY_SYSTEMS.iter().find(|s| s.id == id)
}

/// Look up a body system by its human label (case-insensitive, trimmed). Lets us import the bundled
/// knowledge files (e.g. `condition-map.json`), which key systems by label ("Endocrine System"),
/// into this id-keyed model.
pub fn body_system_by_label(label: &str) -> Option<&'static BodySystem> {
    let want = label.trim().to_ascii_lowercase();
    BODY_SYSTEMS
        .iter()
        .find(|s| s.label.to_ascii_lowercase() == want)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seventeen_systems_and_lookup() {
        assert_eq!(BODY_SYSTEMS.len(), 17);
        assert_eq!(body_system("digestive").unwrap().label, "Digestive System");
        assert_eq!(body_system("digestive").unwrap().plain_label, "digestion");
        assert!(body_system("nope").is_none());
        // Every system carries a non-empty plain-language label (accessibility is core).
        assert!(BODY_SYSTEMS.iter().all(|s| !s.plain_label.is_empty()));
    }

    #[test]
    fn label_lookup_maps_bundled_knowledge_labels_to_ids() {
        // Labels exactly as they appear in bundled condition-map.json.
        assert_eq!(
            body_system_by_label("Endocrine System").unwrap().id,
            "endocrine"
        );
        assert_eq!(
            body_system_by_label("Circulatory (Cardiovascular) System")
                .unwrap()
                .id,
            "circulatory"
        );
        assert_eq!(
            body_system_by_label("  urinary (excretory) system  ")
                .unwrap()
                .id,
            "urinary"
        );
        assert!(body_system_by_label("Not A System").is_none());
    }
}
