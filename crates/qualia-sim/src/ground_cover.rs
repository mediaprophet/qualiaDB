//! Multi-Scale Semantic Ground Cover & Viewport LOD Governor (`ground_cover.rs`)
//!
//! Bridges persistent 10D ecological graph state with ephemeral WebGPU viewport realization:
//! - Semantic Ecotope Patches: species mix, LAI, turgor $P_t$, moisture, trample vectors
//! - 4-Tier Continuous LOD Hierarchy:
//!   - LOD 0 (0–8m): Cosserat Cubic Bézier blades
//!   - LOD 1 (8–25m): Stochastic clump ribbons
//!   - LOD 2 (25–80m): Volumetric heightfield shells
//!   - LOD 3 (>80m): SGGX anisotropic microflake BRDF
//! - Camera-centered toroidal clipmap scrolling (zero heap re-allocation)
//! - Megapixel Governor: screen-space solid-angle density clamping ($\le 1.5\times$ viewport pixels)
//!
//! Strictly zero heap in hot loops (`AGENTS.md` §0).

pub const TOROIDAL_GRID_DIM: usize = 32;
pub const TOROIDAL_TOTAL_CELLS: usize = TOROIDAL_GRID_DIM * TOROIDAL_GRID_DIM;

/// 4-tier continuous representation level.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroundCoverLod {
    /// LOD 0: Near-field 0–8m. Explicit Cosserat Bézier blades.
    Lod0BezierBlades = 0,
    /// LOD 1: Mid-near 8–25m. Stochastic clump ribbons.
    Lod1ClumpRibbons = 1,
    /// LOD 2: Mid-far 25–80m. Volumetric texel shells.
    Lod2VolumetricShells = 2,
    /// LOD 3: Horizon >80m. SGGX anisotropic microflake BRDF on terrain.
    Lod3SggxMicroflakes = 3,
}

/// A compact semantic ground-cover ecotope patch.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EcotopePatch {
    /// Species mixture weights: [0] Ryegrass, [1] Clover, [2] Tussock, [3] Herbs
    pub species_mix: [f32; 4],
    /// Mean culm height in meters
    pub culm_height_m: f32,
    /// Leaf Area Index (LAI, typical range 1.0 - 6.0)
    pub leaf_area_index: f32,
    /// Hydraulic turgor pressure in MPa (governs erect vs wilted droop)
    pub turgor_pressure_mpa: f32,
    /// Soil moisture fraction [0.0, 1.0]
    pub soil_moisture: f32,
    /// Trample plastic deformation vector: [direction_x, direction_z, matted_ratio]
    pub trample_deformation: [f32; 3],
}

impl Default for EcotopePatch {
    fn default() -> Self {
        Self {
            species_mix: [0.65, 0.20, 0.10, 0.05],
            culm_height_m: 0.12,
            leaf_area_index: 3.5,
            turgor_pressure_mpa: 1.2,
            soil_moisture: 0.65,
            trample_deformation: [0.0, 0.0, 0.0],
        }
    }
}

/// Zero-allocation toroidal clipmap buffer centered on the moving camera.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ToroidalGroundCoverClipmap {
    pub cells: [EcotopePatch; TOROIDAL_TOTAL_CELLS],
    /// Center world coordinate X
    pub center_world_x: f32,
    /// Center world coordinate Z
    pub center_world_z: f32,
    /// Physical cell size in meters (e.g. 2.0m per patch)
    pub cell_size_m: f32,
}

impl Default for ToroidalGroundCoverClipmap {
    fn default() -> Self {
        Self::new(0.0, 0.0, 2.0)
    }
}

impl ToroidalGroundCoverClipmap {
    pub const fn new(center_x: f32, center_z: f32, cell_size_m: f32) -> Self {
        Self {
            cells: [EcotopePatch {
                species_mix: [0.65, 0.20, 0.10, 0.05],
                culm_height_m: 0.12,
                leaf_area_index: 3.5,
                turgor_pressure_mpa: 1.2,
                soil_moisture: 0.65,
                trample_deformation: [0.0, 0.0, 0.0],
            }; TOROIDAL_TOTAL_CELLS],
            center_world_x: center_x,
            center_world_z: center_z,
            cell_size_m,
        }
    }

    /// Evaluates toroidal index in `0..TOROIDAL_TOTAL_CELLS` using modulo addressing.
    #[inline]
    pub fn cell_index_at(&self, grid_x: i32, grid_z: i32) -> usize {
        let dim = TOROIDAL_GRID_DIM as i32;
        let wrapped_x = grid_x.rem_euclid(dim) as usize;
        let wrapped_z = grid_z.rem_euclid(dim) as usize;
        wrapped_z * TOROIDAL_GRID_DIM + wrapped_x
    }

    /// Translates the toroidal clipmap center without buffer re-allocation.
    pub fn update_camera_position(&mut self, cam_x: f32, cam_z: f32) {
        self.center_world_x = cam_x;
        self.center_world_z = cam_z;
    }

    /// Injects a vehicle wheel or character boot trample deformation into the nearest cell.
    pub fn inject_trample(&mut self, world_x: f32, world_z: f32, dir_x: f32, dir_z: f32, matted_ratio: f32) {
        let gx = (world_x / self.cell_size_m).floor() as i32;
        let gz = (world_z / self.cell_size_m).floor() as i32;
        let idx = self.cell_index_at(gx, gz);

        self.cells[idx].trample_deformation = [
            dir_x,
            dir_z,
            matted_ratio.clamp(0.0, 1.0),
        ];
    }

    /// Maps distance from camera in meters to the optimal continuous LOD level.
    #[inline]
    pub fn evaluate_lod(&self, distance_m: f32) -> GroundCoverLod {
        if distance_m < 8.0 {
            GroundCoverLod::Lod0BezierBlades
        } else if distance_m < 25.0 {
            GroundCoverLod::Lod1ClumpRibbons
        } else if distance_m < 80.0 {
            GroundCoverLod::Lod2VolumetricShells
        } else {
            GroundCoverLod::Lod3SggxMicroflakes
        }
    }

    /// Evaluates SGGX 3D normal orientation vector for distant LOD 3 microflake BRDF.
    ///
    /// When turgor is high, grass stands vertically ($[0, 1, 0]$), producing velvet grazing sheen.
    /// When trampled or wilted, the fiber tangent collapses horizontally.
    pub fn evaluate_sggx_fiber_direction(&self, patch: &EcotopePatch) -> [f32; 3] {
        let matted = patch.trample_deformation[2];
        let turgor = patch.turgor_pressure_mpa;

        if matted > 0.3 {
            // Flattened along trample direction
            let dx = patch.trample_deformation[0];
            let dz = patch.trample_deformation[1];
            let len = (dx * dx + dz * dz).sqrt().max(1e-4);
            [dx / len, 0.05, dz / len]
        } else if turgor < 0.3 {
            // Wilted droop: random horizontal spread
            [0.6, 0.2, 0.4]
        } else {
            // Erect vertical grass fiber
            [0.0, 1.0, 0.0]
        }
    }

    /// Megapixel Governor: Computes allowable blade count for a tile to avoid sub-pixel overdraw.
    ///
    /// Limits spawned vertices across the viewport to $\le 1.5\times$ total viewport pixels.
    pub fn clamp_blade_density(
        &self,
        distance_m: f32,
        viewport_width_px: u32,
        viewport_height_px: u32,
    ) -> u32 {
        let total_pixels = (viewport_width_px * viewport_height_px) as f32;
        let max_budget_per_m2 = (total_pixels * 1.5) / (std::f32::consts::PI * 80.0 * 80.0); // Spread over 80m radius

        // Distance attenuation: inverse square screen footprint
        let dist = distance_m.max(1.0);
        let screen_density_factor = 1.0 / (dist * dist);

        let nominal_blades_per_m2 = 600.0 * screen_density_factor * 16.0;
        let clamped = nominal_blades_per_m2.min(max_budget_per_m2.max(10.0));

        clamped.clamp(4.0, 800.0) as u32
    }
}

/// Deterministic spatial hash generating consistent pseudo-random offsets from world position $(x, z)$.
#[inline]
pub fn spatial_hash_2d(wx: f32, wz: f32) -> u32 {
    let ix = (wx * 100.0).floor() as i32;
    let iz = (wz * 100.0).floor() as i32;
    let mut h = (ix as u32).wrapping_mul(0x45d9_f3b) ^ (iz as u32).wrapping_mul(0x119d_e1f3);
    h = ((h >> 16) ^ h).wrapping_mul(0x45d9_f3b);
    (h >> 16) ^ h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toroidal_clipmap_wrapping() {
        let clipmap = ToroidalGroundCoverClipmap::default();
        let idx0 = clipmap.cell_index_at(0, 0);
        let idx_wrapped = clipmap.cell_index_at(32, 32);
        assert_eq!(idx0, idx_wrapped, "Modulo wrap must return identical cell");
    }

    #[test]
    fn test_sggx_fiber_trample_transition() {
        let clipmap = ToroidalGroundCoverClipmap::default();
        let erect = EcotopePatch::default();
        let mut trampled = EcotopePatch::default();
        trampled.trample_deformation = [1.0, 0.0, 0.8]; // Matted down toward +X

        let fiber_erect = clipmap.evaluate_sggx_fiber_direction(&erect);
        let fiber_trampled = clipmap.evaluate_sggx_fiber_direction(&trampled);

        assert!(fiber_erect[1] > 0.9, "Erect grass fiber must point along +Y");
        assert!(fiber_trampled[1] < 0.1, "Trampled grass fiber must collapse horizontally");
        assert!(fiber_trampled[0] > 0.8, "Trampled fiber must align with trample vector +X");
    }

    #[test]
    fn test_lod_distance_transitions() {
        let clipmap = ToroidalGroundCoverClipmap::default();
        assert_eq!(clipmap.evaluate_lod(4.0), GroundCoverLod::Lod0BezierBlades);
        assert_eq!(clipmap.evaluate_lod(15.0), GroundCoverLod::Lod1ClumpRibbons);
        assert_eq!(clipmap.evaluate_lod(45.0), GroundCoverLod::Lod2VolumetricShells);
        assert_eq!(clipmap.evaluate_lod(120.0), GroundCoverLod::Lod3SggxMicroflakes);
    }
}
