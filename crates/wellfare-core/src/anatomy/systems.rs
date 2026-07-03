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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seventeen_systems_and_lookup() {
        assert_eq!(BODY_SYSTEMS.len(), 17);
        assert_eq!(body_system("digestive").unwrap().label, "Digestive System");
        assert!(body_system("nope").is_none());
    }
}
