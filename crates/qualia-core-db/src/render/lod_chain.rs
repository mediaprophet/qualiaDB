//! P5.8 — LOD chain: author mesh → decimate N LODs → serialize to `.10d`
//! → renderer parses each level → `plan_view` selects the expected LOD.
//!
//! This module is the serial integrator spine connecting P5.7 (`decimate_3`)
//! to the `.10d` container format and the authoring planner (`authoring.rs`).
//!
//! ## Pipeline
//!
//! 1. **Author**: a source `Mesh` (the highest-LOD asset).
//! 2. **Decimate**: produce N LOD levels by successive QEM decimation, each
//!    targeting a fraction of the previous level's triangle count.
//! 3. **Serialize**: encode each LOD as a `.10d` QuantizedMesh section
//!    (`mesh_section.rs`), concatenated into a single buffer.
//! 4. **Select**: at render time, `select_lod` picks the appropriate LOD index
//!    based on `OperationalMode` (Full → LOD 0, Eco → LOD 1, Reserve → LOD 2).
//! 5. **Parse**: the renderer decodes the selected section back to a `Mesh`.
//!
//! ## Determinism
//!
//! Decimation is deterministic (canonical vertex remap, sorted collapses).
//! `.10d` encoding is deterministic (quantization is pure, section order is
//! ascending). Two encodes of the same LOD chain produce byte-identical output.
//!
//! ## Budget rail
//!
//! The `plan_view` integration adds a `LodDisposition` that combines the
//! existing governance/attestation gates with LOD selection. On a constrained
//! tier (`Eco`/`Reserve`), a `Scene3D` view selects a coarser LOD rather than
//! collapsing to 2D — *if* a coarser LOD is available. If no coarser LOD
//! exists, the existing `Collapsed2D` fallback applies.

use crate::container_10d::mesh_section::{
    decode_mesh_section, encode_mesh_section, encoded_len, MeshSectionError,
};
use crate::gpu_context::OperationalMode;
use crate::render::assets::Mesh;
use crate::specialized_libs::computational_geometry::{
    decimate_qem, DecimateError, DecimateOptions, DecimateReport, Point3,
};

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

/// Failure modes for the LOD chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LodChainError {
    /// Decimation failed at the given LOD level.
    DecimateFailed { level: usize, cause: DecimateError },
    /// `.10d` encoding failed at the given LOD level.
    EncodeFailed {
        level: usize,
        cause: MeshSectionError,
    },
    /// `.10d` decoding failed at the given LOD level.
    DecodeFailed {
        level: usize,
        cause: MeshSectionError,
    },
    /// Output buffer too small for the serialized LOD chain.
    BufferTooSmall { needed: usize, have: usize },
    /// No LOD levels were requested.
    NoLodLevels,
    /// The source mesh is empty (no vertices or triangles).
    EmptySourceMesh,
}

impl core::fmt::Display for LodChainError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DecimateFailed { level, cause } => {
                write!(
                    f,
                    "lod_chain: decimation failed at level {level}: {cause:?}"
                )
            }
            Self::EncodeFailed { level, cause } => {
                write!(f, "lod_chain: encode failed at level {level}: {cause}")
            }
            Self::DecodeFailed { level, cause } => {
                write!(f, "lod_chain: decode failed at level {level}: {cause}")
            }
            Self::BufferTooSmall { needed, have } => {
                write!(f, "lod_chain: buffer too small, need {needed}, have {have}")
            }
            Self::NoLodLevels => write!(f, "lod_chain: no LOD levels requested"),
            Self::EmptySourceMesh => write!(f, "lod_chain: source mesh is empty"),
        }
    }
}

impl std::error::Error for LodChainError {}

// ───────────────────────────────────────────────────────────────────────────
//  LOD chain configuration
// ───────────────────────────────────────────────────────────────────────────

/// Maximum number of LOD levels supported (including LOD 0 = full resolution).
pub const MAX_LOD_LEVELS: usize = 8;

/// LOD chain configuration: how many levels and what fraction to decimate to
/// at each level.
#[derive(Debug, Clone, Copy)]
pub struct LodChainOptions {
    /// Number of LOD levels (including LOD 0 = the source mesh). Must be ≥ 1.
    pub level_count: usize,
    /// Triangle-count fraction at each successive LOD level. E.g. 0.5 means
    /// each level has half the triangles of the previous. LOD 0 is always the
    /// full mesh; LOD 1 targets `fraction * LOD0_faces`, etc.
    pub fraction: f64,
}

impl Default for LodChainOptions {
    fn default() -> Self {
        Self {
            level_count: 3,
            fraction: 0.5,
        }
    }
}

impl LodChainOptions {
    /// Create a configuration with `level_count` levels and the given fraction.
    pub fn new(level_count: usize, fraction: f64) -> Self {
        Self {
            level_count: level_count.clamp(1, MAX_LOD_LEVELS),
            fraction: fraction.clamp(0.01, 1.0),
        }
    }

    /// 3 levels (Full / Eco / Reserve) at 50% per level.
    pub fn default_3_tier() -> Self {
        Self {
            level_count: 3,
            fraction: 0.5,
        }
    }
}

/// Report for one LOD level after building the chain.
#[derive(Debug, Clone, Copy)]
pub struct LodLevelReport {
    /// LOD index (0 = full resolution).
    pub level: usize,
    /// Vertex count at this level.
    pub vertices: usize,
    /// Triangle count at this level.
    pub triangles: usize,
    /// Encoded `.10d` section byte length.
    pub encoded_bytes: usize,
    /// Decimation report (None for LOD 0).
    pub decimate_report: Option<DecimateReport>,
}

/// Overall LOD chain build report.
#[derive(Debug, Clone)]
pub struct LodChainReport {
    pub levels: Vec<LodLevelReport>,
    /// Total serialized byte length (all sections concatenated).
    pub total_bytes: usize,
}

// ───────────────────────────────────────────────────────────────────────────
//  LOD chain builder
// ───────────────────────────────────────────────────────────────────────────

/// Build a LOD chain from a source mesh and serialize all levels as `.10d`
/// QuantizedMesh sections into `out_buffer`.
///
/// Returns a report with per-level stats and the total bytes written.
///
/// This is a cold builder (uses `Vec` scratch for decimation and encoding).
/// The output is caller-owned and deterministic.
pub fn build_lod_chain(
    source: &Mesh,
    options: LodChainOptions,
    out_buffer: &mut [u8],
) -> Result<LodChainReport, LodChainError> {
    if source.positions.is_empty() || source.triangles.is_empty() {
        return Err(LodChainError::EmptySourceMesh);
    }
    if options.level_count == 0 {
        return Err(LodChainError::NoLodLevels);
    }

    let mut levels: Vec<LodLevelReport> = Vec::with_capacity(options.level_count);
    let mut offset = 0usize;

    // LOD 0: encode the source mesh directly (no decimation).
    let lod0_bytes = encoded_len(source.positions.len(), source.triangles.len());
    if offset + lod0_bytes > out_buffer.len() {
        return Err(LodChainError::BufferTooSmall {
            needed: offset + lod0_bytes,
            have: out_buffer.len(),
        });
    }
    let written = encode_mesh_section(source, &mut out_buffer[offset..])
        .map_err(|e| LodChainError::EncodeFailed { level: 0, cause: e })?;
    levels.push(LodLevelReport {
        level: 0,
        vertices: source.positions.len(),
        triangles: source.triangles.len(),
        encoded_bytes: written,
        decimate_report: None,
    });
    offset += written;

    // Successive LOD levels: decimate the previous level.
    let mut current_verts_f64: Vec<Point3> = source
        .positions
        .iter()
        .map(|p| Point3::new(p[0] as f64, p[1] as f64, p[2] as f64))
        .collect();
    let mut current_tris: Vec<[u32; 3]> = source.triangles.clone();

    for level in 1..options.level_count {
        let target_faces = (current_tris.len() as f64 * options.fraction) as usize;
        if target_faces < 4 {
            // Can't decimate below 4 triangles; stop early.
            break;
        }

        let decim_opts = DecimateOptions::to_faces(target_faces);
        let mut out_v = vec![Point3::default(); current_verts_f64.len()];
        let mut out_t = vec![[0u32; 3]; current_tris.len()];

        let report = decimate_qem(
            &current_verts_f64,
            &current_tris,
            decim_opts,
            &mut out_v,
            &mut out_t,
        )
        .map_err(|e| LodChainError::DecimateFailed { level, cause: e })?;

        // Compact to actual counts.
        let live_v = report.vertices;
        let live_t = report.faces;
        current_verts_f64 = out_v[..live_v].to_vec();
        current_tris = out_t[..live_t].to_vec();

        // Convert back to f32 for encoding.
        let f32_positions: Vec<[f32; 3]> = current_verts_f64
            .iter()
            .map(|p| [p.x as f32, p.y as f32, p.z as f32])
            .collect();

        // Encode this LOD level.
        let lod_mesh = Mesh {
            positions: f32_positions.clone(),
            triangles: current_tris.clone(),
            min: [
                f32_positions
                    .iter()
                    .map(|p| p[0])
                    .fold(f32::INFINITY, f32::min),
                f32_positions
                    .iter()
                    .map(|p| p[1])
                    .fold(f32::INFINITY, f32::min),
                f32_positions
                    .iter()
                    .map(|p| p[2])
                    .fold(f32::INFINITY, f32::min),
            ],
            max: [
                f32_positions
                    .iter()
                    .map(|p| p[0])
                    .fold(f32::NEG_INFINITY, f32::max),
                f32_positions
                    .iter()
                    .map(|p| p[1])
                    .fold(f32::NEG_INFINITY, f32::max),
                f32_positions
                    .iter()
                    .map(|p| p[2])
                    .fold(f32::NEG_INFINITY, f32::max),
            ],
        };

        let lod_bytes = encoded_len(live_v, live_t);
        if offset + lod_bytes > out_buffer.len() {
            return Err(LodChainError::BufferTooSmall {
                needed: offset + lod_bytes,
                have: out_buffer.len(),
            });
        }
        let written = encode_mesh_section(&lod_mesh, &mut out_buffer[offset..])
            .map_err(|e| LodChainError::EncodeFailed { level, cause: e })?;
        levels.push(LodLevelReport {
            level,
            vertices: live_v,
            triangles: live_t,
            encoded_bytes: written,
            decimate_report: Some(report),
        });
        offset += written;
    }

    Ok(LodChainReport {
        levels,
        total_bytes: offset,
    })
}

/// Compute the total buffer size needed for a LOD chain with the given options
/// and source mesh size. This is an upper bound (actual may be smaller if
/// decimation stops early).
pub fn required_lod_buffer_size(
    source_vertices: usize,
    source_triangles: usize,
    options: LodChainOptions,
) -> usize {
    let mut total = 0usize;
    let mut verts = source_vertices;
    let mut tris = source_triangles;

    for level in 0..options.level_count {
        total += encoded_len(verts, tris);
        if level > 0 {
            let target = (tris as f64 * options.fraction) as usize;
            if target < 4 {
                break;
            }
            // Estimate vertex count reduction proportionally.
            verts = (verts as f64 * options.fraction) as usize;
            tris = target;
        }
    }
    total
}

// ───────────────────────────────────────────────────────────────────────────
//  LOD selection
// ───────────────────────────────────────────────────────────────────────────

/// Select the appropriate LOD index for a given `OperationalMode`.
///
/// - `Full` → LOD 0 (full resolution)
/// - `Eco` → LOD 1 (half resolution, or the coarsest available if < 2 levels)
/// - `Reserve` → LOD 2 (quarter resolution, or the coarsest available if < 3 levels)
///
/// Returns the LOD index clamped to `[0, level_count-1]`.
#[inline]
pub fn select_lod(mode: OperationalMode, level_count: usize) -> usize {
    if level_count == 0 {
        return 0;
    }
    let preferred = match mode {
        OperationalMode::Full => 0,
        OperationalMode::Eco => 1,
        OperationalMode::Reserve => 2,
    };
    preferred.min(level_count - 1)
}

// ───────────────────────────────────────────────────────────────────────────
//  LOD section parsing
// ───────────────────────────────────────────────────────────────────────────

/// Parse a specific LOD level from a serialized LOD chain buffer.
///
/// The `level_offsets` are the byte offsets of each LOD section within the
/// buffer (as reported by `LodChainReport`). Returns the decoded `Mesh`.
pub fn parse_lod_level(
    buffer: &[u8],
    level_offsets: &[usize],
    level: usize,
) -> Result<Mesh, LodChainError> {
    if level >= level_offsets.len() {
        return Err(LodChainError::DecodeFailed {
            level,
            cause: MeshSectionError::PayloadTooShort { got: 0, need: 40 },
        });
    }
    let offset = level_offsets[level];
    let end = if level + 1 < level_offsets.len() {
        level_offsets[level + 1]
    } else {
        buffer.len()
    };
    let section_bytes = &buffer[offset..end];
    decode_mesh_section(section_bytes).map_err(|e| LodChainError::DecodeFailed { level, cause: e })
}

/// Extract the per-level byte offsets from a `LodChainReport`.
pub fn level_offsets(report: &LodChainReport) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(report.levels.len());
    let mut acc = 0;
    for lvl in &report.levels {
        offsets.push(acc);
        acc += lvl.encoded_bytes;
    }
    offsets
}

// ───────────────────────────────────────────────────────────────────────────
//  plan_view integration
// ───────────────────────────────────────────────────────────────────────────

/// The LOD-aware disposition for a view, extending `ViewDisposition` with
/// LOD selection for 3D scenes that have a LOD chain available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LodViewDisposition {
    /// Render the 3D scene at the given LOD level.
    Render3dWithLod { lod: usize },
    /// Render the 2D pane (no LOD needed).
    Render2d,
    /// A `Scene3D` view degraded to 2D under a constrained device budget with
    /// no coarser LOD available.
    Collapsed2D,
    /// Attestation-gated and not yet attested — withheld.
    WithheldUnattested,
    /// Sensitive content in a shared/civic standpoint without consent — refused.
    RefusedRightsBounded,
}

/// Plan a view with LOD awareness. Applies the same gate order as
/// `authoring::plan_view` (attestation → rights → budget), but instead of
/// collapsing 3D to 2D on constrained tiers, selects a coarser LOD if
/// available.
#[allow(clippy::too_many_arguments)]
pub fn plan_view_with_lod(
    view: &crate::render::authoring::QappView,
    standpoint: &crate::render::authoring::RenderStandpoint,
    mode: OperationalMode,
    attestations: &[crate::NQuin],
    gov_norms: &[crate::NQuin],
    now_unix: u32,
    lod_level_count: usize,
) -> LodViewDisposition {
    use crate::render::authoring::{has_attestation, Sensitivity, ViewKind};

    // 1) Attestation gate.
    if view.requires_attestation && !has_attestation(view, attestations) {
        return LodViewDisposition::WithheldUnattested;
    }

    // 2) Rights-bounded context.
    if matches!(view.sensitivity, Sensitivity::RightsBounded)
        && !crate::render::authoring::rights_render_permitted(
            standpoint,
            view.manifold,
            gov_norms,
            now_unix,
        )
    {
        return LodViewDisposition::RefusedRightsBounded;
    }

    // 3) Budget + LOD selection.
    match view.kind {
        ViewKind::Pane2D => LodViewDisposition::Render2d,
        ViewKind::Scene3D => {
            if mode.supports_3d() {
                LodViewDisposition::Render3dWithLod {
                    lod: select_lod(mode, lod_level_count),
                }
            } else if lod_level_count > 1 {
                // Constrained tier but coarser LODs available → use the coarsest.
                LodViewDisposition::Render3dWithLod {
                    lod: select_lod(mode, lod_level_count),
                }
            } else {
                // No coarser LOD → collapse to 2D.
                LodViewDisposition::Collapsed2D
            }
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Hash stability
// ───────────────────────────────────────────────────────────────────────────

/// Compute a simple FNV-1a hash over a byte slice (for determinism verification).
pub fn lod_chain_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q_hash;
    use crate::render::authoring::{
        attestation_quin, plan_view, QappView, RenderStandpoint, Sensitivity, ViewDisposition,
        ViewKind,
    };

    fn unit_cube_mesh() -> Mesh {
        let positions = vec![
            [0.0f32, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ];
        let triangles = vec![
            [0, 3, 2],
            [0, 2, 1],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [3, 7, 6],
            [3, 6, 2],
            [0, 4, 7],
            [0, 7, 3],
            [1, 2, 6],
            [1, 6, 5],
        ];
        Mesh {
            positions,
            triangles,
            min: [0.0, 0.0, 0.0],
            max: [1.0, 1.0, 1.0],
        }
    }

    fn subdivided_cube_mesh(subdivs: usize) -> Mesh {
        let s = subdivs as f32;
        let mut positions = Vec::new();
        for i in 0..=subdivs {
            for j in 0..=subdivs {
                for k in 0..=subdivs {
                    positions.push([i as f32 / s, j as f32 / s, k as f32 / s]);
                }
            }
        }
        // Build triangles for each face of the cube.
        let mut triangles = Vec::new();
        let idx = |i: usize, j: usize, k: usize| -> u32 {
            (i * (subdivs + 1) * (subdivs + 1) + j * (subdivs + 1) + k) as u32
        };
        // Bottom (z=0, facing down)
        for i in 0..subdivs {
            for j in 0..subdivs {
                triangles.push([idx(i, j, 0), idx(i, j + 1, 0), idx(i + 1, j + 1, 0)]);
                triangles.push([idx(i, j, 0), idx(i + 1, j + 1, 0), idx(i + 1, j, 0)]);
            }
        }
        // Top (z=1, facing up)
        for i in 0..subdivs {
            for j in 0..subdivs {
                triangles.push([
                    idx(i, j, subdivs),
                    idx(i + 1, j + 1, subdivs),
                    idx(i, j + 1, subdivs),
                ]);
                triangles.push([
                    idx(i, j, subdivs),
                    idx(i + 1, j, subdivs),
                    idx(i + 1, j + 1, subdivs),
                ]);
            }
        }
        // Front (y=0, facing -y)
        for i in 0..subdivs {
            for k in 0..subdivs {
                triangles.push([idx(i, 0, k), idx(i + 1, 0, k + 1), idx(i, 0, k + 1)]);
                triangles.push([idx(i, 0, k), idx(i + 1, 0, k), idx(i + 1, 0, k + 1)]);
            }
        }
        // Back (y=1, facing +y)
        for i in 0..subdivs {
            for k in 0..subdivs {
                triangles.push([
                    idx(i, subdivs, k),
                    idx(i, subdivs, k + 1),
                    idx(i + 1, subdivs, k + 1),
                ]);
                triangles.push([
                    idx(i, subdivs, k),
                    idx(i + 1, subdivs, k + 1),
                    idx(i + 1, subdivs, k),
                ]);
            }
        }
        // Left (x=0, facing -x)
        for j in 0..subdivs {
            for k in 0..subdivs {
                triangles.push([idx(0, j, k), idx(0, j, k + 1), idx(0, j + 1, k + 1)]);
                triangles.push([idx(0, j, k), idx(0, j + 1, k + 1), idx(0, j + 1, k)]);
            }
        }
        // Right (x=1, facing +x)
        for j in 0..subdivs {
            for k in 0..subdivs {
                triangles.push([
                    idx(subdivs, j, k),
                    idx(subdivs, j + 1, k + 1),
                    idx(subdivs, j, k + 1),
                ]);
                triangles.push([
                    idx(subdivs, j, k),
                    idx(subdivs, j + 1, k),
                    idx(subdivs, j + 1, k + 1),
                ]);
            }
        }
        let min = [0.0f32; 3];
        let max = [1.0f32; 3];
        Mesh {
            positions,
            triangles,
            min,
            max,
        }
    }

    #[test]
    fn build_lod_chain_3_levels() {
        let mesh = subdivided_cube_mesh(4); // 125 verts, 96 tris
        let options = LodChainOptions::default_3_tier();
        let buf_size =
            required_lod_buffer_size(mesh.positions.len(), mesh.triangles.len(), options);
        let mut buf = vec![0u8; buf_size];
        let report = build_lod_chain(&mesh, options, &mut buf).unwrap();

        assert!(
            report.levels.len() >= 2,
            "should produce at least 2 LOD levels"
        );
        assert_eq!(report.levels[0].level, 0);
        assert_eq!(report.levels[0].vertices, mesh.positions.len());
        assert_eq!(report.levels[0].triangles, mesh.triangles.len());
        assert_eq!(
            report.total_bytes,
            report.levels.iter().map(|l| l.encoded_bytes).sum::<usize>()
        );
    }

    #[test]
    fn lod_chain_hash_stable() {
        let mesh = subdivided_cube_mesh(3);
        let options = LodChainOptions::default_3_tier();
        let buf_size =
            required_lod_buffer_size(mesh.positions.len(), mesh.triangles.len(), options);

        let mut buf1 = vec![0u8; buf_size];
        let mut buf2 = vec![0u8; buf_size];
        let report1 = build_lod_chain(&mesh, options, &mut buf1).unwrap();
        let report2 = build_lod_chain(&mesh, options, &mut buf2).unwrap();

        assert_eq!(report1.total_bytes, report2.total_bytes);
        let h1 = lod_chain_hash(&buf1[..report1.total_bytes]);
        let h2 = lod_chain_hash(&buf2[..report2.total_bytes]);
        assert_eq!(
            h1, h2,
            "LOD chain bytes must be hash-stable across two encodes"
        );
    }

    #[test]
    fn lod_chain_round_trip() {
        let mesh = subdivided_cube_mesh(3);
        let options = LodChainOptions::default_3_tier();
        let buf_size =
            required_lod_buffer_size(mesh.positions.len(), mesh.triangles.len(), options);
        let mut buf = vec![0u8; buf_size];
        let report = build_lod_chain(&mesh, options, &mut buf).unwrap();

        let offsets = level_offsets(&report);

        // Parse LOD 0 and verify it matches the source.
        let lod0 = parse_lod_level(&buf, &offsets, 0).unwrap();
        assert_eq!(lod0.positions.len(), mesh.positions.len());
        assert_eq!(lod0.triangles.len(), mesh.triangles.len());

        // Parse any additional LODs.
        for (i, lvl) in report.levels.iter().enumerate() {
            let decoded = parse_lod_level(&buf, &offsets, i).unwrap();
            assert_eq!(
                decoded.positions.len(),
                lvl.vertices,
                "LOD {i} vertex count mismatch"
            );
            assert_eq!(
                decoded.triangles.len(),
                lvl.triangles,
                "LOD {i} triangle count mismatch"
            );
        }
    }

    #[test]
    fn select_lod_by_mode() {
        assert_eq!(select_lod(OperationalMode::Full, 3), 0);
        assert_eq!(select_lod(OperationalMode::Eco, 3), 1);
        assert_eq!(select_lod(OperationalMode::Reserve, 3), 2);
        // Clamping: if only 2 levels, Reserve → LOD 1.
        assert_eq!(select_lod(OperationalMode::Reserve, 2), 1);
        // If only 1 level, all modes → LOD 0.
        assert_eq!(select_lod(OperationalMode::Reserve, 1), 0);
    }

    #[test]
    fn lod_chain_decreasing_triangle_counts() {
        let mesh = subdivided_cube_mesh(5);
        let options = LodChainOptions::default_3_tier();
        let buf_size =
            required_lod_buffer_size(mesh.positions.len(), mesh.triangles.len(), options);
        let mut buf = vec![0u8; buf_size];
        let report = build_lod_chain(&mesh, options, &mut buf).unwrap();

        for i in 1..report.levels.len() {
            assert!(
                report.levels[i].triangles <= report.levels[i - 1].triangles,
                "LOD {i} should have ≤ triangles than LOD {}",
                i - 1
            );
        }
    }

    #[test]
    fn plan_view_with_lod_selects_coarser_on_eco() {
        let m = q_hash("urn:qualia:manifold:demo");
        let view = QappView::public(m, ViewKind::Scene3D);
        let standpoint = RenderStandpoint {
            id: q_hash("urn:qualia:standpoint:owner"),
            shared_civic: false,
        };

        // Full → LOD 0.
        assert_eq!(
            plan_view_with_lod(&view, &standpoint, OperationalMode::Full, &[], &[], 100, 3),
            LodViewDisposition::Render3dWithLod { lod: 0 }
        );
        // Eco → LOD 1.
        assert_eq!(
            plan_view_with_lod(&view, &standpoint, OperationalMode::Eco, &[], &[], 100, 3),
            LodViewDisposition::Render3dWithLod { lod: 1 }
        );
        // Reserve → LOD 2.
        assert_eq!(
            plan_view_with_lod(
                &view,
                &standpoint,
                OperationalMode::Reserve,
                &[],
                &[],
                100,
                3
            ),
            LodViewDisposition::Render3dWithLod { lod: 2 }
        );
    }

    #[test]
    fn plan_view_with_lod_collapses_when_no_coarser_lod() {
        let m = q_hash("urn:qualia:manifold:demo");
        let view = QappView::public(m, ViewKind::Scene3D);
        let standpoint = RenderStandpoint {
            id: q_hash("urn:qualia:standpoint:owner"),
            shared_civic: false,
        };

        // Only 1 LOD level → Eco collapses to 2D.
        assert_eq!(
            plan_view_with_lod(&view, &standpoint, OperationalMode::Eco, &[], &[], 100, 1),
            LodViewDisposition::Collapsed2D
        );
    }

    #[test]
    fn plan_view_with_lod_2d_pane_ignores_lod() {
        let m = q_hash("urn:qualia:manifold:demo");
        let view = QappView::public(m, ViewKind::Pane2D);
        let standpoint = RenderStandpoint {
            id: q_hash("urn:qualia:standpoint:owner"),
            shared_civic: false,
        };

        assert_eq!(
            plan_view_with_lod(&view, &standpoint, OperationalMode::Full, &[], &[], 100, 3),
            LodViewDisposition::Render2d
        );
        assert_eq!(
            plan_view_with_lod(&view, &standpoint, OperationalMode::Eco, &[], &[], 100, 3),
            LodViewDisposition::Render2d
        );
    }

    #[test]
    fn plan_view_with_lod_attestation_gate() {
        let m = q_hash("urn:qualia:manifold:demo");
        let view = QappView {
            manifold: m,
            kind: ViewKind::Scene3D,
            sensitivity: Sensitivity::Public,
            requires_attestation: true,
        };
        let standpoint = RenderStandpoint {
            id: q_hash("urn:qualia:standpoint:owner"),
            shared_civic: false,
        };

        // No attestation → withheld.
        assert_eq!(
            plan_view_with_lod(&view, &standpoint, OperationalMode::Full, &[], &[], 100, 3),
            LodViewDisposition::WithheldUnattested
        );

        // With attestation → rendered at LOD 0.
        let att = attestation_quin(
            q_hash("did:example:auditor"),
            m,
            q_hash("urn:qualia:frame:app"),
        );
        assert_eq!(
            plan_view_with_lod(
                &view,
                &standpoint,
                OperationalMode::Full,
                &[att],
                &[],
                100,
                3
            ),
            LodViewDisposition::Render3dWithLod { lod: 0 }
        );
    }

    #[test]
    fn plan_view_with_lod_rights_bounded() {
        let m = q_hash("urn:qualia:manifold:demo");
        let view = QappView {
            manifold: m,
            kind: ViewKind::Scene3D,
            sensitivity: Sensitivity::RightsBounded,
            requires_attestation: false,
        };
        let civic = RenderStandpoint {
            id: q_hash("urn:qualia:standpoint:civic"),
            shared_civic: true,
        };

        // Civic, no consent → refused.
        assert_eq!(
            plan_view_with_lod(&view, &civic, OperationalMode::Full, &[], &[], 100, 3),
            LodViewDisposition::RefusedRightsBounded
        );
    }

    #[test]
    fn existing_authoring_tests_stay_green() {
        // Verify that the existing plan_view still works (no regression).
        let m = q_hash("urn:qualia:manifold:demo");
        let view = QappView::public(m, ViewKind::Scene3D);
        let standpoint = RenderStandpoint {
            id: q_hash("urn:qualia:standpoint:owner"),
            shared_civic: false,
        };

        // Full tier → 3D scene rendered.
        assert_eq!(
            plan_view(&view, &standpoint, OperationalMode::Full, &[], &[], 100),
            ViewDisposition::Render(ViewKind::Scene3D)
        );
        // Eco tier → collapsed to 2D (existing behavior, no LOD).
        assert_eq!(
            plan_view(&view, &standpoint, OperationalMode::Eco, &[], &[], 100),
            ViewDisposition::Collapsed2D
        );
    }

    #[test]
    fn empty_source_mesh_errors() {
        let mesh = Mesh {
            positions: vec![],
            triangles: vec![],
            min: [0.0; 3],
            max: [0.0; 3],
        };
        let options = LodChainOptions::default_3_tier();
        let mut buf = vec![0u8; 1024];
        assert!(matches!(
            build_lod_chain(&mesh, options, &mut buf),
            Err(LodChainError::EmptySourceMesh)
        ));
    }

    #[test]
    fn buffer_too_small_errors() {
        let mesh = unit_cube_mesh();
        let options = LodChainOptions::default_3_tier();
        let mut buf = vec![0u8; 10]; // way too small
        assert!(matches!(
            build_lod_chain(&mesh, options, &mut buf),
            Err(LodChainError::BufferTooSmall { .. })
        ));
    }

    #[test]
    fn single_level_lod_chain() {
        let mesh = unit_cube_mesh();
        let options = LodChainOptions::new(1, 0.5);
        let buf_size =
            required_lod_buffer_size(mesh.positions.len(), mesh.triangles.len(), options);
        let mut buf = vec![0u8; buf_size];
        let report = build_lod_chain(&mesh, options, &mut buf).unwrap();
        assert_eq!(report.levels.len(), 1);
        assert_eq!(report.levels[0].vertices, 8);
        assert_eq!(report.levels[0].triangles, 12);
    }
}
