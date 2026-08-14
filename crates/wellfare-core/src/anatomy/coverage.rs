//! Seeded **per-system coverage** — how complete the *Qualia* organ map is today.
//!
//! This is the authority for “which of the 17 systems have parts.” It is not a
//! Python report. Overlay systems (ECS, ENS, glymphatic) are complete *as
//! overlays* when they have hosts, not when they have an organ file.

use super::model::{
    overlay_host_systems, primary_organ_system_pairs, secondary_organ_system_pairs,
    system_representation, SystemRepresentation,
};
use super::registry::{seed_tier, SystemTier};
use super::systems::{BODY_SYSTEMS, BodySystem};

/// One row of the seed coverage matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemCoverage {
    pub system: &'static BodySystem,
    pub representation: SystemRepresentation,
    pub tier: SystemTier,
    /// Organs whose *primary* home is this system.
    pub primary_parts: usize,
    /// Organs that list this system as a secondary membership.
    pub secondary_parts: usize,
    pub overlay_host_count: usize,
}

impl SystemCoverage {
    /// Discrete systems need at least one primary part to paint. Overlays need hosts
    /// (empty hosts = whole-body cue, still valid for ECS).
    pub fn seed_paint_ok(self) -> bool {
        match self.representation {
            SystemRepresentation::DiscreteOrgans => self.primary_parts > 0,
            SystemRepresentation::DistributedOverlay => true,
        }
    }
}

/// Coverage of the 17 seeded systems against the curated organ map (not a pack on disk).
pub fn seed_system_coverage() -> [SystemCoverage; 17] {
    assert_eq!(BODY_SYSTEMS.len(), 17);
    let mut out = [SystemCoverage {
        system: &BODY_SYSTEMS[0],
        representation: SystemRepresentation::DiscreteOrgans,
        tier: SystemTier::CanonicalMajor,
        primary_parts: 0,
        secondary_parts: 0,
        overlay_host_count: 0,
    }; 17];
    for (i, sys) in BODY_SYSTEMS.iter().enumerate() {
        let primary_parts = primary_organ_system_pairs()
            .iter()
            .filter(|(_, s)| *s == sys.id)
            .count();
        let secondary_parts = secondary_organ_system_pairs()
            .iter()
            .filter(|(_, s)| *s == sys.id)
            .count();
        out[i] = SystemCoverage {
            system: sys,
            representation: system_representation(sys.id),
            tier: seed_tier(sys.id),
            primary_parts,
            secondary_parts,
            overlay_host_count: overlay_host_systems(sys.id).len(),
        };
    }
    out
}

/// Markdown table for manuals / CLI (cold path).
pub fn seed_system_coverage_markdown() -> String {
    let mut s = String::from(
        "| System | Paint | Seed parts (primary+secondary) | Improvement |\n\
         |---|---|---|---|\n",
    );
    for row in seed_system_coverage() {
        let paint = match row.representation {
            SystemRepresentation::DiscreteOrgans => "organs",
            SystemRepresentation::DistributedOverlay => {
                if row.overlay_host_count == 0 {
                    "overlay (whole body)"
                } else {
                    "overlay (hosts)"
                }
            }
        };
        let parts = format!("{}+{}", row.primary_parts, row.secondary_parts);
        let note = if row.representation == SystemRepresentation::DistributedOverlay {
            "highlight hosts — do not add a fake organ"
        } else if row.primary_parts <= 2 {
            "thin — needs SA/HRA parts"
        } else {
            "grow named parts + graph join"
        };
        s.push_str(&format!(
            "| {} (`{}`) | {paint} | {parts} | {note} |\n",
            row.system.label, row.system.id
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seventeen_systems_and_overlays_do_not_need_organs() {
        let rows = seed_system_coverage();
        assert_eq!(rows.len(), 17);
        for row in rows {
            assert!(row.seed_paint_ok(), "{}", row.system.id);
            if row.system.id == "ecs" || row.system.id == "ens" || row.system.id == "glymphatic" {
                assert_eq!(row.representation, SystemRepresentation::DistributedOverlay);
                assert_eq!(row.primary_parts, 0);
            }
        }
    }

    #[test]
    fn listed_majors_have_at_least_one_primary_part() {
        for id in [
            "circulatory",
            "respiratory",
            "digestive",
            "nervous",
            "skeletal",
            "muscular",
            "endocrine",
            "immune_lymphatic",
            "integumentary",
            "urinary",
            "reproductive",
            "sensory",
            "vestibular",
            "exocrine",
        ] {
            let row = seed_system_coverage()
                .into_iter()
                .find(|r| r.system.id == id)
                .unwrap();
            assert!(row.primary_parts > 0, "{id} has no primary part");
        }
    }
}
