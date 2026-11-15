//! Manifold bifurcation (w) for multi-head attention

use serde::{Deserialize, Serialize};

/// Manifold domain variants (§2.1)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManifoldDomain {
    /// Biological/Medical
    Medical = 0,
    /// Legal/Jurisdictional
    Legal = 1,
    /// Personal/Agency
    Personal = 2,
    /// Environmental/Sensor
    Environmental = 3,
    /// Socioeconomic/Wellbeing
    Socioeconomic = 4,
    /// Mechanical/Structural Engineering
    MechanicalStructural = 5,
    /// Electromagnetic/Optical Physics
    ElectromagneticOptical = 6,
    /// Acoustic/Wavefield Dynamics
    AcousticWavefield = 7,
    /// Chemical/Molecular Chemistry
    ChemicalMolecular = 8,
    /// Astrophysical/Cosmological Space
    AstrophysicalCosmological = 9,
    /// Quantum/Subatomic Scale
    QuantumSubatomic = 10,
    /// Botanical/Ecological Systems
    BotanicalEcological = 11,
    /// Zoological/Ethological Animal Behavior
    ZoologicalEthological = 12,
}

impl Default for ManifoldDomain {
    fn default() -> Self {
        ManifoldDomain::Medical
    }
}

impl ManifoldDomain {
    /// Builds `ManifoldDomain` from `index`.
    pub fn from_index(index: f32) -> Self {
        match index.round() as u32 {
            0 => ManifoldDomain::Medical,
            1 => ManifoldDomain::Legal,
            2 => ManifoldDomain::Personal,
            3 => ManifoldDomain::Environmental,
            4 => ManifoldDomain::Socioeconomic,
            5 => ManifoldDomain::MechanicalStructural,
            6 => ManifoldDomain::ElectromagneticOptical,
            7 => ManifoldDomain::AcousticWavefield,
            8 => ManifoldDomain::ChemicalMolecular,
            9 => ManifoldDomain::AstrophysicalCosmological,
            10 => ManifoldDomain::QuantumSubatomic,
            11 => ManifoldDomain::BotanicalEcological,
            12 => ManifoldDomain::ZoologicalEthological,
            _ => ManifoldDomain::Medical, // Default fallback
        }
    }

    /// Converts to `index`.
    pub fn to_index(&self) -> f32 {
        *self as u32 as f32
    }

    /// Semantic domain string label.
    pub fn domain_label(&self) -> &'static str {
        match self {
            ManifoldDomain::Medical => "Medical",
            ManifoldDomain::Legal => "Legal",
            ManifoldDomain::Personal => "Personal",
            ManifoldDomain::Environmental => "Environmental",
            ManifoldDomain::Socioeconomic => "Socioeconomic",
            ManifoldDomain::MechanicalStructural => "MechanicalStructural",
            ManifoldDomain::ElectromagneticOptical => "ElectromagneticOptical",
            ManifoldDomain::AcousticWavefield => "AcousticWavefield",
            ManifoldDomain::ChemicalMolecular => "ChemicalMolecular",
            ManifoldDomain::AstrophysicalCosmological => "AstrophysicalCosmological",
            ManifoldDomain::QuantumSubatomic => "QuantumSubatomic",
            ManifoldDomain::BotanicalEcological => "BotanicalEcological",
            ManifoldDomain::ZoologicalEthological => "ZoologicalEthological",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_thirteen_domains_round_trip() {
        let expected_domains = [
            (0, ManifoldDomain::Medical, "Medical"),
            (1, ManifoldDomain::Legal, "Legal"),
            (2, ManifoldDomain::Personal, "Personal"),
            (3, ManifoldDomain::Environmental, "Environmental"),
            (4, ManifoldDomain::Socioeconomic, "Socioeconomic"),
            (5, ManifoldDomain::MechanicalStructural, "MechanicalStructural"),
            (6, ManifoldDomain::ElectromagneticOptical, "ElectromagneticOptical"),
            (7, ManifoldDomain::AcousticWavefield, "AcousticWavefield"),
            (8, ManifoldDomain::ChemicalMolecular, "ChemicalMolecular"),
            (9, ManifoldDomain::AstrophysicalCosmological, "AstrophysicalCosmological"),
            (10, ManifoldDomain::QuantumSubatomic, "QuantumSubatomic"),
            (11, ManifoldDomain::BotanicalEcological, "BotanicalEcological"),
            (12, ManifoldDomain::ZoologicalEthological, "ZoologicalEthological"),
        ];

        for (idx, domain, label) in expected_domains {
            assert_eq!(domain as u32, idx as u32);
            assert_eq!(domain.to_index(), idx as f32);
            assert_eq!(ManifoldDomain::from_index(idx as f32), domain);
            assert_eq!(domain.domain_label(), label);
        }

        // Test fallback
        assert_eq!(ManifoldDomain::from_index(99.0), ManifoldDomain::Medical);
    }
}
