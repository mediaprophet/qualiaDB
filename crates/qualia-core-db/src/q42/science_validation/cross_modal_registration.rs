//! Cross-modal EMF v4, QAC v1, and anatomical `.10d`/`.m64` coordinate frame, rational clock,
//! and material/anatomy registration invariant verifiers (C02-C03, C02-C04, C02-C06).
//!
//! All checks operate in independent `f64` and checked `i128` rational arithmetic:
//! - Ordered `f64` quaternion squared-norm admission $n^2 = (((x^2 + y^2) + z^2) + w^2)$ within
//!   $10^{-12}$ without mutating raw authored bytes, plus derived normalized rotation matrix $\mathbf{R}(q)$.
//! - Active Hamilton quaternion composition $q_{\text{parent}} \otimes q_{\text{local}}$ and proper
//!   rotation verification ($\|\mathbf{R}^T\mathbf{R} - \mathbf{I}\|_{\max} \le 10^{-12}$, $|\det(\mathbf{R}) - 1| \le 10^{-12}$).
//! - Exact reduced `i128` rational timestamps across EMF v4, QAC v1, and `.10d`/`.m64` streams,
//!   rejecting cross-origin clock joins and exposing phase uncertainty $\Delta\phi = 2\pi f u_t$.
//! - Material/anatomical binding verification, distinguishing EMF `material0` (unbound unless
//!   explicit vacuum) from QAC `material0` (declared uniform medium).

use crate::q42::science_validation::stokes_mueller::{scaled_hypot3, C02_ROUNDING_FACTOR};

/// Admission tolerance on authored quaternion squared norm $|n^2 - 1| \le 10^{-12}$ and frame orthogonality.
pub const FRAME_ADMISSION_TOLERANCE: f64 = 1.0e-12;

/// Structured rejection reasons for cross-modal coordinate, clock, and anatomy registration.
#[derive(Debug, Clone, PartialEq)]
pub enum RegistrationValidationError {
    /// Quaternion or position component is NaN or infinite.
    NonFiniteCoordinateValue { value: f64 },
    /// Authored quaternion squared norm $n^2 = (((x^2+y^2)+z^2)+w^2)$ deviated from $1$ by more than $10^{-12}$.
    QuaternionNormOutOfTolerance {
        raw_xyzw: [f64; 4],
        norm_sq: f64,
        deviation: f64,
        tolerance: f64,
    },
    /// Matrix is improper ($\det(\mathbf{R}) < 0$, e.g. reflection) or non-orthogonal (sheared/scaled)
    /// without an authorized parity-aware / affine profile.
    ImproperOrNonOrthogonalFrame {
        determinant: f64,
        max_orthogonality_residual: f64,
        tolerance: f64,
    },
    /// Moving frame has nonzero translational or angular velocity without a qualified kinematic profile.
    MovingFrameRejectedWithoutKinematicProfile,
    /// Rational clock rate or tick denominator is zero, negative, or overflowed `i128`.
    InvalidRationalClockParameter,
    /// Attempted to join clocks with different origins or scales without an explicit calibrated mapping.
    IncompatibleClockOrigins {
        clock_a_origin_id: u64,
        clock_b_origin_id: u64,
        clock_a_scale: u32,
        clock_b_scale: u32,
    },
    /// Phase uncertainty $2\pi f u_t$ exceeded the maximum coherent join phase allowance.
    PhaseUncertaintyExceeded {
        phase_uncertainty_rad: f64,
        max_allowed_rad: f64,
    },
    /// Material or anatomy coupling failed due to mismatched container/table digest, epoch, frame, or clock,
    /// or unbound EMF `material0`.
    MaterialOrAnatomyBindingRejected { reason: &'static str },
    /// Anatomical point lies outside the declared `.10d`/`.m64` bounding box, or surface normal is non-unit.
    AnatomicalGeometryInvariantViolation {
        reason: &'static str,
        value: f64,
    },
}

/// Verified derived rotation from an admitted `[x, y, z, w]` Hamilton quaternion (C02-C04).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VerifiedQuaternionRotation {
    /// Unmodified raw authored quaternion bytes `[x, y, z, w]`.
    pub raw_xyzw: [f64; 4],
    /// Ordered squared norm $n^2 = (((x^2 + y^2) + z^2) + w^2)$.
    pub raw_norm_sq: f64,
    /// Derived normalized quaternion $q_i / \sqrt{n^2}$.
    pub derived_unit_xyzw: [f64; 4],
    /// Active local-to-parent $3\times 3$ rotation matrix $\mathbf{R}(q_{\text{derived}})$.
    pub rotation_matrix: [[f64; 3]; 3],
    /// Maximum entrywise residual $\|\mathbf{R}^T\mathbf{R} - \mathbf{I}\|_{\max}$.
    pub orthogonality_residual: f64,
    /// Determinant $\det(\mathbf{R})$.
    pub determinant: f64,
}

/// Exact signed rational time offset in seconds $num / den$ with $den > 0$ and $\gcd(|num|, den) = 1$.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RationalSeconds {
    pub num: i128,
    pub den: i128,
}

impl RationalSeconds {
    /// Construct and reduce $num / den$.
    pub fn new(num: i128, den: i128) -> Result<Self, RegistrationValidationError> {
        if den <= 0 {
            return Err(RegistrationValidationError::InvalidRationalClockParameter);
        }
        if num == 0 {
            return Ok(Self { num: 0, den: 1 });
        }
        let g = gcd_i128(num.unsigned_abs(), den.unsigned_abs()) as i128;
        Ok(Self {
            num: num / g,
            den: den / g,
        })
    }

    /// Checked addition of two rational second values.
    pub fn checked_add(self, rhs: Self) -> Result<Self, RegistrationValidationError> {
        let g = gcd_i128(self.den.unsigned_abs(), rhs.den.unsigned_abs()) as i128;
        let lcm_den = self
            .den
            .checked_div(g)
            .and_then(|d| d.checked_mul(rhs.den))
            .ok_or(RegistrationValidationError::InvalidRationalClockParameter)?;
        let term_a = self
            .num
            .checked_mul(rhs.den / g)
            .ok_or(RegistrationValidationError::InvalidRationalClockParameter)?;
        let term_b = rhs
            .num
            .checked_mul(self.den / g)
            .ok_or(RegistrationValidationError::InvalidRationalClockParameter)?;
        let sum_num = term_a
            .checked_add(term_b)
            .ok_or(RegistrationValidationError::InvalidRationalClockParameter)?;
        Self::new(sum_num, lcm_den)
    }

    /// Convert to `f64` seconds for diagnostic display only (never used for equality checks).
    #[inline]
    pub fn to_f64_lossy(self) -> f64 {
        (self.num as f64) / (self.den as f64)
    }
}

/// Shared Schema 1 rational clock descriptor (C02-C03).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RationalClockDescriptor {
    pub origin_id: u64,
    pub scale: u32,
    pub epoch_seconds: i64,
    pub epoch_nanoseconds: u32,
    pub ticks_per_second_num: u64,
    pub ticks_per_second_den: u64,
}

/// Compute the ordered quaternion squared norm $n^2 = (((x\cdot x + y\cdot y) + z\cdot z) + w\cdot w)$
/// without fused-multiply-add reassociation, as mandated by C02-C04.
#[inline]
pub fn ordered_quaternion_norm_sq(xyzw: [f64; 4]) -> f64 {
    let xx = xyzw[0] * xyzw[0];
    let yy = xyzw[1] * xyzw[1];
    let zz = xyzw[2] * xyzw[2];
    let ww = xyzw[3] * xyzw[3];
    ((xx + yy) + zz) + ww
}

/// Admit an authored `[x, y, z, w]` Hamilton quaternion per C02-C04:
/// 1. Verify all 4 components are finite and $|n^2 - 1.0| \le 10^{-12}$ using the ordered sum.
/// 2. Retain `raw_xyzw` unchanged while computing the normalized derived quaternion $q_i / \sqrt{n^2}$.
/// 3. Construct the active local-to-parent $3\times 3$ rotation matrix $\mathbf{R}(q_{\text{derived}})$
///    and verify $\|\mathbf{R}^T\mathbf{R} - \mathbf{I}\|_{\max} \le 64\cdot 2^{-52}$.
pub fn verify_and_derive_quaternion_rotation(
    raw_xyzw: [f64; 4],
) -> Result<VerifiedQuaternionRotation, RegistrationValidationError> {
    for &c in &raw_xyzw {
        if !c.is_finite() {
            return Err(RegistrationValidationError::NonFiniteCoordinateValue { value: c });
        }
    }
    let n2 = ordered_quaternion_norm_sq(raw_xyzw);
    let deviation = (n2 - 1.0).abs();
    if !n2.is_finite() || deviation > FRAME_ADMISSION_TOLERANCE {
        return Err(RegistrationValidationError::QuaternionNormOutOfTolerance {
            raw_xyzw,
            norm_sq: n2,
            deviation,
            tolerance: FRAME_ADMISSION_TOLERANCE,
        });
    }
    let n = n2.sqrt();
    let q = [
        raw_xyzw[0] / n,
        raw_xyzw[1] / n,
        raw_xyzw[2] / n,
        raw_xyzw[3] / n,
    ];
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let rot = [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y - w * z),
            2.0 * (x * z + w * y),
        ],
        [
            2.0 * (x * y + w * z),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z - w * x),
        ],
        [
            2.0 * (x * z - w * y),
            2.0 * (y * z + w * x),
            1.0 - 2.0 * (x * x + y * y),
        ],
    ];
    let orth_res = max_orthogonality_residual(&rot);
    let det = determinant_3x3(&rot);
    if orth_res > C02_ROUNDING_FACTOR || (det - 1.0).abs() > C02_ROUNDING_FACTOR {
        return Err(RegistrationValidationError::ImproperOrNonOrthogonalFrame {
            determinant: det,
            max_orthogonality_residual: orth_res,
            tolerance: C02_ROUNDING_FACTOR,
        });
    }
    Ok(VerifiedQuaternionRotation {
        raw_xyzw,
        raw_norm_sq: n2,
        derived_unit_xyzw: q,
        rotation_matrix: rot,
        orthogonality_residual: orth_res,
        determinant: det,
    })
}

/// Active Hamilton quaternion multiplication $q_{\text{parent}} \otimes q_{\text{local}}$ in `[x, y, z, w]` order
/// (apply `q_local` first, then `q_parent`).
pub fn compose_hamilton_quaternions_xyzw(q_parent: [f64; 4], q_local: [f64; 4]) -> [f64; 4] {
    let (xp, yp, zp, wp) = (q_parent[0], q_parent[1], q_parent[2], q_parent[3]);
    let (xl, yl, zl, wl) = (q_local[0], q_local[1], q_local[2], q_local[3]);
    [
        wp * xl + xp * wl + yp * zl - zp * yl,
        wp * yl - xp * zl + yp * wl + zp * xl,
        wp * zl + xp * yl - yp * xl + zp * wl,
        wp * wl - xp * xl - yp * yl - zp * zl,
    ]
}

/// Map a local vector $\mathbf{v}_{\text{local}}$ and local position $\mathbf{x}_{\text{local}}$
/// to the parent frame:
/// $\mathbf{v}_{\text{parent}} = \mathbf{R}\mathbf{v}_{\text{local}}$,
/// $\mathbf{x}_{\text{parent}} = \mathbf{o}_{\text{parent}} + \mathbf{R}\mathbf{x}_{\text{local}}$.
pub fn transform_local_to_parent(
    origin_parent_m: [f64; 3],
    rot: &[[f64; 3]; 3],
    local_position_m: [f64; 3],
    local_vector: [f64; 3],
) -> ([f64; 3], [f64; 3]) {
    let rx = mat3_vec3_mul(rot, local_position_m);
    let rv = mat3_vec3_mul(rot, local_vector);
    let pos_parent = [
        origin_parent_m[0] + rx[0],
        origin_parent_m[1] + rx[1],
        origin_parent_m[2] + rx[2],
    ];
    (pos_parent, rv)
}

/// Verify that a $3\times 3$ matrix $\mathbf{R}$ is a stationary proper orthogonal rotation
/// ($\|\mathbf{R}^T\mathbf{R} - \mathbf{I}\|_{\max} \le 10^{-12}$, $\det(\mathbf{R}) > 0$, $|\det(\mathbf{R}) - 1| \le 10^{-12}$)
/// and reject improper/reflected, sheared, or uncalibrated moving frames (C02-C04).
pub fn verify_stationary_proper_rotation_matrix(
    rot: &[[f64; 3]; 3],
    has_frame_velocity: bool,
    has_kinematic_profile: bool,
) -> Result<(f64, f64), RegistrationValidationError> {
    if has_frame_velocity && !has_kinematic_profile {
        return Err(RegistrationValidationError::MovingFrameRejectedWithoutKinematicProfile);
    }
    for r in 0..3 {
        for c in 0..3 {
            if !rot[r][c].is_finite() {
                return Err(RegistrationValidationError::NonFiniteCoordinateValue {
                    value: rot[r][c],
                });
            }
        }
    }
    let orth_res = max_orthogonality_residual(rot);
    let det = determinant_3x3(rot);
    if orth_res > FRAME_ADMISSION_TOLERANCE
        || det <= 0.0
        || (det - 1.0).abs() > FRAME_ADMISSION_TOLERANCE
    {
        return Err(RegistrationValidationError::ImproperOrNonOrthogonalFrame {
            determinant: det,
            max_orthogonality_residual: orth_res,
            tolerance: FRAME_ADMISSION_TOLERANCE,
        });
    }
    Ok((det, orth_res))
}

/// Compute exact sample timestamp offset from clock epoch in reduced `i128` rational seconds (C02-C03):
///
/// $$t(i) - t_{\text{epoch}} = \frac{\text{start\_tick}\cdot\text{tick\_den}}{\text{tick\_num}} + \frac{i\cdot\text{rate\_den}}{\text{rate\_num}} + \frac{\text{offset\_num}\cdot\text{tick\_den}}{\text{offset\_den}\cdot\text{tick\_num}}.$$
pub fn evaluate_exact_sample_instant(
    clock: &RationalClockDescriptor,
    start_tick: i64,
    sample_index: i64,
    sample_rate_num: u64,
    sample_rate_den: u64,
    component_tick_offset_num: i64,
    component_tick_offset_den: u64,
) -> Result<RationalSeconds, RegistrationValidationError> {
    if clock.ticks_per_second_num == 0
        || clock.ticks_per_second_den == 0
        || sample_rate_num == 0
        || sample_rate_den == 0
        || component_tick_offset_den == 0
        || clock.epoch_nanoseconds >= 1_000_000_000
    {
        return Err(RegistrationValidationError::InvalidRationalClockParameter);
    }
    let tick_num = clock.ticks_per_second_num as i128;
    let tick_den = clock.ticks_per_second_den as i128;

    let start_num = (start_tick as i128)
        .checked_mul(tick_den)
        .ok_or(RegistrationValidationError::InvalidRationalClockParameter)?;
    let start_sec = RationalSeconds::new(start_num, tick_num)?;

    let sample_num = (sample_index as i128)
        .checked_mul(sample_rate_den as i128)
        .ok_or(RegistrationValidationError::InvalidRationalClockParameter)?;
    let sample_sec = RationalSeconds::new(sample_num, sample_rate_num as i128)?;

    let offset_num = (component_tick_offset_num as i128)
        .checked_mul(tick_den)
        .ok_or(RegistrationValidationError::InvalidRationalClockParameter)?;
    let offset_den = (component_tick_offset_den as i128)
        .checked_mul(tick_num)
        .ok_or(RegistrationValidationError::InvalidRationalClockParameter)?;
    let offset_sec = RationalSeconds::new(offset_num, offset_den)?;

    start_sec.checked_add(sample_sec)?.checked_add(offset_sec)
}

/// Verify cross-stream clock compatibility and phase uncertainty bound $\Delta\phi = 2\pi f u_t$
/// for phase-coherent joining across EMF v4, QAC v1, and anatomical streams (C02-C03).
pub fn verify_cross_modal_clock_join(
    clock_a: &RationalClockDescriptor,
    clock_b: &RationalClockDescriptor,
    has_explicit_origin_mapping: bool,
    time_uncertainty_s: f64,
    carrier_hz: f64,
    max_phase_uncertainty_rad: Option<f64>,
) -> Result<f64, RegistrationValidationError> {
    if (clock_a.origin_id != clock_b.origin_id
        || clock_a.scale != clock_b.scale
        || clock_a.epoch_seconds != clock_b.epoch_seconds
        || clock_a.epoch_nanoseconds != clock_b.epoch_nanoseconds)
        && !has_explicit_origin_mapping
    {
        return Err(RegistrationValidationError::IncompatibleClockOrigins {
            clock_a_origin_id: clock_a.origin_id,
            clock_b_origin_id: clock_b.origin_id,
            clock_a_scale: clock_a.scale,
            clock_b_scale: clock_b.scale,
        });
    }
    if !time_uncertainty_s.is_finite() || time_uncertainty_s < 0.0 || !carrier_hz.is_finite() || carrier_hz < 0.0 {
        return Err(RegistrationValidationError::NonFiniteCoordinateValue {
            value: time_uncertainty_s,
        });
    }
    let phase_uncertainty_rad = 2.0 * core::f64::consts::PI * carrier_hz * time_uncertainty_s;
    if let Some(max_rad) = max_phase_uncertainty_rad {
        if phase_uncertainty_rad > max_rad {
            return Err(RegistrationValidationError::PhaseUncertaintyExceeded {
                phase_uncertainty_rad,
                max_allowed_rad: max_rad,
            });
        }
    }
    Ok(phase_uncertainty_rad)
}

/// Domain family for material entry `0` disambiguation (C02-C06).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialDomainFamily {
    /// EMF v4: `material_entry == 0` is unbound unless explicit vacuum is declared.
    EmfV4 { explicit_vacuum: bool },
    /// QAC v1: `material_entry == 0` refers to the explicitly declared uniform medium.
    QacV1 { has_declared_uniform_medium: bool },
}

/// Verify material & anatomical coupling identity across streams (C02-C06):
/// requires matching non-zero 32-byte object and material-table digests, schema revision,
/// epoch, coordinate frame, and compatible clock, never accepting a Morton/Hilbert key match alone.
#[allow(clippy::too_many_arguments)]
pub fn verify_material_and_anatomy_coupling(
    family: MaterialDomainFamily,
    material_entry_id: u32,
    expected_object_digest: &[u8; 32],
    actual_object_digest: &[u8; 32],
    expected_table_digest: &[u8; 32],
    actual_table_digest: &[u8; 32],
    expected_epoch: u64,
    actual_epoch: u64,
    frame_compatible: bool,
    clock_compatible: bool,
) -> Result<(), RegistrationValidationError> {
    match family {
        MaterialDomainFamily::EmfV4 { explicit_vacuum } => {
            if material_entry_id == 0 && !explicit_vacuum {
                return Err(
                    RegistrationValidationError::MaterialOrAnatomyBindingRejected {
                        reason: "emf_material0_unbound_without_explicit_vacuum",
                    },
                );
            }
        }
        MaterialDomainFamily::QacV1 {
            has_declared_uniform_medium,
        } => {
            if material_entry_id == 0 && !has_declared_uniform_medium {
                return Err(
                    RegistrationValidationError::MaterialOrAnatomyBindingRejected {
                        reason: "qac_material0_missing_uniform_medium_declaration",
                    },
                );
            }
        }
    }
    if expected_object_digest != actual_object_digest || expected_object_digest == &[0u8; 32] {
        return Err(
            RegistrationValidationError::MaterialOrAnatomyBindingRejected {
                reason: "object_digest_mismatch_or_zero",
            },
        );
    }
    if expected_table_digest != actual_table_digest || expected_table_digest == &[0u8; 32] {
        return Err(
            RegistrationValidationError::MaterialOrAnatomyBindingRejected {
                reason: "material_table_digest_mismatch_or_zero",
            },
        );
    }
    if expected_epoch != actual_epoch {
        return Err(
            RegistrationValidationError::MaterialOrAnatomyBindingRejected {
                reason: "material_table_epoch_mismatch",
            },
        );
    }
    if !frame_compatible || !clock_compatible {
        return Err(
            RegistrationValidationError::MaterialOrAnatomyBindingRejected {
                reason: "incompatible_frame_or_clock_for_material_join",
            },
        );
    }
    Ok(())
}

/// Verify that a transformed sensor/dipole/ultrasound sample position lies inside the target
/// anatomical `.10d`/`.m64` axis-aligned bounding box (AABB) and that the local surface normal
/// is a unit vector ($\|\mathbf{n}\|_2 = 1$ within $10^{-12}$).
pub fn verify_anatomical_region_registration(
    sample_position_m: [f64; 3],
    aabb_min_m: [f64; 3],
    aabb_max_m: [f64; 3],
    surface_normal: [f64; 3],
) -> Result<(), RegistrationValidationError> {
    for i in 0..3 {
        let p = sample_position_m[i];
        let lo = aabb_min_m[i];
        let hi = aabb_max_m[i];
        if !p.is_finite() || !lo.is_finite() || !hi.is_finite() || lo > hi {
            return Err(
                RegistrationValidationError::AnatomicalGeometryInvariantViolation {
                    reason: "invalid_aabb_bounds",
                    value: p,
                },
            );
        }
        if p < lo || p > hi {
            return Err(
                RegistrationValidationError::AnatomicalGeometryInvariantViolation {
                    reason: "position_outside_anatomical_aabb",
                    value: p,
                },
            );
        }
    }
    let n_norm = scaled_hypot3(surface_normal[0], surface_normal[1], surface_normal[2]);
    if !n_norm.is_finite() || (n_norm - 1.0).abs() > FRAME_ADMISSION_TOLERANCE {
        return Err(
            RegistrationValidationError::AnatomicalGeometryInvariantViolation {
                reason: "non_unit_anatomical_surface_normal",
                value: n_norm,
            },
        );
    }
    Ok(())
}

#[inline]
fn mat3_vec3_mul(m: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

#[inline]
fn max_orthogonality_residual(r: &[[f64; 3]; 3]) -> f64 {
    let mut max_res = 0.0f64;
    for i in 0..3 {
        for j in 0..3 {
            let dot = r[0][i] * r[0][j] + r[1][i] * r[1][j] + r[2][i] * r[2][j];
            let target = if i == j { 1.0 } else { 0.0 };
            let res = (dot - target).abs();
            if res > max_res {
                max_res = res;
            }
        }
    }
    max_res
}

#[inline]
fn determinant_3x3(m: &[[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

#[inline]
const fn gcd_i128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    if a == 0 { 1 } else { a }
}
