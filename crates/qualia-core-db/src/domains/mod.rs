//! Domain-specific engines for QualiaDB
//!
//! This module organizes domain-specific computation engines by category,
//! providing specialized capabilities for biological, chemical, physical,
//! financial, mathematical, and geospatial domains.
//!
//! # Decoupling Architecture and Unconditional Callers
//!
//! Several core governance and platform paths in `qualia-core-db` interface
//! directly with domain engines:
//! - `biological`: Referenced by `governance::webizen::vm` (sequence alignment,
//!   k-mer frequency, FASTA parsing, Tanimoto fingerprint similarity).
//! - `chemical`: Referenced by `governance::webizen::vm` (SMILES/InChI validation,
//!   descriptors, Lipinski/Veber/Ghose/Egan rules, functional groups, pKa, chiral
//!   centers, circular fingerprints, thermodynamics, and kinetics).
//! - `financial`: Referenced by `governance::webizen::vm` (Monte Carlo VaR) and
//!   `governance::webizen::mod` (`TaxRuleSchema`).
//! - `geospatial`: Referenced by `governance::webizen::vm` (`CanvasRightsModel`) and
//!   `lib.rs::tests` (`SpatiotemporalQuadTree`, `embed_h3_context`).
//! - `mathematical`: Referenced by `lib.rs::tests` (`VectorSectorMap`,
//!   `extract_spatial_projection`) and `platform::npu_ffi` (`LorentzVector`).
//! - `physical`: Referenced by `governance::webizen::vm` (`ThermodynamicSampler`).
//!
//! Under default and monolithic configurations, 100% of these domain modules
//! remain unconditionally exported to ensure zero broken callers. The
//! zero-heap capability function [`active_domain_mask`] allows runtimes to inspect
//! active domain boundaries at compile-time and runtime.

pub mod biological;
pub mod biosphere;
pub mod chemical;
pub mod financial;
pub mod geospatial;
pub mod mathematical;
pub mod physical;

// ── Domain Bitmask Constants (1 << N) ────────────────────────────────────────

/// Biological domain engine (bioinformatics, k-mer analysis, sequence alignment).
pub const DOMAIN_BIOLOGICAL: u32 = 1 << 0;

/// Chemical domain engine (organic chemistry, cheminformatics, SMILES/InChI).
pub const DOMAIN_CHEMICAL: u32 = 1 << 1;

/// Financial domain engine (quantitative economics, tax schema, Monte Carlo VaR).
pub const DOMAIN_FINANCIAL: u32 = 1 << 2;

/// Geospatial domain engine (H3 indexing, quadtree, canvas rights, GIS adapters).
pub const DOMAIN_GEOSPATIAL: u32 = 1 << 3;

/// Mathematical domain engine (geometric algebra, Lorentz vectors, Voronoi cells).
pub const DOMAIN_MATHEMATICAL: u32 = 1 << 4;

/// Physical domain engine (thermodynamics, MCMC sampling, reaction kinetics).
pub const DOMAIN_PHYSICAL: u32 = 1 << 5;

/// Combined bitmask representing all domain capabilities.
pub const ALL_DOMAINS_MASK: u32 = DOMAIN_BIOLOGICAL
    | DOMAIN_CHEMICAL
    | DOMAIN_FINANCIAL
    | DOMAIN_GEOSPATIAL
    | DOMAIN_MATHEMATICAL
    | DOMAIN_PHYSICAL;

// ── Zero-Heap Capability Query Functions ─────────────────────────────────────

/// Returns the 32-bit bitmask of active domain engines compiled into this binary.
///
/// Pure `const fn` executing with zero heap allocations (Tier-1 contract).
#[inline(always)]
pub const fn active_domain_mask() -> u32 {
    let mut mask = 0u32;
    mask |= DOMAIN_BIOLOGICAL;
    mask |= DOMAIN_CHEMICAL;
    mask |= DOMAIN_FINANCIAL;
    mask |= DOMAIN_GEOSPATIAL;
    mask |= DOMAIN_MATHEMATICAL;
    mask |= DOMAIN_PHYSICAL;
    mask
}

/// Checks whether a specific domain capability bit is active in the current build.
///
/// Returns `true` if the corresponding domain flag is set, `false` otherwise.
#[inline(always)]
pub const fn is_domain_active(domain_bit: u32) -> bool {
    (active_domain_mask() & domain_bit) != 0
}

/// Returns the canonical kebab-case name of a domain capability bit.
///
/// Returns `"unknown"` if an unrecognized bit or multiple bits are passed.
pub const fn domain_name(domain_bit: u32) -> &'static str {
    match domain_bit {
        DOMAIN_BIOLOGICAL => "biological",
        DOMAIN_CHEMICAL => "chemical",
        DOMAIN_FINANCIAL => "financial",
        DOMAIN_GEOSPATIAL => "geospatial",
        DOMAIN_MATHEMATICAL => "mathematical",
        DOMAIN_PHYSICAL => "physical",
        _ => "unknown",
    }
}

// ── Antigravity Unit Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_domain_mask_all_active() {
        let mask = active_domain_mask();
        assert_eq!(
            mask, ALL_DOMAINS_MASK,
            "All six core domains must be active in default build"
        );
        assert_eq!(mask.count_ones(), 6, "Expected exactly 6 active domains");
    }

    #[test]
    fn test_is_domain_active_positive() {
        assert!(is_domain_active(DOMAIN_BIOLOGICAL));
        assert!(is_domain_active(DOMAIN_CHEMICAL));
        assert!(is_domain_active(DOMAIN_FINANCIAL));
        assert!(is_domain_active(DOMAIN_GEOSPATIAL));
        assert!(is_domain_active(DOMAIN_MATHEMATICAL));
        assert!(is_domain_active(DOMAIN_PHYSICAL));
    }

    #[test]
    fn test_is_domain_active_negative_invalid_bits() {
        // High bits not assigned to any domain must report inactive
        assert!(!is_domain_active(1 << 6));
        assert!(!is_domain_active(1 << 15));
        assert!(!is_domain_active(1 << 31));
        assert!(!is_domain_active(0));
    }

    #[test]
    fn test_domain_name_labels() {
        assert_eq!(domain_name(DOMAIN_BIOLOGICAL), "biological");
        assert_eq!(domain_name(DOMAIN_CHEMICAL), "chemical");
        assert_eq!(domain_name(DOMAIN_FINANCIAL), "financial");
        assert_eq!(domain_name(DOMAIN_GEOSPATIAL), "geospatial");
        assert_eq!(domain_name(DOMAIN_MATHEMATICAL), "mathematical");
        assert_eq!(domain_name(DOMAIN_PHYSICAL), "physical");
    }

    #[test]
    fn test_domain_name_unknown() {
        assert_eq!(domain_name(0), "unknown");
        assert_eq!(domain_name(1 << 6), "unknown");
        assert_eq!(domain_name(DOMAIN_BIOLOGICAL | DOMAIN_CHEMICAL), "unknown");
    }

    #[test]
    fn test_domain_bits_are_disjoint_powers_of_two() {
        let bits = [
            DOMAIN_BIOLOGICAL,
            DOMAIN_CHEMICAL,
            DOMAIN_FINANCIAL,
            DOMAIN_GEOSPATIAL,
            DOMAIN_MATHEMATICAL,
            DOMAIN_PHYSICAL,
        ];
        for (i, &b1) in bits.iter().enumerate() {
            assert!(b1.is_power_of_two(), "Domain bit {b1} must be a power of two");
            for (j, &b2) in bits.iter().enumerate() {
                if i != j {
                    assert_eq!(
                        b1 & b2,
                        0,
                        "Domain bits {b1} and {b2} must be strictly disjoint"
                    );
                }
            }
        }
    }
}
