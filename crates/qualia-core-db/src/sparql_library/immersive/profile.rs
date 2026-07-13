//! QISP Tensor10D profile — the fixed ordered dimensions and their honest
//! classification, plus inline-value validation (plan §3.5, §3.6).
//!
//! The Qualia profile fixes the ordered dimensions as
//! `[q, v, w, x, y, z, t, alpha, mu, sigma]`. **Not all ten are physical
//! dimensions.** `x, y, z` are spatial and `t` is temporal, but `q, v, w, alpha,
//! mu, sigma` carry Qualia-specific semantics and are only meaningful under an
//! explicit profile IRI ([`TENSOR10D_PROFILE_IRI`]) — an external system may treat
//! the payload as an opaque asset while still reading this profile metadata.
//!
//! A `sigma` value is the shared-EMF-truth spectral quantity σ (the 10th axis of
//! Tensor10D); it is classified [`DimClass::Spectral`] here.

use super::value::QispError;

/// Absolute IRI of the Qualia Tensor10D profile. Any inline Tensor10D value MUST be
/// accompanied by this profile IRI to be interpreted (plan §3.5, §3.6).
pub const TENSOR10D_PROFILE_IRI: &str =
    "https://standards.qualiadb.org/immersive/0.1#Tensor10DProfile";

/// How a Tensor10D dimension is classified. This governs how a dimension is
/// interpreted and rendered; it deliberately does **not** claim all ten axes are
/// physical (plan §3.5).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DimClass {
    /// A physical spatial axis (metres in the declared coordinate frame).
    Spatial = 0,
    /// A physical temporal axis.
    Temporal = 1,
    /// A Qualia-specific epistemic quantity (belief/quality/uncertainty-like).
    Epistemic = 2,
    /// A spectral / shared-EMF-truth quantity (σ and spectral-coded values).
    Spectral = 3,
    /// A categorical / discriminant label dimension.
    Categorical = 4,
}

/// The fixed ordered Tensor10D dimensions with their classification (plan §3.5).
///
/// Order is normative: `[q, v, w, x, y, z, t, alpha, mu, sigma]`. Classification is
/// honest — only `x, y, z` (spatial) and `t` (temporal) are physical; the rest are
/// Qualia-specific and require [`TENSOR10D_PROFILE_IRI`]:
///
/// - `q`   — Epistemic (Qualia quality/qualia measure)
/// - `v`   — Categorical (Qualia-specific label axis)
/// - `w`   — Categorical (Qualia-specific label axis)
/// - `x`,`y`,`z` — Spatial (physical)
/// - `t`   — Temporal (physical)
/// - `alpha` — Spectral (Qualia-specific spectral-coded amplitude)
/// - `mu`  — Epistemic (Qualia-specific mean/uncertainty parameter)
/// - `sigma` — Spectral (shared EMF truth σ)
pub const TENSOR10D_DIMS: [(&str, DimClass); 10] = [
    ("q", DimClass::Epistemic),
    ("v", DimClass::Categorical),
    ("w", DimClass::Categorical),
    ("x", DimClass::Spatial),
    ("y", DimClass::Spatial),
    ("z", DimClass::Spatial),
    ("t", DimClass::Temporal),
    ("alpha", DimClass::Spectral),
    ("mu", DimClass::Epistemic),
    ("sigma", DimClass::Spectral),
];

/// Validate an inline Tensor10D value: it must be **exactly ten finite values**
/// (plan §3.6 "exactly ten finite values plus a profile IRI"). Wrong arity or any
/// non-finite (NaN/±Inf) component is rejected with [`QispError::ProfileMismatch`]
/// before any native dispatch (plan §3.6 "Reject non-canonical, non-finite,
/// mixed-profile, or ambiguous-unit values before native dispatch").
///
/// The caller is responsible for supplying the accompanying profile IRI; this
/// function validates the numeric payload shape and finiteness.
pub fn validate_inline_tensor10d(values: &[f64]) -> Result<[f64; 10], QispError> {
    if values.len() != 10 {
        return Err(QispError::ProfileMismatch);
    }
    let mut out = [0.0f64; 10];
    for (i, &v) in values.iter().enumerate() {
        if !v.is_finite() {
            return Err(QispError::ProfileMismatch);
        }
        out[i] = v;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dim_order_is_fixed_and_normative() {
        let names: [&str; 10] = TENSOR10D_DIMS.map(|(n, _)| n);
        assert_eq!(
            names,
            ["q", "v", "w", "x", "y", "z", "t", "alpha", "mu", "sigma"]
        );
    }

    #[test]
    fn physical_axes_are_classified_honestly() {
        // x, y, z spatial; t temporal.
        assert_eq!(TENSOR10D_DIMS[3], ("x", DimClass::Spatial));
        assert_eq!(TENSOR10D_DIMS[4], ("y", DimClass::Spatial));
        assert_eq!(TENSOR10D_DIMS[5], ("z", DimClass::Spatial));
        assert_eq!(TENSOR10D_DIMS[6], ("t", DimClass::Temporal));
        // sigma is the shared-EMF-truth spectral axis.
        assert_eq!(TENSOR10D_DIMS[9], ("sigma", DimClass::Spectral));
        // The Qualia-specific axes are NOT presented as physical (spatial/temporal).
        for &(name, class) in &[
            TENSOR10D_DIMS[0], // q
            TENSOR10D_DIMS[1], // v
            TENSOR10D_DIMS[2], // w
            TENSOR10D_DIMS[7], // alpha
            TENSOR10D_DIMS[8], // mu
        ] {
            assert!(
                class != DimClass::Spatial && class != DimClass::Temporal,
                "Qualia-specific dim {name} must not be classified physical"
            );
        }
    }

    #[test]
    fn profile_iri_is_the_provisional_tensor10d_iri() {
        assert_eq!(
            TENSOR10D_PROFILE_IRI,
            "https://standards.qualiadb.org/immersive/0.1#Tensor10DProfile"
        );
    }

    #[test]
    fn validate_accepts_ten_finite_values() {
        let v = [0.0, 1.0, -2.5, 3.0, 4.0, 5.0, 6.0, 0.25, -0.75, 100.0];
        let out = validate_inline_tensor10d(&v).unwrap();
        assert_eq!(out, v);
    }

    #[test]
    fn validate_rejects_wrong_arity() {
        assert_eq!(
            validate_inline_tensor10d(&[1.0, 2.0, 3.0]),
            Err(QispError::ProfileMismatch)
        );
        let eleven = [1.0f64; 11];
        assert_eq!(
            validate_inline_tensor10d(&eleven),
            Err(QispError::ProfileMismatch)
        );
        assert_eq!(
            validate_inline_tensor10d(&[]),
            Err(QispError::ProfileMismatch)
        );
    }

    #[test]
    fn validate_rejects_non_finite() {
        let mut nan_v = [0.0f64; 10];
        nan_v[4] = f64::NAN;
        assert_eq!(
            validate_inline_tensor10d(&nan_v),
            Err(QispError::ProfileMismatch)
        );

        let mut inf_v = [0.0f64; 10];
        inf_v[9] = f64::INFINITY;
        assert_eq!(
            validate_inline_tensor10d(&inf_v),
            Err(QispError::ProfileMismatch)
        );

        let mut neg_inf_v = [0.0f64; 10];
        neg_inf_v[0] = f64::NEG_INFINITY;
        assert_eq!(
            validate_inline_tensor10d(&neg_inf_v),
            Err(QispError::ProfileMismatch)
        );
    }
}
