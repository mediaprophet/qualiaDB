//! P13.4 - Quadtree / octree balanced meshing.
//!
//! Size-field-driven quadtree (2-D) and octree (3-D) decomposition with the
//! **2:1 balance constraint** (no two face-adjacent leaves differ by more than
//! one refinement level) and a **conforming triangulation** of the 2-D leaves
//! that resolves hanging nodes (T-junctions) via per-cell templates.
//!
//! ## Algorithm
//!
//! 1. **Refine.** Starting from a single root cell covering the bounding box,
//!    any leaf whose diameter exceeds the target size at its centre (per a
//!    caller-supplied `should_refine` predicate — typically derived from a
//!    [`SizeField`]) is split into 4 (quad) or 8 (oct) equal children. This
//!    repeats to a fixpoint, bounded by `max_level` and `max_cells`.
//! 2. **Balance (2:1).** Repeatedly, for every leaf and every face-direction,
//!    if the face-neighbour is a leaf more than one level coarser, that
//!    coarser neighbour is subdivided. Iterated to a fixpoint. This is the
//!    standard quadtree/octree 2:1 balance pass; it guarantees that a leaf
//!    edge/face has at most one hanging node (the edge midpoint / face
//!    centroid) on it.
//! 3. **Extract.** The leaves are collected. For 2-D, [`quadtree_to_triangles`]
//!    emits a conforming triangle mesh: a leaf with no hanging nodes on any
//!    edge is split along a diagonal (2 triangles); a leaf with one or more
//!    hanging edge-midpoints is fanned from its centre, with each boundary
//!    segment (corner-to-midpoint or corner-to-corner) forming one triangle
//!    with the centre. Shared corners and midpoints are deduplicated by
//!    quantising to the finest-level grid, so adjacent leaves share vertices
//!    exactly. For 3-D, [`octtree_to_hexahedra`] emits the leaf hexahedra
//!    (with hanging nodes preserved on faces), and [`octtree_to_tetrahedra`]
//!    splits each hex into 6 tetrahedra — conforming when no hanging nodes are
//!    present (uniform mesh); with hanging nodes the hex mesh is the
//!    conforming product and the tet split is documented as non-conforming
//!    across T-junction faces (use the hex mesh + mortar, or a uniform size,
//!    for a fully conforming tet mesh).
//!
//! ## Neighbour finding
//!
//! Face-neighbours are found by the classical location-code walk: from a leaf,
//! walk up to the nearest ancestor that is not on the boundary in the query
//! direction, cross to its sibling (the child index bit on the crossed axis
//! flipped), then descend the mirrored path. This is `O(depth)` and needs only
//! the per-node parent link and child index — no hashing, no global coordinate
//! search. Deterministic: identical input → identical neighbour resolution.
//!
//! ## Determinism
//!
//! The refine and balance fixpoints process leaves in a deterministic order
//! (BFS by node index; balance passes scan leaves in index order). Vertex
//! deduplication uses a `BTreeMap` keyed on quantised grid coordinates, so the
//! output vertex order is deterministic. Identical input → bit-identical
//! output.
//!
//! Tier-2 cold construction: bounded `Vec`/`BTreeMap` scratch during the build;
//! the public output is returned as grown `Vec`s (caller may move them into
//! caller-owned buffers after the build completes).

use super::mesh_quality::SizeField;
use super::primitives::{Point2, Point3};

// ---------------------------------------------------------------------------
//  Shared constants
// ---------------------------------------------------------------------------

/// Sentinel for "no parent" (the root) and "no children" (a leaf).
const NONE: u32 = u32::MAX;

// ---------------------------------------------------------------------------
//  Errors
// ---------------------------------------------------------------------------

/// Quadtree meshing error.
#[derive(Debug, Clone, PartialEq)]
pub enum QuadMeshError {
    /// `max_level` exceeds the supported cap (21 for the quadtree).
    MaxLevelTooLarge { requested: u8, max: u8 },
    /// `root_half` was non-finite or non-positive.
    InvalidRootHalf { got: f64 },
    /// The `max_cells` cap was reached before the refine fixpoint stabilised.
    MaxCellsReached { cells: usize, cap: usize },
    /// A `target_size` query returned a non-finite or non-positive value.
    InvalidTargetSize { at: [f64; 2], got: f64 },
}

impl core::fmt::Display for QuadMeshError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MaxLevelTooLarge { requested, max } => write!(
                f,
                "quadtree: max_level {requested} exceeds cap {max}"
            ),
            Self::InvalidRootHalf { got } => {
                write!(f, "quadtree: root_half must be finite > 0, got {got}")
            }
            Self::MaxCellsReached { cells, cap } => write!(
                f,
                "quadtree: max_cells cap reached ({cells} cells, cap {cap})"
            ),
            Self::InvalidTargetSize { at, got } => write!(
                f,
                "quadtree: target_size at ({}, {}) returned {got} (must be finite > 0)",
                at[0], at[1]
            ),
        }
    }
}

impl std::error::Error for QuadMeshError {}

/// Octree meshing error.
#[derive(Debug, Clone, PartialEq)]
pub enum OctMeshError {
    /// `max_level` exceeds the supported cap (13 for the octree).
    MaxLevelTooLarge { requested: u8, max: u8 },
    /// `root_half` was non-finite or non-positive.
    InvalidRootHalf { got: f64 },
    /// The `max_cells` cap was reached before the refine fixpoint stabilised.
    MaxCellsReached { cells: usize, cap: usize },
    /// A `target_size` query returned a non-finite or non-positive value.
    InvalidTargetSize { at: [f64; 3], got: f64 },
}

impl core::fmt::Display for OctMeshError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MaxLevelTooLarge { requested, max } => write!(
                f,
                "octree: max_level {requested} exceeds cap {max}"
            ),
            Self::InvalidRootHalf { got } => {
                write!(f, "octree: root_half must be finite > 0, got {got}")
            }
            Self::MaxCellsReached { cells, cap } => write!(
                f,
                "octree: max_cells cap reached ({cells} cells, cap {cap})"
            ),
            Self::InvalidTargetSize { at, got } => write!(
                f,
                "octree: target_size at ({}, {}, {}) returned {got} (must be finite > 0)",
                at[0], at[1], at[2]
            ),
        }
    }
}

impl std::error::Error for OctMeshError {}

// ===========================================================================
//  Quadtree (2-D)
// ===========================================================================

/// Maximum supported quadtree depth (2 bits/level → 42 bits in a u64 loc code,
/// leaving headroom; we cap at 21 for safety).
pub const QUAD_MAX_LEVEL: u8 = 21;

/// A single quadtree node. Stored in a flat `Vec`; children of an internal
/// node are contiguous at `first_child + 0..4`.
#[derive(Debug, Clone, Copy)]
pub struct QuadNode {
    /// Cell centre x.
    pub cx: f64,
    /// Cell centre y.
    pub cy: f64,
    /// Half edge length (cell spans `[cx - half, cx + half]`).
    pub half: f64,
    /// Refinement level (root = 0).
    pub level: u8,
    /// Child index within parent (0 = SW, 1 = SE, 2 = NW, 3 = NE); 0 for root.
    pub child_idx: u8,
    /// Parent node index, or `u32::MAX` for the root.
    pub parent: u32,
    /// First child index, or `u32::MAX` if this is a leaf.
    pub first_child: u32,
}

impl QuadNode {
    #[inline]
    fn is_leaf(&self) -> bool {
        self.first_child == NONE
    }
}

/// A quadtree leaf snapshot (centre, half, level) — the public face of a leaf
/// for refinement predicates and mesh extraction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadLeaf {
    pub cx: f64,
    pub cy: f64,
    pub half: f64,
    pub level: u8,
}

/// Quadtree meshing options.
#[derive(Debug, Clone, Copy)]
pub struct QuadMeshOptions {
    /// Maximum refinement depth (root = 0). Capped at [`QUAD_MAX_LEVEL`].
    pub max_level: u8,
    /// Hard cap on total node count (leaves + internal). Bounds memory.
    pub max_cells: usize,
    /// If `true`, enforce the 2:1 balance constraint after refining.
    pub balance_2to1: bool,
}

impl Default for QuadMeshOptions {
    fn default() -> Self {
        Self {
            max_level: 10,
            max_cells: 100_000,
            balance_2to1: true,
        }
    }
}

/// A built quadtree.
#[derive(Debug, Clone)]
pub struct QuadTree {
    pub(crate) nodes: Vec<QuadNode>,
    pub(crate) root_center: [f64; 2],
    pub(crate) root_half: f64,
    pub(crate) max_level: u8,
}

impl QuadTree {
    /// Number of nodes (leaves + internal).
    #[inline]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Iterate over the leaves in deterministic (node-index) order.
    pub fn leaves(&self) -> Vec<QuadLeaf> {
        let mut out = Vec::new();
        for n in &self.nodes {
            if n.is_leaf() {
                out.push(QuadLeaf {
                    cx: n.cx,
                    cy: n.cy,
                    half: n.half,
                    level: n.level,
                });
            }
        }
        out
    }

    /// Subdivide leaf `idx` into 4 children. Returns the first child index, or
    /// `None` if the cell cap would be exceeded (caller decides how to handle).
    fn subdivide(&mut self, idx: u32) -> Option<u32> {
        let n = self.nodes[idx as usize];
        debug_assert!(n.is_leaf());
        let h2 = n.half * 0.5;
        let lvl = n.level + 1;
        let base = self.nodes.len() as u32;
        // child_idx: bit0 = x (E), bit1 = y (N). 0=SW,1=SE,2=NW,3=NE.
        let offsets = [
            (-h2, -h2, 0u8),
            (h2, -h2, 1u8),
            (-h2, h2, 2u8),
            (h2, h2, 3u8),
        ];
        for (ox, oy, ci) in offsets {
            self.nodes.push(QuadNode {
                cx: n.cx + ox,
                cy: n.cy + oy,
                half: h2,
                level: lvl,
                child_idx: ci,
                parent: idx,
                first_child: NONE,
            });
        }
        self.nodes[idx as usize].first_child = base;
        Some(base)
    }

    /// Face-neighbour of leaf/internal node `idx` in direction `(axis, sign)`
    /// where `axis` is 0 (x) or 1 (y) and `sign` is +1 or -1.
    ///
    /// Returns the neighbour node index at the same level as `idx` (if the
    /// neighbour region is subdivided at least to `idx`'s level), or a coarser
    /// leaf containing the neighbour region, or `None` if `idx` is on the
    /// domain boundary in that direction.
    ///
    /// The returned node may be internal (neighbour is finer than `idx`) — the
    /// caller decides what to do.
    fn face_neighbor(&self, idx: u32, axis: u8, sign: i8) -> Option<u32> {
        // Walk up collecting child indices until the ancestor that is not on
        // the boundary side in (axis, sign). `path` holds child indices from
        // `idx` up to (but excluding) the crossing ancestor's child. Fixed
        // stack array (depth <= QUAD_MAX_LEVEL) → zero-heap neighbour finding.
        let mut path: [u8; QUAD_MAX_LEVEL as usize] = [0; QUAD_MAX_LEVEL as usize];
        let mut path_len: usize = 0;
        let mut cur = idx;
        loop {
            let p_idx = self.nodes[cur as usize].parent;
            if p_idx == NONE {
                return None; // hit root on the boundary side
            }
            let ci = self.nodes[cur as usize].child_idx;
            let bit_a = (ci >> axis) & 1;
            // On the boundary side iff the child sits on the `sign` extreme of
            // the parent along `axis`: sign=+1 → bit_a==1 (high side); sign=-1
            // → bit_a==0 (low side).
            let on_boundary = if sign > 0 { bit_a == 1 } else { bit_a == 0 };
            if !on_boundary {
                // Crossing ancestor = p_idx. Sibling = flip the axis bit at
                // the crossing level.
                let sib = ci ^ (1 << axis);
                let mut nbr = self.nodes[p_idx as usize].first_child + sib as u32;
                // Descend the mirrored path. Below the crossing the neighbour
                // is on the opposite side of the crossed axis, so each child
                // index along the descent has the axis bit flipped relative to
                // the node's path.
                for i in (0..path_len).rev() {
                    if self.nodes[nbr as usize].is_leaf() {
                        return Some(nbr); // coarser leaf neighbour
                    }
                    let step = path[i] ^ (1 << axis);
                    nbr = self.nodes[nbr as usize].first_child + step as u32;
                }
                return Some(nbr);
            }
            // on_boundary: keep walking up. Bounded by max_level <= QUAD_MAX_LEVEL.
            path[path_len] = ci;
            path_len += 1;
            cur = p_idx;
        }
    }
}

// ---------------------------------------------------------------------------
//  Quadtree build
// ---------------------------------------------------------------------------

/// Build a quadtree by refining the root until `should_refine` returns `false`
/// for every leaf (or `max_level` / `max_cells` is reached), then optionally
/// enforcing the 2:1 balance constraint.
///
/// `should_refine` is called on each leaf and should return `true` if that
/// leaf should be subdivided. Use [`size_target_refiner_2d`] to build a
/// predicate from a target-size function, or supply a custom one for
/// feature-driven refinement (e.g. refine cells intersecting a boundary).
pub fn build_quadtree(
    root_center: [f64; 2],
    root_half: f64,
    should_refine: &dyn Fn(&QuadLeaf) -> bool,
    opts: &QuadMeshOptions,
) -> Result<QuadTree, QuadMeshError> {
    if opts.max_level > QUAD_MAX_LEVEL {
        return Err(QuadMeshError::MaxLevelTooLarge {
            requested: opts.max_level,
            max: QUAD_MAX_LEVEL,
        });
    }
    if !root_half.is_finite() || root_half <= 0.0 {
        return Err(QuadMeshError::InvalidRootHalf { got: root_half });
    }

    let root = QuadNode {
        cx: root_center[0],
        cy: root_center[1],
        half: root_half,
        level: 0,
        child_idx: 0,
        parent: NONE,
        first_child: NONE,
    };
    let mut tree = QuadTree {
        nodes: Vec::with_capacity(64),
        root_center,
        root_half,
        max_level: opts.max_level,
    };
    tree.nodes.push(root);

    // Refine to a fixpoint: BFS over leaves, subdividing any that the
    // predicate selects (and that are below max_level and within max_cells).
    let mut worklist: Vec<u32> = vec![0];
    while let Some(idx) = worklist.pop() {
        let n = tree.nodes[idx as usize];
        if !n.is_leaf() {
            // Push children for traversal.
            for k in 0..4u32 {
                worklist.push(n.first_child + k);
            }
            continue;
        }
        if n.level >= opts.max_level {
            continue;
        }
        let leaf = QuadLeaf {
            cx: n.cx,
            cy: n.cy,
            half: n.half,
            level: n.level,
        };
        if !should_refine(&leaf) {
            continue;
        }
        if tree.nodes.len() + 4 > opts.max_cells {
            return Err(QuadMeshError::MaxCellsReached {
                cells: tree.nodes.len(),
                cap: opts.max_cells,
            });
        }
        let base = tree.subdivide(idx).unwrap();
        for k in 0..4u32 {
            worklist.push(base + k);
        }
    }

    if opts.balance_2to1 {
        balance_quadtree_2to1(&mut tree, opts.max_cells)?;
    }

    Ok(tree)
}

/// Enforce the 2:1 balance constraint on a quadtree in place. Repeatedly
/// subdivides any leaf that has a face-neighbour more than one level coarser,
/// until a full pass makes no change. Bounded by `max_cells`.
pub fn balance_quadtree_2to1(
    tree: &mut QuadTree,
    max_cells: usize,
) -> Result<(), QuadMeshError> {
    loop {
        // Snapshot the current leaf indices to scan in deterministic order.
        let leaves: Vec<u32> = tree
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.is_leaf())
            .map(|(i, _)| i as u32)
            .collect();

        let mut subdivided: Vec<u32> = Vec::new();
        for &leaf in &leaves {
            let lvl = tree.nodes[leaf as usize].level;
            // For each of the 4 face-directions, find the neighbour.
            for (axis, sign) in [(0u8, 1i8), (0, -1), (1, 1), (1, -1)] {
                if let Some(nbr) = tree.face_neighbor(leaf, axis, sign) {
                    let nn = tree.nodes[nbr as usize];
                    if nn.is_leaf() && nn.level + 1 < lvl {
                        // Neighbour is >1 level coarser → subdivide it.
                        subdivided.push(nbr);
                    }
                }
            }
        }

        if subdivided.is_empty() {
            return Ok(());
        }
        // Deduplicate (a coarser leaf may be flagged by several finer leaves).
        subdivded_sort_dedupe(&mut subdivided);

        for nbr in subdivided {
            if !tree.nodes[nbr as usize].is_leaf() {
                continue; // already subdivided this pass
            }
            if tree.nodes[nbr as usize].level >= tree.max_level {
                continue;
            }
            if tree.nodes.len() + 4 > max_cells {
                return Err(QuadMeshError::MaxCellsReached {
                    cells: tree.nodes.len(),
                    cap: max_cells,
                });
            }
            let _ = tree.subdivide(nbr);
        }
        // Loop again: the new children may themselves violate 2:1 with their
        // other neighbours.
    }
}

/// Sort and deduplicate a `Vec<u32>` in place (deterministic).
fn subdivded_sort_dedupe(v: &mut Vec<u32>) {
    v.sort_unstable();
    v.dedup();
}

// ---------------------------------------------------------------------------
//  Quadtree → conforming triangle mesh
// ---------------------------------------------------------------------------

/// Direction (axis, sign) for each of the 4 CCW edges of a quad.
/// Edge 0 = S (dy=-1), 1 = E (dx=+1), 2 = N (dy=+1), 3 = W (dx=-1).
const QUAD_EDGE_DIRS: [(u8, i8); 4] = [(1, -1), (0, 1), (1, 1), (0, -1)];

/// Extract a conforming triangle mesh from a balanced quadtree.
///
/// Returns `(vertices, triangles)`. Leaves with no hanging edge-midpoint are
/// split along the SW-NE diagonal (2 triangles). Leaves with one or more
/// hanging midpoints are fanned from the cell centre (one triangle per
/// boundary segment). Shared corners and midpoints are deduplicated by
/// quantising to the finest-level grid, so adjacent leaves share vertices
/// exactly and the mesh is fully conforming (no T-junctions).
pub fn quadtree_to_triangles(tree: &QuadTree) -> (Vec<Point2>, Vec<[u32; 3]>) {
    let grid = tree.root_half / (1u64 << tree.max_level) as f64;
    let xmin = tree.root_center[0] - tree.root_half;
    let ymin = tree.root_center[1] - tree.root_half;

    let mut index: std::collections::BTreeMap<(i64, i64), u32> = std::collections::BTreeMap::new();
    let mut verts: Vec<Point2> = Vec::new();
    let mut tris: Vec<[u32; 3]> = Vec::new();

    for (ni, n) in tree.nodes.iter().enumerate() {
        if !n.is_leaf() {
            continue;
        }
        let ni = ni as u32;
        // Corners in CCW order: SW, SE, NE, NW.
        let h = n.half;
        let sw = Point2::new(n.cx - h, n.cy - h);
        let se = Point2::new(n.cx + h, n.cy - h);
        let ne = Point2::new(n.cx + h, n.cy + h);
        let nw = Point2::new(n.cx - h, n.cy + h);
        let corners = [sw, se, ne, nw];

        // Determine which edges have a finer neighbour (→ hanging midpoint).
        // A finer neighbour manifests as an internal equal-level node (the
        // neighbour region is subdivided past our level).
        let mut has_mid = [false; 4];
        for e in 0..4 {
            let (axis, sign) = QUAD_EDGE_DIRS[e];
            if let Some(nbr) = tree.face_neighbor(ni, axis, sign) {
                if !tree.nodes[nbr as usize].is_leaf() {
                    has_mid[e] = true;
                }
            }
        }

        let any_mid = has_mid.iter().any(|&b| b);

        if !any_mid {
            // Diagonal split (SW-NE): two CCW triangles.
            let c0 = add_vert_2d(sw, xmin, ymin, grid, &mut index, &mut verts);
            let c1 = add_vert_2d(se, xmin, ymin, grid, &mut index, &mut verts);
            let c2 = add_vert_2d(ne, xmin, ymin, grid, &mut index, &mut verts);
            let c3 = add_vert_2d(nw, xmin, ymin, grid, &mut index, &mut verts);
            tris.push([c0, c1, c2]);
            tris.push([c0, c2, c3]);
        } else {
            // Centre fan. Build the CCW boundary with midpoints inserted.
            let center = Point2::new(n.cx, n.cy);
            let mut boundary: Vec<Point2> = Vec::with_capacity(8);
            for e in 0..4 {
                boundary.push(corners[e]);
                if has_mid[e] {
                    let m = Point2::new(
                        (corners[e].x + corners[(e + 1) % 4].x) * 0.5,
                        (corners[e].y + corners[(e + 1) % 4].y) * 0.5,
                    );
                    boundary.push(m);
                }
            }
            let cc = add_vert_2d(center, xmin, ymin, grid, &mut index, &mut verts);
            let nb = boundary.len();
            let mut bidx = Vec::with_capacity(nb);
            for p in &boundary {
                bidx.push(add_vert_2d(*p, xmin, ymin, grid, &mut index, &mut verts));
            }
            for i in 0..nb {
                let a = bidx[i];
                let b = bidx[(i + 1) % nb];
                // (boundary[i], boundary[i+1], center) is CCW for a CCW boundary.
                tris.push([a, b, cc]);
            }
        }
    }

    (verts, tris)
}

/// Insert a 2-D vertex with grid-quantised dedup, returning its index.
fn add_vert_2d(
    p: Point2,
    xmin: f64,
    ymin: f64,
    grid: f64,
    index: &mut std::collections::BTreeMap<(i64, i64), u32>,
    verts: &mut Vec<Point2>,
) -> u32 {
    let key = quantize_2d(p, xmin, ymin, grid);
    if let Some(&id) = index.get(&key) {
        id
    } else {
        let id = verts.len() as u32;
        verts.push(p);
        index.insert(key, id);
        id
    }
}

/// Quantise a 2-D point to the finest-level grid → exact dedup key.
fn quantize_2d(p: Point2, xmin: f64, ymin: f64, grid: f64) -> (i64, i64) {
    let gx = ((p.x - xmin) / grid).round() as i64;
    let gy = ((p.y - ymin) / grid).round() as i64;
    (gx, gy)
}

// ---------------------------------------------------------------------------
//  Size-field adapters
// ---------------------------------------------------------------------------

/// Build a `should_refine` predicate for [`build_quadtree`] from a target-size
/// function: a leaf is refined iff its diameter `2 * half` exceeds
/// `target_size(centre) * (1 + tolerance)`.
///
/// Takes the target-size function by value so the returned predicate owns it
/// (no borrowed lifetime) — this keeps the predicate coercible to
/// `&dyn Fn(&QuadLeaf) -> bool`.
pub fn size_target_refiner_2d<F: Fn([f64; 2]) -> f64>(
    target_size: F,
    tolerance: f64,
) -> impl Fn(&QuadLeaf) -> bool {
    move |leaf| {
        let diam = 2.0 * leaf.half;
        let target = target_size([leaf.cx, leaf.cy]);
        target.is_finite() && target > 0.0 && diam > target * (1.0 + tolerance)
    }
}

/// Wrap a [`SizeField`] (planar, `z = 0`) as a 2-D target-size function.
///
/// Clones the field into the returned closure so the closure is self-owned
/// (no borrowed lifetime).
pub fn size_field_2d_fn(sf: &SizeField) -> impl Fn([f64; 2]) -> f64 {
    let sf = sf.clone();
    move |p| sf.size_at(Point3::new(p[0], p[1], 0.0)).unwrap_or(1e-12).max(1e-12)
}

// ===========================================================================
//  Octree (3-D)
// ===========================================================================

/// Maximum supported octree depth (3 bits/level → 39 bits at depth 13).
pub const OCT_MAX_LEVEL: u8 = 13;

/// A single octree node. Children of an internal node are contiguous at
/// `first_child + 0..8`. Child index: bit0 = x, bit1 = y, bit2 = z.
#[derive(Debug, Clone, Copy)]
pub struct OctNode {
    pub cx: f64,
    pub cy: f64,
    pub cz: f64,
    pub half: f64,
    pub level: u8,
    pub child_idx: u8,
    pub parent: u32,
    pub first_child: u32,
}

impl OctNode {
    #[inline]
    fn is_leaf(&self) -> bool {
        self.first_child == NONE
    }
}

/// An octree leaf snapshot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OctLeaf {
    pub cx: f64,
    pub cy: f64,
    pub cz: f64,
    pub half: f64,
    pub level: u8,
}

/// Octree meshing options.
#[derive(Debug, Clone, Copy)]
pub struct OctMeshOptions {
    /// Maximum refinement depth (root = 0). Capped at [`OCT_MAX_LEVEL`].
    pub max_level: u8,
    /// Hard cap on total node count.
    pub max_cells: usize,
    /// If `true`, enforce the 2:1 balance constraint after refining.
    pub balance_2to1: bool,
}

impl Default for OctMeshOptions {
    fn default() -> Self {
        Self {
            max_level: 8,
            max_cells: 200_000,
            balance_2to1: true,
        }
    }
}

/// A built octree.
#[derive(Debug, Clone)]
pub struct OctTree {
    pub(crate) nodes: Vec<OctNode>,
    pub(crate) root_center: [f64; 3],
    pub(crate) root_half: f64,
    pub(crate) max_level: u8,
}

impl OctTree {
    #[inline]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn leaves(&self) -> Vec<OctLeaf> {
        let mut out = Vec::new();
        for n in &self.nodes {
            if n.is_leaf() {
                out.push(OctLeaf {
                    cx: n.cx,
                    cy: n.cy,
                    cz: n.cz,
                    half: n.half,
                    level: n.level,
                });
            }
        }
        out
    }

    fn subdivide(&mut self, idx: u32) -> Option<u32> {
        let n = self.nodes[idx as usize];
        debug_assert!(n.is_leaf());
        let h2 = n.half * 0.5;
        let lvl = n.level + 1;
        let base = self.nodes.len() as u32;
        // child_idx: bit0=x, bit1=y, bit2=z.
        for k in 0..8u8 {
            let bx = (k & 1) as i8;
            let by = ((k >> 1) & 1) as i8;
            let bz = ((k >> 2) & 1) as i8;
            let ox = if bx == 0 { -h2 } else { h2 };
            let oy = if by == 0 { -h2 } else { h2 };
            let oz = if bz == 0 { -h2 } else { h2 };
            self.nodes.push(OctNode {
                cx: n.cx + ox,
                cy: n.cy + oy,
                cz: n.cz + oz,
                half: h2,
                level: lvl,
                child_idx: k,
                parent: idx,
                first_child: NONE,
            });
        }
        self.nodes[idx as usize].first_child = base;
        Some(base)
    }

    /// Face-neighbour of `idx` in direction `(axis, sign)` (axis 0/1/2).
    fn face_neighbor(&self, idx: u32, axis: u8, sign: i8) -> Option<u32> {
        let mut path: [u8; OCT_MAX_LEVEL as usize] = [0; OCT_MAX_LEVEL as usize];
        let mut path_len: usize = 0;
        let mut cur = idx;
        loop {
            let p_idx = self.nodes[cur as usize].parent;
            if p_idx == NONE {
                return None;
            }
            let ci = self.nodes[cur as usize].child_idx;
            let bit_a = (ci >> axis) & 1;
            let on_boundary = if sign > 0 { bit_a == 1 } else { bit_a == 0 };
            if !on_boundary {
                let sib = ci ^ (1 << axis);
                let mut nbr = self.nodes[p_idx as usize].first_child + sib as u32;
                for i in (0..path_len).rev() {
                    if self.nodes[nbr as usize].is_leaf() {
                        return Some(nbr);
                    }
                    let step = path[i] ^ (1 << axis);
                    nbr = self.nodes[nbr as usize].first_child + step as u32;
                }
                return Some(nbr);
            }
            path[path_len] = ci;
            path_len += 1;
            cur = p_idx;
        }
    }
}

/// Build an octree by refining the root until `should_refine` returns `false`
/// for every leaf (or `max_level` / `max_cells` is reached), then optionally
/// enforcing 2:1 balance.
pub fn build_octtree(
    root_center: [f64; 3],
    root_half: f64,
    should_refine: &dyn Fn(&OctLeaf) -> bool,
    opts: &OctMeshOptions,
) -> Result<OctTree, OctMeshError> {
    if opts.max_level > OCT_MAX_LEVEL {
        return Err(OctMeshError::MaxLevelTooLarge {
            requested: opts.max_level,
            max: OCT_MAX_LEVEL,
        });
    }
    if !root_half.is_finite() || root_half <= 0.0 {
        return Err(OctMeshError::InvalidRootHalf { got: root_half });
    }

    let root = OctNode {
        cx: root_center[0],
        cy: root_center[1],
        cz: root_center[2],
        half: root_half,
        level: 0,
        child_idx: 0,
        parent: NONE,
        first_child: NONE,
    };
    let mut tree = OctTree {
        nodes: Vec::with_capacity(128),
        root_center,
        root_half,
        max_level: opts.max_level,
    };
    tree.nodes.push(root);

    let mut worklist: Vec<u32> = vec![0];
    while let Some(idx) = worklist.pop() {
        let n = tree.nodes[idx as usize];
        if !n.is_leaf() {
            for k in 0..8u32 {
                worklist.push(n.first_child + k);
            }
            continue;
        }
        if n.level >= opts.max_level {
            continue;
        }
        let leaf = OctLeaf {
            cx: n.cx,
            cy: n.cy,
            cz: n.cz,
            half: n.half,
            level: n.level,
        };
        if !should_refine(&leaf) {
            continue;
        }
        if tree.nodes.len() + 8 > opts.max_cells {
            return Err(OctMeshError::MaxCellsReached {
                cells: tree.nodes.len(),
                cap: opts.max_cells,
            });
        }
        let base = tree.subdivide(idx).unwrap();
        for k in 0..8u32 {
            worklist.push(base + k);
        }
    }

    if opts.balance_2to1 {
        balance_octtree_2to1(&mut tree, opts.max_cells)?;
    }

    Ok(tree)
}

/// Enforce the 2:1 balance constraint on an octree in place.
pub fn balance_octtree_2to1(tree: &mut OctTree, max_cells: usize) -> Result<(), OctMeshError> {
    loop {
        let leaves: Vec<u32> = tree
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.is_leaf())
            .map(|(i, _)| i as u32)
            .collect();

        let mut subdivided: Vec<u32> = Vec::new();
        for &leaf in &leaves {
            let lvl = tree.nodes[leaf as usize].level;
            for (axis, sign) in [
                (0u8, 1i8),
                (0, -1),
                (1, 1),
                (1, -1),
                (2, 1),
                (2, -1),
            ] {
                if let Some(nbr) = tree.face_neighbor(leaf, axis, sign) {
                    let nn = tree.nodes[nbr as usize];
                    if nn.is_leaf() && nn.level + 1 < lvl {
                        subdivided.push(nbr);
                    }
                }
            }
        }

        if subdivided.is_empty() {
            return Ok(());
        }
        subdivded_sort_dedupe(&mut subdivided);
        for nbr in subdivided {
            if !tree.nodes[nbr as usize].is_leaf() {
                continue;
            }
            if tree.nodes[nbr as usize].level >= tree.max_level {
                continue;
            }
            if tree.nodes.len() + 8 > max_cells {
                return Err(OctMeshError::MaxCellsReached {
                    cells: tree.nodes.len(),
                    cap: max_cells,
                });
            }
            let _ = tree.subdivide(nbr);
        }
    }
}

// ---------------------------------------------------------------------------
//  Octree → hexahedra / tetrahedra
// ---------------------------------------------------------------------------

/// Extract the leaf hexahedra from a (balanced) octree.
///
/// Returns `(vertices, hexahedra)` where each hex is 8 vertex indices in a
/// fixed corner order (the 8 octant corners, index = bit0=x | bit1=y |
/// bit2=z, i.e. 0=(---),1=(+--),2=(-+-),3=(++-),4=(--+),5=(+-+),6=(-++),7=(+++)).
/// Shared corners are deduplicated by quantising to the finest-level grid.
pub fn octtree_to_hexahedra(tree: &OctTree) -> (Vec<Point3>, Vec<[u32; 8]>) {
    let leaves = tree.leaves();
    let grid = tree.root_half / (1u64 << tree.max_level) as f64;
    let xmin = tree.root_center[0] - tree.root_half;
    let ymin = tree.root_center[1] - tree.root_half;
    let zmin = tree.root_center[2] - tree.root_half;

    let mut index: std::collections::BTreeMap<(i64, i64, i64), u32> =
        std::collections::BTreeMap::new();
    let mut verts: Vec<Point3> = Vec::new();
    let mut hexes: Vec<[u32; 8]> = Vec::new();

    for leaf in &leaves {
        let h = leaf.half;
        let mut ids = [0u32; 8];
        for k in 0..8u8 {
            let bx = (k & 1) as i8;
            let by = ((k >> 1) & 1) as i8;
            let bz = ((k >> 2) & 1) as i8;
            let px = leaf.cx + if bx == 0 { -h } else { h };
            let py = leaf.cy + if by == 0 { -h } else { h };
            let pz = leaf.cz + if bz == 0 { -h } else { h };
            let p = Point3::new(px, py, pz);
            let key = quantize_3d(p, xmin, ymin, zmin, grid);
            let id = if let Some(&id) = index.get(&key) {
                id
            } else {
                let id = verts.len() as u32;
                verts.push(p);
                index.insert(key, id);
                id
            };
            ids[k as usize] = id;
        }
        hexes.push(ids);
    }

    (verts, hexes)
}

/// Extract a tetrahedral mesh from an octree by splitting each leaf hex into
/// 6 tetrahedra via the standard Freudenthal triangulation of the cube.
///
/// **Conformance caveat:** this is a fully conforming tet mesh **only when no
/// hanging nodes are present** (e.g. a uniform octree, or one refined to a
/// single level). With hanging nodes (a graded, balanced octree), hex faces
/// with T-junctions produce non-conforming tet interfaces — use
/// [`octtree_to_hexahedra`] (the hex mesh with hanging nodes) + mortar
/// methods, or a uniform size, for a fully conforming tet mesh. The 2-D
/// counterpart [`quadtree_to_triangles`] handles hanging nodes exactly and is
/// always conforming.
pub fn octtree_to_tetrahedra(tree: &OctTree) -> (Vec<Point3>, Vec<[u32; 4]>) {
    let (verts, hexes) = octtree_to_hexahedra(tree);
    let mut tets: Vec<[u32; 4]> = Vec::with_capacity(hexes.len() * 6);

    // Canonical 6-tet Freudenthal triangulation of a cube with corners
    // ordered 0=(---),1=(+--),2=(-+-),3=(++-),4=(--+),5=(+-+),6=(-++),7=(+++).
    // All 6 tets share the body diagonal 0-7 and exactly partition the cube
    // (each tet has volume = cube_volume / 6).
    for h in &hexes {
        let [c0, c1, c2, c3, c4, c5, c6, c7] = *h;
        tets.push([c0, c1, c3, c7]);
        tets.push([c0, c3, c2, c7]);
        tets.push([c0, c2, c6, c7]);
        tets.push([c0, c6, c4, c7]);
        tets.push([c0, c4, c5, c7]);
        tets.push([c0, c5, c1, c7]);
    }

    (verts, tets)
}

/// Quantise a 3-D point to the finest-level grid → exact dedup key.
fn quantize_3d(p: Point3, xmin: f64, ymin: f64, zmin: f64, grid: f64) -> (i64, i64, i64) {
    let gx = ((p.x - xmin) / grid).round() as i64;
    let gy = ((p.y - ymin) / grid).round() as i64;
    let gz = ((p.z - zmin) / grid).round() as i64;
    (gx, gy, gz)
}

/// Build a `should_refine` predicate for [`build_octtree`] from a target-size
/// function: a leaf is refined iff its diameter `2 * half` exceeds
/// `target_size(centre) * (1 + tolerance)`.
///
/// Takes the target-size function by value so the returned predicate owns it.
pub fn size_target_refiner_3d<F: Fn([f64; 3]) -> f64>(
    target_size: F,
    tolerance: f64,
) -> impl Fn(&OctLeaf) -> bool {
    move |leaf| {
        let diam = 2.0 * leaf.half;
        let target = target_size([leaf.cx, leaf.cy, leaf.cz]);
        target.is_finite() && target > 0.0 && diam > target * (1.0 + tolerance)
    }
}

/// A constant target-size function for the octree (useful for uniform meshes
/// and as a fallback, since [`SizeField::Background`] is planar/2-D only).
pub fn const_size_fn_3d(h: f64) -> impl Fn([f64; 3]) -> f64 {
    move |_p| h
}

// ===========================================================================
//  Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::mesh_quality::{
        check_field_conformance_tri, AnisotropyField, MetricTensor, SizeField,
    };

    // ---- helpers ------------------------------------------------------------

    /// Uniform target size `h` everywhere.
    fn uniform_2d(h: f64) -> impl Fn([f64; 2]) -> f64 {
        move |_p| h
    }
    fn uniform_3d(h: f64) -> impl Fn([f64; 3]) -> f64 {
        move |_p| h
    }

    /// Target size that grows with distance from the centre (graded radially).
    fn graded_2d(scale: f64) -> impl Fn([f64; 2]) -> f64 {
        move |p| {
            let r = ((p[0] - 0.5).powi(2) + (p[1] - 0.5).powi(2)).sqrt();
            0.02 + scale * r
        }
    }

    /// Signed area of a 2-D triangle (CCW > 0).
    fn tri_signed_area(v: &[Point2], t: &[u32; 3]) -> f64 {
        let a = v[t[0] as usize];
        let b = v[t[1] as usize];
        let c = v[t[2] as usize];
        0.5 * ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y))
    }

    /// Total area of a triangle mesh (sum of absolute signed areas).
    fn mesh_total_area(v: &[Point2], tris: &[[u32; 3]]) -> f64 {
        tris.iter().map(|t| tri_signed_area(v, t).abs()).sum()
    }

    /// Count inverted (CW) triangles.
    fn count_inverted(v: &[Point2], tris: &[[u32; 3]]) -> usize {
        tris.iter().filter(|t| tri_signed_area(v, t) < 0.0).count()
    }

    /// Brute-force check that a triangle mesh has no T-junctions: every
    /// midpoint of every triangle edge that lies on another triangle's vertex
    /// must itself be a vertex (i.e. no vertex lies in the interior of a
    /// non-incident edge). Approximate (within grid epsilon).
    fn no_t_junctions(v: &[Point2], tris: &[[u32; 3]]) -> bool {
        let eps = 1e-9;
        for vi in 0..v.len() {
            let p = v[vi];
            for t in tris {
                for e in 0..3 {
                    let a = v[t[e] as usize];
                    let b = v[t[(e + 1) % 3] as usize];
                    // Skip edges incident to vi.
                    if t[e] as usize == vi || t[(e + 1) % 3] as usize == vi {
                        continue;
                    }
                    if point_on_segment(p, a, b, eps) {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn point_on_segment(p: Point2, a: Point2, b: Point2, eps: f64) -> bool {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let len2 = dx * dx + dy * dy;
        if len2 < eps {
            return false;
        }
        let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / len2;
        if t < eps || t > 1.0 - eps {
            return false; // not strictly interior
        }
        let cx = a.x + t * dx;
        let cy = a.y + t * dy;
        ((p.x - cx).abs() < eps) && ((p.y - cy).abs() < eps)
    }

    // ---- error paths --------------------------------------------------------

    #[test]
    fn quadtree_rejects_oversized_max_level() {
        let opts = QuadMeshOptions {
            max_level: QUAD_MAX_LEVEL + 1,
            ..Default::default()
        };
        let r = build_quadtree([0.5; 2], 1.0, &|_| false, &opts);
        match r {
            Err(QuadMeshError::MaxLevelTooLarge { requested, max }) => {
                assert_eq!(requested, QUAD_MAX_LEVEL + 1);
                assert_eq!(max, QUAD_MAX_LEVEL);
            }
            other => panic!("expected MaxLevelTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn quadtree_rejects_invalid_root_half() {
        let opts = QuadMeshOptions::default();
        assert!(matches!(
            build_quadtree([0.5; 2], 0.0, &|_| false, &opts),
            Err(QuadMeshError::InvalidRootHalf { .. })
        ));
        assert!(matches!(
            build_quadtree([0.5; 2], f64::NAN, &|_| false, &opts),
            Err(QuadMeshError::InvalidRootHalf { .. })
        ));
        assert!(matches!(
            build_quadtree([0.5; 2], -1.0, &|_| false, &opts),
            Err(QuadMeshError::InvalidRootHalf { .. })
        ));
    }

    #[test]
    fn octree_rejects_oversized_max_level() {
        let opts = OctMeshOptions {
            max_level: OCT_MAX_LEVEL + 1,
            ..Default::default()
        };
        let r = build_octtree([0.5; 3], 1.0, &|_| false, &opts);
        match r {
            Err(OctMeshError::MaxLevelTooLarge { requested, max }) => {
                assert_eq!(requested, OCT_MAX_LEVEL + 1);
                assert_eq!(max, OCT_MAX_LEVEL);
            }
            other => panic!("expected MaxLevelTooLarge, got {other:?}"),
        }
    }

    // ---- uniform refinement -------------------------------------------------

    #[test]
    fn quadtree_uniform_refine_to_target() {
        // Root covers [0,1]^2 (center 0.5, half 0.5). Target h = 0.25 →
        // refine until cell diameter 2*half <= 0.25*(1+tol). With tol=0,
        // diameter <= 0.25 means half <= 0.125 = level 2 (root half 0.5 →
        // 0.25 → 0.125). So all leaves at level 2.
        let target = uniform_2d(0.25);
        let pred = size_target_refiner_2d(target, 0.0);
        let opts = QuadMeshOptions {
            max_level: 6,
            max_cells: 10_000,
            balance_2to1: false,
        };
        let tree = build_quadtree([0.5; 2], 0.5, &pred, &opts).unwrap();
        let leaves = tree.leaves();
        assert!(!leaves.is_empty());
        for l in &leaves {
            assert_eq!(l.level, 2, "all leaves should be level 2, got {:?}", l);
            assert!((2.0 * l.half - 0.25).abs() < 1e-12);
        }
        // 4^2 = 16 uniform leaves.
        assert_eq!(leaves.len(), 16);
    }

    #[test]
    fn quadtree_no_refine_returns_single_leaf() {
        let pred = |_: &QuadLeaf| false;
        let opts = QuadMeshOptions::default();
        let tree = build_quadtree([0.5; 2], 0.5, &pred, &opts).unwrap();
        let leaves = tree.leaves();
        assert_eq!(leaves.len(), 1);
        assert_eq!(leaves[0].level, 0);
    }

    #[test]
    fn quadtree_max_level_caps_refinement() {
        // Absurdly small target would refine forever; max_level caps it.
        let target = uniform_2d(1e-9);
        let pred = size_target_refiner_2d(target, 0.0);
        let opts = QuadMeshOptions {
            max_level: 3,
            max_cells: 1_000_000,
            balance_2to1: false,
        };
        let tree = build_quadtree([0.5; 2], 0.5, &pred, &opts).unwrap();
        for l in tree.leaves() {
            assert!(l.level <= 3);
        }
    }

    // ---- 2:1 balance --------------------------------------------------------

    #[test]
    fn quadtree_balance_enforces_2to1() {
        // Sharp step field: very fine left of x=0.5, coarse right of x=0.5.
        // Without balance this produces a >1 level jump across x=0.5; with
        // balance, none.
        let step = |p: [f64; 2]| if p[0] < 0.5 { 0.02 } else { 0.4 };
        let pred = size_target_refiner_2d(step, 0.0);
        let opts_no_bal = QuadMeshOptions {
            max_level: 6,
            max_cells: 100_000,
            balance_2to1: false,
        };
        let opts_bal = QuadMeshOptions {
            max_level: 6,
            max_cells: 100_000,
            balance_2to1: true,
        };
        let tree_no = build_quadtree([0.5; 2], 0.5, &pred, &opts_no_bal).unwrap();
        let tree_bal = build_quadtree([0.5; 2], 0.5, &pred, &opts_bal).unwrap();

        // The unbalanced tree should have at least one >1 level violation
        // across the x=0.5 step (sanity that the test is meaningful).
        let leaves_no: Vec<u32> = tree_no
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.is_leaf())
            .map(|(i, _)| i as u32)
            .collect();
        let mut has_violation = false;
        for &leaf in &leaves_no {
            let lvl = tree_no.nodes[leaf as usize].level;
            for (axis, sign) in [(0u8, 1i8), (0, -1), (1, 1), (1, -1)] {
                if let Some(nbr) = tree_no.face_neighbor(leaf, axis, sign) {
                    let nn = tree_no.nodes[nbr as usize];
                    if nn.is_leaf() && (lvl as i32 - nn.level as i32).abs() > 1 {
                        has_violation = true;
                    }
                }
            }
        }
        assert!(
            has_violation,
            "step field should produce a 2:1 violation without balancing"
        );

        // Check 2:1 invariant on the balanced tree by inspecting all
        // face-neighbour pairs.
        let leaves_bal: Vec<u32> = tree_bal
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.is_leaf())
            .map(|(i, _)| i as u32)
            .collect();
        for &leaf in &leaves_bal {
            let lvl = tree_bal.nodes[leaf as usize].level;
            for (axis, sign) in [(0u8, 1i8), (0, -1), (1, 1), (1, -1)] {
                if let Some(nbr) = tree_bal.face_neighbor(leaf, axis, sign) {
                    let nn = tree_bal.nodes[nbr as usize];
                    if nn.is_leaf() {
                        assert!(
                            (lvl as i32 - nn.level as i32).abs() <= 1,
                            "2:1 violated: leaf level {lvl} neighbour level {}",
                            nn.level
                        );
                    }
                }
            }
        }

        // Balance must not decrease the leaf count (it only subdivides).
        assert!(tree_bal.leaves().len() >= tree_no.leaves().len());
    }

    #[test]
    fn quadtree_balance_idempotent() {
        let target = graded_2d(0.3);
        let pred = size_target_refiner_2d(target, 0.0);
        let opts = QuadMeshOptions {
            max_level: 6,
            max_cells: 100_000,
            balance_2to1: true,
        };
        let mut tree = build_quadtree([0.5; 2], 0.5, &pred, &opts).unwrap();
        let n_before = tree.leaves().len();
        // Balancing an already-balanced tree should be a no-op.
        balance_quadtree_2to1(&mut tree, opts.max_cells).unwrap();
        assert_eq!(tree.leaves().len(), n_before);
    }

    // ---- triangulation ------------------------------------------------------

    #[test]
    fn quadtree_uniform_mesh_area_and_conformance() {
        let target = uniform_2d(0.25);
        let pred = size_target_refiner_2d(target, 0.0);
        let opts = QuadMeshOptions {
            max_level: 6,
            max_cells: 10_000,
            balance_2to1: true,
        };
        let tree = build_quadtree([0.5; 2], 0.5, &pred, &opts).unwrap();
        let (v, tris) = quadtree_to_triangles(&tree);

        // Domain area = 1.0 (root half 0.5 → side 1.0).
        let area = mesh_total_area(&v, &tris);
        assert!((area - 1.0).abs() < 1e-9, "total area {area} != 1.0");

        // No inverted triangles.
        assert_eq!(count_inverted(&v, &tris), 0, "inverted triangles present");

        // No T-junctions (conforming).
        assert!(no_t_junctions(&v, &tris), "T-junctions present");

        // Uniform 16 leaves × 2 tris = 32 triangles.
        assert_eq!(tris.len(), 32);
    }

    #[test]
    fn quadtree_graded_mesh_is_conforming() {
        let target = graded_2d(0.3);
        let pred = size_target_refiner_2d(target, 0.0);
        let opts = QuadMeshOptions {
            max_level: 6,
            max_cells: 100_000,
            balance_2to1: true,
        };
        let tree = build_quadtree([0.5; 2], 0.5, &pred, &opts).unwrap();
        let (v, tris) = quadtree_to_triangles(&tree);

        // Area must still be exactly 1.0.
        let area = mesh_total_area(&v, &tris);
        assert!((area - 1.0).abs() < 1e-9, "graded total area {area} != 1.0");

        // No inverted triangles.
        assert_eq!(count_inverted(&v, &tris), 0);

        // No T-junctions — the hanging-node templates must resolve them all.
        assert!(no_t_junctions(&v, &tris), "graded mesh has T-junctions");

        // Graded mesh should have more triangles than the uniform one.
        assert!(tris.len() > 32, "graded mesh should have >32 triangles, got {}", tris.len());
    }

    #[test]
    fn quadtree_single_leaf_mesh() {
        let pred = |_: &QuadLeaf| false;
        let opts = QuadMeshOptions::default();
        let tree = build_quadtree([0.5; 2], 0.5, &pred, &opts).unwrap();
        let (v, tris) = quadtree_to_triangles(&tree);
        assert_eq!(v.len(), 4);
        assert_eq!(tris.len(), 2);
        assert!((mesh_total_area(&v, &tris) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn quadtree_mesh_dedups_shared_vertices() {
        // Uniform 16 leaves: interior grid vertices are shared by 4 leaves.
        let target = uniform_2d(0.25);
        let pred = size_target_refiner_2d(target, 0.0);
        let opts = QuadMeshOptions {
            max_level: 6,
            max_cells: 10_000,
            balance_2to1: true,
        };
        let tree = build_quadtree([0.5; 2], 0.5, &pred, &opts).unwrap();
        let (v, tris) = quadtree_to_triangles(&tree);
        // A 4×4 grid of cells has a 5×5 grid of vertices = 25.
        assert_eq!(v.len(), 25, "uniform 4x4 grid should have 25 unique vertices");
        // All triangle indices in range.
        for t in &tris {
            for &i in t {
                assert!((i as usize) < v.len());
            }
        }
    }

    // ---- size-field integration --------------------------------------------

    #[test]
    fn quadtree_consumes_sizefield_constant() {
        let sf = SizeField::Constant { h: 0.25 };
        let target = size_field_2d_fn(&sf);
        let pred = size_target_refiner_2d(target, 0.0);
        let opts = QuadMeshOptions {
            max_level: 6,
            max_cells: 10_000,
            balance_2to1: true,
        };
        let tree = build_quadtree([0.5; 2], 0.5, &pred, &opts).unwrap();
        let leaves = tree.leaves();
        for l in &leaves {
            assert_eq!(l.level, 2);
        }
        assert_eq!(leaves.len(), 16);
    }

    #[test]
    fn quadtree_uniform_mesh_conforms_to_sizefield() {
        // Build a uniform mesh at target h, then verify it conforms to an
        // isotropic anisotropy field with the same target h, within a 1.5x
        // band. Cell edges are exactly h (ratio 1.0); the diagonal-split
        // triangle hypotenuse is sqrt(2)*h (ratio ~1.414 <= 1.5). No edge is
        // shorter than h in a uniform mesh, so min_ratio >= 1.0.
        let h = 0.25;
        let sf = SizeField::Constant { h };
        let target = size_field_2d_fn(&sf);
        let pred = size_target_refiner_2d(target, 0.0);
        let opts = QuadMeshOptions {
            max_level: 6,
            max_cells: 10_000,
            balance_2to1: true,
        };
        let tree = build_quadtree([0.5; 2], 0.5, &pred, &opts).unwrap();
        let (v, tris) = quadtree_to_triangles(&tree);

        let v3: Vec<Point3> = v.iter().map(|p| Point3::new(p.x, p.y, 0.0)).collect();
        let field = AnisotropyField::Uniform {
            metric: MetricTensor::isotropic(h),
        };
        let conf = check_field_conformance_tri(&v3, &tris, &field).unwrap();
        assert!(
            conf.max_ratio <= 1.5,
            "max edge ratio {} > 1.5 (edges too long for target {h})",
            conf.max_ratio
        );
        assert!(
            conf.min_ratio >= 0.99,
            "min edge ratio {} < 1.0 (uniform mesh should have no over-refinement)",
            conf.min_ratio
        );
        assert!(conf.edge_count > 0);
    }

    // ---- determinism --------------------------------------------------------

    #[test]
    fn quadtree_build_is_deterministic() {
        let target = graded_2d(0.3);
        let pred = size_target_refiner_2d(target, 0.0);
        let opts = QuadMeshOptions {
            max_level: 6,
            max_cells: 100_000,
            balance_2to1: true,
        };
        let t1 = build_quadtree([0.5; 2], 0.5, &pred, &opts).unwrap();
        let t2 = build_quadtree([0.5; 2], 0.5, &pred, &opts).unwrap();
        // Same node count, same leaf centres/levels in order.
        assert_eq!(t1.node_count(), t2.node_count());
        let l1 = t1.leaves();
        let l2 = t2.leaves();
        assert_eq!(l1.len(), l2.len());
        for (a, b) in l1.iter().zip(l2.iter()) {
            assert_eq!(a, b);
        }
        let (v1, tr1) = quadtree_to_triangles(&t1);
        let (v2, tr2) = quadtree_to_triangles(&t2);
        assert_eq!(v1, v2);
        assert_eq!(tr1, tr2);
    }

    // ---- octree tests -------------------------------------------------------

    #[test]
    fn octree_uniform_refine_to_target() {
        // Root covers [0,1]^3 (center 0.5, half 0.5). Target h = 0.25 →
        // leaves at level 2, 8^2 = 64 leaves.
        let target = uniform_3d(0.25);
        let pred = size_target_refiner_3d(target, 0.0);
        let opts = OctMeshOptions {
            max_level: 6,
            max_cells: 100_000,
            balance_2to1: false,
        };
        let tree = build_octtree([0.5; 3], 0.5, &pred, &opts).unwrap();
        let leaves = tree.leaves();
        assert_eq!(leaves.len(), 64);
        for l in &leaves {
            assert_eq!(l.level, 2);
        }
    }

    #[test]
    fn octree_no_refine_returns_single_leaf() {
        let pred = |_: &OctLeaf| false;
        let opts = OctMeshOptions::default();
        let tree = build_octtree([0.5; 3], 0.5, &pred, &opts).unwrap();
        assert_eq!(tree.leaves().len(), 1);
    }

    #[test]
    fn octree_balance_enforces_2to1() {
        // Graded 3-D field: fine near centre, coarse at corners.
        let target = |p: [f64; 3]| {
            let r = ((p[0] - 0.5).powi(2) + (p[1] - 0.5).powi(2) + (p[2] - 0.5).powi(2)).sqrt();
            0.02 + 0.3 * r
        };
        let pred = size_target_refiner_3d(target, 0.0);
        let opts = OctMeshOptions {
            max_level: 5,
            max_cells: 200_000,
            balance_2to1: true,
        };
        let tree = build_octtree([0.5; 3], 0.5, &pred, &opts).unwrap();

        let leaves: Vec<u32> = tree
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.is_leaf())
            .map(|(i, _)| i as u32)
            .collect();
        for &leaf in &leaves {
            let lvl = tree.nodes[leaf as usize].level;
            for (axis, sign) in [
                (0u8, 1i8),
                (0, -1),
                (1, 1),
                (1, -1),
                (2, 1),
                (2, -1),
            ] {
                if let Some(nbr) = tree.face_neighbor(leaf, axis, sign) {
                    let nn = tree.nodes[nbr as usize];
                    if nn.is_leaf() {
                        assert!(
                            (lvl as i32 - nn.level as i32).abs() <= 1,
                            "octree 2:1 violated: leaf {lvl} neighbour {}",
                            nn.level
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn octtree_to_hexahedra_uniform() {
        let target = uniform_3d(0.25);
        let pred = size_target_refiner_3d(target, 0.0);
        let opts = OctMeshOptions {
            max_level: 6,
            max_cells: 100_000,
            balance_2to1: true,
        };
        let tree = build_octtree([0.5; 3], 0.5, &pred, &opts).unwrap();
        let (v, hexes) = octtree_to_hexahedra(&tree);
        // 64 leaves → 64 hexes.
        assert_eq!(hexes.len(), 64);
        // Uniform 4×4×4 cell grid → 5×5×5 = 125 unique vertices.
        assert_eq!(v.len(), 125);
        // All indices in range.
        for h in &hexes {
            for &i in h {
                assert!((i as usize) < v.len());
            }
        }
    }

    #[test]
    fn octtree_to_tetrahedra_uniform_count() {
        let target = uniform_3d(0.25);
        let pred = size_target_refiner_3d(target, 0.0);
        let opts = OctMeshOptions {
            max_level: 6,
            max_cells: 100_000,
            balance_2to1: true,
        };
        let tree = build_octtree([0.5; 3], 0.5, &pred, &opts).unwrap();
        let (v, tets) = octtree_to_tetrahedra(&tree);
        // 64 hexes × 6 tets = 384 tets.
        assert_eq!(tets.len(), 384);
        // Same vertex count as the hex mesh.
        assert_eq!(v.len(), 125);
        for t in &tets {
            for &i in t {
                assert!((i as usize) < v.len());
            }
        }
    }

    #[test]
    fn octtree_to_tetrahedra_uniform_volume() {
        // Total tet volume should equal the domain volume (1.0).
        let target = uniform_3d(0.25);
        let pred = size_target_refiner_3d(target, 0.0);
        let opts = OctMeshOptions {
            max_level: 6,
            max_cells: 100_000,
            balance_2to1: true,
        };
        let tree = build_octtree([0.5; 3], 0.5, &pred, &opts).unwrap();
        let (v, tets) = octtree_to_tetrahedra(&tree);
        let total: f64 = tets
            .iter()
            .map(|t| {
                let a = v[t[0] as usize];
                let b = v[t[1] as usize];
                let c = v[t[2] as usize];
                let d = v[t[3] as usize];
                tet_volume(a, b, c, d)
            })
            .sum();
        assert!((total - 1.0).abs() < 1e-9, "total tet volume {total} != 1.0");
    }

    fn tet_volume(a: Point3, b: Point3, c: Point3, d: Point3) -> f64 {
        let ab = [b.x - a.x, b.y - a.y, b.z - a.z];
        let ac = [c.x - a.x, c.y - a.y, c.z - a.z];
        let ad = [d.x - a.x, d.y - a.y, d.z - a.z];
        let cross = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        let det = cross[0] * ad[0] + cross[1] * ad[1] + cross[2] * ad[2];
        det.abs() / 6.0
    }

    #[test]
    fn octree_deterministic() {
        let target = |p: [f64; 3]| {
            let r = ((p[0] - 0.5).powi(2) + (p[1] - 0.5).powi(2) + (p[2] - 0.5).powi(2)).sqrt();
            0.02 + 0.3 * r
        };
        let pred = size_target_refiner_3d(target, 0.0);
        let opts = OctMeshOptions {
            max_level: 5,
            max_cells: 200_000,
            balance_2to1: true,
        };
        let t1 = build_octtree([0.5; 3], 0.5, &pred, &opts).unwrap();
        let t2 = build_octtree([0.5; 3], 0.5, &pred, &opts).unwrap();
        assert_eq!(t1.node_count(), t2.node_count());
        assert_eq!(t1.leaves(), t2.leaves());
    }

    #[test]
    fn const_size_fn_3d_uniform() {
        let f = const_size_fn_3d(0.1);
        assert_eq!(f([0.0; 3]), 0.1);
        assert_eq!(f([1.0, 2.0, 3.0]), 0.1);
    }

    // ---- neighbour-finding edge cases --------------------------------------

    #[test]
    fn quadtree_root_face_neighbor_is_none_on_boundary() {
        // A single root leaf has no neighbours.
        let pred = |_: &QuadLeaf| false;
        let opts = QuadMeshOptions::default();
        let tree = build_quadtree([0.5; 2], 0.5, &pred, &opts).unwrap();
        for (axis, sign) in [(0u8, 1i8), (0, -1), (1, 1), (1, -1)] {
            assert_eq!(tree.face_neighbor(0, axis, sign), None);
        }
    }

    #[test]
    fn quadtree_sibling_neighbors() {
        // One level of refinement: 4 siblings. Each sibling's inward-facing
        // neighbour is the opposite sibling.
        let target = uniform_2d(0.6); // diameter 1.0 > 0.6 → refine root once.
        let pred = size_target_refiner_2d(target, 0.0);
        let opts = QuadMeshOptions {
            max_level: 4,
            max_cells: 1000,
            balance_2to1: false,
        };
        let tree = build_quadtree([0.5; 2], 0.5, &pred, &opts).unwrap();
        // Root (idx 0) is internal; children at 1..4 (SW,SE,NW,NE).
        let sw = 1u32; // child_idx 0
        let se = 2u32; // child_idx 1 (x-bit set)
        // SW's +x neighbour is SE.
        assert_eq!(tree.face_neighbor(sw, 0, 1), Some(se));
        // SE's -x neighbour is SW.
        assert_eq!(tree.face_neighbor(se, 0, -1), Some(sw));
        // SW's +x boundary neighbour beyond the domain: SW +x is internal
        // (sibling SE), so not boundary. SW -x is the domain boundary → None.
        assert_eq!(tree.face_neighbor(sw, 0, -1), None);
    }

    #[test]
    fn quadtree_coarser_neighbor_detected() {
        // Asymmetric refinement: refine only the SW child further, so its
        // neighbour across the +x face is the SE sibling (coarser by 1 level,
        // which is allowed). The SW child's grandchild at the +x boundary
        // should see SE as a coarser leaf.
        let target = |p: [f64; 2]| {
            // Very small near SW corner (0,0), large elsewhere.
            let r = (p[0].powi(2) + p[1].powi(2)).sqrt();
            0.02 + 0.5 * r
        };
        let pred = size_target_refiner_2d(target, 0.0);
        let opts = QuadMeshOptions {
            max_level: 5,
            max_cells: 100_000,
            balance_2to1: false,
        };
        let tree = build_quadtree([0.5; 2], 0.5, &pred, &opts).unwrap();
        // Find a deep leaf in the SW region and check it has a coarser
        // neighbour (the unrefined SE/NW/NE region).
        let sw_leaves: Vec<u32> = tree
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.is_leaf() && n.cx < 0.5 && n.cy < 0.5)
            .map(|(i, _)| i as u32)
            .collect();
        assert!(!sw_leaves.is_empty(), "should have refined leaves in SW");
        // At least one SW leaf should have a coarser +x or +y neighbour.
        let mut found_coarser = false;
        for &leaf in &sw_leaves {
            let lvl = tree.nodes[leaf as usize].level;
            for (axis, sign) in [(0u8, 1i8), (1, 1)] {
                if let Some(nbr) = tree.face_neighbor(leaf, axis, sign) {
                    let nn = tree.nodes[nbr as usize];
                    if nn.is_leaf() && nn.level < lvl {
                        found_coarser = true;
                    }
                }
            }
        }
        assert!(found_coarser, "expected a coarser neighbour of a refined SW leaf");
    }

    #[test]
    fn quadtree_max_cells_cap_errors() {
        // Tiny cap with a refining predicate → MaxCellsReached.
        let target = uniform_2d(1e-6);
        let pred = size_target_refiner_2d(target, 0.0);
        let opts = QuadMeshOptions {
            max_level: 20,
            max_cells: 10, // absurdly small
            balance_2to1: false,
        };
        let r = build_quadtree([0.5; 2], 0.5, &pred, &opts);
        assert!(matches!(r, Err(QuadMeshError::MaxCellsReached { .. })));
    }
}
