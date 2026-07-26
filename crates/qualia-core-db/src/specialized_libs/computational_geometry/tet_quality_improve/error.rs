//! Error type for the tetrahedral-mesh improvement run.

// ---------------------------------------------------------------------------
//  Errors
// ---------------------------------------------------------------------------

/// Tetrahedral mesh improvement error.
#[derive(Debug, Clone, PartialEq)]
pub enum TetImproveError {
    /// Fewer than 4 vertices or fewer than 1 tet in the input.
    DegenerateInput { vertices: usize, tets: usize },
    /// A tet referenced a vertex index outside the vertex array.
    IndexOutOfBounds { tet: usize, vertex: u32 },
    /// A coordinate was non-finite (NaN / ±∞).
    NonFiniteCoordinate { index: u32 },
    /// An input tet was inverted (signed volume <= 0). Improvement requires a
    /// valid input mesh; orientation repair is a separate concern.
    InvertedInputTet { tet: usize, signed_volume: f64 },
    /// The Steiner insertion cap was reached before fixpoint.
    SteinerCapReached { inserted: u32 },
}

impl core::fmt::Display for TetImproveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DegenerateInput { vertices, tets } => write!(
                f,
                "tet_quality_improve: degenerate input — {vertices} vertices, {tets} tets (need ≥4 vertices and ≥1 tet)"
            ),
            Self::IndexOutOfBounds { tet, vertex } => write!(
                f,
                "tet_quality_improve: tet {tet} references vertex {vertex} out of bounds"
            ),
            Self::NonFiniteCoordinate { index } => write!(
                f,
                "tet_quality_improve: non-finite coordinate at vertex {index}"
            ),
            Self::InvertedInputTet { tet, signed_volume } => write!(
                f,
                "tet_quality_improve: input tet {tet} is inverted (signed_volume={signed_volume}); repair orientation first"
            ),
            Self::SteinerCapReached { inserted } => write!(
                f,
                "tet_quality_improve: Steiner cap reached after {inserted} insertions"
            ),
        }
    }
}

impl std::error::Error for TetImproveError {}
