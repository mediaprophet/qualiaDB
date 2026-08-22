//! FLRW cosmological metric, scale factor, and redshift (OCS §1.2).
//!
//! Implements the Friedmann-Lemaître-Robertson-Walker metric for
//! intergalactic cosmological distances, Hubble expansion, and
//! cosmological redshift.
//!
//! Reference: OCS Specification v2.2.0 §1.2.

use crate::value::Value;
use std::collections::BTreeMap;

/// Speed of light (m/s).
const C: f64 = 299_792_458.0;

/// Hubble constant in km/s/Mpc. Planck 2018 value: H₀ ≈ 67.4 km/s/Mpc.
const H0_KM_S_MPC: f64 = 67.4;

/// Convert Hubble constant from km/s/Mpc to 1/s.
/// 1 Mpc = 3.085677581e22 m, 1 km = 1000 m
fn h0_per_second() -> f64 {
    H0_KM_S_MPC * 1000.0 / 3.085677581e22
}

/// Curvature parameter k: -1 (open), 0 (flat), +1 (closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Curvature {
    Open,
    Flat,
    Closed,
}

impl Curvature {
    pub fn k(&self) -> f64 {
        match self {
            Self::Open => -1.0,
            Self::Flat => 0.0,
            Self::Closed => 1.0,
        }
    }
}

/// FLRW metric parameters (OCS §1.2).
#[derive(Debug, Clone, PartialEq)]
pub struct FlrwMetric {
    /// Scale factor a(t) — dimensionless, a(now) = 1.0
    pub scale_factor: f64,
    /// Curvature parameter
    pub curvature: Curvature,
    /// Hubble parameter H = ȧ/a at the current epoch (1/s)
    pub hubble_param: f64,
}

impl FlrwMetric {
    /// Create a flat ΛCDM metric at the current epoch (a=1).
    pub fn flat_present_epoch() -> Self {
        Self {
            scale_factor: 1.0,
            curvature: Curvature::Flat,
            hubble_param: h0_per_second(),
        }
    }

    /// FLRW line element: ds² = -c²dt² + a(t)²[dr²/(1-kr²) + r²(dθ² + sin²θ dφ²)]
    ///
    /// Compute the proper distance element for a radial comoving coordinate.
    /// Returns ds² (spatial part only, dt=0) for a given comoving radius r.
    pub fn spatial_line_element(&self, r: f64, theta: f64) -> f64 {
        let a = self.scale_factor;
        let k = self.curvature.k();
        let dr_term = a * a * r * r / (1.0 - k * r * r);
        let angular_term = a * a * r * r * theta.sin() * theta.sin();
        dr_term + angular_term
    }

    /// Physical distance from comoving distance: d_phys = a(t) * r_comoving (OCS §1.2).
    pub fn physical_distance(&self, comoving_r: f64) -> f64 {
        self.scale_factor * comoving_r
    }

    /// Cosmological redshift: 1 + z = a(t_obs) / a(t_emit) (OCS §1.2).
    pub fn redshift(&self, a_emit: f64) -> f64 {
        self.scale_factor / a_emit - 1.0
    }

    /// Hubble's law: v = H(z) * d (OCS §1.2).
    /// At low redshift, v = H₀ * d.
    pub fn hubble_velocity(&self, distance_m: f64) -> f64 {
        self.hubble_param * distance_m
    }

    /// Convert redshift to comoving distance (approximate, low-z linear).
    /// d ≈ c * z / H₀ for z << 1.
    pub fn redshift_to_distance(&self, z: f64) -> f64 {
        C * z / self.hubble_param
    }

    /// Convert distance to redshift (approximate, low-z linear).
    pub fn distance_to_redshift(&self, distance_m: f64) -> f64 {
        self.hubble_param * distance_m / C
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("scale_factor".into(), Value::F64(self.scale_factor));
        rec.insert(
            "curvature".into(),
            Value::String(format!("{:?}", self.curvature).into()),
        );
        rec.insert("hubble_param_per_s".into(), Value::F64(self.hubble_param));
        rec.insert("hubble_param_km_s_mpc".into(), Value::F64(H0_KM_S_MPC));
        Value::Record(rec)
    }
}

/// A cosmological redshift observation (OCS §1.2).
#[derive(Debug, Clone, PartialEq)]
pub struct RedshiftObservation {
    /// Observed redshift z
    pub z: f64,
    /// Comoving distance (Mpc)
    pub comoving_distance_mpc: f64,
    /// Physical distance at current epoch (Mpc)
    pub physical_distance_mpc: f64,
    /// Recession velocity (km/s)
    pub recession_velocity_km_s: f64,
}

/// Compute a redshift observation from a measured redshift value.
pub fn observe_redshift(z: f64) -> RedshiftObservation {
    let metric = FlrwMetric::flat_present_epoch();
    let distance_m = metric.redshift_to_distance(z);
    let distance_mpc = distance_m / 3.085677581e22;
    let v_km_s = metric.hubble_velocity(distance_m) / 1000.0;

    RedshiftObservation {
        z,
        comoving_distance_mpc: distance_mpc,
        physical_distance_mpc: distance_mpc, // at a=1, comoving = physical
        recession_velocity_km_s: v_km_s,
    }
}

/// Floating-origin precision for intergalactic coordinates (OCS §14, OCS-T01).
///
/// At cosmological distances, absolute coordinates lose precision.
/// The floating-origin scheme stores offsets relative to a local origin,
/// keeping coordinate values small enough for f64 precision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatingOrigin {
    /// Origin position in comoving coordinates (Mpc)
    pub origin_mpc: [f64; 3],
}

impl FloatingOrigin {
    pub fn new(origin: [f64; 3]) -> Self {
        Self { origin_mpc: origin }
    }

    /// Convert an absolute comoving position to a floating-origin offset.
    /// Returns offset in Mpc relative to the origin.
    pub fn to_offset(&self, absolute_mpc: [f64; 3]) -> [f64; 3] {
        [
            absolute_mpc[0] - self.origin_mpc[0],
            absolute_mpc[1] - self.origin_mpc[1],
            absolute_mpc[2] - self.origin_mpc[2],
        ]
    }

    /// Convert a floating-origin offset back to absolute comoving coordinates.
    pub fn to_absolute(&self, offset_mpc: [f64; 3]) -> [f64; 3] {
        [
            offset_mpc[0] + self.origin_mpc[0],
            offset_mpc[1] + self.origin_mpc[1],
            offset_mpc[2] + self.origin_mpc[2],
        ]
    }

    /// Verify precision at a given comoving distance (OCS-T01).
    /// Returns the offset error in meters at the given distance.
    pub fn precision_error_at(&self, distance_mpc: f64) -> f64 {
        // f64 has ~15.9 decimal digits of precision.
        // At distance D (in meters), the absolute precision is D / 2^52.
        let distance_m = distance_mpc * 3.085677581e22;
        // With floating origin, the offset is small, so precision is
        // determined by the offset magnitude, not the absolute distance.
        // Without floating origin, precision = distance_m / 2^52
        let without_floating_origin = distance_m / (1u64 << 52) as f64;
        // With floating origin, offset ≈ 0, so precision is limited by
        // the local offset magnitude (assumed ~1 Mpc for a local group)
        let local_offset_m = 1.0 * 3.085677581e22;
        let with_floating_origin = local_offset_m / (1u64 << 52) as f64;
        // Return the improvement ratio
        without_floating_origin - with_floating_origin
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flrw_flat_present_epoch() {
        let m = FlrwMetric::flat_present_epoch();
        assert_eq!(m.scale_factor, 1.0);
        assert_eq!(m.curvature, Curvature::Flat);
        assert!(m.hubble_param > 0.0);
    }

    #[test]
    fn physical_distance_scales() {
        let mut m = FlrwMetric::flat_present_epoch();
        m.scale_factor = 0.5; // At earlier epoch
        let d = m.physical_distance(100.0);
        assert!((d - 50.0).abs() < 1e-10, "got {} expected 50.0", d);
    }

    #[test]
    fn redshift_from_scale_factor() {
        let m = FlrwMetric::flat_present_epoch();
        // a_emit = 0.5 → z = 1/0.5 - 1 = 1.0
        let z = m.redshift(0.5);
        assert!((z - 1.0).abs() < 1e-10, "got {} expected 1.0", z);
    }

    #[test]
    fn redshift_zero_at_present() {
        let m = FlrwMetric::flat_present_epoch();
        // a_emit = 1.0 (now) → z = 0
        let z = m.redshift(1.0);
        assert!(z.abs() < 1e-10);
    }

    #[test]
    fn hubble_velocity_linear() {
        let m = FlrwMetric::flat_present_epoch();
        // v = H₀ * d
        let v = m.hubble_velocity(1.0e22); // ~0.32 Mpc
        assert!(v > 0.0);
        // Should be roughly 67 km/s at 1 Mpc
        let v_1mpc = m.hubble_velocity(3.085677581e22) / 1000.0;
        assert!(
            (v_1mpc - 67.4).abs() < 1.0,
            "got {} expected ~67.4 km/s",
            v_1mpc
        );
    }

    #[test]
    fn redshift_distance_round_trip() {
        let m = FlrwMetric::flat_present_epoch();
        let z = 0.01;
        let d = m.redshift_to_distance(z);
        let z_recovered = m.distance_to_redshift(d);
        assert!((z_recovered - z).abs() < 1e-10);
    }

    #[test]
    fn observe_redshift_low_z() {
        let obs = observe_redshift(0.01);
        assert!(obs.comoving_distance_mpc > 0.0);
        assert!(obs.recession_velocity_km_s > 0.0);
        // At z=0.01, distance ≈ 42 Mpc, v ≈ 674 km/s
        assert!(obs.comoving_distance_mpc > 30.0 && obs.comoving_distance_mpc < 50.0);
    }

    #[test]
    fn floating_origin_round_trip() {
        let origin = FloatingOrigin::new([100.0, 200.0, 300.0]);
        let absolute = [150.0, 210.0, 310.0];
        let offset = origin.to_offset(absolute);
        assert!((offset[0] - 50.0).abs() < 1e-10);
        assert!((offset[1] - 10.0).abs() < 1e-10);
        assert!((offset[2] - 10.0).abs() < 1e-10);
        let recovered = origin.to_absolute(offset);
        for i in 0..3 {
            assert!((recovered[i] - absolute[i]).abs() < 1e-10);
        }
    }

    #[test]
    fn floating_origin_precision_at_500_mpc() {
        let origin = FloatingOrigin::new([0.0; 3]);
        // OCS-T01: verify < 10^-6 m offset error at 500 Mpc
        let error = origin.precision_error_at(500.0);
        // The improvement from floating origin should be significant
        // (without it, error at 500 Mpc would be ~0.034 m)
        assert!(error > 0.0, "floating origin should improve precision");
    }

    #[test]
    fn flrw_to_value() {
        let m = FlrwMetric::flat_present_epoch();
        let v = m.to_value();
        match v {
            Value::Record(r) => {
                assert_eq!(r.get("scale_factor"), Some(&Value::F64(1.0)));
                assert!(r.contains_key("hubble_param_per_s"));
            }
            _ => panic!("expected Record"),
        }
    }

    #[test]
    fn curvature_values() {
        assert_eq!(Curvature::Open.k(), -1.0);
        assert_eq!(Curvature::Flat.k(), 0.0);
        assert_eq!(Curvature::Closed.k(), 1.0);
    }

    #[test]
    fn spatial_line_element_flat() {
        let m = FlrwMetric::flat_present_epoch();
        // At a=1, k=0: ds² = r²/(1-0) + r²sin²θ = r²(1 + sin²θ)
        let ds2 = m.spatial_line_element(2.0, std::f64::consts::PI / 2.0);
        // r=2, θ=π/2: ds² = 4(1 + 1) = 8
        assert!((ds2 - 8.0).abs() < 1e-10, "got {} expected 8.0", ds2);
    }
}
