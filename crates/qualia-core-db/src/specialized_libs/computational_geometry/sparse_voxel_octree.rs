//! P21.3 — Sparse voxel octree (SVO) occupancy structure + deterministic
//! DDA ray traversal for biome-scale canopy/light queries.
//!
//! The octree stores occupancy only at leaf nodes; interior children are
//! created on insertion. Ray traversal is an exact 3-D DDA over the leaf
//! grid (2^depth cells/axis), mapping each crossed cell to its octree leaf —
//! no recursion, canonical traversal order, bit-exact across runs.

use super::geometry_workspace::DEFAULT_COLD_GEOMETRY_BUDGET;
use super::primitives::Point3;

/// Empty child slot in [`SvoNode::children`].
pub const SVO_EMPTY: u32 = u32::MAX;
/// Maximum admitted node arena for one cold SVO build (16 GiB).
pub const MAX_SVO_BUDGET_BYTES: u64 = 16 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SvoError {
    InvalidDepth,
    InvalidBounds,
    InvalidBudget,
    OutOfNodes { required: usize },
    OutOfLeaves { required: usize },
}

/// One octree node: `children[i]` indexes `nodes` (interior) or
/// `SVO_EMPTY`; `occupied` marks a leaf containing geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SvoNode {
    pub children: [u32; 8],
    pub occupied: bool,
}

impl Default for SvoNode {
    fn default() -> Self {
        Self {
            children: [SVO_EMPTY; 8],
            occupied: false,
        }
    }
}

/// A sparse voxel octree over `origin + size³` world units, leaf level
/// `depth` (`2^depth` cells per axis).
#[derive(Debug, Clone)]
pub struct Svo {
    pub origin: Point3,
    pub size: f64,
    pub depth: u8,
    pub nodes: Vec<SvoNode>,
    node_budget_bytes: usize,
}

/// Ray hit record from [`Svo::raycast`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SvoRayHit {
    pub t: f64,
    pub leaf: [u32; 3],
}

#[inline]
fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

impl Svo {
    /// Create an empty SVO. `nodes` starts with the root only.
    pub fn new(origin: Point3, size: f64, depth: u8) -> Result<Self, SvoError> {
        Self::new_with_budget(origin, size, depth, DEFAULT_COLD_GEOMETRY_BUDGET)
    }

    /// Create with a caller-selected byte budget for the node arena.
    pub fn new_with_budget(
        origin: Point3,
        size: f64,
        depth: u8,
        node_budget_bytes: usize,
    ) -> Result<Self, SvoError> {
        if node_budget_bytes < core::mem::size_of::<SvoNode>()
            || (node_budget_bytes as u64) > MAX_SVO_BUDGET_BYTES
        {
            return Err(SvoError::InvalidBudget);
        }
        if depth == 0
            || depth > 10
            || !(size > 0.0 && size.is_finite())
            || ![origin.x, origin.y, origin.z].iter().all(|x| x.is_finite())
            || ![origin.x + size, origin.y + size, origin.z + size]
                .iter()
                .all(|x| x.is_finite())
        {
            return Err(SvoError::InvalidDepth);
        }
        Ok(Self {
            origin,
            size,
            depth,
            nodes: vec![SvoNode::default()],
            node_budget_bytes,
        })
    }

    /// Leaf grid resolution (`2^depth` cells per axis).
    #[inline]
    pub fn leaf_grid_dim(&self) -> usize {
        1usize << self.depth
    }

    /// Octant index `(x?1:0)|(y?2:0)|(z?4:0)` of `p` inside a node box.
    #[inline]
    fn octant(&self, p: Point3, node_min: Point3, node_size: f64) -> usize {
        let mid = node_size * 0.5;
        let c = sub(p, node_min);
        (if c.x >= mid { 1 } else { 0 })
            | (if c.y >= mid { 2 } else { 0 })
            | (if c.z >= mid { 4 } else { 0 })
    }

    /// Mark the leaf voxel containing `p` occupied. Points outside the root
    /// volume are ignored.
    pub fn insert_point(&mut self, p: Point3) -> Result<(), SvoError> {
        if ![p.x, p.y, p.z].iter().all(|x| x.is_finite()) {
            return Err(SvoError::InvalidBounds);
        }
        let c = sub(p, self.origin);
        if c.x < 0.0
            || c.y < 0.0
            || c.z < 0.0
            || c.x >= self.size
            || c.y >= self.size
            || c.z >= self.size
        {
            return Ok(());
        }
        let mut node_idx = 0u32;
        let mut node_min = self.origin;
        let mut node_size = self.size;
        for _ in 0..self.depth {
            let oct = self.octant(p, node_min, node_size);
            node_size *= 0.5;
            let off = Point3::new(
                if oct & 1 != 0 { node_size } else { 0.0 },
                if oct & 2 != 0 { node_size } else { 0.0 },
                if oct & 4 != 0 { node_size } else { 0.0 },
            );
            node_min = Point3::new(node_min.x + off.x, node_min.y + off.y, node_min.z + off.z);
            let child = self.nodes[node_idx as usize].children[oct];
            if child == SVO_EMPTY {
                let required = self.nodes.len() + 1;
                let max_nodes = self.node_budget_bytes / core::mem::size_of::<SvoNode>();
                if required > max_nodes || required > u32::MAX as usize {
                    return Err(SvoError::OutOfNodes { required });
                }
                if self.nodes.len() == self.nodes.capacity() {
                    let target = self
                        .nodes
                        .capacity()
                        .saturating_mul(2)
                        .max(8)
                        .min(max_nodes);
                    self.nodes
                        .try_reserve_exact(target - self.nodes.len())
                        .map_err(|_| SvoError::OutOfNodes { required })?;
                }
                self.nodes.push(SvoNode::default());
                let new_idx = (self.nodes.len() - 1) as u32;
                self.nodes[node_idx as usize].children[oct] = new_idx;
                node_idx = new_idx;
            } else {
                node_idx = child;
            }
        }
        self.nodes[node_idx as usize].occupied = true;
        Ok(())
    }

    /// Mark leaves covering each triangle's bounding box (conservative
    /// rasterization — sufficient for canopy occupancy at leaf level).
    pub fn insert_triangle_bbox(
        &mut self,
        a: Point3,
        b: Point3,
        c: Point3,
    ) -> Result<(), SvoError> {
        if [a, b, c]
            .iter()
            .any(|p| ![p.x, p.y, p.z].iter().all(|x| x.is_finite()))
        {
            return Err(SvoError::InvalidBounds);
        }
        let leaf = self.size / (self.leaf_grid_dim() as f64);
        let lo = Point3::new(
            a.x.min(b.x).min(c.x),
            a.y.min(b.y).min(c.y),
            a.z.min(b.z).min(c.z),
        );
        let hi = Point3::new(
            a.x.max(b.x).max(c.x),
            a.y.max(b.y).max(c.y),
            a.z.max(b.z).max(c.z),
        );
        let max = Point3::new(
            self.origin.x + self.size,
            self.origin.y + self.size,
            self.origin.z + self.size,
        );
        if hi.x < self.origin.x
            || hi.y < self.origin.y
            || hi.z < self.origin.z
            || lo.x >= max.x
            || lo.y >= max.y
            || lo.z >= max.z
        {
            return Ok(());
        }
        let dim = self.leaf_grid_dim() as isize;
        let index =
            |x: f64, origin: f64| (((x - origin) / leaf).floor() as isize).clamp(0, dim - 1);
        let (ix0, iy0, iz0) = (
            index(lo.x, self.origin.x),
            index(lo.y, self.origin.y),
            index(lo.z, self.origin.z),
        );
        let (ix1, iy1, iz1) = (
            index(hi.x, self.origin.x),
            index(hi.y, self.origin.y),
            index(hi.z, self.origin.z),
        );
        for iz in iz0..=iz1 {
            for iy in iy0..=iy1 {
                for ix in ix0..=ix1 {
                    self.insert_point(Point3::new(
                        self.origin.x + (ix as f64 + 0.5) * leaf,
                        self.origin.y + (iy as f64 + 0.5) * leaf,
                        self.origin.z + (iz as f64 + 0.5) * leaf,
                    ))?;
                }
            }
        }
        Ok(())
    }

    /// Is the leaf cell `(ix, iy, iz)` occupied?
    pub fn leaf_occupied(&self, ix: u32, iy: u32, iz: u32) -> bool {
        let g = self.leaf_grid_dim() as u32;
        if ix >= g || iy >= g || iz >= g {
            return false;
        }
        let mut node = &self.nodes[0];
        let (mut x, mut y, mut z) = (0u32, 0u32, 0u32);
        for level in 0..self.depth {
            let half = g >> (level + 1);
            let oct = (if ix >= x + half { 1usize } else { 0 })
                | (if iy >= y + half { 2 } else { 0 })
                | (if iz >= z + half { 4 } else { 0 });
            let child = node.children[oct];
            if child == SVO_EMPTY {
                return false;
            }
            if oct & 1 != 0 {
                x += half;
            }
            if oct & 2 != 0 {
                y += half;
            }
            if oct & 4 != 0 {
                z += half;
            }
            node = &self.nodes[child as usize];
        }
        node.occupied
    }

    /// Deterministic DDA ray traversal over the leaf grid. Returns the first
    /// occupied leaf hit `(t, leaf_index)` or `None`. The ray must be
    /// normalized; `t_max` bounds the march.
    pub fn raycast(&self, origin: Point3, dir: Point3, t_max: f64) -> Option<SvoRayHit> {
        let leaf = self.size / (self.leaf_grid_dim() as f64);
        // Ray vs root AABB (slab method).
        let (mut t_lo, mut t_hi) = (0.0f64, t_max.min(f64::MAX / 2.0));
        for (o, d, mn, mx) in [
            (origin.x, dir.x, self.origin.x, self.origin.x + self.size),
            (origin.y, dir.y, self.origin.y, self.origin.y + self.size),
            (origin.z, dir.z, self.origin.z, self.origin.z + self.size),
        ] {
            if d.abs() < 1e-15 {
                if o < mn || o >= mx {
                    return None;
                }
            } else {
                let inv = 1.0 / d;
                let (mut t0, mut t1) = ((mn - o) * inv, (mx - o) * inv);
                if t0 > t1 {
                    core::mem::swap(&mut t0, &mut t1);
                }
                t_lo = t_lo.max(t0);
                t_hi = t_hi.min(t1);
            }
        }
        if t_lo >= t_hi || t_hi <= 0.0 {
            return None;
        }
        let mut t = t_lo.max(0.0);
        let g = self.leaf_grid_dim() as i64;
        let step = |d: f64| if d > 0.0 { 1i64 } else { -1 };
        let (sx, sy, sz) = (step(dir.x), step(dir.y), step(dir.z));
        let cell_of = |p: f64, o: f64| ((p - o) / leaf).floor() as i64;
        let entry = Point3::new(
            origin.x + dir.x * t,
            origin.y + dir.y * t,
            origin.z + dir.z * t,
        );
        let (mut ix, mut iy, mut iz) = (
            cell_of(entry.x, self.origin.x).clamp(0, g - 1),
            cell_of(entry.y, self.origin.y).clamp(0, g - 1),
            cell_of(entry.z, self.origin.z).clamp(0, g - 1),
        );
        let next_t = |o: f64, d: f64, i: i64, mn: f64, st: i64| -> f64 {
            if d.abs() < 1e-15 {
                f64::INFINITY
            } else {
                let bound = mn + (i + if st > 0 { 1 } else { 0 }) as f64 * leaf;
                (bound - o) / d
            }
        };
        loop {
            if ix < 0 || iy < 0 || iz < 0 || ix >= g || iy >= g || iz >= g || t > t_hi {
                return None;
            }
            if self.leaf_occupied(ix as u32, iy as u32, iz as u32) {
                return Some(SvoRayHit {
                    t,
                    leaf: [ix as u32, iy as u32, iz as u32],
                });
            }
            let tx = next_t(origin.x, dir.x, ix, self.origin.x, sx);
            let ty = next_t(origin.y, dir.y, iy, self.origin.y, sy);
            let tz = next_t(origin.z, dir.z, iz, self.origin.z, sz);
            if tx <= ty && tx <= tz {
                ix += sx;
                t = tx;
            } else if ty <= tz {
                iy += sy;
                t = ty;
            } else {
                iz += sz;
                t = tz;
            }
        }
    }

    /// Build an SVO from triangle vertices (marks each vertex's leaf).
    pub fn from_vertices(vertices: &[Point3], depth: u8, pad: f64) -> Result<Self, SvoError> {
        Self::from_vertices_with_budget(vertices, depth, pad, DEFAULT_COLD_GEOMETRY_BUDGET)
    }

    /// Build from vertices with a caller-selected node-arena budget.
    pub fn from_vertices_with_budget(
        vertices: &[Point3],
        depth: u8,
        pad: f64,
        node_budget_bytes: usize,
    ) -> Result<Self, SvoError> {
        if vertices.is_empty()
            || !pad.is_finite()
            || pad <= 0.0
            || vertices
                .iter()
                .any(|p| ![p.x, p.y, p.z].iter().all(|x| x.is_finite()))
        {
            return Err(SvoError::InvalidBounds);
        }
        let mut lo = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut hi = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for &p in vertices {
            lo.x = lo.x.min(p.x);
            lo.y = lo.y.min(p.y);
            lo.z = lo.z.min(p.z);
            hi.x = hi.x.max(p.x);
            hi.y = hi.y.max(p.y);
            hi.z = hi.z.max(p.z);
        }
        let size = (hi.x - lo.x).max(hi.y - lo.y).max(hi.z - lo.z) + pad;
        let mut svo = Self::new_with_budget(
            Point3::new(lo.x - pad * 0.5, lo.y - pad * 0.5, lo.z - pad * 0.5),
            size,
            depth,
            node_budget_bytes,
        )?;
        for &p in vertices {
            svo.insert_point(p)?;
        }
        Ok(svo)
    }

    /// Number of occupied leaf voxels (used as an LAI/shadow proxy).
    pub fn occupied_leaf_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.occupied).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svo_insert_lookup_and_raycast() {
        let mut svo = Svo::new(Point3::new(0.0, 0.0, 0.0), 8.0, 3).unwrap();
        svo.insert_point(Point3::new(5.0, 5.0, 5.0)).unwrap();
        assert!(svo.leaf_occupied(5, 5, 5));
        assert!(!svo.leaf_occupied(4, 5, 5));
        assert_eq!(svo.occupied_leaf_count(), 1);

        // Ray along +x through the occupied leaf at z=y=5·leaf scale.
        let hit = svo
            .raycast(
                Point3::new(0.5, 5.5, 5.5),
                Point3::new(1.0, 0.0, 0.0),
                100.0,
            )
            .expect("ray must hit the occupied leaf");
        assert_eq!(hit.leaf, [5, 5, 5]);
        assert!((hit.t - 4.5).abs() < 1e-9, "t={}", hit.t);

        // Miss: ray along a different row.
        assert!(svo
            .raycast(
                Point3::new(0.5, 0.5, 0.5),
                Point3::new(1.0, 0.0, 0.0),
                100.0
            )
            .is_none());
    }

    #[test]
    fn svo_from_vertices_covers_points() {
        let pts = [Point3::new(0.1, 0.1, 0.1), Point3::new(3.9, 3.9, 3.9)];
        let svo = Svo::from_vertices(&pts, 4, 0.2).unwrap();
        assert!(svo.occupied_leaf_count() >= 2);
    }

    #[test]
    fn non_finite_input_is_rejected() {
        let mut svo = Svo::new(Point3::default(), 1.0, 3).unwrap();
        assert_eq!(
            svo.insert_point(Point3::new(f64::NAN, 0.0, 0.0)),
            Err(SvoError::InvalidBounds)
        );
        assert_eq!(svo.nodes.len(), 1);
        assert!(matches!(
            Svo::from_vertices(&[Point3::new(f64::NAN, 0.0, 0.0)], 3, 0.1),
            Err(SvoError::InvalidBounds)
        ));
    }

    #[test]
    fn explicit_node_budget_fails_closed() {
        let mut svo = Svo::new_with_budget(
            Point3::default(),
            1.0,
            3,
            2 * core::mem::size_of::<SvoNode>(),
        )
        .unwrap();
        assert!(matches!(
            svo.insert_point(Point3::new(0.2, 0.2, 0.2)),
            Err(SvoError::OutOfNodes { .. })
        ));
        assert!(svo.nodes.len() <= 2);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn high_detail_budget_is_admitted_without_up_front_reservation() {
        let svo = Svo::new_with_budget(Point3::default(), 1.0, 3, 3 * 1024 * 1024 * 1024).unwrap();
        assert_eq!(svo.nodes.len(), 1);
    }
}
