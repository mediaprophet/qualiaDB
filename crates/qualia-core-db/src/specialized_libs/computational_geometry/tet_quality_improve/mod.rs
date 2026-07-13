//! P13.7 - Tetrahedral quality improvement and sliver handling.
//!
//! Improves the quality distribution of an existing tetrahedral mesh through
//! four classic passes, each of which **preserves the domain boundary and the
//! positive orientation of every tet**, and each of which accepts a local
//! operation only when it **monotonically improves** the selected quality
//! objective over the affected cells. The passes are iterated to a fixpoint
//! (a full sweep applies no operation) or to the `max_passes` cap.
//!
//! ## Passes
//!
//! * **Flip** ([`flip_pass`]): 2-3 and 3-2 bistellar flips on interior faces /
//!   edges. A 2-3 flip replaces two tets sharing a face with three tets around
//!   the new edge joining their apices; a 3-2 flip is the reverse. Accepted
//!   only when the worst score of the new configuration exceeds the worst
//!   score of the old configuration.
//! * **Smooth** ([`smooth_pass`]): optimisation-based vertex smoothing. For
//!   each interior vertex a deterministic set of candidate positions
//!   (Laplacian centroid, volume-weighted tet centroid, and a fixed
//!   direction-probe set around the current position) is evaluated; the
//!   candidate maximising the minimum score over the incident tets is
//!   accepted, but only if it beats the current minimum. Boundary and
//!   caller-fixed vertices are pinned.
//! * **Insert** ([`insert_pass`]): Delaunay-style cavity refinement. The
//!   worst tet is located, its circumcenter is computed, the Delaunay cavity
//!   around that point is flood-filled (tets whose circumsphere contains the
//!   point), the cavity is checked to be star-shaped w.r.t. the new point,
//!   and the cavity tets are replaced by a star of new tets joining the
//!   boundary triangles to the new point. Accepted only when the new tets'
//!   minimum score beats the removed tets'. Steiner count is capped.
//! * **Exude** ([`exude_pass`]): sliver exudation by local perturbation. For
//!   each sliver (min dihedral below the threshold) each of its interior
//!   vertices is perturbed over a fixed deterministic direction/magnitude
//!   probe set; the first perturbation that removes the sliver without
//!   inverting any incident tet or creating a new sliver in the one-ring is
//!   accepted. This is the practical local-perturbation form of sliver
//!   exudation (Cheng-Dey-Edelsbrunner style), not a global weighting
//!   perturbation.
//!
//! ## Invariants (acceptance gate)
//!
//! * **Domain preservation.** Boundary faces (faces incident to exactly one
//!   tet) are never flipped and never removed by a cavity. Boundary vertices
//!   (vertices incident to a boundary face) and caller-supplied fixed
//!   vertices are never moved, smoothed, or perturbed. The boundary of the
//!   mesh is therefore preserved exactly.
//! * **Orientation preservation.** Every accepted operation validates that
//!   all affected tets have strictly positive signed volume
//!   (`det(v1-v0, v2-v0, v3-v0) > 0`); any candidate that would invert a tet
//!   is rejected.
//! * **Monotonic improvement.** Every accepted flip/smooth/insert/exude
//!   operation must strictly increase the local worst-case score (the minimum
//!   score over the affected cells). The global worst-case score is therefore
//!   non-decreasing across the whole run; the reported `stats_before` /
//!   `stats_after` pair makes the improvement measurable.
//!
//! ## Determinism
//!
//! Cells and vertices are processed in a deterministic order: worst-score
//! first with canonical tie-break (lowest index wins on equal score) for the
//! flip/insert/exude passes, and ascending vertex index for the smooth pass.
//! Perturbation directions and magnitudes are drawn from fixed arrays scaled
//! to the local edge length - no RNG. Identical input -> bit-identical output.
//!
//! Tier-2 cold construction: bounded `Vec`/`BTreeMap` scratch during the
//! build; the public output is returned as grown `Vec`s.

use super::mesh_quality::{tet_mesh_quality_slice, tet_quality_points, TetMeshQualityStats};
use super::mesh_quality::TetQuality;
use super::primitives::Point3;

mod adjacency;
mod driver;
mod error;
mod exude;
mod flip;
mod geom;
mod insert;
mod objective;
mod smooth;
mod types;
mod validate;
#[cfg(test)]
mod tests;

// -- Public surface (external paths resolve exactly as before) --------------
pub use error::TetImproveError;
pub use objective::TetImproveObjective;
pub use types::{TetImproveOptions, TetImproveResult};
pub use driver::improve_tet_mesh;
pub use validate::verify_improvement;

// -- Internal helpers re-imported into the module root so sibling submodules
//    (each opening with `use super::*;`) resolve them exactly as they did when
//    everything lived in one file. These are `pub(super)` in their submodules,
//    so nothing new is exposed outside `tet_quality_improve`. --------------
use adjacency::*;
use exude::*;
use flip::*;
use geom::*;
use insert::*;
use objective::{score, score_corners};
use smooth::*;
use validate::validate_input;
