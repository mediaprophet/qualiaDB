//! POET Geometry-Topology Complex (`Construct → Manifold → Geometry [harbouring M Graphs] → Container`).
//!
//! - [`header`]: 64-byte `PoetGeometryHeader`, 64-byte `HarbouredGraphHeader`, 16-byte `CellAnchor`,
//!   and morphological/invariant descriptors.
//! - [`csr_view`]: Zero-copy `HarbouredCsrView<'a>` over 64B-aligned CSR/CSC and `CellAnchor` tables.
//! - [`geometry_view`]: Zero-copy `PoetGeometryView<'a>` and $O(1)$ cross-graph functorial routing.
//! - [`delta_ring`]: 16-byte `TopologicalDeltaEntry`, fixed-capacity `TopologicalDeltaRing`, and `MergedGraphView`.
//! - [`invariants`]: Non-recursive zero-heap invariant verifiers (`ACYCLIC`, `TEMPORAL_MONOTONIC`,
//!   `DEONTIC_GUARDED`, `PARACONSISTENT_ISOLATED`, `SENSITIVITY_MONOTONIC`, Euler $\chi$, Betti $\beta_0, \beta_1$).
//! - [`pullback`]: 10D manifold frustum pullback traversal and T16 `Epistemic10D` projection.
//! - [`induce_intrinsic`]: Tier-2 induction of intrinsic graphs from `computational_geometry` complexes.
//! - [`compactor`]: Tier-2 `GTOP` segment builder and delta-ring sealing engine.

pub mod compactor;
pub mod csr_view;
pub mod delta_ring;
pub mod geometry_view;
pub mod header;
pub mod induce_intrinsic;
pub mod invariants;
pub mod pullback;
#[cfg(test)]
mod tests;

pub use compactor::*;
pub use csr_view::*;
pub use delta_ring::*;
pub use geometry_view::*;
pub use header::*;
pub use induce_intrinsic::*;
pub use invariants::*;
pub use pullback::*;
