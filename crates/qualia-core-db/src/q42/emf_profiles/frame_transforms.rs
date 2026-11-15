//! Rigid and invertible affine frame transforms (Schema 2 Pose and Schema 106).

use super::schemas::{i64_at, TransformKind};
use super::*;

/// Active local-to-parent rigid pose (`rotation_xyzw` unit Hamilton quaternion).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FramePose {
    pub origin_m: [f64; 3],
    pub rotation_xyzw: [f64; 4],
}

impl FramePose {
    pub fn identity() -> Self {
        Self {
            origin_m: [0.0; 3],
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
        }
    }

    pub fn from_block(block: Block<'_>) -> Result<Self, Error> {
        if block.schema_type() != 2 || block.revision() != 1 {
            return Err(Error::Shared(semantics::Error::ReferenceSchema));
        }
        let orig = required(block, 3)?.value();
        let quat = required(block, 4)?.value();
        if orig.len() != 24 || quat.len() != 32 {
            return Err(Error::Frame);
        }
        let pose = Self {
            origin_m: triple(orig, 0),
            rotation_xyzw: [
                f64_at(quat, 0),
                f64_at(quat, 8),
                f64_at(quat, 16),
                f64_at(quat, 24),
            ],
        };
        pose.validated()
    }

    /// Admits raw norm-squared within 1e-12 and normalizes derived quaternion.
    pub fn validated(self) -> Result<Self, Error> {
        if self.origin_m.iter().any(|v| !v.is_finite())
            || self.rotation_xyzw.iter().any(|v| !v.is_finite())
        {
            return Err(Error::NonFinite);
        }
        let [x, y, z, w] = self.rotation_xyzw;
        let norm_sq = ((x * x + y * y) + z * z) + w * w;
        if (norm_sq - 1.0).abs() > 1.0e-12 {
            return Err(Error::Frame);
        }
        let inv = 1.0 / norm_sq.sqrt();
        Ok(Self {
            origin_m: self.origin_m,
            rotation_xyzw: [x * inv, y * inv, z * inv, w * inv],
        })
    }

    /// Active 3x3 rotation matrix `R(q)` mapping local vector components to parent frame.
    pub fn rotation_matrix(self) -> [[f64; 3]; 3] {
        let [x, y, z, w] = self.rotation_xyzw;
        let (xx, yy, zz) = (x * x, y * y, z * z);
        let (xy, xz, yz) = (x * y, x * z, y * z);
        let (wx, wy, wz) = (w * x, w * y, w * z);
        [
            [1.0 - 2.0 * (yy + zz), 2.0 * (xy - wz), 2.0 * (xz + wy)],
            [2.0 * (xy + wz), 1.0 - 2.0 * (xx + zz), 2.0 * (yz - wx)],
            [2.0 * (xz - wy), 2.0 * (yz + wx), 1.0 - 2.0 * (xx + yy)],
        ]
    }

    pub fn transform_point(self, local: [f64; 3]) -> [f64; 3] {
        let v = self.transform_vector(local);
        [
            v[0] + self.origin_m[0],
            v[1] + self.origin_m[1],
            v[2] + self.origin_m[2],
        ]
    }

    pub fn transform_vector(self, local: [f64; 3]) -> [f64; 3] {
        let r = self.rotation_matrix();
        [
            r[0][0] * local[0] + r[0][1] * local[1] + r[0][2] * local[2],
            r[1][0] * local[0] + r[1][1] * local[1] + r[1][2] * local[2],
            r[2][0] * local[0] + r[2][1] * local[1] + r[2][2] * local[2],
        ]
    }

    pub fn inverse_transform_vector(self, parent: [f64; 3]) -> [f64; 3] {
        let r = self.rotation_matrix();
        [
            r[0][0] * parent[0] + r[1][0] * parent[1] + r[2][0] * parent[2],
            r[0][1] * parent[0] + r[1][1] * parent[1] + r[2][1] * parent[2],
            r[0][2] * parent[0] + r[1][2] * parent[1] + r[2][2] * parent[2],
        ]
    }

    /// Rotates Cauchy stress tensor: `sigma_parent = R * sigma_local * R^T`.
    pub fn transform_stress_tensor(self, local: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
        let r = self.rotation_matrix();
        let mut tmp = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                tmp[i][j] = r[i][0] * local[0][j] + r[i][1] * local[1][j] + r[i][2] * local[2][j];
            }
        }
        let mut out = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                out[i][j] = tmp[i][0] * r[j][0] + tmp[i][1] * r[j][1] + tmp[i][2] * r[j][2];
            }
        }
        out
    }

    /// Rotates Voigt `[s_xx, s_yy, s_zz, s_yz, s_xz, s_xy]` stress components.
    pub fn transform_voigt6(self, v: [f64; 6]) -> [f64; 6] {
        let full = [[v[0], v[5], v[4]], [v[5], v[1], v[3]], [v[4], v[3], v[2]]];
        let out = self.transform_stress_tensor(full);
        [
            out[0][0], out[1][1], out[2][2], out[1][2], out[0][2], out[0][1],
        ]
    }
}

#[inline]
pub fn det_3x3(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

pub fn invert_3x3(m: [[f64; 3]; 3]) -> Result<[[f64; 3]; 3], Error> {
    let det = det_3x3(m);
    if !det.is_finite() || det.abs() <= 1.0e-15 {
        return Err(Error::Frame);
    }
    let inv = 1.0 / det;
    Ok([
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv,
        ],
    ])
}

/// Qualified schema 106 4x4 transform supporting proper rigid and invertible affine maps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Schema106Transform {
    pub kind: TransformKind,
    pub matrix: [f64; 16],
    pub validity_ticks: Option<(i64, i64)>,
}

impl Schema106Transform {
    pub fn from_block(block: Block<'_>) -> Result<Self, Error> {
        if block.schema_type() != 106 || block.revision() != 1 {
            return Err(Error::Shared(semantics::Error::ReferenceSchema));
        }
        let kind = TransformKind::parse(text(required(block, 3)?)?)?;
        let raw = required(block, 4)?.value();
        if raw.len() != 128 {
            return Err(Error::Frame);
        }
        let mut matrix = [0.0f64; 16];
        for i in 0..16 {
            let v = f64_at(raw, i * 8);
            if !v.is_finite() {
                return Err(Error::NonFinite);
            }
            matrix[i] = v;
        }
        if matrix[12].abs() > 1.0e-12
            || matrix[13].abs() > 1.0e-12
            || matrix[14].abs() > 1.0e-12
            || (matrix[15] - 1.0).abs() > 1.0e-12
        {
            return Err(Error::Frame);
        }
        let lin = [
            [matrix[0], matrix[1], matrix[2]],
            [matrix[4], matrix[5], matrix[6]],
            [matrix[8], matrix[9], matrix[10]],
        ];
        let det = det_3x3(lin);
        match kind {
            TransformKind::RigidMatrix4 => {
                if (det - 1.0).abs() > 1.0e-9 {
                    return Err(Error::Frame);
                }
                for i in 0..3 {
                    for j in 0..3 {
                        let dot = lin[0][i] * lin[0][j]
                            + lin[1][i] * lin[1][j]
                            + lin[2][i] * lin[2][j];
                        let target = if i == j { 1.0 } else { 0.0 };
                        if (dot - target).abs() > 1.0e-9 {
                            return Err(Error::Frame);
                        }
                    }
                }
            }
            TransformKind::AffineMatrix4 => {
                if det.abs() <= 1.0e-15 {
                    return Err(Error::Frame);
                }
            }
        }
        let validity_ticks = if let Some(vt) = field(block, 6) {
            let from = i64_at(vt.value(), 0);
            let until = i64_at(vt.value(), 8);
            if from >= until {
                return Err(Error::Time);
            }
            Some((from, until))
        } else {
            None
        };
        Ok(Self {
            kind,
            matrix,
            validity_ticks,
        })
    }

    pub fn linear_part(self) -> [[f64; 3]; 3] {
        [
            [self.matrix[0], self.matrix[1], self.matrix[2]],
            [self.matrix[4], self.matrix[5], self.matrix[6]],
            [self.matrix[8], self.matrix[9], self.matrix[10]],
        ]
    }

    pub fn transform_point(self, p: [f64; 3]) -> [f64; 3] {
        [
            self.matrix[0] * p[0] + self.matrix[1] * p[1] + self.matrix[2] * p[2] + self.matrix[3],
            self.matrix[4] * p[0] + self.matrix[5] * p[1] + self.matrix[6] * p[2] + self.matrix[7],
            self.matrix[8] * p[0]
                + self.matrix[9] * p[1]
                + self.matrix[10] * p[2]
                + self.matrix[11],
        ]
    }

    /// Covectors / gradients transform by `(M_lin)^{-T}`.
    pub fn transform_covector(self, w: [f64; 3]) -> Result<[f64; 3], Error> {
        let inv = invert_3x3(self.linear_part())?;
        Ok([
            inv[0][0] * w[0] + inv[1][0] * w[1] + inv[2][0] * w[2],
            inv[0][1] * w[0] + inv[1][1] * w[1] + inv[2][1] * w[2],
            inv[0][2] * w[0] + inv[1][2] * w[1] + inv[2][2] * w[2],
        ])
    }
}
