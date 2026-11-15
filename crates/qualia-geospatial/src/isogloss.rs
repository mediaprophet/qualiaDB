//! Diachronic Isogloss Mapping & Geospatial Sonic Toponym Streamer.
//!
//! Projects dynamic linguistic variety boundaries (isoglosses) and acoustic wave-fields
//! over 3D terrain geometry, linking oral mother-tongues, sacred liturgical chants,
//! and cadastral toponyms to geodetic coordinates.

use crate::frame::{GeodeticCoord, GeodeticFrame};
pub use qualia_core_db::modalities::spatiotemporal_linguistics::{
    SpatioTemporalLect, TimeFrequencyBox,
};

/// Diachronic Isogloss boundary representing the spatial and temporal distribution
/// of a linguistic feature or lect variety over a geographical region.
#[derive(Debug, Clone, PartialEq)]
pub struct DiachronicIsogloss {
    /// Lect variety identifier
    pub lect_id_hash: u64,
    /// Bounding polygon center coordinate in geodetic frame
    pub center_coord: GeodeticCoord,
    /// Spatial radius in meters from center where the feature is active
    pub radius_m: f64,
    /// Historical era start year (e.g. 1850.0)
    pub era_start_year: f32,
    /// Historical era end year (e.g. 1920.0)
    pub era_end_year: f32,
}

impl DiachronicIsogloss {
    pub fn new(
        lect_id_hash: u64,
        center_coord: GeodeticCoord,
        radius_m: f64,
        era_start_year: f32,
        era_end_year: f32,
    ) -> Self {
        Self {
            lect_id_hash,
            center_coord,
            radius_m,
            era_start_year,
            era_end_year,
        }
    }

    /// Evaluates whether a target coordinate and timestamp fall within this isogloss.
    pub fn contains_point(
        &self,
        coord: &GeodeticCoord,
        year: f32,
        frame: &GeodeticFrame,
    ) -> bool {
        if year < self.era_start_year || year > self.era_end_year {
            return false;
        }

        let (x1_mm, y1_mm, _) = frame.project_to_local_mm(&self.center_coord);
        let (x2_mm, y2_mm, _) = frame.project_to_local_mm(coord);

        let dx_m = ((x2_mm - x1_mm) as f64) / 1000.0;
        let dy_m = ((y2_mm - y1_mm) as f64) / 1000.0;
        let dist_m = (dx_m * dx_m + dy_m * dy_m).sqrt();

        dist_m <= self.radius_m
    }

    /// Evaluates acoustic prosody attenuation weight $w \in [0.0, 1.0]$ based on radial distance.
    pub fn evaluate_attenuation_weight(
        &self,
        coord: &GeodeticCoord,
        year: f32,
        frame: &GeodeticFrame,
    ) -> f64 {
        if year < self.era_start_year || year > self.era_end_year {
            return 0.0;
        }

        let (x1_mm, y1_mm, _) = frame.project_to_local_mm(&self.center_coord);
        let (x2_mm, y2_mm, _) = frame.project_to_local_mm(coord);

        let dx_m = ((x2_mm - x1_mm) as f64) / 1000.0;
        let dy_m = ((y2_mm - y1_mm) as f64) / 1000.0;
        let dist_m = (dx_m * dx_m + dy_m * dy_m).sqrt();

        if dist_m >= self.radius_m {
            0.0
        } else {
            // Smooth cosine bell falloff
            let norm_dist = dist_m / self.radius_m;
            0.5 * (1.0 + (norm_dist * std::f64::consts::PI).cos())
        }
    }
}

/// Geospatial Sonic Toponym Streamer binding raw physical acoustic wave-fields
/// to 3D terrain coordinates, with terrain-aware acoustic propagation.
#[derive(Debug, Clone, PartialEq)]
pub struct SonicToponymStreamer {
    /// Geodesic anchor coordinate of the sonic toponym / sacred site
    pub site_coord: GeodeticCoord,
    /// Time-frequency bounds of the acoustic recording
    pub wave_bounds: TimeFrequencyBox,
    /// Custodial Scope DID Hash
    pub custodial_scope_id: u64,
    /// Base sound pressure level in dB at 1 meter
    pub base_spl_db: f32,
}

impl SonicToponymStreamer {
    pub fn new(
        site_coord: GeodeticCoord,
        wave_bounds: TimeFrequencyBox,
        custodial_scope_id: u64,
        base_spl_db: f32,
    ) -> Self {
        Self {
            site_coord,
            wave_bounds,
            custodial_scope_id,
            base_spl_db,
        }
    }

    /// Calculates acoustic attenuation in dB at a listener coordinate over terrain.
    ///
    /// Follows the inverse-square law: $L = L_0 - 20 \log_{10}(d)$ with elevation damping.
    pub fn evaluate_listener_spl_db(
        &self,
        listener_coord: &GeodeticCoord,
        frame: &GeodeticFrame,
    ) -> f32 {
        let (x1_mm, y1_mm, z1_mm) = frame.project_to_local_mm(&self.site_coord);
        let (x2_mm, y2_mm, z2_mm) = frame.project_to_local_mm(listener_coord);

        let dx_m = (x2_mm - x1_mm) as f64 / 1000.0;
        let dy_m = (y2_mm - y1_mm) as f64 / 1000.0;
        let dz_m = (z2_mm - z1_mm) as f64 / 1000.0;

        let distance_m = (dx_m * dx_m + dy_m * dy_m + dz_m * dz_m).sqrt().max(1.0);
        let attenuation_db = 20.0 * (distance_m as f32).log10();

        (self.base_spl_db - attenuation_db).max(0.0)
    }

    /// Evaluates Doppler pitch shift factor $f' / f = 1 + v_r / c$ for a moving listener.
    pub fn evaluate_doppler_factor(
        &self,
        listener_velocity_m_s: [f64; 3],
        listener_coord: &GeodeticCoord,
        frame: &GeodeticFrame,
    ) -> f64 {
        const SPEED_OF_SOUND_M_S: f64 = 343.0; // At ~20°C in air

        let (x1_mm, y1_mm, z1_mm) = frame.project_to_local_mm(&self.site_coord);
        let (x2_mm, y2_mm, z2_mm) = frame.project_to_local_mm(listener_coord);

        let dx = (x1_mm - x2_mm) as f64 / 1000.0;
        let dy = (y1_mm - y2_mm) as f64 / 1000.0;
        let dz = (z1_mm - z2_mm) as f64 / 1000.0;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt().max(0.001);

        // Unit vector pointing towards source
        let dir = [dx / dist, dy / dist, dz / dist];

        // Radial velocity towards source: v_r = v . dir
        let v_radial = listener_velocity_m_s[0] * dir[0]
            + listener_velocity_m_s[1] * dir[1]
            + listener_velocity_m_s[2] * dir[2];

        1.0 + (v_radial / SPEED_OF_SOUND_M_S)
    }
}

