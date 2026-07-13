//! Kernel-class taxonomy — the small, fixed set of compute *shapes* the engine
//! benchmarks and routes per class (HARDWARE_BACKEND_AUTOSELECT_PLAN.md §3).
//!
//! The fastest backend for a dense GEMV is **not** the fastest for an FFT or a
//! branch-heavy Monte-Carlo step — kernel *shape* (memory-bound vs compute-bound
//! vs divergent) changes the answer. So acceleration is decided **per class**, and
//! every hot STEM function classifies itself into exactly one of these. The set is
//! deliberately tiny: each class is one representative microkernel benched on each
//! available circuit×backend, and most STEM kernels map to ≤ 3 classes.
//!
//! This type is `Copy`/`Eq`/`Hash` and carries no allocation, so it is a free key
//! in the per-class capability matrix and in the O(1) `ComputePolicy::select`.

use serde::{Deserialize, Serialize};

/// The compute-shape class of a hot function. See the plan's §3 table for the
/// STEM functions that map to each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KernelClass {
    /// GEMV / GEMM — dense linear algebra, ML dense layers, FEM dense blocks.
    DenseLinear,
    /// Fused `a·x + b` over a large vector — activations, unit conversions, field updates.
    ElementwiseMap,
    /// sum / max / dot over a large vector — statistics aggregates, norms, energy sums.
    Reduction,
    /// 3-point / 7-point stencil pass — PDE/diffusion grids (physics, engineering).
    Stencil,
    /// N-body / pairwise distance — gravity/MD, chemistry non-bonded, medical similarity.
    AllPairs,
    /// 1-D transform — physics spectral, chemistry, signal/medical.
    Fft,
    /// Prefix sum — statistics CDFs, cumulative integrals.
    Scan,
    /// Branch-heavy Monte-Carlo step — finance pricing, statistical sampling, medical risk.
    Divergent,
}

impl KernelClass {
    /// Every class, in a stable order (panel/iteration order).
    pub const ALL: [KernelClass; 8] = [
        KernelClass::DenseLinear,
        KernelClass::ElementwiseMap,
        KernelClass::Reduction,
        KernelClass::Stencil,
        KernelClass::AllPairs,
        KernelClass::Fft,
        KernelClass::Scan,
        KernelClass::Divergent,
    ];

    /// Stable short label (also the per-class passport key).
    pub fn label(self) -> &'static str {
        match self {
            KernelClass::DenseLinear => "DenseLinear",
            KernelClass::ElementwiseMap => "ElementwiseMap",
            KernelClass::Reduction => "Reduction",
            KernelClass::Stencil => "Stencil",
            KernelClass::AllPairs => "AllPairs",
            KernelClass::Fft => "Fft",
            KernelClass::Scan => "Scan",
            KernelClass::Divergent => "Divergent",
        }
    }

    /// Whether the class is, in general, GPU-amenable. Branch-divergent Monte-Carlo
    /// (`Divergent`) frequently measures *slower* on a weak iGPU than on a many-core
    /// CPU — this is a hint, never the decision (the measured panel decides). A
    /// `false` here just biases the tie-break toward CPU when measurements are absent
    /// or within noise; it never overrides a real measured GPU win.
    pub fn is_typically_gpu_amenable(self) -> bool {
        !matches!(self, KernelClass::Divergent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_is_complete_and_unique() {
        assert_eq!(KernelClass::ALL.len(), 8);
        for (i, a) in KernelClass::ALL.iter().enumerate() {
            for b in &KernelClass::ALL[i + 1..] {
                assert_ne!(a, b, "duplicate class in ALL");
            }
        }
    }

    #[test]
    fn labels_are_distinct() {
        let mut seen = std::collections::HashSet::new();
        for c in KernelClass::ALL {
            assert!(seen.insert(c.label()), "duplicate label {}", c.label());
        }
    }

    #[test]
    fn divergent_is_flagged_cpu_biased() {
        assert!(!KernelClass::Divergent.is_typically_gpu_amenable());
        assert!(KernelClass::DenseLinear.is_typically_gpu_amenable());
    }
}
