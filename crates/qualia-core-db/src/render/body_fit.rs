//! Apply a person-authored [`AnatomyBodyFit`] to decoded organ vertices.
//!
//! The numbers come from `wellfare_core::anatomy::BodyFit` (same serde shape). This module
//! lives in the renderer so the portal WASM can fit a body without depending on wellfare-core.

use serde::{Deserialize, Serialize};

/// View transform applied in CCF space before the orbit-frame normalise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnatomyBodyFit {
    #[serde(default = "one")]
    pub stature_scale: f32,
    #[serde(default = "one")]
    pub torso_scale_y: f32,
    #[serde(default = "one")]
    pub leg_scale_y: f32,
    #[serde(default = "one")]
    pub arm_span_scale_x: f32,
    #[serde(default = "one")]
    pub shoulder_scale_x: f32,
    #[serde(default = "one")]
    pub chest_radial: f32,
    #[serde(default = "one")]
    pub waist_radial: f32,
    #[serde(default = "one")]
    pub hip_radial: f32,
    #[serde(default)]
    pub pregnancy_abdomen: f32,
    #[serde(default = "pelvis_y")]
    pub pelvis_y_norm: f32,
    #[serde(default = "waist_y")]
    pub waist_y_norm: f32,
    #[serde(default = "chest_y")]
    pub chest_y_norm: f32,
    #[serde(default = "shoulder_y")]
    pub shoulder_y_norm: f32,
    #[serde(default)]
    pub hidden_keys: Vec<String>,
    #[serde(default)]
    pub identity: bool,
}

fn one() -> f32 {
    1.0
}
fn pelvis_y() -> f32 {
    0.42
}
fn waist_y() -> f32 {
    0.52
}
fn chest_y() -> f32 {
    0.68
}
fn shoulder_y() -> f32 {
    0.78
}

impl Default for AnatomyBodyFit {
    fn default() -> Self {
        Self {
            stature_scale: 1.0,
            torso_scale_y: 1.0,
            leg_scale_y: 1.0,
            arm_span_scale_x: 1.0,
            shoulder_scale_x: 1.0,
            chest_radial: 1.0,
            waist_radial: 1.0,
            hip_radial: 1.0,
            pregnancy_abdomen: 0.0,
            pelvis_y_norm: 0.42,
            waist_y_norm: 0.52,
            chest_y_norm: 0.68,
            shoulder_y_norm: 0.78,
            hidden_keys: Vec::new(),
            identity: true,
        }
    }
}

impl AnatomyBodyFit {
    pub fn transform_point(&self, p: [f32; 3], gmin: [f32; 3], gmax: [f32; 3]) -> [f32; 3] {
        let span_y = (gmax[1] - gmin[1]).max(1e-6);
        let mid_x = (gmin[0] + gmax[0]) * 0.5;
        let mid_z = (gmin[2] + gmax[2]) * 0.5;
        let y_norm = ((p[1] - gmin[1]) / span_y).clamp(0.0, 1.0);

        let mut x = p[0] - mid_x;
        let mut y = p[1];
        let mut z = p[2] - mid_z;

        let y_seg = if y_norm < self.pelvis_y_norm {
            self.leg_scale_y
        } else {
            self.torso_scale_y
        };
        y = gmin[1] + (y - gmin[1]) * y_seg;

        let radial = if y_norm < self.pelvis_y_norm {
            lerp(
                1.0,
                self.hip_radial,
                smoothstep(0.20, self.pelvis_y_norm, y_norm),
            )
        } else if y_norm < self.waist_y_norm {
            lerp(
                self.hip_radial,
                self.waist_radial,
                smoothstep(self.pelvis_y_norm, self.waist_y_norm, y_norm),
            )
        } else if y_norm < self.chest_y_norm {
            lerp(
                self.waist_radial,
                self.chest_radial,
                smoothstep(self.waist_y_norm, self.chest_y_norm, y_norm),
            )
        } else {
            lerp(
                self.chest_radial,
                self.shoulder_scale_x,
                smoothstep(self.chest_y_norm, self.shoulder_y_norm, y_norm),
            )
        };
        x *= radial * self.arm_span_scale_x;
        z *= radial;

        if self.pregnancy_abdomen > 0.0 {
            let band = bump(y_norm, 0.48, 0.10);
            z += self.pregnancy_abdomen * band * span_y * 0.18;
            x *= 1.0 + self.pregnancy_abdomen * band * 0.12;
        }

        [
            mid_x + x * self.stature_scale,
            gmin[1] + (y - gmin[1]) * self.stature_scale,
            mid_z + z * self.stature_scale,
        ]
    }

    pub fn apply_in_place(&self, positions: &mut [[f32; 3]], gmin: [f32; 3], gmax: [f32; 3]) {
        if self.identity {
            return;
        }
        for p in positions.iter_mut() {
            *p = self.transform_point(*p, gmin, gmax);
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0).max(1e-6)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn bump(x: f32, center: f32, width: f32) -> f32 {
    let t = ((x - center) / width).abs();
    if t >= 1.0 {
        0.0
    } else {
        1.0 - t * t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_leaves_vertices() {
        let fit = AnatomyBodyFit::default();
        let mut pts = [[0.1, 0.5, 0.0], [0.2, 1.0, 0.1]];
        let orig = pts;
        fit.apply_in_place(&mut pts, [0.0, 0.0, 0.0], [0.4, 1.8, 0.3]);
        assert_eq!(pts, orig);
    }

    #[test]
    fn stature_scale_raises_crown() {
        let fit = AnatomyBodyFit {
            stature_scale: 1.2,
            identity: false,
            ..AnatomyBodyFit::default()
        };
        let p = fit.transform_point([0.0, 1.8, 0.0], [0.0, 0.0, 0.0], [0.4, 1.8, 0.3]);
        assert!((p[1] - 2.16).abs() < 1e-3);
    }
}
