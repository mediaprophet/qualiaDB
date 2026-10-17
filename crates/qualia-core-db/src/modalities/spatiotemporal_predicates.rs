//! Spatiotemporal Datetime Predicates & Relativistic Coordinate Modality.
//!
//! Provides coordinate-anchored temporal reasoning across micro-to-macro scales:
//! relativistic light-travel offsets, gravitational potential proper time gradients,
//! quantum biological decoherence rates, temporal mereology, and ecological persistence horizons.
//!
//! Reference: `docs/plans/holo/Spatiotemporal Datetime Predicates in QualiaDB.md`

use crate::NQuin;

/// Physical constants
pub const SPEED_OF_LIGHT_M_S: f64 = 299_792_458.0;
pub const STANDARD_GRAVITY_M_S2: f64 = 9.80665;
pub const C_SQUARED: f64 = SPEED_OF_LIGHT_M_S * SPEED_OF_LIGHT_M_S;

/// Modality opcodes (0x70–0x7F reserved for Spatiotemporal according to AGENTS.md)
pub const OP_SPATIOTEMPORAL_ANCHOR: u8 = 0x70;      // observer:referenceFrame / anchor
pub const OP_TIME_AT_OBSERVER: u8 = 0x71;           // observer:timeAtObserver
pub const OP_PROPER_TIME_GRADIENT: u8 = 0x72;       // quantum:properTimeGradient
pub const OP_SPATIAL_DECOHERENCE: u8 = 0x73;        // quantum:spatialDecoherenceRate
pub const OP_TEMPORAL_CONTAINMENT: u8 = 0x74;       // mereology:temporalContainment
pub const OP_PERSISTENCE_HORIZON: u8 = 0x75;        // environmental:persistenceHorizon
pub const OP_ECOLOGICAL_RESIDUE: u8 = 0x76;         // environmental:ecologicalResidue
pub const OP_BIOMAGNIFICATION_LAG: u8 = 0x77;       // environmental:biomagnificationLag
pub const OP_SPATIOTEMPORAL_COINCIDENCE: u8 = 0x78; // topology:spatiallyCoincident
pub const OP_ASYMMETRIC_RECURRENCE: u8 = 0x79;      // temporal:asymmetricRecurrence
pub const OP_LATENT_EPOCH: u8 = 0x7A;               // biological:latentEpoch
pub const OP_CIRCADIAN_OVERRIDE: u8 = 0x7B;         // causality:circadianEntrainmentOverride
pub const OP_MEREOLOGY_LIFESPAN: u8 = 0x7C;         // mereology:constituentLifespan
pub const OP_INDIGENOUS_EPOCH: u8 = 0x7D;           // ecology:indigenousEpoch
pub const OP_INTRODUCTION_EVENT: u8 = 0x7E;         // ecology:introductionEvent
pub const OP_NATURALIZATION_ONSET: u8 = 0x7F;       // ecology:naturalizationOnset

/// Observer status at perception time
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObserverStatus {
    Stationary = 0,
    InTransit = 1,
    Accelerating = 2,
    Obstructed = 3,
}

/// A precise spatiotemporal point anchoring a nanosecond timestamp to physical 3D space.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatiotemporalPoint {
    /// 3D coordinates (x, y, z) in meters relative to datum origin
    pub pos_xyz_m: [f64; 3],
    /// Geodetic Spatial Reference ID (e.g. 7844 for GDA2020, 4326 for WGS84, 5711 for AHD)
    pub srid_code: u32,
    /// Absolute POSIX timestamp in integer nanoseconds (10⁻⁹ s since UNIX epoch)
    pub unix_time_nanos: u64,
    /// Altitude above vertical datum in meters (AHD) for gravitational dilation
    pub elevation_m: f64,
    /// Velocity vector in meters/sec for kinematic Lorentz dilation
    pub velocity_m_s: [f64; 3],
    /// Unique identifier (FNV-1a hash) of the spatial reference frame
    pub reference_frame_id: u64,
    /// Observer status at timestamp
    pub observer_status: ObserverStatus,
}

impl Default for SpatiotemporalPoint {
    /// Origin at epoch 0 in GDA2020 (SRID 7844), stationary, no
    /// elevation/velocity, unresolved frame (`reference_frame_id` 0).
    fn default() -> Self {
        Self {
            pos_xyz_m: [0.0; 3],
            srid_code: 7844, // GDA2020
            unix_time_nanos: 0,
            elevation_m: 0.0,
            velocity_m_s: [0.0; 3],
            reference_frame_id: 0,
            observer_status: ObserverStatus::Stationary,
        }
    }
}

impl SpatiotemporalPoint {
    /// A stationary point at `(x, y, z)` metres in spatial reference
    /// `srid` at `nanos` ns since Unix epoch — velocity zeroed, frame
    /// unresolved, `Stationary` status.
    pub fn new(x: f64, y: f64, z: f64, srid: u32, nanos: u64, elevation_m: f64) -> Self {
        Self {
            pos_xyz_m: [x, y, z],
            srid_code: srid,
            unix_time_nanos: nanos,
            elevation_m,
            velocity_m_s: [0.0; 3],
            reference_frame_id: 0,
            observer_status: ObserverStatus::Stationary,
        }
    }

    /// Evaluates scalar speed in m/s.
    #[inline]
    pub fn speed_m_s(&self) -> f64 {
        let v2 = self.velocity_m_s[0] * self.velocity_m_s[0]
            + self.velocity_m_s[1] * self.velocity_m_s[1]
            + self.velocity_m_s[2] * self.velocity_m_s[2];
        v2.sqrt()
    }
}

/// Evaluates light-travel offset between source and observer in nanoseconds.
///
/// $\Delta t = \frac{\|\mathbf{x}_{\text{obs}} - \mathbf{x}_{\text{src}}\|}{c} \times 10^9$
#[inline]
pub fn evaluate_light_travel_offset_nanos(
    source: &SpatiotemporalPoint,
    observer: &SpatiotemporalPoint,
) -> u64 {
    let dx = observer.pos_xyz_m[0] - source.pos_xyz_m[0];
    let dy = observer.pos_xyz_m[1] - source.pos_xyz_m[1];
    let dz = observer.pos_xyz_m[2] - source.pos_xyz_m[2];
    let distance_m = (dx * dx + dy * dy + dz * dz).sqrt();
    let seconds = distance_m / SPEED_OF_LIGHT_M_S;
    (seconds * 1_000_000_000.0).round() as u64
}

/// Evaluates the true source time given an observer timestamp and their respective positions.
///
/// $t_{\text{src}} = t_{\text{obs}} - \Delta t_{\text{light}}$
#[inline]
pub fn evaluate_observed_source_time(
    observer_time_nanos: u64,
    source: &SpatiotemporalPoint,
    observer: &SpatiotemporalPoint,
) -> u64 {
    let offset_nanos = evaluate_light_travel_offset_nanos(source, observer);
    observer_time_nanos.saturating_sub(offset_nanos)
}

/// Evaluates General Relativistic proper time rate $d\tau / dt$ at a given elevation and speed.
///
/// Combines gravitational blueshift ($+gh/c^2$) and kinematic dilation ($-v^2/2c^2$):
/// $\frac{d\tau}{dt} \approx 1.0 + \frac{g \cdot h}{c^2} - \frac{v^2}{2 c^2}$
#[inline]
pub fn evaluate_proper_time_rate(elevation_ahd_m: f64, speed_m_s: f64) -> f64 {
    let grav_term = (STANDARD_GRAVITY_M_S2 * elevation_ahd_m) / C_SQUARED;
    let kin_term = (speed_m_s * speed_m_s) / (2.0 * C_SQUARED);
    1.0 + grav_term - kin_term
}

/// Evaluates net proper time drift in nanoseconds relative to standard sea-level coordinate time
/// over a given elapsed duration in seconds.
///
/// $\Delta \tau_{\text{drift}} = ( \frac{d\tau}{dt} - 1.0 ) \times \text{duration} \times 10^9$
#[inline]
pub fn evaluate_proper_time_drift_nanos(
    elevation_ahd_m: f64,
    speed_m_s: f64,
    duration_seconds: f64,
) -> f64 {
    let rate = evaluate_proper_time_rate(elevation_ahd_m, speed_m_s);
    (rate - 1.0) * duration_seconds * 1_000_000_000.0
}

/// Evaluates quantum proper time gradient across microscopic nodes.
///
/// $t_{\text{localized}} = \text{round}(t_{\text{base}} \times \text{dilation\_factor})$
#[inline]
pub fn evaluate_quantum_gradient_nanos(base_time_nanos: u64, dilation_factor: f64) -> u64 {
    ((base_time_nanos as f64) * dilation_factor).round() as u64
}

/// Evaluates an anthropogenic chemical or pesticide persistence horizon in nanoseconds.
///
/// $t_{\text{horizon}} = t_{\text{application}} + (\text{half\_lives} \times t_{\text{half\_life}})$
#[inline]
pub fn evaluate_persistence_horizon_nanos(
    app_time_nanos: u64,
    half_life_nanos: u64,
    half_lives: u32,
) -> u64 {
    app_time_nanos.saturating_add((half_lives as u64).saturating_mul(half_life_nanos))
}

/// Evaluates temporal collision and exposure window between two spatiotemporal intervals
/// using Allen's Interval Algebra.
///
/// Returns `Some((exposure_start, exposure_end))` if the intervals overlap, or `None`.
#[inline]
pub fn evaluate_spatiotemporal_exposure_window(
    subject_span: (u64, u64),
    contaminant_span: (u64, u64),
) -> Option<(u64, u64)> {
    let (sub_start, sub_end) = subject_span;
    let (con_start, con_end) = contaminant_span;

    // Allen interval overlap condition: sub_start <= con_end && sub_end >= con_start
    if sub_start <= con_end && sub_end >= con_start {
        let start = sub_start.max(con_start);
        let end = sub_end.min(con_end);
        Some((start, end))
    } else {
        None
    }
}

/// Evaluates temporal mereology containment: asserts that a child's lifecycle
/// falls entirely within the parent organism or structure's lifespan.
#[inline]
pub fn evaluate_temporal_containment(
    child_span: (u64, u64),
    parent_span: (u64, u64),
) -> bool {
    child_span.0 >= parent_span.0 && child_span.1 <= parent_span.1
}

/// Evaluates spatial coincidence: checks if two 3D positions are within an epsilon tolerance.
#[inline]
pub fn evaluate_spatial_coincidence(
    pos_a: &[f64; 3],
    pos_b: &[f64; 3],
    tolerance_m: f64,
) -> bool {
    let dx = pos_a[0] - pos_b[0];
    let dy = pos_a[1] - pos_b[1];
    let dz = pos_a[2] - pos_b[2];
    (dx * dx + dy * dy + dz * dz) <= (tolerance_m * tolerance_m)
}

/// Compiles a coordinate-anchored spatiotemporal fact into a 48-byte Super-Quin (`NQuin`).
///
/// - `subject`: DID hash of the spatial entity or work
/// - `predicate`: `opcode` in bits [0..7], property hash in [8..62]
/// - `object`: nanosecond timestamp or scalar payload
/// - `context`: world / bounding-box context DID hash
/// - `metadata`: Lamport logical clock in [32..60], opcode in [0..7]
/// - `parity`: XOR fold of subject ^ predicate ^ object ^ context
pub fn compile_spatiotemporal_quin(
    entity_did_hash: u64,
    property_hash: u64,
    opcode: u8,
    timestamp_nanos: u64,
    context_did_hash: u64,
    lamport_clock: u32,
) -> NQuin {
    let predicate = ((property_hash & 0x007F_FFFF_FFFF_FFFF) << 8) | (opcode as u64);
    let metadata = ((lamport_clock as u64 & 0x1FFF_FFFF) << 32) | (opcode as u64);
    let parity = entity_did_hash ^ predicate ^ timestamp_nanos ^ context_did_hash;

    NQuin {
        subject: entity_did_hash,
        predicate,
        object: timestamp_nanos,
        context: context_did_hash,
        metadata,
        parity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_light_travel_offset_and_source_time() {
        let src = SpatiotemporalPoint::new(0.0, 0.0, 0.0, 7844, 1_000_000_000, 385.0);
        // Observer located 299,792.458 meters away (~300 km) -> exact 1 millisecond (1,000,000 ns)
        let obs = SpatiotemporalPoint::new(SPEED_OF_LIGHT_M_S / 1000.0, 0.0, 0.0, 7844, 0, 385.0);

        let dt_nanos = evaluate_light_travel_offset_nanos(&src, &obs);
        assert_eq!(dt_nanos, 1_000_000); // 1.0 ms in nanoseconds

        let obs_time = 2_000_000_000u64;
        let deduced_src_time = evaluate_observed_source_time(obs_time, &src, &obs);
        assert_eq!(deduced_src_time, 1_999_000_000u64);
    }

    #[test]
    fn test_gravitational_and_kinematic_proper_time_dilation() {
        // At sea level stationary: dtau/dt = 1.0
        let rate_sea_level = evaluate_proper_time_rate(0.0, 0.0);
        assert!((rate_sea_level - 1.0).abs() < 1e-15);

        // At Mount Kosciuszko summit (2228m elevation): clocks tick faster
        let rate_summit = evaluate_proper_time_rate(2228.0, 0.0);
        assert!(rate_summit > 1.0);
        let fractional_advance = rate_summit - 1.0;
        // ~2.43e-13 advance per second
        assert!(fractional_advance > 2.0e-13 && fractional_advance < 3.0e-13);

        // Drift over 1 day (86,400 seconds) at Kosciuszko summit vs sea level
        let drift_day_nanos = evaluate_proper_time_drift_nanos(2228.0, 0.0, 86400.0);
        // ~21 nanoseconds/day faster
        assert!(drift_day_nanos > 15.0 && drift_day_nanos < 25.0);
    }

    #[test]
    fn test_quantum_proper_time_gradient() {
        let base_nanos = 1_729_123_456_789_000_000u64;
        let dilation_factor = 1.0000000000000002;
        let target_nanos = evaluate_quantum_gradient_nanos(base_nanos, dilation_factor);
        assert!(target_nanos >= base_nanos);
    }

    #[test]
    fn test_persistence_horizon_and_allen_exposure() {
        let app_time = 1_000_000_000u64;
        let half_life = 40_000_000u64; // e.g. 40ms half-life for test
        let horizon = evaluate_persistence_horizon_nanos(app_time, half_life, 5); // 5 half-lives
        assert_eq!(horizon, 1_200_000_000);

        // Case 1: Crop planted during active residue
        let crop_span = (1_050_000_000, 1_300_000_000);
        let residue_span = (app_time, horizon);
        let exposure = evaluate_spatiotemporal_exposure_window(crop_span, residue_span);
        assert_eq!(exposure, Some((1_050_000_000, 1_200_000_000)));

        // Case 2: Crop planted after residue horizon has passed
        let late_crop = (1_250_000_000, 1_400_000_000);
        let no_exposure = evaluate_spatiotemporal_exposure_window(late_crop, residue_span);
        assert_eq!(no_exposure, None);
    }

    #[test]
    fn test_temporal_mereology_containment() {
        let host_lifespan = (1_000_000_000u64, 5_000_000_000u64);
        let rbc_lifespan = (2_000_000_000u64, 2_120_000_000u64); // Red blood cell ~120 days
        assert!(evaluate_temporal_containment(rbc_lifespan, host_lifespan));

        let out_of_bounds = (900_000_000u64, 2_000_000_000u64);
        assert!(!evaluate_temporal_containment(out_of_bounds, host_lifespan));
    }

    #[test]
    fn test_spatial_coincidence() {
        let pos1 = [148.29, -35.78, 350.0];
        let pos2 = [148.29, -35.78, 350.005]; // 5mm away
        assert!(evaluate_spatial_coincidence(&pos1, &pos2, 0.01)); // within 1cm
        assert!(!evaluate_spatial_coincidence(&pos1, &pos2, 0.001)); // outside 1mm
    }

    #[test]
    fn test_compile_spatiotemporal_quin() {
        let quin = compile_spatiotemporal_quin(
            0x1234_5678,
            0x9ABC_DEF0,
            OP_SPATIOTEMPORAL_ANCHOR,
            1_744_675_200_000_000_000,
            0xFEED_FACE,
            42,
        );
        assert_eq!(quin.subject, 0x1234_5678);
        assert_eq!(quin.object, 1_744_675_200_000_000_000);
        assert_eq!((quin.predicate & 0xFF) as u8, OP_SPATIOTEMPORAL_ANCHOR);
        assert_eq!(
            quin.parity,
            quin.subject ^ quin.predicate ^ quin.object ^ quin.context
        );
    }
}
