//! Vector Corridor Conformer (Hermite Cubic Spline Roadbed & Creek Bed Excavation).
//!
//! Conforms terrain elevation and subsurface hydrology to linear vector features
//! without cracking or seam artefacts.

use qualia_core_db::q42::m64_matter::M64StrataColumnRecord;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CorridorKind {
    Road { width_mm: u32, crown_height_mm: i16 },
    Creek { width_mm: u32, depth_mm: u16, flow_velocity_xy: [i16; 2] },
}

/// A 2D control point on a corridor centerline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplineControlPoint {
    pub pos_xy_mm: [f64; 2],
    pub elevation_mm: i32,
    pub tangent_xy: [f64; 2],
}

/// Hermite cubic spline evaluation for a corridor segment.
pub struct HermiteCorridorSegment {
    pub p0: SplineControlPoint,
    pub p1: SplineControlPoint,
    pub kind: CorridorKind,
}

impl HermiteCorridorSegment {
    /// Evaluates point along the spline at parameter $t \in [0, 1]$.
    pub fn sample_centerline(&self, t: f64) -> ([f64; 2], f64) {
        let t2 = t * t;
        let t3 = t2 * t;

        // Hermite basis functions
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        let x = h00 * self.p0.pos_xy_mm[0]
            + h10 * self.p0.tangent_xy[0]
            + h01 * self.p1.pos_xy_mm[0]
            + h11 * self.p1.tangent_xy[0];

        let y = h00 * self.p0.pos_xy_mm[1]
            + h10 * self.p0.tangent_xy[1]
            + h01 * self.p1.pos_xy_mm[1]
            + h11 * self.p1.tangent_xy[1];

        let elev = h00 * (self.p0.elevation_mm as f64) + h01 * (self.p1.elevation_mm as f64);

        ([x, y], elev)
    }

    /// Evaluates shortest distance from a terrain point $(px, py)$ to the segment.
    pub fn distance_to_point(&self, px: f64, py: f64) -> (f64, f64) {
        // Discrete search over spline to find closest point and parameter t
        let mut min_dist = f64::MAX;
        let mut best_t = 0.0;
        for step in 0..=20 {
            let t = (step as f64) / 20.0;
            let (pt, _) = self.sample_centerline(t);
            let dx = pt[0] - px;
            let dy = pt[1] - py;
            let d = (dx * dx + dy * dy).sqrt();
            if d < min_dist {
                min_dist = d;
                best_t = t;
            }
        }
        (min_dist, best_t)
    }

    /// Conforms a strata column record to this corridor.
    pub fn conform_strata_column(
        &self,
        col_x_mm: f64,
        col_y_mm: f64,
        record: &mut M64StrataColumnRecord,
    ) -> bool {
        let (dist_mm, t) = self.distance_to_point(col_x_mm, col_y_mm);
        let (_, target_elev) = self.sample_centerline(t);

        match self.kind {
            CorridorKind::Road { width_mm, crown_height_mm } => {
                let half_width = (width_mm as f64) * 0.5;
                if dist_mm <= half_width {
                    // Parabolic crown cross-section
                    let rel_dist = dist_mm / half_width;
                    let crown = (1.0 - rel_dist * rel_dist) * (crown_height_mm as f64);
                    record.surface_elevation_mm = (target_elev + crown).round() as i32;

                    // Compaction increases under roadbed (Layer 0)
                    record.layer_compaction_pct[0] = 240;
                    record.update_crc();
                    return true;
                }
            }
            CorridorKind::Creek { width_mm, depth_mm, flow_velocity_xy } => {
                let half_width = (width_mm as f64) * 0.5;
                if dist_mm <= half_width {
                    // Parabolic riverbed excavation
                    let rel_dist = dist_mm / half_width;
                    let bed_depth = (1.0 - rel_dist * rel_dist) * (depth_mm as f64);
                    record.surface_elevation_mm = (target_elev - bed_depth).round() as i32;

                    // Flowing surface fluid
                    record.surface_fluid_depth_mm = (bed_depth * 0.8) as u16;
                    record.fluid_velocity_xy = flow_velocity_xy;
                    record.layer_moisture_pct[0] = 255; // 100% saturation
                    record.update_crc();
                    return true;
                }
            }
        }
        false
    }
}
