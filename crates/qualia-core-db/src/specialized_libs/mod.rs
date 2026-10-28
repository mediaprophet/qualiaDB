//! Specialized Libraries Module
//!
//! This module contains high-performance specialized mathematical and scientific libraries
//! that leverage Phase 2 architectural enhancements for unprecedented performance and capabilities.
//!
//! # Cold Domain Decoupling and Module Caller Audit
//!
//! Cold scientific and hardware domains are staged for opt-in feature gating:
//! - `chemistry_modeling`: Quantum chemistry, DFT, and Roothaan-Hall SCF. Referenced by
//!   `mcp::mcp_tool_impls::chemistry` and `poet_host::invoke::science::chem`.
//! - `financial_modeling`: Option pricing, Black-Scholes, and portfolio risk. Referenced
//!   by `poet_host::invoke::econ::{ml_finance, portfolio}`.
//! - `qpu_bridge`: Quantum processing unit job scheduler and driver interfaces. Referenced
//!   by `mcp::mcp_stub_impls`.
//! - `quantum_biology`: Biological quantum coherence and state synchronization. A true
//!   leaf module with zero external callers across `qualia-core-db`. Gated cleanly under
//!   `all(not(target_arch = "wasm32"), not(feature = "minimal-fast-build"))`.
//!
//! All public module re-exports are 100% preserved under default/monolith builds.
//! Use [`active_specialized_lib_mask`] to query active modules with zero heap allocations.

#[cfg(not(target_arch = "wasm32"))]
pub mod category_theory;
#[cfg(not(target_arch = "wasm32"))]
pub mod chemistry_modeling;
/// Computational economics and finance coordination layer: capability matrices,
/// shared categorical transforms, and native economics kernels. Kept available
/// to WASM because the first layer is metadata + zero-dependency utilities.
pub mod computational_economics;
/// Native computational geometry: robust predicates, topology/graph structures,
/// Q42/10D adapters, and the computational-geometry algorithm families. Unlike the older
/// specialized libraries this module is available to browser/WASM builds.
pub mod computational_geometry;
pub mod computer_vision;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub mod constructibility;
#[cfg(not(target_arch = "wasm32"))]
pub mod cryptographic_library;
#[cfg(not(target_arch = "wasm32"))]
pub mod engineering_analysis;
#[cfg(not(target_arch = "wasm32"))]
pub mod financial_modeling;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "portal",
    feature = "wasm-webcivics",
    feature = "wasm-logic",
    feature = "wasm-scientific"
))]
pub mod linear_algebra;
#[cfg(not(target_arch = "wasm32"))]
pub mod machine_learning;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "portal",
    feature = "wasm-logic",
    feature = "wasm-scientific"
))]
pub mod medical_computing;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub mod multivar_calculus;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub mod physics_simulation;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub mod polynomial_algebra;
#[cfg(not(target_arch = "wasm32"))]
pub mod qpu_bridge;
#[cfg(all(not(target_arch = "wasm32"), not(feature = "minimal-fast-build")))]
pub mod quantum_biology;
#[cfg(not(target_arch = "wasm32"))]
pub mod statistical_computing;
pub mod symbolic_algebra;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub mod symbolic_assumptions;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub mod symbolic_integration;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub mod symbolic_limits;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub mod symbolic_ode;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub mod symbolic_series;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub mod symbolic_solve;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub mod symbolic_trig;

// Shared zero-heap utilities
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "portal",
    feature = "wasm-logic",
    feature = "wasm-scientific"
))]
pub mod shared;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "portal",
    feature = "wasm-logic",
    feature = "wasm-scientific"
))]
pub use shared::{FixedArray, FixedQueue, FixedStack, RingBuffer};

// ── Library Bitmask Constants (1 << N) ───────────────────────────────────────

pub const LIB_CATEGORY_THEORY: u32 = 1 << 0;
pub const LIB_CHEMISTRY_MODELING: u32 = 1 << 1;
pub const LIB_COMPUTATIONAL_ECONOMICS: u32 = 1 << 2;
pub const LIB_COMPUTATIONAL_GEOMETRY: u32 = 1 << 3;
pub const LIB_COMPUTER_VISION: u32 = 1 << 4;
pub const LIB_CONSTRUCTIBILITY: u32 = 1 << 5;
pub const LIB_CRYPTOGRAPHIC_LIBRARY: u32 = 1 << 6;
pub const LIB_ENGINEERING_ANALYSIS: u32 = 1 << 7;
pub const LIB_FINANCIAL_MODELING: u32 = 1 << 8;
pub const LIB_LINEAR_ALGEBRA: u32 = 1 << 9;
pub const LIB_MACHINE_LEARNING: u32 = 1 << 10;
pub const LIB_MEDICAL_COMPUTING: u32 = 1 << 11;
pub const LIB_MULTIVAR_CALCULUS: u32 = 1 << 12;
pub const LIB_PHYSICS_SIMULATION: u32 = 1 << 13;
pub const LIB_POLYNOMIAL_ALGEBRA: u32 = 1 << 14;
pub const LIB_QPU_BRIDGE: u32 = 1 << 15;
pub const LIB_QUANTUM_BIOLOGY: u32 = 1 << 16;
pub const LIB_STATISTICAL_COMPUTING: u32 = 1 << 17;
pub const LIB_SYMBOLIC_ALGEBRA: u32 = 1 << 18;
pub const LIB_SYMBOLIC_ASSUMPTIONS: u32 = 1 << 19;
pub const LIB_SYMBOLIC_INTEGRATION: u32 = 1 << 20;
pub const LIB_SYMBOLIC_LIMITS: u32 = 1 << 21;
pub const LIB_SYMBOLIC_ODE: u32 = 1 << 22;
pub const LIB_SYMBOLIC_SERIES: u32 = 1 << 23;
pub const LIB_SYMBOLIC_SOLVE: u32 = 1 << 24;
pub const LIB_SYMBOLIC_TRIG: u32 = 1 << 25;
pub const LIB_SHARED: u32 = 1 << 26;

// ── Zero-Heap Capability Query Functions ─────────────────────────────────────

/// Returns the 32-bit bitmask of active specialized libraries compiled into this binary.
///
/// Pure `const fn` executing with zero heap allocations (Tier-1 contract).
#[inline(always)]
pub const fn active_specialized_lib_mask() -> u32 {
    let mut mask = 0u32;

    #[cfg(not(target_arch = "wasm32"))]
    {
        mask |= LIB_CATEGORY_THEORY;
        mask |= LIB_CHEMISTRY_MODELING;
        mask |= LIB_CRYPTOGRAPHIC_LIBRARY;
        mask |= LIB_ENGINEERING_ANALYSIS;
        mask |= LIB_FINANCIAL_MODELING;
        mask |= LIB_MACHINE_LEARNING;
        mask |= LIB_QPU_BRIDGE;
        mask |= LIB_STATISTICAL_COMPUTING;
    }

    #[cfg(all(not(target_arch = "wasm32"), not(feature = "minimal-fast-build")))]
    {
        mask |= LIB_QUANTUM_BIOLOGY;
    }

    // Always available (native and WASM)
    mask |= LIB_COMPUTATIONAL_ECONOMICS;
    mask |= LIB_COMPUTATIONAL_GEOMETRY;
    mask |= LIB_COMPUTER_VISION;
    mask |= LIB_SYMBOLIC_ALGEBRA;

    #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
    {
        mask |= LIB_CONSTRUCTIBILITY;
        mask |= LIB_MULTIVAR_CALCULUS;
        mask |= LIB_PHYSICS_SIMULATION;
        mask |= LIB_POLYNOMIAL_ALGEBRA;
        mask |= LIB_SYMBOLIC_ASSUMPTIONS;
        mask |= LIB_SYMBOLIC_INTEGRATION;
        mask |= LIB_SYMBOLIC_LIMITS;
        mask |= LIB_SYMBOLIC_ODE;
        mask |= LIB_SYMBOLIC_SERIES;
        mask |= LIB_SYMBOLIC_SOLVE;
        mask |= LIB_SYMBOLIC_TRIG;
    }

    #[cfg(any(
        not(target_arch = "wasm32"),
        feature = "portal",
        feature = "wasm-webcivics",
        feature = "wasm-logic",
        feature = "wasm-scientific"
    ))]
    {
        mask |= LIB_LINEAR_ALGEBRA;
    }

    #[cfg(any(
        not(target_arch = "wasm32"),
        feature = "portal",
        feature = "wasm-logic",
        feature = "wasm-scientific"
    ))]
    {
        mask |= LIB_MEDICAL_COMPUTING;
        mask |= LIB_SHARED;
    }

    mask
}

/// Checks whether a specific specialized library capability bit is active in the current build.
#[inline(always)]
pub const fn is_specialized_lib_active(lib_bit: u32) -> bool {
    (active_specialized_lib_mask() & lib_bit) != 0
}

/// Returns the canonical snake_case name of a specialized library capability bit.
pub const fn specialized_lib_name(lib_bit: u32) -> &'static str {
    match lib_bit {
        LIB_CATEGORY_THEORY => "category_theory",
        LIB_CHEMISTRY_MODELING => "chemistry_modeling",
        LIB_COMPUTATIONAL_ECONOMICS => "computational_economics",
        LIB_COMPUTATIONAL_GEOMETRY => "computational_geometry",
        LIB_COMPUTER_VISION => "computer_vision",
        LIB_CONSTRUCTIBILITY => "constructibility",
        LIB_CRYPTOGRAPHIC_LIBRARY => "cryptographic_library",
        LIB_ENGINEERING_ANALYSIS => "engineering_analysis",
        LIB_FINANCIAL_MODELING => "financial_modeling",
        LIB_LINEAR_ALGEBRA => "linear_algebra",
        LIB_MACHINE_LEARNING => "machine_learning",
        LIB_MEDICAL_COMPUTING => "medical_computing",
        LIB_MULTIVAR_CALCULUS => "multivar_calculus",
        LIB_PHYSICS_SIMULATION => "physics_simulation",
        LIB_POLYNOMIAL_ALGEBRA => "polynomial_algebra",
        LIB_QPU_BRIDGE => "qpu_bridge",
        LIB_QUANTUM_BIOLOGY => "quantum_biology",
        LIB_STATISTICAL_COMPUTING => "statistical_computing",
        LIB_SYMBOLIC_ALGEBRA => "symbolic_algebra",
        LIB_SYMBOLIC_ASSUMPTIONS => "symbolic_assumptions",
        LIB_SYMBOLIC_INTEGRATION => "symbolic_integration",
        LIB_SYMBOLIC_LIMITS => "symbolic_limits",
        LIB_SYMBOLIC_ODE => "symbolic_ode",
        LIB_SYMBOLIC_SERIES => "symbolic_series",
        LIB_SYMBOLIC_SOLVE => "symbolic_solve",
        LIB_SYMBOLIC_TRIG => "symbolic_trig",
        LIB_SHARED => "shared",
        _ => "unknown",
    }
}

// ── Antigravity Unit Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_specialized_lib_mask_native() {
        let mask = active_specialized_lib_mask();
        // In native builds without minimal-fast-build, all 27 flags must be active
        #[cfg(not(target_arch = "wasm32"))]
        {
            assert_eq!(
                mask.count_ones(),
                27,
                "Native builds must include all 27 specialized libraries"
            );
            assert!(mask & LIB_CHEMISTRY_MODELING != 0);
            assert!(mask & LIB_FINANCIAL_MODELING != 0);
            assert!(mask & LIB_QPU_BRIDGE != 0);
            assert!(mask & LIB_QUANTUM_BIOLOGY != 0);
        }
    }

    #[test]
    fn test_is_specialized_lib_active_positive() {
        assert!(is_specialized_lib_active(LIB_COMPUTATIONAL_GEOMETRY));
        assert!(is_specialized_lib_active(LIB_COMPUTATIONAL_ECONOMICS));
        assert!(is_specialized_lib_active(LIB_COMPUTER_VISION));
        assert!(is_specialized_lib_active(LIB_SYMBOLIC_ALGEBRA));
    }

    #[test]
    fn test_is_specialized_lib_active_negative_unused_bits() {
        // Bits 27 to 31 are unused
        assert!(!is_specialized_lib_active(1 << 27));
        assert!(!is_specialized_lib_active(1 << 28));
        assert!(!is_specialized_lib_active(1 << 31));
        assert!(!is_specialized_lib_active(0));
    }

    #[test]
    fn test_specialized_lib_name_mapping() {
        assert_eq!(specialized_lib_name(LIB_CATEGORY_THEORY), "category_theory");
        assert_eq!(specialized_lib_name(LIB_CHEMISTRY_MODELING), "chemistry_modeling");
        assert_eq!(specialized_lib_name(LIB_FINANCIAL_MODELING), "financial_modeling");
        assert_eq!(specialized_lib_name(LIB_QPU_BRIDGE), "qpu_bridge");
        assert_eq!(specialized_lib_name(LIB_QUANTUM_BIOLOGY), "quantum_biology");
        assert_eq!(specialized_lib_name(LIB_SHARED), "shared");
    }

    #[test]
    fn test_specialized_lib_name_unknown() {
        assert_eq!(specialized_lib_name(0), "unknown");
        assert_eq!(specialized_lib_name(1 << 28), "unknown");
        assert_eq!(
            specialized_lib_name(LIB_CATEGORY_THEORY | LIB_CHEMISTRY_MODELING),
            "unknown"
        );
    }

    #[test]
    fn test_specialized_lib_bits_disjoint_powers_of_two() {
        let all_bits = [
            LIB_CATEGORY_THEORY,
            LIB_CHEMISTRY_MODELING,
            LIB_COMPUTATIONAL_ECONOMICS,
            LIB_COMPUTATIONAL_GEOMETRY,
            LIB_COMPUTER_VISION,
            LIB_CONSTRUCTIBILITY,
            LIB_CRYPTOGRAPHIC_LIBRARY,
            LIB_ENGINEERING_ANALYSIS,
            LIB_FINANCIAL_MODELING,
            LIB_LINEAR_ALGEBRA,
            LIB_MACHINE_LEARNING,
            LIB_MEDICAL_COMPUTING,
            LIB_MULTIVAR_CALCULUS,
            LIB_PHYSICS_SIMULATION,
            LIB_POLYNOMIAL_ALGEBRA,
            LIB_QPU_BRIDGE,
            LIB_QUANTUM_BIOLOGY,
            LIB_STATISTICAL_COMPUTING,
            LIB_SYMBOLIC_ALGEBRA,
            LIB_SYMBOLIC_ASSUMPTIONS,
            LIB_SYMBOLIC_INTEGRATION,
            LIB_SYMBOLIC_LIMITS,
            LIB_SYMBOLIC_ODE,
            LIB_SYMBOLIC_SERIES,
            LIB_SYMBOLIC_SOLVE,
            LIB_SYMBOLIC_TRIG,
            LIB_SHARED,
        ];
        for (i, &b1) in all_bits.iter().enumerate() {
            assert!(b1.is_power_of_two(), "Lib bit {b1} must be a power of two");
            for (j, &b2) in all_bits.iter().enumerate() {
                if i != j {
                    assert_eq!(
                        b1 & b2,
                        0,
                        "Lib bits {b1} and {b2} must be strictly disjoint"
                    );
                }
            }
        }
    }

    #[test]
    fn test_quantum_biology_fast_build_leaf_isolation() {
        // Quantum biology is an isolated leaf with no external callers in the monolith.
        // Ensure its constant is properly bounded within the 32-bit word.
        assert_eq!(LIB_QUANTUM_BIOLOGY, 1 << 16);
        assert!(LIB_QUANTUM_BIOLOGY < (1 << 27));
    }
}
