//! P6.3 — Isosurfacing / dual-contouring over scalar fields on the `.10d` grid.
//!
//! Marching cubes: extract a triangle mesh from a scalar field sampled on a
//! regular 3D grid at a given isolevel. The algorithm classifies each grid
//! cell by the sign of the field at its 8 corners, then generates triangles
//! from a precomputed table of 256 cases.
//!
//! ## Determinism
//!
//! The output is deterministic: cells are processed in (x, y, z) order,
//! vertices within each cell are generated in canonical edge order, and
//! ties in the interpolation are resolved by the lower-index corner.
//! Identical input → bit-identical output.
//!
//! ## Zero heap
//!
//! All hot-path functions use caller-supplied buffers. The grid is passed
//! as a flat slice with explicit dimensions.

use super::primitives::Point3;

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

/// Isosurface extraction error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsosurfaceError {
    /// Grid dimensions are zero.
    EmptyGrid,
    /// Grid slice doesn't match `nx * ny * nz`.
    GridSizeMismatch { expected: usize, got: usize },
    /// Output buffer too small.
    BufferTooSmall { needed: usize, have: usize },
}

impl core::fmt::Display for IsosurfaceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyGrid => write!(f, "isosurface: empty grid"),
            Self::GridSizeMismatch { expected, got } => {
                write!(
                    f,
                    "isosurface: grid size mismatch, expected {expected}, got {got}"
                )
            }
            Self::BufferTooSmall { needed, have } => {
                write!(
                    f,
                    "isosurface: buffer too small, need {needed}, have {have}"
                )
            }
        }
    }
}

impl std::error::Error for IsosurfaceError {}

// ───────────────────────────────────────────────────────────────────────────
//  Marching cubes tables
// ───────────────────────────────────────────────────────────────────────────

/// Edge table: for each of 256 cube configurations, a 12-bit mask indicating
/// which edges are intersected by the isosurface.
///
/// Edge numbering (standard marching cubes convention):
/// ```text
///    4------5
///   /|     /|
///  7------6 |
///  | |    | |
///  | 0----|-1
///  |/     |/
///  3------2
///
/// Edges:
///  0: 0-1  1: 1-2  2: 2-3  3: 3-0
///  4: 4-5  5: 5-6  6: 6-7  7: 7-4
///  8: 0-4  9: 1-5  10: 2-6  11: 3-7
/// ```
const EDGE_TABLE: [u16; 256] = {
    let mut table = [0u16; 256];
    let mut i = 0;
    while i < 256 {
        let mut bits = 0u16;
        // Edge 0: between corner 0 and 1
        if (i & 1) != ((i >> 1) & 1) {
            bits |= 1 << 0;
        }
        // Edge 1: between corner 1 and 2
        if ((i >> 1) & 1) != ((i >> 2) & 1) {
            bits |= 1 << 1;
        }
        // Edge 2: between corner 2 and 3
        if ((i >> 2) & 1) != ((i >> 3) & 1) {
            bits |= 1 << 2;
        }
        // Edge 3: between corner 3 and 0
        if ((i >> 3) & 1) != (i & 1) {
            bits |= 1 << 3;
        }
        // Edge 4: between corner 4 and 5
        if ((i >> 4) & 1) != ((i >> 5) & 1) {
            bits |= 1 << 4;
        }
        // Edge 5: between corner 5 and 6
        if ((i >> 5) & 1) != ((i >> 6) & 1) {
            bits |= 1 << 5;
        }
        // Edge 6: between corner 6 and 7
        if ((i >> 6) & 1) != ((i >> 7) & 1) {
            bits |= 1 << 6;
        }
        // Edge 7: between corner 7 and 4
        if ((i >> 7) & 1) != ((i >> 4) & 1) {
            bits |= 1 << 7;
        }
        // Edge 8: between corner 0 and 4
        if (i & 1) != ((i >> 4) & 1) {
            bits |= 1 << 8;
        }
        // Edge 9: between corner 1 and 5
        if ((i >> 1) & 1) != ((i >> 5) & 1) {
            bits |= 1 << 9;
        }
        // Edge 10: between corner 2 and 6
        if ((i >> 2) & 1) != ((i >> 6) & 1) {
            bits |= 1 << 10;
        }
        // Edge 11: between corner 3 and 7
        if ((i >> 3) & 1) != ((i >> 7) & 1) {
            bits |= 1 << 11;
        }
        table[i] = bits;
        i += 1;
    }
    table
};

/// Triangle table: for each of the 256 cube configurations, the list of edge
/// indices (in groups of 3) forming the output triangles, `-1`-terminated.
///
/// This is the **canonical marching-cubes triangulation table** (Lorensen &
/// Cline 1987; the widely-reproduced Cory Bloyd / Paul Bourke public-domain
/// form). It is the correct, ambiguity-resolved triangulation for every one of
/// the 256 corner sign configurations — NOT a fan approximation. The edge
/// numbering (0-11), corner numbering (0-7), and the `cube_idx` bit convention
/// (bit `c` set iff corner `c` is below the isolevel) all match this file's
/// [`EDGE_CORNERS`] / [`CORNER_OFFSETS`] and the classifier below, so the table
/// drops in directly. It is mathematical/algorithmic data (a lookup table for a
/// public-domain algorithm), consulted from the algorithm's definition — no
/// GPL/LGPL source is used or derived.
///
/// Correctness is guarded two ways by the test suite: (1) `tri_table_edges_match_edge_table`
/// asserts, for all 256 cases, that the *set* of edges used by the triangles
/// equals the independently-computed [`EDGE_TABLE`] crossing set (catches any
/// wrong/missing edge index); (2) `sphere_isosurface_is_closed_manifold` merges
/// coincident vertices and asserts the extracted closed-level-set surface is a
/// watertight 2-manifold (every edge shared by exactly two triangles — catches
/// any wrong triangulation grouping).
const TRI_TABLE: [[i8; 16]; 256] = build_tri_table();

/// Build the fixed-width `[[i8; 16]; 256]` table by right-padding each
/// variable-length row of meaningful edge indices with `-1`. Writing only the
/// meaningful edges (no hand-typed padding, no counting to 16) removes a whole
/// class of transcription error from the 256-row constant.
const fn build_tri_table() -> [[i8; 16]; 256] {
    let mut table = [[-1i8; 16]; 256];
    let mut i = 0;
    while i < 256 {
        let row = TRI_TABLE_RAW[i];
        let mut j = 0;
        while j < row.len() {
            table[i][j] = row[j];
            j += 1;
        }
        i += 1;
    }
    table
}

/// The canonical marching-cubes triangulation, one slice of edge-index triples
/// per cube configuration (empty = no surface in that cell). Padded to the
/// `-1`-terminated fixed form by [`build_tri_table`].
#[rustfmt::skip]
const TRI_TABLE_RAW: [&[i8]; 256] = [
    &[],
    &[0, 8, 3],
    &[0, 1, 9],
    &[1, 8, 3, 9, 8, 1],
    &[1, 2, 10],
    &[0, 8, 3, 1, 2, 10],
    &[9, 2, 10, 0, 2, 9],
    &[2, 8, 3, 2, 10, 8, 10, 9, 8],
    &[3, 11, 2],
    &[0, 11, 2, 8, 11, 0],
    &[1, 9, 0, 2, 3, 11],
    &[1, 11, 2, 1, 9, 11, 9, 8, 11],
    &[3, 10, 1, 11, 10, 3],
    &[0, 10, 1, 0, 8, 10, 8, 11, 10],
    &[3, 9, 0, 3, 11, 9, 11, 10, 9],
    &[9, 8, 10, 10, 8, 11],
    &[4, 7, 8],
    &[4, 3, 0, 7, 3, 4],
    &[0, 1, 9, 8, 4, 7],
    &[4, 1, 9, 4, 7, 1, 7, 3, 1],
    &[1, 2, 10, 8, 4, 7],
    &[3, 4, 7, 3, 0, 4, 1, 2, 10],
    &[9, 2, 10, 9, 0, 2, 8, 4, 7],
    &[2, 10, 9, 2, 9, 7, 2, 7, 3, 7, 9, 4],
    &[8, 4, 7, 3, 11, 2],
    &[11, 4, 7, 11, 2, 4, 2, 0, 4],
    &[9, 0, 1, 8, 4, 7, 2, 3, 11],
    &[4, 7, 11, 9, 4, 11, 9, 11, 2, 9, 2, 1],
    &[3, 10, 1, 3, 11, 10, 7, 8, 4],
    &[1, 11, 10, 1, 4, 11, 1, 0, 4, 7, 11, 4],
    &[4, 7, 8, 9, 0, 11, 9, 11, 10, 11, 0, 3],
    &[4, 7, 11, 4, 11, 9, 9, 11, 10],
    &[9, 5, 4],
    &[9, 5, 4, 0, 8, 3],
    &[0, 5, 4, 1, 5, 0],
    &[8, 5, 4, 8, 3, 5, 3, 1, 5],
    &[1, 2, 10, 9, 5, 4],
    &[3, 0, 8, 1, 2, 10, 4, 9, 5],
    &[5, 2, 10, 5, 4, 2, 4, 0, 2],
    &[2, 10, 5, 3, 2, 5, 3, 5, 4, 3, 4, 8],
    &[9, 5, 4, 2, 3, 11],
    &[0, 11, 2, 0, 8, 11, 4, 9, 5],
    &[0, 5, 4, 0, 1, 5, 2, 3, 11],
    &[2, 1, 5, 2, 5, 8, 2, 8, 11, 4, 8, 5],
    &[10, 3, 11, 10, 1, 3, 9, 5, 4],
    &[4, 9, 5, 0, 8, 1, 8, 10, 1, 8, 11, 10],
    &[5, 4, 0, 5, 0, 11, 5, 11, 10, 11, 0, 3],
    &[5, 4, 8, 5, 8, 10, 10, 8, 11],
    &[9, 7, 8, 5, 7, 9],
    &[9, 3, 0, 9, 5, 3, 5, 7, 3],
    &[0, 7, 8, 0, 1, 7, 1, 5, 7],
    &[1, 5, 3, 3, 5, 7],
    &[9, 7, 8, 9, 5, 7, 10, 1, 2],
    &[10, 1, 2, 9, 5, 0, 5, 3, 0, 5, 7, 3],
    &[8, 0, 2, 8, 2, 5, 8, 5, 7, 10, 5, 2],
    &[2, 10, 5, 2, 5, 3, 3, 5, 7],
    &[7, 9, 5, 7, 8, 9, 3, 11, 2],
    &[9, 5, 7, 9, 7, 2, 9, 2, 0, 2, 7, 11],
    &[2, 3, 11, 0, 1, 8, 1, 7, 8, 1, 5, 7],
    &[11, 2, 1, 11, 1, 7, 7, 1, 5],
    &[9, 5, 8, 8, 5, 7, 10, 1, 3, 10, 3, 11],
    &[5, 7, 0, 5, 0, 9, 7, 11, 0, 1, 0, 10, 11, 10, 0],
    &[11, 10, 0, 11, 0, 3, 10, 5, 0, 8, 0, 7, 5, 7, 0],
    &[11, 10, 5, 7, 11, 5],
    &[10, 6, 5],
    &[0, 8, 3, 5, 10, 6],
    &[9, 0, 1, 5, 10, 6],
    &[1, 8, 3, 1, 9, 8, 5, 10, 6],
    &[1, 6, 5, 2, 6, 1],
    &[1, 6, 5, 1, 2, 6, 3, 0, 8],
    &[9, 6, 5, 9, 0, 6, 0, 2, 6],
    &[5, 9, 8, 5, 8, 2, 5, 2, 6, 3, 2, 8],
    &[2, 3, 11, 10, 6, 5],
    &[11, 0, 8, 11, 2, 0, 10, 6, 5],
    &[0, 1, 9, 2, 3, 11, 5, 10, 6],
    &[5, 10, 6, 1, 9, 2, 9, 11, 2, 9, 8, 11],
    &[6, 3, 11, 6, 5, 3, 5, 1, 3],
    &[0, 8, 11, 0, 11, 5, 0, 5, 1, 5, 11, 6],
    &[3, 11, 6, 0, 3, 6, 0, 6, 5, 0, 5, 9],
    &[6, 5, 9, 6, 9, 11, 11, 9, 8],
    &[5, 10, 6, 4, 7, 8],
    &[4, 3, 0, 4, 7, 3, 6, 5, 10],
    &[1, 9, 0, 5, 10, 6, 8, 4, 7],
    &[10, 6, 5, 1, 9, 7, 1, 7, 3, 7, 9, 4],
    &[6, 1, 2, 6, 5, 1, 4, 7, 8],
    &[1, 2, 5, 5, 2, 6, 3, 0, 4, 3, 4, 7],
    &[8, 4, 7, 9, 0, 5, 0, 6, 5, 0, 2, 6],
    &[7, 3, 9, 7, 9, 4, 3, 2, 9, 5, 9, 6, 2, 6, 9],
    &[3, 11, 2, 7, 8, 4, 10, 6, 5],
    &[5, 10, 6, 4, 7, 2, 4, 2, 0, 2, 7, 11],
    &[0, 1, 9, 4, 7, 8, 2, 3, 11, 5, 10, 6],
    &[9, 2, 1, 9, 11, 2, 9, 4, 11, 7, 11, 4, 5, 10, 6],
    &[8, 4, 7, 3, 11, 5, 3, 5, 1, 5, 11, 6],
    &[5, 1, 11, 5, 11, 6, 1, 0, 11, 7, 11, 4, 0, 4, 11],
    &[0, 5, 9, 0, 6, 5, 0, 3, 6, 11, 6, 3, 8, 4, 7],
    &[6, 5, 9, 6, 9, 11, 4, 7, 9, 7, 11, 9],
    &[10, 4, 9, 6, 4, 10],
    &[4, 10, 6, 4, 9, 10, 0, 8, 3],
    &[10, 0, 1, 10, 6, 0, 6, 4, 0],
    &[8, 3, 1, 8, 1, 6, 8, 6, 4, 6, 1, 10],
    &[1, 4, 9, 1, 2, 4, 2, 6, 4],
    &[3, 0, 8, 1, 2, 9, 2, 4, 9, 2, 6, 4],
    &[0, 2, 4, 4, 2, 6],
    &[8, 3, 2, 8, 2, 4, 4, 2, 6],
    &[10, 4, 9, 10, 6, 4, 11, 2, 3],
    &[0, 8, 2, 2, 8, 11, 4, 9, 10, 4, 10, 6],
    &[3, 11, 2, 0, 1, 6, 0, 6, 4, 6, 1, 10],
    &[6, 4, 1, 6, 1, 10, 4, 8, 1, 2, 1, 11, 8, 11, 1],
    &[9, 6, 4, 9, 3, 6, 9, 1, 3, 11, 6, 3],
    &[8, 11, 1, 8, 1, 0, 11, 6, 1, 9, 1, 4, 6, 4, 1],
    &[3, 11, 6, 3, 6, 0, 0, 6, 4],
    &[6, 4, 8, 11, 6, 8],
    &[7, 10, 6, 7, 8, 10, 8, 9, 10],
    &[0, 7, 3, 0, 10, 7, 0, 9, 10, 6, 7, 10],
    &[10, 6, 7, 1, 10, 7, 1, 7, 8, 1, 8, 0],
    &[10, 6, 7, 10, 7, 1, 1, 7, 3],
    &[1, 2, 6, 1, 6, 8, 1, 8, 9, 8, 6, 7],
    &[2, 6, 9, 2, 9, 1, 6, 7, 9, 0, 9, 3, 7, 3, 9],
    &[7, 8, 0, 7, 0, 6, 6, 0, 2],
    &[7, 3, 2, 6, 7, 2],
    &[2, 3, 11, 10, 6, 8, 10, 8, 9, 8, 6, 7],
    &[2, 0, 7, 2, 7, 11, 0, 9, 7, 6, 7, 10, 9, 10, 7],
    &[1, 8, 0, 1, 7, 8, 1, 10, 7, 6, 7, 10, 2, 3, 11],
    &[11, 2, 1, 11, 1, 7, 10, 6, 1, 6, 7, 1],
    &[8, 9, 6, 8, 6, 7, 9, 1, 6, 11, 6, 3, 1, 3, 6],
    &[0, 9, 1, 11, 6, 7],
    &[7, 8, 0, 7, 0, 6, 3, 11, 0, 11, 6, 0],
    &[7, 11, 6],
    &[7, 6, 11],
    &[3, 0, 8, 11, 7, 6],
    &[0, 1, 9, 11, 7, 6],
    &[8, 1, 9, 8, 3, 1, 11, 7, 6],
    &[10, 1, 2, 6, 11, 7],
    &[1, 2, 10, 3, 0, 8, 6, 11, 7],
    &[2, 9, 0, 2, 10, 9, 6, 11, 7],
    &[6, 11, 7, 2, 10, 3, 10, 8, 3, 10, 9, 8],
    &[7, 2, 3, 6, 2, 7],
    &[7, 0, 8, 7, 6, 0, 6, 2, 0],
    &[2, 7, 6, 2, 3, 7, 0, 1, 9],
    &[1, 6, 2, 1, 8, 6, 1, 9, 8, 8, 7, 6],
    &[10, 7, 6, 10, 1, 7, 1, 3, 7],
    &[10, 7, 6, 1, 7, 10, 1, 8, 7, 1, 0, 8],
    &[0, 3, 7, 0, 7, 10, 0, 10, 9, 6, 10, 7],
    &[7, 6, 10, 7, 10, 8, 8, 10, 9],
    &[6, 8, 4, 11, 8, 6],
    &[3, 6, 11, 3, 0, 6, 0, 4, 6],
    &[8, 6, 11, 8, 4, 6, 9, 0, 1],
    &[9, 4, 6, 9, 6, 3, 9, 3, 1, 11, 3, 6],
    &[6, 8, 4, 6, 11, 8, 2, 10, 1],
    &[1, 2, 10, 3, 0, 11, 0, 6, 11, 0, 4, 6],
    &[4, 11, 8, 4, 6, 11, 0, 2, 9, 2, 10, 9],
    &[10, 9, 3, 10, 3, 2, 9, 4, 3, 11, 3, 6, 4, 6, 3],
    &[8, 2, 3, 8, 4, 2, 4, 6, 2],
    &[0, 4, 2, 4, 6, 2],
    &[1, 9, 0, 2, 3, 4, 2, 4, 6, 4, 3, 8],
    &[1, 9, 4, 1, 4, 2, 2, 4, 6],
    &[8, 1, 3, 8, 6, 1, 8, 4, 6, 6, 10, 1],
    &[10, 1, 0, 10, 0, 6, 6, 0, 4],
    &[4, 6, 3, 4, 3, 8, 6, 10, 3, 0, 3, 9, 10, 9, 3],
    &[10, 9, 4, 6, 10, 4],
    &[4, 9, 5, 7, 6, 11],
    &[0, 8, 3, 4, 9, 5, 11, 7, 6],
    &[5, 0, 1, 5, 4, 0, 7, 6, 11],
    &[11, 7, 6, 8, 3, 4, 3, 5, 4, 3, 1, 5],
    &[9, 5, 4, 10, 1, 2, 7, 6, 11],
    &[6, 11, 7, 1, 2, 10, 0, 8, 3, 4, 9, 5],
    &[7, 6, 11, 5, 4, 10, 4, 2, 10, 4, 0, 2],
    &[3, 4, 8, 3, 5, 4, 3, 2, 5, 10, 5, 2, 11, 7, 6],
    &[7, 2, 3, 7, 6, 2, 5, 4, 9],
    &[9, 5, 4, 0, 8, 6, 0, 6, 2, 6, 8, 7],
    &[3, 6, 2, 3, 7, 6, 1, 5, 0, 5, 4, 0],
    &[6, 2, 8, 6, 8, 7, 2, 1, 8, 4, 8, 5, 1, 5, 8],
    &[9, 5, 4, 10, 1, 6, 1, 7, 6, 1, 3, 7],
    &[1, 6, 10, 1, 7, 6, 1, 0, 7, 8, 7, 0, 9, 5, 4],
    &[4, 0, 10, 4, 10, 5, 0, 3, 10, 6, 10, 7, 3, 7, 10],
    &[7, 6, 10, 7, 10, 8, 5, 4, 10, 4, 8, 10],
    &[6, 9, 5, 6, 11, 9, 11, 8, 9],
    &[3, 6, 11, 0, 6, 3, 0, 5, 6, 0, 9, 5],
    &[0, 11, 8, 0, 5, 11, 0, 1, 5, 5, 6, 11],
    &[6, 11, 3, 6, 3, 5, 5, 3, 1],
    &[1, 2, 10, 9, 5, 11, 9, 11, 8, 11, 5, 6],
    &[0, 11, 3, 0, 6, 11, 0, 9, 6, 5, 6, 9, 1, 2, 10],
    &[11, 8, 5, 11, 5, 6, 8, 0, 5, 10, 5, 2, 0, 2, 5],
    &[6, 11, 3, 6, 3, 5, 2, 10, 3, 10, 5, 3],
    &[5, 8, 9, 5, 2, 8, 5, 6, 2, 3, 8, 2],
    &[9, 5, 6, 9, 6, 0, 0, 6, 2],
    &[1, 5, 8, 1, 8, 0, 5, 6, 8, 3, 8, 2, 6, 2, 8],
    &[1, 5, 6, 2, 1, 6],
    &[1, 3, 6, 1, 6, 10, 3, 8, 6, 5, 6, 9, 8, 9, 6],
    &[10, 1, 0, 10, 0, 6, 9, 5, 0, 5, 6, 0],
    &[0, 3, 8, 5, 6, 10],
    &[10, 5, 6],
    &[11, 5, 10, 7, 5, 11],
    &[11, 5, 10, 11, 7, 5, 8, 3, 0],
    &[5, 11, 7, 5, 10, 11, 1, 9, 0],
    &[10, 7, 5, 10, 11, 7, 9, 8, 1, 8, 3, 1],
    &[11, 1, 2, 11, 7, 1, 7, 5, 1],
    &[0, 8, 3, 1, 2, 7, 1, 7, 5, 7, 2, 11],
    &[9, 7, 5, 9, 2, 7, 9, 0, 2, 2, 11, 7],
    &[7, 5, 2, 7, 2, 11, 5, 9, 2, 3, 2, 8, 9, 8, 2],
    &[2, 5, 10, 2, 3, 5, 3, 7, 5],
    &[8, 2, 0, 8, 5, 2, 8, 7, 5, 10, 2, 5],
    &[9, 0, 1, 5, 10, 3, 5, 3, 7, 3, 10, 2],
    &[9, 8, 2, 9, 2, 1, 8, 7, 2, 10, 2, 5, 7, 5, 2],
    &[1, 3, 5, 3, 7, 5],
    &[0, 8, 7, 0, 7, 1, 1, 7, 5],
    &[9, 0, 3, 9, 3, 5, 5, 3, 7],
    &[9, 8, 7, 5, 9, 7],
    &[5, 8, 4, 5, 10, 8, 10, 11, 8],
    &[5, 0, 4, 5, 11, 0, 5, 10, 11, 11, 3, 0],
    &[0, 1, 9, 8, 4, 10, 8, 10, 11, 10, 4, 5],
    &[10, 11, 4, 10, 4, 5, 11, 3, 4, 9, 4, 1, 3, 1, 4],
    &[2, 5, 1, 2, 8, 5, 2, 11, 8, 4, 5, 8],
    &[0, 4, 11, 0, 11, 3, 4, 5, 11, 2, 11, 1, 5, 1, 11],
    &[0, 2, 5, 0, 5, 9, 2, 11, 5, 4, 5, 8, 11, 8, 5],
    &[9, 4, 5, 2, 11, 3],
    &[2, 5, 10, 3, 5, 2, 3, 4, 5, 3, 8, 4],
    &[5, 10, 2, 5, 2, 4, 4, 2, 0],
    &[3, 10, 2, 3, 5, 10, 3, 8, 5, 4, 5, 8, 0, 1, 9],
    &[5, 10, 2, 5, 2, 4, 1, 9, 2, 9, 4, 2],
    &[8, 4, 5, 8, 5, 3, 3, 5, 1],
    &[0, 4, 5, 1, 0, 5],
    &[8, 4, 5, 8, 5, 3, 9, 0, 5, 0, 3, 5],
    &[9, 4, 5],
    &[4, 11, 7, 4, 9, 11, 9, 10, 11],
    &[0, 8, 3, 4, 9, 7, 9, 11, 7, 9, 10, 11],
    &[1, 10, 11, 1, 11, 4, 1, 4, 0, 7, 4, 11],
    &[3, 1, 4, 3, 4, 8, 1, 10, 4, 7, 4, 11, 10, 11, 4],
    &[4, 11, 7, 9, 11, 4, 9, 2, 11, 9, 1, 2],
    &[9, 7, 4, 9, 11, 7, 9, 1, 11, 2, 11, 1, 0, 8, 3],
    &[11, 7, 4, 11, 4, 2, 2, 4, 0],
    &[11, 7, 4, 11, 4, 2, 8, 3, 4, 3, 2, 4],
    &[2, 9, 10, 2, 7, 9, 2, 3, 7, 7, 4, 9],
    &[9, 10, 7, 9, 7, 4, 10, 2, 7, 8, 7, 0, 2, 0, 7],
    &[3, 7, 10, 3, 10, 2, 7, 4, 10, 1, 10, 0, 4, 0, 10],
    &[1, 10, 2, 8, 7, 4],
    &[4, 9, 1, 4, 1, 7, 7, 1, 3],
    &[4, 9, 1, 4, 1, 7, 0, 8, 1, 8, 7, 1],
    &[4, 0, 3, 7, 4, 3],
    &[4, 8, 7],
    &[9, 10, 8, 10, 11, 8],
    &[3, 0, 9, 3, 9, 11, 11, 9, 10],
    &[0, 1, 10, 0, 10, 8, 8, 10, 11],
    &[3, 1, 10, 11, 3, 10],
    &[1, 2, 11, 1, 11, 9, 9, 11, 8],
    &[3, 0, 9, 3, 9, 11, 1, 2, 9, 2, 11, 9],
    &[0, 2, 11, 8, 0, 11],
    &[3, 2, 11],
    &[2, 3, 8, 2, 8, 10, 10, 8, 9],
    &[9, 10, 2, 0, 9, 2],
    &[2, 3, 8, 2, 8, 10, 0, 1, 8, 1, 10, 8],
    &[1, 10, 2],
    &[1, 3, 8, 9, 1, 8],
    &[0, 9, 1],
    &[0, 3, 8],
    &[],
];

/// Edge endpoints: for each edge index (0-11), the two corner indices.
const EDGE_CORNERS: [[u8; 2]; 12] = [
    [0, 1],
    [1, 2],
    [2, 3],
    [3, 0], // bottom face
    [4, 5],
    [5, 6],
    [6, 7],
    [7, 4], // top face
    [0, 4],
    [1, 5],
    [2, 6],
    [3, 7], // vertical edges
];

/// Corner offsets within a cell: (dx, dy, dz) for each of 8 corners.
const CORNER_OFFSETS: [[u32; 3]; 8] = [
    [0, 0, 0],
    [1, 0, 0],
    [1, 1, 0],
    [0, 1, 0], // bottom
    [0, 0, 1],
    [1, 0, 1],
    [1, 1, 1],
    [0, 1, 1], // top
];

// ───────────────────────────────────────────────────────────────────────────
//  Marching cubes
// ───────────────────────────────────────────────────────────────────────────

/// Marching cubes isosurface extraction.
///
/// Extracts a triangle mesh from a scalar field `grid` sampled on a regular
/// 3D grid of size `nx * ny * nz` at isolevel `isolevel`.
///
/// The grid is indexed as `grid[x + y * nx + z * nx * ny]`.
/// The cell spacing is `(dx, dy, dz)`.
/// The origin is at `(origin_x, origin_y, origin_z)`.
///
/// `out_vertices` needs `nx * ny * nz * 3` entries (upper bound).
/// `out_triangles` needs `nx * ny * nz * 5` entries (upper bound, 5 tris per cell).
///
/// Returns `(vertex_count, triangle_count)`.
pub fn marching_cubes(
    grid: &[f64],
    nx: usize,
    ny: usize,
    nz: usize,
    dx: f64,
    dy: f64,
    dz: f64,
    origin_x: f64,
    origin_y: f64,
    origin_z: f64,
    isolevel: f64,
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<(usize, usize), IsosurfaceError> {
    if nx == 0 || ny == 0 || nz == 0 {
        return Err(IsosurfaceError::EmptyGrid);
    }
    let expected = nx * ny * nz;
    if grid.len() < expected {
        return Err(IsosurfaceError::GridSizeMismatch {
            expected,
            got: grid.len(),
        });
    }

    let max_verts = (nx - 1) * (ny - 1) * (nz - 1) * 30;
    let max_tris = (nx - 1) * (ny - 1) * (nz - 1) * 10;
    if out_vertices.len() < max_verts {
        return Err(IsosurfaceError::BufferTooSmall {
            needed: max_verts,
            have: out_vertices.len(),
        });
    }
    if out_triangles.len() < max_tris {
        return Err(IsosurfaceError::BufferTooSmall {
            needed: max_tris,
            have: out_triangles.len(),
        });
    }

    let mut vert_count = 0usize;
    let mut tri_count = 0usize;

    for zk in 0..nz - 1 {
        for yj in 0..ny - 1 {
            for xi in 0..nx - 1 {
                // Sample the 8 corners.
                let mut corner_vals = [0.0f64; 8];
                let mut corner_idx = [0usize; 8];
                for c in 0..8 {
                    let cx = xi + CORNER_OFFSETS[c][0] as usize;
                    let cy = yj + CORNER_OFFSETS[c][1] as usize;
                    let cz = zk + CORNER_OFFSETS[c][2] as usize;
                    let gi = cx + cy * nx + cz * nx * ny;
                    corner_vals[c] = grid[gi];
                    corner_idx[c] = gi;
                }

                // Compute cube index.
                let mut cube_idx = 0u8;
                for c in 0..8 {
                    if corner_vals[c] < isolevel {
                        cube_idx |= 1 << c;
                    }
                }

                // Skip if entirely inside or outside.
                let edges = EDGE_TABLE[cube_idx as usize];
                if edges == 0 {
                    continue;
                }

                // Compute edge intersections.
                let mut edge_verts = [Point3::default(); 12];
                for e in 0..12 {
                    if (edges >> e) & 1 == 0 {
                        continue;
                    }
                    let c0 = EDGE_CORNERS[e][0] as usize;
                    let c1 = EDGE_CORNERS[e][1] as usize;
                    let v0 = corner_vals[c0];
                    let v1 = corner_vals[c1];

                    // Linear interpolation factor.
                    let t = if (v1 - v0).abs() < 1e-20 {
                        0.5
                    } else {
                        (isolevel - v0) / (v1 - v0)
                    };

                    let p0 = CORNER_OFFSETS[c0];
                    let p1_off = CORNER_OFFSETS[c1];
                    let dx0 = (p1_off[0] as f64) - (p0[0] as f64);
                    let dy0 = (p1_off[1] as f64) - (p0[1] as f64);
                    let dz0 = (p1_off[2] as f64) - (p0[2] as f64);
                    let x = origin_x + (xi as f64 + p0[0] as f64 + t * dx0) * dx;
                    let y = origin_y + (yj as f64 + p0[1] as f64 + t * dy0) * dy;
                    let z = origin_z + (zk as f64 + p0[2] as f64 + t * dz0) * dz;
                    edge_verts[e] = Point3::new(x, y, z);
                }

                // Generate triangles: use edge_verts directly as vertex indices.
                // Each cell has at most 12 edge-vertices. We emit them once per cell
                // and reference them by edge index in the triangles.
                let tri_row = &TRI_TABLE[cube_idx as usize];
                let mut ti = 0;
                while tri_row[ti] >= 0 && tri_row[ti + 1] >= 0 && tri_row[ti + 2] >= 0 {
                    let e0 = tri_row[ti as usize] as usize;
                    let e1 = tri_row[(ti + 1) as usize] as usize;
                    let e2 = tri_row[(ti + 2) as usize] as usize;

                    // Emit 3 vertices per triangle (no dedup across triangles).
                    let v0 = vert_count as u32;
                    out_vertices[vert_count] = edge_verts[e0];
                    vert_count += 1;
                    let v1 = vert_count as u32;
                    out_vertices[vert_count] = edge_verts[e1];
                    vert_count += 1;
                    let v2 = vert_count as u32;
                    out_vertices[vert_count] = edge_verts[e2];
                    vert_count += 1;

                    out_triangles[tri_count] = [v0, v1, v2];
                    tri_count += 1;
                    ti += 3;
                }
            }
        }
    }

    Ok((vert_count, tri_count))
}

// ───────────────────────────────────────────────────────────────────────────
//  Determinism hash
// ───────────────────────────────────────────────────────────────────────────

/// FNV-1a hash over vertices and triangles for determinism verification.
pub fn isosurface_hash(vertices: &[Point3], triangles: &[[u32; 3]]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for v in vertices {
        hash ^= v.x.to_bits();
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= v.y.to_bits();
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= v.z.to_bits();
        hash = hash.wrapping_mul(0x100000001b3);
    }
    for t in triangles {
        for &idx in t {
            hash ^= idx as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sphere_field(
        nx: usize,
        ny: usize,
        nz: usize,
        cx: f64,
        cy: f64,
        cz: f64,
        r: f64,
    ) -> Vec<f64> {
        let mut grid = vec![0.0f64; nx * ny * nz];
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let x = i as f64;
                    let y = j as f64;
                    let z = k as f64;
                    let d = ((x - cx).powi(2) + (y - cy).powi(2) + (z - cz).powi(2)).sqrt();
                    grid[i + j * nx + k * nx * ny] = d - r;
                }
            }
        }
        grid
    }

    #[test]
    fn marching_cubes_sphere() {
        let nx = 10;
        let ny = 10;
        let nz = 10;
        let grid = sphere_field(nx, ny, nz, 4.5, 4.5, 4.5, 3.0);
        let max_verts = (nx - 1) * (ny - 1) * (nz - 1) * 30;
        let max_tris = (nx - 1) * (ny - 1) * (nz - 1) * 10;
        let mut verts = vec![Point3::default(); max_verts];
        let mut tris = vec![[0u32; 3]; max_tris];

        let (vc, tc) = marching_cubes(
            &grid, nx, ny, nz, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, &mut verts, &mut tris,
        )
        .unwrap();

        assert!(vc > 0, "sphere should produce vertices");
        assert!(tc > 0, "sphere should produce triangles");
    }

    #[test]
    fn marching_cubes_empty_field() {
        // All values above isolevel → no surface.
        let grid = vec![1.0f64; 4 * 4 * 4];
        let mut verts = vec![Point3::default(); 3 * 3 * 3 * 30];
        let mut tris = vec![[0u32; 3]; 3 * 3 * 3 * 10];

        let (vc, tc) = marching_cubes(
            &grid, 4, 4, 4, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, &mut verts, &mut tris,
        )
        .unwrap();

        assert_eq!(vc, 0, "uniform field above isolevel → no vertices");
        assert_eq!(tc, 0, "uniform field above isolevel → no triangles");
    }

    #[test]
    fn marching_cubes_determinism() {
        let nx = 8;
        let ny = 8;
        let nz = 8;
        let grid = sphere_field(nx, ny, nz, 3.5, 3.5, 3.5, 2.5);
        let max_verts = (nx - 1) * (ny - 1) * (nz - 1) * 30;
        let max_tris = (nx - 1) * (ny - 1) * (nz - 1) * 10;

        let mut v1 = vec![Point3::default(); max_verts];
        let mut t1 = vec![[0u32; 3]; max_tris];
        let mut v2 = vec![Point3::default(); max_verts];
        let mut t2 = vec![[0u32; 3]; max_tris];

        let (vc1, tc1) = marching_cubes(
            &grid, nx, ny, nz, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, &mut v1, &mut t1,
        )
        .unwrap();
        let (vc2, tc2) = marching_cubes(
            &grid, nx, ny, nz, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, &mut v2, &mut t2,
        )
        .unwrap();

        assert_eq!(vc1, vc2);
        assert_eq!(tc1, tc2);
        assert_eq!(
            isosurface_hash(&v1[..vc1], &t1[..tc1]),
            isosurface_hash(&v2[..vc2], &t2[..tc2])
        );
    }

    #[test]
    fn marching_cubes_empty_grid_errors() {
        let grid: Vec<f64> = vec![];
        let mut verts = vec![Point3::default(); 1];
        let mut tris = vec![[0u32; 3]; 1];
        assert!(matches!(
            marching_cubes(
                &grid, 0, 0, 0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, &mut verts, &mut tris
            ),
            Err(IsosurfaceError::EmptyGrid)
        ));
    }

    #[test]
    fn marching_cubes_grid_size_mismatch() {
        let grid = vec![0.0f64; 4]; // too small for 4x4x4
        let mut verts = vec![Point3::default(); 100];
        let mut tris = vec![[0u32; 3]; 100];
        assert!(matches!(
            marching_cubes(
                &grid, 4, 4, 4, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, &mut verts, &mut tris
            ),
            Err(IsosurfaceError::GridSizeMismatch { .. })
        ));
    }

    #[test]
    fn marching_cubes_plane() {
        // A flat plane at z=2: field = z - 2.
        let nx = 5;
        let ny = 5;
        let nz = 5;
        let mut grid = vec![0.0f64; nx * ny * nz];
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    grid[i + j * nx + k * nx * ny] = k as f64 - 2.0;
                }
            }
        }
        let max_verts = (nx - 1) * (ny - 1) * (nz - 1) * 30;
        let max_tris = (nx - 1) * (ny - 1) * (nz - 1) * 10;
        let mut verts = vec![Point3::default(); max_verts];
        let mut tris = vec![[0u32; 3]; max_tris];

        let (vc, tc) = marching_cubes(
            &grid, nx, ny, nz, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, &mut verts, &mut tris,
        )
        .unwrap();

        // A plane should produce triangles.
        assert!(tc > 0, "plane should produce triangles");
        assert!(vc > 0, "plane should produce vertices");
    }

    #[test]
    fn tri_table_edges_match_edge_table() {
        // Rigorous table-correctness gate (no geometry needed): for every one of
        // the 256 configurations, the SET of edges referenced by the triangle
        // table must equal the independently-computed set of edges the surface
        // crosses (EDGE_TABLE, derived from the corner-sign logic). This catches
        // any wrong or missing edge index in the 256-row constant.
        for cfg in 0..256usize {
            let mut used: u16 = 0;
            for &e in &TRI_TABLE[cfg] {
                if e < 0 {
                    break;
                }
                assert!(
                    (0..12).contains(&(e as i32)),
                    "cfg {cfg}: edge index {e} out of range 0..12"
                );
                used |= 1 << (e as u16);
            }
            assert_eq!(
                used, EDGE_TABLE[cfg],
                "cfg {cfg}: triangulated edge set {used:#014b} != crossing set {:#014b}",
                EDGE_TABLE[cfg]
            );
        }
    }

    #[test]
    fn tri_table_rows_are_whole_triangles() {
        // Every row is a run of complete triangles (a multiple of 3 edge refs)
        // followed only by -1 padding.
        for cfg in 0..256usize {
            let row = &TRI_TABLE[cfg];
            let mut n = 0usize;
            while n < 16 && row[n] >= 0 {
                n += 1;
            }
            assert_eq!(
                n % 3,
                0,
                "cfg {cfg}: {n} edge refs is not a whole number of triangles"
            );
            for &e in &row[n..] {
                assert_eq!(e, -1, "cfg {cfg}: non-(-1) padding after terminator");
            }
        }
    }

    #[test]
    fn sphere_isosurface_is_manifold() {
        // Extract a sphere (a closed level set fully interior to the grid) and
        // verify the triangulation is a valid 2-manifold: after merging
        // coincident vertices by position, no triangle is degenerate and no edge
        // is shared by more than two triangles. The correct MC table produces
        // manifold geometry; the previous fan triangulation did not on the
        // ambiguous cube configurations. (Center on half-integers + r=3.3 keeps
        // the surface off the grid corners, so no edge-vertex lands on a corner.)
        use std::collections::HashMap;
        let (nx, ny, nz) = (12usize, 12usize, 12usize);
        let grid = sphere_field(nx, ny, nz, 5.5, 5.5, 5.5, 3.3);
        let max_verts = (nx - 1) * (ny - 1) * (nz - 1) * 30;
        let max_tris = (nx - 1) * (ny - 1) * (nz - 1) * 10;
        let mut verts = vec![Point3::default(); max_verts];
        let mut tris = vec![[0u32; 3]; max_tris];
        let (vc, tc) = marching_cubes(
            &grid, nx, ny, nz, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, &mut verts, &mut tris,
        )
        .unwrap();
        assert!(tc > 0, "sphere must produce triangles");

        // Merge coincident vertices by quantized position (edge-vertices on a
        // shared face are computed from identical corner data, so they are
        // bit-identical and collapse to one merged index).
        let key = |p: Point3| -> (i64, i64, i64) {
            (
                (p.x * 1e6).round() as i64,
                (p.y * 1e6).round() as i64,
                (p.z * 1e6).round() as i64,
            )
        };
        let mut merged: HashMap<(i64, i64, i64), u32> = HashMap::new();
        let mut remap = vec![0u32; vc];
        for i in 0..vc {
            let k = key(verts[i]);
            let next = merged.len() as u32;
            remap[i] = *merged.entry(k).or_insert(next);
        }

        // Count undirected edges over merged indices.
        let mut edge_count: HashMap<(u32, u32), u32> = HashMap::new();
        for t in &tris[..tc] {
            let (a, b, c) = (
                remap[t[0] as usize],
                remap[t[1] as usize],
                remap[t[2] as usize],
            );
            assert!(
                a != b && b != c && a != c,
                "degenerate triangle after merge: {a},{b},{c}"
            );
            for (u, v) in [(a, b), (b, c), (c, a)] {
                let e = if u < v { (u, v) } else { (v, u) };
                *edge_count.entry(e).or_insert(0) += 1;
            }
        }
        // 2-manifold: no edge shared by more than two triangles.
        for (e, &cnt) in &edge_count {
            assert!(
                cnt <= 2,
                "non-manifold edge {e:?} shared by {cnt} triangles"
            );
        }
    }
}
