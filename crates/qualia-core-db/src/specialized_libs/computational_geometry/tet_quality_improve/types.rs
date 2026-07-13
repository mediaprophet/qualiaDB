//! Public option and result types for an improvement run.

use super::*;

// ---------------------------------------------------------------------------
//  Options + result
// ---------------------------------------------------------------------------

/// Options controlling the improvement run.
#[derive(Debug, Clone, Copy)]
pub struct TetImproveOptions {
    /// Quality objective to maximise (worst-case).
    pub objective: TetImproveObjective,
    /// Hard cap on the number of outer pass sweeps.
    pub max_passes: u32,
    /// Enable / disable individual passes.
    pub flip_enabled: bool,
    pub smooth_enabled: bool,
    pub insert_enabled: bool,
    pub exude_enabled: bool,
    /// Hard cap on the number of Steiner points the insert pass may add.
    pub max_steiner: u32,
    /// Sliver threshold for the exude pass: a tet with `min_dihedral <
    /// sliver_min_dihedral_deg` (in degrees) is a sliver to be exuded.
    pub sliver_min_dihedral_deg: f64,
    /// Number of deterministic probe directions used by smooth and exude.
    /// The probe set is the 6 axis directions, 12 face-diagonals, 8
    /// body-diagonals = 26 "unit cube" neighbour directions, taken in fixed
    /// order. This field caps how many of them are tried (lower = faster,
    /// higher = more thorough).
    pub probe_count: u8,
    /// Magnitudes of perturbation for smooth/exude, as fractions of the local
    /// average incident edge length. Tried in order.
    pub perturb_fractions: [f64; 4],
}

impl Default for TetImproveOptions {
    fn default() -> Self {
        Self {
            objective: TetImproveObjective::MinDihedral,
            max_passes: 20,
            flip_enabled: true,
            smooth_enabled: true,
            insert_enabled: true,
            exude_enabled: true,
            max_steiner: 1_000,
            sliver_min_dihedral_deg: 15.0,
            probe_count: 26,
            perturb_fractions: [0.20, 0.10, 0.05, 0.40],
        }
    }
}

/// Result of an improvement run.
#[derive(Debug, Clone)]
pub struct TetImproveResult {
    /// Improved vertex positions (Steiner points appended at the end).
    pub vertices: Vec<Point3>,
    /// Improved tet list (positively oriented).
    pub tets: Vec<[u32; 4]>,
    /// Aggregate quality before the run.
    pub stats_before: TetMeshQualityStats,
    /// Aggregate quality after the run.
    pub stats_after: TetMeshQualityStats,
    /// Number of 2-3 / 3-2 flips applied.
    pub flips_applied: u32,
    /// Number of vertex smooths applied.
    pub smooths_applied: u32,
    /// Number of Steiner points inserted.
    pub inserts_applied: u32,
    /// Number of sliver exude perturbations applied.
    pub exudes_applied: u32,
    /// Number of outer pass sweeps executed.
    pub passes_run: u32,
}
