//! FLRW cosmological metric, scale factor, redshift, and comoving intercept solver (OCS §1.2, OCS-12).
//!
//! Implements the Friedmann-Lemaître-Robertson-Walker metric for
//! intergalactic cosmological distances, Hubble expansion, cosmological redshift,
//! and comoving intercept trajectory estimation with explicit horizon failure states.
//!
//! Reference: OCS Specification v2.2.0 §1.2 & Playbook v4 Packet OCS-12.

use crate::value::Value;
use std::collections::BTreeMap;

/// Speed of light (m/s).
const C: f64 = 299_792_458.0;

/// Speed of light in km/s.
const C_KM_S: f64 = C / 1000.0;

/// 1 Megaparsec in meters.
pub const MPC_IN_METERS: f64 = 3.085677581e22;

/// Hubble constant in km/s/Mpc. Planck 2018 value: H₀ ≈ 67.4 km/s/Mpc.
const H0_KM_S_MPC: f64 = 67.4;

/// Convert Hubble constant from km/s/Mpc to 1/s.
fn h0_per_second() -> f64 {
    H0_KM_S_MPC * 1000.0 / MPC_IN_METERS
}

/// Curvature parameter k: -1 (open), 0 (flat), +1 (closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Curvature {
    Open,
    Flat,
    Closed,
}

impl Curvature {
    /// Curvature constant.
    pub fn k(&self) -> f64 {
        match self {
            Self::Open => -1.0,
            Self::Flat => 0.0,
            Self::Closed => 1.0,
        }
    }
}

/// Failure states for cosmological comoving intercept calculation (Packet OCS-12).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlrwInterceptError {
    /// Interceptor speed fraction $\beta = v / c_0$ must satisfy $0 < \beta \le 1.0$.
    InvalidBeta(f64),
    /// Cosmological scale factor $a(t)$ or Hubble parameter $H(t)$ is non-positive or non-finite.
    InvalidCosmologicalEpoch { scale_factor: f64, hubble_param: f64 },
    /// Target recession velocity exceeds or equals interceptor speed ($v_{\text{H}} \ge \beta c_0$).
    BeyondCosmologicalHorizon {
        recession_velocity_km_s: f64,
        max_interceptor_speed_km_s: f64,
        hubble_horizon_mpc: f64,
    },
    /// Target peculiar + Hubble motion has no real positive intercept time with the given $\beta$.
    NoInterceptSolution,
}

/// Explicit comoving intercept solution with stated cosmological epoch and trajectory assumptions (OCS-12).
#[derive(Debug, Clone, PartialEq)]
pub struct ComovingInterceptSolution {
    /// Unit direction vector in comoving coordinates toward the intercept point.
    pub unit_direction: [f64; 3],
    /// Comoving coordinates of the intercept point (Mpc).
    pub intercept_comoving_pos_mpc: [f64; 3],
    /// Coordinate time to intercept (s) in the local comoving frame at epoch $a(t)$.
    pub time_to_intercept_s: f64,
    /// Cosmological scale factor $a(t_0)$ at launch epoch.
    pub launch_scale_factor: f64,
    /// Estimated scale factor $a(t_{\text{intercept}}) \approx a(t_0)(1 + H \Delta t)$ at arrival.
    pub arrival_scale_factor_est: f64,
    /// Hubble recession velocity of the target at launch epoch (km/s).
    pub hubble_recession_km_s: f64,
    /// Explicit trajectory assumption tag (`"LinearizedComovingConstantBeta"`).
    pub trajectory_assumption: &'static str,
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

    /// Compute a comoving intercept vector and arrival solution for a target at
    /// `target_comoving_pos_mpc` with peculiar velocity `target_peculiar_vel_km_s`
    /// pursued by an interceptor traveling at speed $v_{\text{int}} = \beta c_0$ (Packet OCS-12).
    ///
    /// ### Cosmological Epoch & Trajectory Assumptions
    /// - Evaluates physical separation $\mathbf{r}_0 = a(t_0)\,\boldsymbol{\chi}_0$ at epoch $a(t_0)$.
    /// - Target effective velocity in the local comoving chart combines Hubble flow and peculiar velocity:
    ///   $\mathbf{v}_{\text{eff}} = \mathbf{v}_{\text{pec}} / a(t_0)$ in comoving coordinates, while the
    ///   physical radial recession check enforces $v_{\text{H}} = H(t_0) \|\mathbf{r}_0\| < \beta c_0$.
    /// - Solves the quadratic intercept condition $\|\mathbf{r}_0 + \mathbf{v}_{\text{eff}} \Delta t\|^2 = (\beta_{\text{eff}} c_0 \Delta t)^2$
    ///   where $\beta_{\text{eff}} c_0 = \beta c_0 - v_{\text{H}}$ accounts for Hubble expansion along the line of sight.
    pub fn intercept_vector_comoving(
        &self,
        target_comoving_pos_mpc: [f64; 3],
        target_peculiar_vel_km_s: [f64; 3],
        interceptor_beta: f64,
    ) -> Result<ComovingInterceptSolution, FlrwInterceptError> {
        if !interceptor_beta.is_finite() || interceptor_beta <= 0.0 || interceptor_beta > 1.0 {
            return Err(FlrwInterceptError::InvalidBeta(interceptor_beta));
        }
        if !self.scale_factor.is_finite()
            || self.scale_factor <= 0.0
            || !self.hubble_param.is_finite()
            || self.hubble_param < 0.0
        {
            return Err(FlrwInterceptError::InvalidCosmologicalEpoch {
                scale_factor: self.scale_factor,
                hubble_param: self.hubble_param,
            });
        }

        let r_comoving_mpc = (target_comoving_pos_mpc[0] * target_comoving_pos_mpc[0]
            + target_comoving_pos_mpc[1] * target_comoving_pos_mpc[1]
            + target_comoving_pos_mpc[2] * target_comoving_pos_mpc[2])
            .sqrt();

        if r_comoving_mpc < 1e-15 {
            return Ok(ComovingInterceptSolution {
                unit_direction: [1.0, 0.0, 0.0],
                intercept_comoving_pos_mpc: target_comoving_pos_mpc,
                time_to_intercept_s: 0.0,
                launch_scale_factor: self.scale_factor,
                arrival_scale_factor_est: self.scale_factor,
                hubble_recession_km_s: 0.0,
                trajectory_assumption: "LinearizedComovingConstantBeta",
            });
        }

        let phys_dist_m = self.scale_factor * r_comoving_mpc * MPC_IN_METERS;
        let hubble_recession_km_s = self.hubble_velocity(phys_dist_m) / 1000.0;
        let max_interceptor_speed_km_s = interceptor_beta * C_KM_S;
        let hubble_horizon_mpc = if self.hubble_param > 0.0 {
            (interceptor_beta * C) / (self.hubble_param * MPC_IN_METERS)
        } else {
            f64::INFINITY
        };

        if hubble_recession_km_s >= max_interceptor_speed_km_s {
            return Err(FlrwInterceptError::BeyondCosmologicalHorizon {
                recession_velocity_km_s: hubble_recession_km_s,
                max_interceptor_speed_km_s,
                hubble_horizon_mpc,
            });
        }

        // Convert comoving position to km and solve in comoving-scaled physical km coordinates
        let mpc_to_km = MPC_IN_METERS / 1000.0;
        let p0_km = [
            target_comoving_pos_mpc[0] * self.scale_factor * mpc_to_km,
            target_comoving_pos_mpc[1] * self.scale_factor * mpc_to_km,
            target_comoving_pos_mpc[2] * self.scale_factor * mpc_to_km,
        ];
        // Effective closing speed after subtracting Hubble expansion rate
        let v_close_km_s = max_interceptor_speed_km_s - hubble_recession_km_s;
        let v_pec = target_peculiar_vel_km_s;

        // Solve |p0 + v_pec * t|^2 = (v_close * t)^2
        // (v_pec·v_pec - v_close^2) t^2 + 2 (p0·v_pec) t + (p0·p0) = 0
        let v_pec_sq = v_pec[0] * v_pec[0] + v_pec[1] * v_pec[1] + v_pec[2] * v_pec[2];
        let p0_sq = p0_km[0] * p0_km[0] + p0_km[1] * p0_km[1] + p0_km[2] * p0_km[2];
        let p0_dot_v = p0_km[0] * v_pec[0] + p0_km[1] * v_pec[1] + p0_km[2] * v_pec[2];

        let a_quad = v_pec_sq - v_close_km_s * v_close_km_s;
        let b_quad = 2.0 * p0_dot_v;
        let c_quad = p0_sq;

        let t_s = if a_quad.abs() < 1e-12 {
            if b_quad >= 0.0 {
                return Err(FlrwInterceptError::NoInterceptSolution);
            }
            -c_quad / b_quad
        } else {
            let disc = b_quad * b_quad - 4.0 * a_quad * c_quad;
            if disc < 0.0 {
                return Err(FlrwInterceptError::NoInterceptSolution);
            }
            let sqrt_disc = disc.sqrt();
            let t1 = (-b_quad - sqrt_disc) / (2.0 * a_quad);
            let t2 = (-b_quad + sqrt_disc) / (2.0 * a_quad);
            match (t1 > 0.0, t2 > 0.0) {
                (true, true) => t1.min(t2),
                (true, false) => t1,
                (false, true) => t2,
                (false, false) => return Err(FlrwInterceptError::NoInterceptSolution),
            }
        };

        if !t_s.is_finite() || t_s <= 0.0 {
            return Err(FlrwInterceptError::NoInterceptSolution);
        }

        let intercept_phys_km = [
            p0_km[0] + v_pec[0] * t_s,
            p0_km[1] + v_pec[1] * t_s,
            p0_km[2] + v_pec[2] * t_s,
        ];
        let norm_km = (intercept_phys_km[0] * intercept_phys_km[0]
            + intercept_phys_km[1] * intercept_phys_km[1]
            + intercept_phys_km[2] * intercept_phys_km[2])
            .sqrt();
        if norm_km < 1e-12 {
            return Err(FlrwInterceptError::NoInterceptSolution);
        }

        let unit_direction = [
            intercept_phys_km[0] / norm_km,
            intercept_phys_km[1] / norm_km,
            intercept_phys_km[2] / norm_km,
        ];
        let inv_scale_mpc = 1.0 / (self.scale_factor * mpc_to_km);
        let intercept_comoving_pos_mpc = [
            intercept_phys_km[0] * inv_scale_mpc,
            intercept_phys_km[1] * inv_scale_mpc,
            intercept_phys_km[2] * inv_scale_mpc,
        ];
        let arrival_scale_factor_est = self.scale_factor * (1.0 + self.hubble_param * t_s);

        Ok(ComovingInterceptSolution {
            unit_direction,
            intercept_comoving_pos_mpc,
            time_to_intercept_s: t_s,
            launch_scale_factor: self.scale_factor,
            arrival_scale_factor_est,
            hubble_recession_km_s,
            trajectory_assumption: "LinearizedComovingConstantBeta",
        })
    }

    /// Converts to value.
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

/// Top-level helper for [`FlrwMetric::intercept_vector_comoving`] (Packet OCS-12).
pub fn intercept_vector_comoving(
    metric: &FlrwMetric,
    target_comoving_pos_mpc: [f64; 3],
    target_peculiar_vel_km_s: [f64; 3],
    interceptor_beta: f64,
) -> Result<ComovingInterceptSolution, FlrwInterceptError> {
    metric.intercept_vector_comoving(
        target_comoving_pos_mpc,
        target_peculiar_vel_km_s,
        interceptor_beta,
    )
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
    let distance_mpc = distance_m / MPC_IN_METERS;
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
    /// Creates a new `FloatingOrigin`.
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
        let distance_m = distance_mpc * MPC_IN_METERS;
        let without_floating_origin = distance_m / (1u64 << 52) as f64;
        let local_offset_m = 1.0 * MPC_IN_METERS;
        let with_floating_origin = local_offset_m / (1u64 << 52) as f64;
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
        m.scale_factor = 0.5;
        let d = m.physical_distance(100.0);
        assert!((d - 50.0).abs() < 1e-10, "got {} expected 50.0", d);
    }

    #[test]
    fn redshift_from_scale_factor() {
        let m = FlrwMetric::flat_present_epoch();
        let z = m.redshift(0.5);
        assert!((z - 1.0).abs() < 1e-10, "got {} expected 1.0", z);
    }

    #[test]
    fn redshift_zero_at_present() {
        let m = FlrwMetric::flat_present_epoch();
        let z = m.redshift(1.0);
        assert!(z.abs() < 1e-10);
    }

    #[test]
    fn hubble_velocity_linear() {
        let m = FlrwMetric::flat_present_epoch();
        let v = m.hubble_velocity(1.0e22);
        assert!(v > 0.0);
        let v_1mpc = m.hubble_velocity(MPC_IN_METERS) / 1000.0;
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
        let error = origin.precision_error_at(500.0);
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
        let ds2 = m.spatial_line_element(2.0, std::f64::consts::PI / 2.0);
        assert!((ds2 - 8.0).abs() < 1e-10, "got {} expected 8.0", ds2);
    }

    #[test]
    fn intercept_vector_comoving_valid_and_horizon_failures() {
        let m = FlrwMetric::flat_present_epoch();
        // Nearby galaxy at 10 Mpc along +X with modest transverse peculiar velocity (+300 km/s in Y)
        // Interceptor at beta = 0.1 (30,000 km/s >> H0 * 10 Mpc ≈ 674 km/s)
        let sol = intercept_vector_comoving(&m, [10.0, 0.0, 0.0], [0.0, 300.0, 0.0], 0.1).unwrap();
        assert!(sol.time_to_intercept_s > 0.0);
        assert!(sol.unit_direction[0] > 0.99);
        assert!(sol.unit_direction[1] > 0.0); // Leads the target in +Y
        assert_eq!(sol.launch_scale_factor, 1.0);
        assert!(sol.arrival_scale_factor_est > 1.0);
        assert_eq!(
            sol.trajectory_assumption,
            "LinearizedComovingConstantBeta"
        );

        // Target at 1000 Mpc (v_H ≈ 67,400 km/s) cannot be intercepted by beta = 0.1 (30,000 km/s)
        let horizon_err =
            intercept_vector_comoving(&m, [1000.0, 0.0, 0.0], [0.0, 0.0, 0.0], 0.1).unwrap_err();
        assert!(matches!(
            horizon_err,
            FlrwInterceptError::BeyondCosmologicalHorizon { .. }
        ));

        // Invalid beta (superluminal > 1.0 or <= 0.0) must fail closed
        assert_eq!(
            intercept_vector_comoving(&m, [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1.2).unwrap_err(),
            FlrwInterceptError::InvalidBeta(1.2)
        );
    }
}
