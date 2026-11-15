//! 3D Spherical-Harmonic Wigner-D / Ivanic-Ruedenberg Real SH Rotation Matrices
//! (`wigner_rotation.rs`).
//!
//! Adapts `etalon_signal::hoa` quaternion admission, tensor rotation, and Ivanic-Ruedenberg
//! real spherical-harmonic Wigner-D blocks $\mathbf{D}^{(l)}(\mathbf{R})$ ($l = 0..3$)
//! into QAC Mode `0x0102` and C02-C04 error contracts.

pub use etalon_signal::hoa::{
    hamilton_product_xyzw, quaternion_conjugate_xyzw, quaternion_ordered_norm_sq_f64,
    QUATERNION_NORM_SQ_TOLERANCE,
};

use super::hoa_acn_sn3d::{HoaOrder, SpatialDspError};

/// Validate an authored `[x, y, z, w]` quaternion under C02-C04 (`|n2 - 1| <= 1e-12`)
/// and return the derived normalized unit quaternion `(qx, qy, qz, qw)` together with
/// the raw squared norm `n2`.
#[inline]
pub fn admit_and_derive_unit_quaternion_f64(
    xyzw: [f64; 4],
) -> Result<([f64; 4], f64), SpatialDspError> {
    etalon_signal::hoa::admit_and_derive_unit_quaternion_f64(xyzw).map_err(|err| match err {
        etalon_signal::SignalError::InvalidParameter => SpatialDspError::InvalidQuaternionNorm,
        other => SpatialDspError::from(other),
    })
}

/// Construct the $3 \times 3$ active Hamilton rotation matrix $\mathbf{R}(q)$ from an
/// admitted `[x, y, z, w]` quaternion.
#[inline]
pub fn rotation_matrix_from_quaternion_f64(
    xyzw: [f64; 4],
) -> Result<[[f64; 3]; 3], SpatialDspError> {
    etalon_signal::hoa::rotation_matrix_from_quaternion_f64(xyzw).map_err(|err| match err {
        etalon_signal::SignalError::InvalidParameter => SpatialDspError::InvalidQuaternionNorm,
        other => SpatialDspError::from(other),
    })
}

/// Apply active rotation $\mathbf{v}_{\text{parent}} = \mathbf{R}(q)\mathbf{v}_{\text{local}}$.
#[inline]
pub fn rotate_vector_active_f64(
    xyzw: [f64; 4],
    v_local: [f64; 3],
) -> Result<[f64; 3], SpatialDspError> {
    etalon_signal::hoa::rotate_vector_active_f64(xyzw, v_local).map_err(|err| match err {
        etalon_signal::SignalError::InvalidParameter => SpatialDspError::InvalidQuaternionNorm,
        other => SpatialDspError::from(other),
    })
}

/// Transform a 6-component symmetric tensor `[xx, yy, zz, xy, yz, xz]` under active
/// rotation $\mathbf{T}' = \mathbf{R}\mathbf{T}\mathbf{R}^T$.
#[inline]
pub fn rotate_symmetric_tensor_active_f64(
    xyzw: [f64; 4],
    tensor_xx_yy_zz_xy_yz_xz: [f64; 6],
) -> Result<[f64; 6], SpatialDspError> {
    etalon_signal::hoa::rotate_symmetric_tensor_active_f64(xyzw, tensor_xx_yy_zz_xy_yz_xz).map_err(
        |err| match err {
            etalon_signal::SignalError::InvalidParameter => SpatialDspError::InvalidQuaternionNorm,
            other => SpatialDspError::from(other),
        },
    )
}

/// Block-diagonal real spherical harmonic Wigner-D rotation matrices up to order $L = 3$
/// (`ACN 0..15`), backed by [`etalon_signal::hoa::HoaWignerRotation`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HoaWignerRotation {
    pub cartesian_r: [[f64; 3]; 3],
    pub d1: [[f64; 3]; 3],
    pub d2: [[f64; 5]; 5],
    pub d3: [[f64; 7]; 7],
}

impl From<etalon_signal::hoa::HoaWignerRotation> for HoaWignerRotation {
    #[inline]
    fn from(inner: etalon_signal::hoa::HoaWignerRotation) -> Self {
        Self {
            cartesian_r: inner.cartesian_r,
            d1: inner.d1,
            d2: inner.d2,
            d3: inner.d3,
        }
    }
}

impl From<HoaWignerRotation> for etalon_signal::hoa::HoaWignerRotation {
    #[inline]
    fn from(outer: HoaWignerRotation) -> Self {
        Self {
            cartesian_r: outer.cartesian_r,
            d1: outer.d1,
            d2: outer.d2,
            d3: outer.d3,
        }
    }
}

impl HoaWignerRotation {
    /// Construct the real spherical harmonic Wigner-D rotation blocks up to $L = 3$
    /// from an active Hamilton quaternion `xyzw` (validating C02-C04 squared norm within `1e-12`).
    pub fn from_quaternion_xyzw(xyzw: [f64; 4]) -> Result<Self, SpatialDspError> {
        let inner =
            etalon_signal::hoa::HoaWignerRotation::from_quaternion_xyzw(xyzw).map_err(|err| {
                match err {
                    etalon_signal::SignalError::InvalidParameter => {
                        SpatialDspError::InvalidQuaternionNorm
                    }
                    other => SpatialDspError::from(other),
                }
            })?;
        Ok(Self::from(inner))
    }

    /// Construct the Wigner-D rotation blocks from a $3 \times 3$ proper orthogonal matrix $\mathbf{R}$.
    #[inline]
    pub fn from_cartesian_rotation_unchecked(r: [[f64; 3]; 3]) -> Self {
        Self::from(etalon_signal::hoa::HoaWignerRotation::from_cartesian_rotation_unchecked(r))
    }

    /// Return the inverse (transpose) rotation $\mathbf{D}(\mathbf{R}^T) = \mathbf{D}(\mathbf{R})^T$.
    #[inline]
    pub fn transpose(&self) -> Self {
        Self::from(etalon_signal::hoa::HoaWignerRotation::from(*self).transpose())
    }

    /// Rotate a single HOA frame (`in_frame[..ch]`) into `out_frame[..ch]` in `f64`.
    #[inline]
    pub fn apply_frame_f64(
        &self,
        order: HoaOrder,
        in_frame: &[f64],
        out_frame: &mut [f64],
    ) -> Result<(), SpatialDspError> {
        Ok(etalon_signal::hoa::HoaWignerRotation::from(*self)
            .apply_frame_f64(order, in_frame, out_frame)?)
    }

    /// Rotate a single HOA frame in-place in `f64`.
    #[inline]
    pub fn apply_frame_in_place_f64(
        &self,
        order: HoaOrder,
        frame: &mut [f64],
    ) -> Result<(), SpatialDspError> {
        Ok(etalon_signal::hoa::HoaWignerRotation::from(*self)
            .apply_frame_in_place_f64(order, frame)?)
    }

    /// Rotate an interleaved `time-channel` HOA stream (`in_stream`) into `out_stream` in `f32`.
    #[inline]
    pub fn apply_stream_into_f32(
        &self,
        order: HoaOrder,
        in_stream: &[f32],
        out_stream: &mut [f32],
    ) -> Result<usize, SpatialDspError> {
        Ok(etalon_signal::hoa::HoaWignerRotation::from(*self)
            .apply_stream_into_f32(order, in_stream, out_stream)?)
    }

    /// Maximum orthogonality residual $\max_{i,j} |(\mathbf{D}^T \mathbf{D} - \mathbf{I})_{i,j}|$.
    #[inline]
    pub fn max_orthogonality_residual(&self, order: HoaOrder) -> f64 {
        etalon_signal::hoa::HoaWignerRotation::from(*self).max_orthogonality_residual(order)
    }
}
