//! The body-system taxonomy — mirrors `bundled/qapps/Anatomy/Knowledge/system-map.json` so the
//! native 3D view and the accumulation engine agree on system identity.

/// A body system (id/label mirror the Anatomy qapp's `system-map.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodySystem {
    pub id: &'static str,
    pub label: &'static str,
}

/// The 17 seeded body systems (extensible — jurisdiction/ontology packs can add more later).
pub static BODY_SYSTEMS: &[BodySystem] = &[
    BodySystem { id: "circulatory", label: "Circulatory (Cardiovascular) System" },
    BodySystem { id: "respiratory", label: "Respiratory System" },
    BodySystem { id: "digestive", label: "Digestive System" },
    BodySystem { id: "nervous", label: "Nervous System" },
    BodySystem { id: "muscular", label: "Muscular System" },
    BodySystem { id: "skeletal", label: "Skeletal System" },
    BodySystem { id: "endocrine", label: "Endocrine System" },
    BodySystem { id: "immune_lymphatic", label: "Immune / Lymphatic System" },
    BodySystem { id: "integumentary", label: "Integumentary System" },
    BodySystem { id: "urinary", label: "Urinary (Excretory) System" },
    BodySystem { id: "reproductive", label: "Reproductive System" },
    BodySystem { id: "sensory", label: "Sensory System" },
    BodySystem { id: "vestibular", label: "Vestibular System" },
    BodySystem { id: "exocrine", label: "Exocrine System" },
    BodySystem { id: "ecs", label: "Endocannabinoid System (ECS)" },
    BodySystem { id: "ens", label: "Enteric Nervous System (ENS)" },
    BodySystem { id: "glymphatic", label: "Glymphatic System" },
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
    BODY_SYSTEMS.iter().find(|s| s.label.to_ascii_lowercase() == want)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seventeen_systems_and_lookup() {
        assert_eq!(BODY_SYSTEMS.len(), 17);
        assert_eq!(body_system("digestive").unwrap().label, "Digestive System");
        assert!(body_system("nope").is_none());
    }

    #[test]
    fn label_lookup_maps_bundled_knowledge_labels_to_ids() {
        // Labels exactly as they appear in bundled condition-map.json.
        assert_eq!(body_system_by_label("Endocrine System").unwrap().id, "endocrine");
        assert_eq!(
            body_system_by_label("Circulatory (Cardiovascular) System").unwrap().id,
            "circulatory"
        );
        assert_eq!(body_system_by_label("  urinary (excretory) system  ").unwrap().id, "urinary");
        assert!(body_system_by_label("Not A System").is_none());
    }
}
