//! Spatial coordinate frames, Hamilton quaternions (`<= 1e-12`), unit directions,
//! checked `i128` rational clocks/instants, provenance, and calibration specs (`Schemas 1, 2, 4, 5, 202`).

use super::types::{
    validate_identifier_str, ProfileError, BASE_FRAME_CONVENTION, DIRECTION_NORM_TOLERANCE,
    MAX_CLOCK_DEN, MAX_CLOCK_NUM, QUATERNION_NORM_TOLERANCE,
};

// ---------------------------------------------------------------------------
// Active Hamilton quaternion (`[x, y, z, w]`) & unit direction admission
// ---------------------------------------------------------------------------

/// Active right-handed Hamilton unit quaternion `[x, y, z, w]` (`q * p * q*`).
///
/// Norm admission uses the exact `f64` evaluation order specified in `quantities-models.md`:
/// `abs((((x*x + y*y) + z*z) + w*w) - 1.0) <= 1e-12` without silent normalization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuaternionXyzw {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

impl QuaternionXyzw {
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    pub fn from_xyzw(xyzw: [f64; 4]) -> Result<Self, ProfileError> {
        for v in xyzw {
            if !v.is_finite() {
                return Err(ProfileError::InvalidQuaternion);
            }
        }
        let [x, y, z, w] = xyzw;
        let sq = ((x * x + y * y) + z * z) + w * w;
        if (sq - 1.0).abs() > QUATERNION_NORM_TOLERANCE {
            return Err(ProfileError::InvalidQuaternion);
        }
        Ok(Self { x, y, z, w })
    }

    pub fn from_axis_angle(axis: [f64; 3], angle_rad: f64) -> Result<Self, ProfileError> {
        if !angle_rad.is_finite() || axis.iter().any(|v| !v.is_finite()) {
            return Err(ProfileError::InvalidQuaternion);
        }
        let norm_sq = axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2];
        if norm_sq <= 0.0 {
            return Err(ProfileError::InvalidQuaternion);
        }
        let inv = 1.0 / norm_sq.sqrt();
        let half = 0.5 * angle_rad;
        let s = half.sin();
        Self::from_xyzw([axis[0] * inv * s, axis[1] * inv * s, axis[2] * inv * s, half.cos()])
    }

    /// Derived unit quaternion (`q / norm(q)`) for floating-point matrix algebra.
    pub fn normalized_for_derived_use(self) -> [f64; 4] {
        let norm = (((self.x * self.x + self.y * self.y) + self.z * self.z) + self.w * self.w).sqrt();
        [self.x / norm, self.y / norm, self.z / norm, self.w / norm]
    }

    pub fn inverse(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: self.w,
        }
    }

    /// Active 3x3 orthogonal rotation matrix `R` in row-major order.
    pub fn rotation_matrix(self) -> [[f64; 3]; 3] {
        let [x, y, z, w] = self.normalized_for_derived_use();
        let xx = x * x;
        let yy = y * y;
        let zz = z * z;
        let xy = x * y;
        let xz = x * z;
        let yz = y * z;
        let wx = w * x;
        let wy = w * y;
        let wz = w * z;
        [
            [1.0 - 2.0 * (yy + zz), 2.0 * (xy - wz), 2.0 * (xz + wy)],
            [2.0 * (xy + wz), 1.0 - 2.0 * (xx + zz), 2.0 * (yz - wx)],
            [2.0 * (xz - wy), 2.0 * (yz + wx), 1.0 - 2.0 * (xx + yy)],
        ]
    }

    pub fn rotate_vec3(self, v: [f64; 3]) -> [f64; 3] {
        let r = self.rotation_matrix();
        [
            r[0][0] * v[0] + r[0][1] * v[1] + r[0][2] * v[2],
            r[1][0] * v[0] + r[1][1] * v[1] + r[1][2] * v[2],
            r[2][0] * v[0] + r[2][1] * v[1] + r[2][2] * v[2],
        ]
    }

    /// Rotates symmetric tensor `s = [xx, yy, zz, xy, yz, xz]` via `R * S * R^T` in `f64`.
    pub fn rotate_symmetric_tensor6(self, s: [f64; 6]) -> [f64; 6] {
        let r = self.rotation_matrix();
        let mat = [
            [s[0], s[3], s[5]],
            [s[3], s[1], s[4]],
            [s[5], s[4], s[2]],
        ];
        let mut rs = [[0.0f64; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                rs[i][j] = r[i][0] * mat[0][j] + r[i][1] * mat[1][j] + r[i][2] * mat[2][j];
            }
        }
        let mut out = [[0.0f64; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                out[i][j] = rs[i][0] * r[j][0] + rs[i][1] * r[j][1] + rs[i][2] * r[j][2];
            }
        }
        [
            out[0][0],
            out[1][1],
            out[2][2],
            0.5 * (out[0][1] + out[1][0]),
            0.5 * (out[1][2] + out[2][1]),
            0.5 * (out[0][2] + out[2][0]),
        ]
    }
}

/// Validated unit vector `[x, y, z]` (`abs(|d|^2 - 1) <= 1e-5` evaluated in `f64`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitDirection3(pub [f32; 3]);

impl UnitDirection3 {
    pub fn new(v: [f32; 3]) -> Result<Self, ProfileError> {
        for c in v {
            if !c.is_finite() {
                return Err(ProfileError::NonFiniteFloat);
            }
        }
        let x = f64::from(v[0]);
        let y = f64::from(v[1]);
        let z = f64::from(v[2]);
        let sq = (x * x + y * y) + z * z;
        if (sq - 1.0).abs() > DIRECTION_NORM_TOLERANCE {
            return Err(ProfileError::InvalidUnitVector);
        }
        Ok(Self(v))
    }
}

// ---------------------------------------------------------------------------
// Checked rational clock & instant types (`Schema 1` & `Schema 5`)
// ---------------------------------------------------------------------------

#[inline]
pub const fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RationalRate {
    pub num: u64,
    pub den: u64,
}

impl RationalRate {
    pub fn new(num: u64, den: u64) -> Result<Self, ProfileError> {
        if num == 0 || den == 0 {
            return Err(ProfileError::InvalidTimeSpec);
        }
        let g = gcd_u64(num, den);
        let rn = num / g;
        let rd = den / g;
        if rn > MAX_CLOCK_NUM || rd > MAX_CLOCK_DEN {
            return Err(ProfileError::InvalidTimeSpec);
        }
        Ok(Self { num: rn, den: rd })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockSpec {
    pub scale: u32,
    pub epoch_seconds: i64,
    pub epoch_nanoseconds: u32,
    pub ticks_per_second: RationalRate,
}

impl ClockSpec {
    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.scale > 2 || self.epoch_nanoseconds >= 1_000_000_000 {
            return Err(ProfileError::InvalidTimeSpec);
        }
        if self.ticks_per_second.num == 0
            || self.ticks_per_second.den == 0
            || gcd_u64(self.ticks_per_second.num, self.ticks_per_second.den) != 1
            || self.ticks_per_second.num > MAX_CLOCK_NUM
            || self.ticks_per_second.den > MAX_CLOCK_DEN
        {
            return Err(ProfileError::InvalidTimeSpec);
        }
        Ok(())
    }
}

/// Exact instant under a referenced `Schema 1` clock (`Schema 5`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstantSpec {
    pub clock_block_id: u32,
    pub tick: i64,
    pub subtick_num: u64,
    pub subtick_den: u64,
}

impl InstantSpec {
    pub fn new(
        clock_block_id: u32,
        tick: i64,
        subtick_num: u64,
        subtick_den: u64,
    ) -> Result<Self, ProfileError> {
        if clock_block_id == 0
            || subtick_den == 0
            || subtick_num >= subtick_den
            || subtick_den > MAX_CLOCK_DEN
        {
            return Err(ProfileError::InvalidTimeSpec);
        }
        if (subtick_num == 0 && subtick_den != 1)
            || (subtick_num != 0 && gcd_u64(subtick_num, subtick_den) != 1)
        {
            return Err(ProfileError::InvalidTimeSpec);
        }
        Ok(Self {
            clock_block_id,
            tick,
            subtick_num,
            subtick_den,
        })
    }

    /// Converts to exact rational seconds relative to `clock`'s epoch using checked `i128` math.
    pub fn as_exact_tai_or_epoch_seconds(
        &self,
        clock: &ClockSpec,
    ) -> Result<(i128, i128), ProfileError> {
        clock.validate()?;
        let sub_den = i128::from(self.subtick_den);
        let t_num = i128::from(clock.ticks_per_second.num);
        let t_den = i128::from(clock.ticks_per_second.den);
        let total_ticks_num = i128::from(self.tick)
            .checked_mul(sub_den)
            .and_then(|v| v.checked_add(i128::from(self.subtick_num)))
            .ok_or(ProfileError::Overflow)?;
        let rel_num = total_ticks_num.checked_mul(t_den).ok_or(ProfileError::Overflow)?;
        let rel_den = sub_den.checked_mul(t_num).ok_or(ProfileError::Overflow)?;
        let epoch_ns = i128::from(clock.epoch_seconds)
            .checked_mul(1_000_000_000)
            .and_then(|v| v.checked_add(i128::from(clock.epoch_nanoseconds)))
            .ok_or(ProfileError::Overflow)?;
        let final_num = epoch_ns
            .checked_mul(rel_den)
            .and_then(|lhs| lhs.checked_add(rel_num.checked_mul(1_000_000_000)?))
            .ok_or(ProfileError::Overflow)?;
        let final_den = 1_000_000_000i128
            .checked_mul(rel_den)
            .ok_or(ProfileError::Overflow)?;
        Ok((final_num, final_den))
    }

    /// Exact rational comparison (`self` vs `other`) without rounding; clocks must match `scale`.
    pub fn cmp_exact(
        &self,
        self_clock: &ClockSpec,
        other: &InstantSpec,
        other_clock: &ClockSpec,
    ) -> Result<std::cmp::Ordering, ProfileError> {
        if self_clock.scale != other_clock.scale {
            return Err(ProfileError::ClockScaleMismatch);
        }
        let (ln, ld) = self.as_exact_tai_or_epoch_seconds(self_clock)?;
        let (rn, rd) = other.as_exact_tai_or_epoch_seconds(other_clock)?;
        if let (Some(lhs), Some(rhs)) = (ln.checked_mul(rd), rn.checked_mul(ld)) {
            return Ok(lhs.cmp(&rhs));
        }
        let g = gcd_i128(ld, rd);
        let ld_r = ld / g;
        let rd_r = rd / g;
        let lhs = ln.checked_mul(rd_r).ok_or(ProfileError::Overflow)?;
        let rhs = rn.checked_mul(ld_r).ok_or(ProfileError::Overflow)?;
        Ok(lhs.cmp(&rhs))
    }
}

#[inline]
const fn gcd_i128(mut a: i128, mut b: i128) -> i128 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    if a == 0 { 1 } else { a }
}

/// Shared coordinate frame specification (`Schema 2`).
#[derive(Debug, Clone, PartialEq)]
pub struct FrameSpec<'a> {
    pub kind: u32,
    pub parent_block_id: Option<u32>,
    pub origin_m: [f64; 3],
    pub rotation: QuaternionXyzw,
    pub grid_spacing_m: Option<[f64; 3]>,
    pub grid_dimensions: Option<[u32; 3]>,
    pub index_order: Option<u32>,
    pub convention: &'a str,
    pub crs_definition: Option<&'a [u8]>,
}

impl<'a> FrameSpec<'a> {
    pub fn local_default() -> Self {
        Self {
            kind: 0,
            parent_block_id: None,
            origin_m: [0.0, 0.0, 0.0],
            rotation: QuaternionXyzw::IDENTITY,
            grid_spacing_m: None,
            grid_dimensions: None,
            index_order: None,
            convention: BASE_FRAME_CONVENTION,
            crs_definition: None,
        }
    }
}

/// Shared provenance specification (`Schema 4`).
#[derive(Debug, Clone)]
pub struct ProvenanceSpec<'a> {
    pub role: u32,
    pub source_digests: &'a [[u8; 32]],
    pub algorithm: Option<&'a str>,
    pub algorithm_version: Option<&'a str>,
    pub instrument_uri: Option<&'a str>,
    pub authority_receipt_digest: Option<[u8; 32]>,
    pub lineage: Option<&'a str>,
}

/// Shared acoustic calibration (`Schema 202`).
#[derive(Debug, Clone)]
pub struct CalibrationSpec<'a> {
    pub input_quantity: &'a str,
    pub output_units: &'a str,
    pub scale: f64,
    pub offset: f64,
    pub reference_pressure_pa: Option<f64>,
    pub response_entry: Option<u32>,
    pub uncertainty_entry: Option<u32>,
    pub calibration_digest: Option<[u8; 32]>,
    pub valid_from: Option<u32>,
    pub valid_until: Option<u32>,
}

impl<'a> CalibrationSpec<'a> {
    pub fn validate(&self) -> Result<(), ProfileError> {
        validate_identifier_str(self.input_quantity, "calibration.input_quantity")?;
        validate_identifier_str(self.output_units, "calibration.output_units")?;
        if !self.scale.is_finite() || self.scale == 0.0 || !self.offset.is_finite() {
            return Err(ProfileError::InvalidMetadata("calibration.scale_offset"));
        }
        if let Some(p0) = self.reference_pressure_pa {
            if !p0.is_finite() || p0 <= 0.0 {
                return Err(ProfileError::InvalidMetadata("calibration.reference_pressure_pa"));
            }
        }
        Ok(())
    }

    /// Half-open validity interval `[from, until)`. Fails closed if `from >= until` or clock scales mismatch.
    pub fn is_valid_at(
        now: &InstantSpec,
        now_clock: &ClockSpec,
        from: Option<(&InstantSpec, &ClockSpec)>,
        until: Option<(&InstantSpec, &ClockSpec)>,
    ) -> Result<bool, ProfileError> {
        if let (Some((f_inst, f_clk)), Some((u_inst, u_clk))) = (from, until) {
            if f_inst.cmp_exact(f_clk, u_inst, u_clk)? != std::cmp::Ordering::Less {
                return Err(ProfileError::InvalidTimeSpec);
            }
        }
        if let Some((f_inst, f_clk)) = from {
            if now.cmp_exact(now_clock, f_inst, f_clk)? == std::cmp::Ordering::Less {
                return Ok(false);
            }
        }
        if let Some((u_inst, u_clk)) = until {
            if now.cmp_exact(now_clock, u_inst, u_clk)? != std::cmp::Ordering::Less {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
