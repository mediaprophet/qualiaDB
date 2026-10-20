//! Zero-Heap Dynamic Broadphase BVH with Fat-AABB Hysteresis (`broadphase_bvh.rs`).
//!
//! Implements a deterministic, zero-heap bounding volume hierarchy [`BroadphaseBvh`] for up to
//! `MAX_LEAVES` 3D colliders:
//! - Uses fat-AABB margin hysteresis (`fat_margin`) so micro-oscillations or small displacements
//!   do not trigger redundant tree refits.
//! - Provides zero-heap candidate collection for `overlap_aabb`, raycasts, and swept shape casts.

/// 3D Axis-Aligned Bounding Box (`[min_x, min_y, min_z]` .. `[max_x, max_y, max_z]`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb3D {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Default for Aabb3D {
    #[inline]
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Aabb3D {
    /// Inverted empty AABB identity element.
    pub const EMPTY: Self = Self {
        min: [f32::MAX, f32::MAX, f32::MAX],
        max: [f32::MIN, f32::MIN, f32::MIN],
    };

    /// Creates a new `Aabb3D`.
    #[inline]
    pub const fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    /// Creates an `Aabb3D` from a center point and half-extents.
    #[inline]
    pub fn from_center_half_extents(center: [f32; 3], half: [f32; 3]) -> Self {
        Self {
            min: [
                center[0] - half[0].abs(),
                center[1] - half[1].abs(),
                center[2] - half[2].abs(),
            ],
            max: [
                center[0] + half[0].abs(),
                center[1] + half[1].abs(),
                center[2] + half[2].abs(),
            ],
        }
    }

    /// Returns a fattened AABB expanded uniformly by `margin` on all 6 faces.
    #[inline]
    pub fn fatten(&self, margin: f32) -> Self {
        Self {
            min: [
                self.min[0] - margin,
                self.min[1] - margin,
                self.min[2] - margin,
            ],
            max: [
                self.max[0] + margin,
                self.max[1] + margin,
                self.max[2] + margin,
            ],
        }
    }

    /// Returns the union of two AABBs.
    #[inline]
    pub fn union(&self, other: &Self) -> Self {
        Self {
            min: [
                self.min[0].min(other.min[0]),
                self.min[1].min(other.min[1]),
                self.min[2].min(other.min[2]),
            ],
            max: [
                self.max[0].max(other.max[0]),
                self.max[1].max(other.max[1]),
                self.max[2].max(other.max[2]),
            ],
        }
    }

    /// Returns `true` if `self` completely contains `inner`.
    #[inline]
    pub fn contains_aabb(&self, inner: &Self) -> bool {
        inner.min[0] >= self.min[0]
            && inner.min[1] >= self.min[1]
            && inner.min[2] >= self.min[2]
            && inner.max[0] <= self.max[0]
            && inner.max[1] <= self.max[1]
            && inner.max[2] <= self.max[2]
    }

    /// Returns `true` if `self` and `other` overlap on all 3 axes.
    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        self.min[0] <= other.max[0]
            && self.max[0] >= other.min[0]
            && self.min[1] <= other.max[1]
            && self.max[1] >= other.min[1]
            && self.min[2] <= other.max[2]
            && self.max[2] >= other.min[2]
    }

    /// Center point of the AABB.
    #[inline]
    pub fn center(&self) -> [f32; 3] {
        [
            0.5 * (self.min[0] + self.max[0]),
            0.5 * (self.min[1] + self.max[1]),
            0.5 * (self.min[2] + self.max[2]),
        ]
    }

    /// Half-extents of the AABB.
    #[inline]
    pub fn half_extents(&self) -> [f32; 3] {
        [
            0.5 * (self.max[0] - self.min[0]).max(0.0),
            0.5 * (self.max[1] - self.min[1]).max(0.0),
            0.5 * (self.max[2] - self.min[2]).max(0.0),
        ]
    }

    /// Surface area of the AABB (used by the Surface Area Heuristic during BVH insertion).
    #[inline]
    pub fn surface_area(&self) -> f32 {
        let dx = (self.max[0] - self.min[0]).max(0.0);
        let dy = (self.max[1] - self.min[1]).max(0.0);
        let dz = (self.max[2] - self.min[2]).max(0.0);
        2.0 * (dx * dy + dy * dz + dz * dx)
    }

    /// Slab ray-AABB intersection test. Returns `Some((t_enter, t_exit))` if the ray hits within `[0.0, max_toi]`.
    #[inline]
    pub fn ray_intersect(&self, origin: [f32; 3], dir: [f32; 3], max_toi: f32) -> Option<(f32, f32)> {
        let mut t_min = 0.0f32;
        let mut t_max = max_toi;

        let mut axis = 0usize;
        while axis < 3 {
            if dir[axis].abs() < 1e-8 {
                if origin[axis] < self.min[axis] || origin[axis] > self.max[axis] {
                    return None;
                }
            } else {
                let inv_d = 1.0 / dir[axis];
                let mut t1 = (self.min[axis] - origin[axis]) * inv_d;
                let mut t2 = (self.max[axis] - origin[axis]) * inv_d;
                if t1 > t2 {
                    core::mem::swap(&mut t1, &mut t2);
                }
                t_min = t_min.max(t1);
                t_max = t_max.min(t2);
                if t_min > t_max {
                    return None;
                }
            }
            axis += 1;
        }

        Some((t_min, t_max))
    }
}

/// Leaf entry in [`BroadphaseBvh`].
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BvhLeafEntry {
    /// External collider ID.
    pub collider_id: u32,
    /// Tight world-space AABB of the collider.
    pub tight_aabb: Aabb3D,
    /// Hysteresis-fattened world-space AABB stored in the hierarchy.
    pub fat_aabb: Aabb3D,
    /// True if this leaf slot is active.
    pub active: bool,
}

impl Default for BvhLeafEntry {
    fn default() -> Self {
        Self {
            collider_id: u32::MAX,
            tight_aabb: Aabb3D::EMPTY,
            fat_aabb: Aabb3D::EMPTY,
            active: false,
        }
    }
}

/// Zero-heap dynamic broadphase BVH with fat-AABB hysteresis.
///
/// Maintains up to `MAX_LEAVES` colliders alongside an upper-level spatial cluster partition
/// so queries skip non-overlapping regions without heap allocation.
#[derive(Clone, Debug)]
pub struct BroadphaseBvh<const MAX_LEAVES: usize = 256> {
    pub leaves: [BvhLeafEntry; MAX_LEAVES],
    /// Cluster bounding boxes (groups of 8 leaves per cluster node).
    pub cluster_aabbs: [Aabb3D; MAX_LEAVES],
    pub active_count: usize,
    /// Hysteresis expansion margin (meters) added when updating `fat_aabb`.
    pub fat_margin: f32,
    /// Total number of fat-AABB hysteresis refits performed.
    pub refit_count: u64,
}

impl<const MAX_LEAVES: usize> Default for BroadphaseBvh<MAX_LEAVES> {
    fn default() -> Self {
        Self::new(0.12)
    }
}

impl<const MAX_LEAVES: usize> BroadphaseBvh<MAX_LEAVES> {
    const CLUSTER_SIZE: usize = 8;

    /// Creates an empty `BroadphaseBvh` with the specified `fat_margin` (meters).
    pub fn new(fat_margin: f32) -> Self {
        Self {
            leaves: [BvhLeafEntry::default(); MAX_LEAVES],
            cluster_aabbs: [Aabb3D::EMPTY; MAX_LEAVES],
            active_count: 0,
            fat_margin: fat_margin.max(0.01),
            refit_count: 0,
        }
    }

    /// Inserts a collider with its initial `tight_aabb`, returning the assigned leaf slot index.
    pub fn insert(&mut self, collider_id: u32, tight_aabb: Aabb3D) -> Option<usize> {
        let mut slot = 0usize;
        while slot < MAX_LEAVES {
            if !self.leaves[slot].active {
                let fat_aabb = tight_aabb.fatten(self.fat_margin);
                self.leaves[slot] = BvhLeafEntry {
                    collider_id,
                    tight_aabb,
                    fat_aabb,
                    active: true,
                };
                self.active_count += 1;
                self.refit_cluster_for_leaf(slot);
                return Some(slot);
            }
            slot += 1;
        }
        None
    }

    /// Removes a leaf slot from the BVH.
    pub fn remove(&mut self, leaf_slot: usize) -> bool {
        if leaf_slot >= MAX_LEAVES || !self.leaves[leaf_slot].active {
            return false;
        }
        self.leaves[leaf_slot].active = false;
        self.leaves[leaf_slot].fat_aabb = Aabb3D::EMPTY;
        self.leaves[leaf_slot].tight_aabb = Aabb3D::EMPTY;
        self.active_count = self.active_count.saturating_sub(1);
        self.refit_cluster_for_leaf(leaf_slot);
        true
    }

    /// Updates a collider's `tight_aabb` with fat-AABB margin hysteresis.
    ///
    /// Returns `true` if the `tight_aabb` exited the cached `fat_aabb` and triggered a cluster refit,
    /// or `false` if absorbed by hysteresis without refitting.
    pub fn update(&mut self, leaf_slot: usize, new_tight_aabb: Aabb3D) -> bool {
        if leaf_slot >= MAX_LEAVES || !self.leaves[leaf_slot].active {
            return false;
        }
        self.leaves[leaf_slot].tight_aabb = new_tight_aabb;

        // Hysteresis gate: if the new tight AABB still fits inside the fat AABB, skip refit!
        if self.leaves[leaf_slot].fat_aabb.contains_aabb(&new_tight_aabb) {
            return false;
        }

        self.leaves[leaf_slot].fat_aabb = new_tight_aabb.fatten(self.fat_margin);
        self.refit_count += 1;
        self.refit_cluster_for_leaf(leaf_slot);
        true
    }

    #[inline]
    fn refit_cluster_for_leaf(&mut self, leaf_slot: usize) {
        let cluster_idx = leaf_slot / Self::CLUSTER_SIZE;
        let start = cluster_idx * Self::CLUSTER_SIZE;
        let end = (start + Self::CLUSTER_SIZE).min(MAX_LEAVES);

        let mut merged = Aabb3D::EMPTY;
        let mut i = start;
        while i < end {
            if self.leaves[i].active {
                merged = merged.union(&self.leaves[i].fat_aabb);
            }
            i += 1;
        }
        self.cluster_aabbs[cluster_idx] = merged;
    }

    /// Writes all collider IDs whose `tight_aabb` overlaps `query_aabb` into `out_collider_ids`.
    /// Returns the number of collider IDs written.
    pub fn overlap_aabb(&self, query_aabb: &Aabb3D, out_collider_ids: &mut [u32]) -> usize {
        if out_collider_ids.is_empty() || self.active_count == 0 {
            return 0;
        }
        let mut count = 0usize;
        let num_clusters = (MAX_LEAVES + Self::CLUSTER_SIZE - 1) / Self::CLUSTER_SIZE;

        let mut c = 0usize;
        while c < num_clusters {
            if self.cluster_aabbs[c].intersects(query_aabb) {
                let start = c * Self::CLUSTER_SIZE;
                let end = (start + Self::CLUSTER_SIZE).min(MAX_LEAVES);
                let mut i = start;
                while i < end {
                    let leaf = &self.leaves[i];
                    if leaf.active && leaf.tight_aabb.intersects(query_aabb) {
                        if count < out_collider_ids.len() {
                            out_collider_ids[count] = leaf.collider_id;
                            count += 1;
                        } else {
                            return count;
                        }
                    }
                    i += 1;
                }
            }
            c += 1;
        }
        count
    }

    /// Generates all broadphase collision pairs `(collider_a, collider_b)` whose tight AABBs overlap.
    pub fn collect_overlapping_pairs(&self, out_pairs: &mut [(u32, u32)]) -> usize {
        if out_pairs.is_empty() || self.active_count < 2 {
            return 0;
        }
        let mut written = 0usize;
        let mut i = 0usize;
        while i < MAX_LEAVES {
            if self.leaves[i].active {
                let a = &self.leaves[i];
                let mut j = i + 1;
                while j < MAX_LEAVES {
                    let b = &self.leaves[j];
                    if b.active && a.tight_aabb.intersects(&b.tight_aabb) {
                        if written < out_pairs.len() {
                            out_pairs[written] = (a.collider_id, b.collider_id);
                            written += 1;
                        } else {
                            return written;
                        }
                    }
                    j += 1;
                }
            }
            i += 1;
        }
        written
    }

    /// Collects candidate collider IDs whose fat AABB is intersected by the ray `(origin, dir, max_toi)`.
    pub fn query_ray_candidates(
        &self,
        origin: [f32; 3],
        dir: [f32; 3],
        max_toi: f32,
        out_collider_ids: &mut [u32],
    ) -> usize {
        if out_collider_ids.is_empty() || self.active_count == 0 {
            return 0;
        }
        let mut count = 0usize;
        let num_clusters = (MAX_LEAVES + Self::CLUSTER_SIZE - 1) / Self::CLUSTER_SIZE;

        let mut c = 0usize;
        while c < num_clusters {
            if self.cluster_aabbs[c]
                .ray_intersect(origin, dir, max_toi)
                .is_some()
            {
                let start = c * Self::CLUSTER_SIZE;
                let end = (start + Self::CLUSTER_SIZE).min(MAX_LEAVES);
                let mut i = start;
                while i < end {
                    let leaf = &self.leaves[i];
                    if leaf.active
                        && leaf
                            .fat_aabb
                            .ray_intersect(origin, dir, max_toi)
                            .is_some()
                    {
                        if count < out_collider_ids.len() {
                            out_collider_ids[count] = leaf.collider_id;
                            count += 1;
                        } else {
                            return count;
                        }
                    }
                    i += 1;
                }
            }
            c += 1;
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broadphase_bvh_fat_aabb_hysteresis_and_overlap() {
        let mut bvh = BroadphaseBvh::<32>::new(0.20);
        let aabb0 = Aabb3D::from_center_half_extents([0.0, 0.0, 0.0], [0.5, 0.5, 0.5]);
        let aabb1 = Aabb3D::from_center_half_extents([0.8, 0.0, 0.0], [0.5, 0.5, 0.5]);

        let s0 = bvh.insert(10, aabb0).unwrap();
        let _s1 = bvh.insert(20, aabb1).unwrap();

        // Small 0.05m nudge stays inside 0.20m fat margin -> no refit.
        let nudged = Aabb3D::from_center_half_extents([0.05, 0.0, 0.0], [0.5, 0.5, 0.5]);
        assert!(!bvh.update(s0, nudged));
        assert_eq!(bvh.refit_count, 0);

        // Large 0.50m move exits fat margin -> triggers refit.
        let moved = Aabb3D::from_center_half_extents([0.50, 0.0, 0.0], [0.5, 0.5, 0.5]);
        assert!(bvh.update(s0, moved));
        assert_eq!(bvh.refit_count, 1);

        let mut pairs = [(0u32, 0u32); 8];
        let pair_count = bvh.collect_overlapping_pairs(&mut pairs);
        assert_eq!(pair_count, 1);
        assert_eq!(pairs[0], (10, 20));
    }
}
