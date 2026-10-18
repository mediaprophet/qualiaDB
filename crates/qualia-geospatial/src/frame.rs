//! Geodetic Frame, Coordinate Systems, Datums & Discrete Global Grid Systems (DGGS).
//!
//! Provides coordinate semantics:
//! - Horizontal: WGS84 (`EPSG:4326`), GDA2020 (`EPSG:7844` plate-fixed at epoch 2020.0), Web Mercator (`EPSG:3857`).
//! - Vertical: Australian Height Datum (AHD, `EPSG:5711`) vs Ellipsoidal (`EPSG:4979`).
//! - AusPix / rHEALPix DGGS Cell vocabulary for global-to-regional joins.

pub const SRID_WGS84: u32 = 4326;
pub const SRID_GDA2020: u32 = 7844;
pub const SRID_WEB_MERCATOR: u32 = 3857;
pub const SRID_MGA_ZONE_55: u32 = 7855;
pub const SRID_MGA_ZONE_56: u32 = 7856;

pub const VERTICAL_DATUM_AHD: u16 = 5711;
pub const VERTICAL_DATUM_ELLIPSOIDAL: u16 = 4979;

/// Geodetic Coordinate in decimal degrees and altitude in millimeters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeodeticCoord {
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub elevation_ahd_mm: i32,
}

impl GeodeticCoord {
    pub const fn new(latitude_deg: f64, longitude_deg: f64, elevation_ahd_mm: i32) -> Self {
        Self {
            latitude_deg,
            longitude_deg,
            elevation_ahd_mm,
        }
    }
}

/// Geodetic Reference Frame descriptor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeodeticFrame {
    pub srid: u32,
    pub vertical_datum: u16,
    pub epoch_year: f32, // e.g. 2020.0 for GDA2020
    pub origin_lat_deg: f64,
    pub origin_lon_deg: f64,
    pub origin_elevation_mm: i32,
}

impl Default for GeodeticFrame {
    fn default() -> Self {
        Self {
            srid: SRID_GDA2020,
            vertical_datum: VERTICAL_DATUM_AHD,
            epoch_year: 2020.0,
            origin_lat_deg: -35.78, // Talbingo / Snowy Mountains, NSW
            origin_lon_deg: 148.29,
            origin_elevation_mm: 350_000, // 350m AHD
        }
    }
}

impl GeodeticFrame {
    /// Projects a geodetic coordinate into local tangent plane coordinates (X, Y in mm) relative to origin.
    ///
    /// Uses equirectangular approximation valid for local chunk footprints (< 50 km).
    #[inline]
    pub fn project_to_local_mm(&self, coord: &GeodeticCoord) -> (i64, i64, i32) {
        const EARTH_RADIUS_MM: f64 = 6_378_137_000.0; // WGS84 / GRS80 semi-major axis in mm

        let lat_rad = coord.latitude_deg.to_radians();
        let origin_lat_rad = self.origin_lat_deg.to_radians();
        let mean_lat_rad = (lat_rad + origin_lat_rad) * 0.5;

        let d_lat = (coord.latitude_deg - self.origin_lat_deg).to_radians();
        let d_lon = (coord.longitude_deg - self.origin_lon_deg).to_radians();

        let y_mm = (d_lat * EARTH_RADIUS_MM) as i64;
        let x_mm = (d_lon * EARTH_RADIUS_MM * mean_lat_rad.cos()) as i64;
        let z_mm = coord.elevation_ahd_mm - self.origin_elevation_mm;

        (x_mm, y_mm, z_mm)
    }

    /// Converts local tangent plane coordinates back to geodetic lon/lat.
    #[inline]
    pub fn unproject_from_local_mm(&self, x_mm: i64, y_mm: i64) -> (f64, f64) {
        const EARTH_RADIUS_MM: f64 = 6_378_137_000.0;
        let origin_lat_rad = self.origin_lat_deg.to_radians();

        let d_lat_rad = (y_mm as f64) / EARTH_RADIUS_MM;
        let lat_deg = self.origin_lat_deg + d_lat_rad.to_degrees();

        let mean_lat_rad = (lat_deg.to_radians() + origin_lat_rad) * 0.5;
        let d_lon_rad = (x_mm as f64) / (EARTH_RADIUS_MM * mean_lat_rad.cos());
        let lon_deg = self.origin_lon_deg + d_lon_rad.to_degrees();

        (lat_deg, lon_deg)
    }

    /// Generates an AusPix / rHEALPix DGGS Cell identifier hash for cross-layer joins.
    ///
    /// `resolution_level`: 0 (continental) to 15 (~1 meter cell size).
    pub fn to_dggs_cell_hash(&self, coord: &GeodeticCoord, resolution_level: u8) -> u64 {
        let res = resolution_level.min(15) as u32;
        let lat_norm = ((coord.latitude_deg + 90.0) / 180.0).clamp(0.0, 1.0);
        let lon_norm = ((coord.longitude_deg + 180.0) / 360.0).clamp(0.0, 1.0);

        let grid_size = 1u64 << res;
        let grid_x = (lon_norm * (grid_size as f64)).floor() as u64;
        let grid_y = (lat_norm * (grid_size as f64)).floor() as u64;

        // FNV-1a hash combining resolution, grid_x, and grid_y
        let mut hash = 0xcbf29ce484222325u64;
        hash = (hash ^ (res as u64)).wrapping_mul(0x100000001b3);
        hash = (hash ^ grid_x).wrapping_mul(0x100000001b3);
        hash = (hash ^ grid_y).wrapping_mul(0x100000001b3);
        hash
    }

    /// Converts a geodetic coordinate to a QualiaDB `SpatiotemporalPoint`, referencing local tangent
    /// plane coordinates (in meters) relative to the frame origin, with elevation in AHD meters.
    pub fn to_spatiotemporal_point(
        &self,
        coord: &GeodeticCoord,
        unix_time_nanos: u64,
        velocity_m_s: [f64; 3],
    ) -> qualia_core_db::modalities::spatiotemporal_predicates::SpatiotemporalPoint {
        let (x_mm, y_mm, z_mm) = self.project_to_local_mm(coord);
        let x_m = (x_mm as f64) / 1000.0;
        let y_m = (y_mm as f64) / 1000.0;
        let z_m = (z_mm as f64) / 1000.0;
        let elevation_m = (coord.elevation_ahd_mm as f64) / 1000.0;

        let mut pt = qualia_core_db::modalities::spatiotemporal_predicates::SpatiotemporalPoint::new(
            x_m,
            y_m,
            z_m,
            self.srid,
            unix_time_nanos,
            elevation_m,
        );
        pt.velocity_m_s = velocity_m_s;
        pt.reference_frame_id = (self.srid as u64) | ((self.vertical_datum as u64) << 32);
        pt
    }

    /// Evaluates the relativistic light-travel offset in nanoseconds between two geodetic coordinates.
    pub fn evaluate_light_travel_offset_nanos(
        &self,
        coord_a: &GeodeticCoord,
        coord_b: &GeodeticCoord,
    ) -> u64 {
        let pt_a = self.to_spatiotemporal_point(coord_a, 0, [0.0; 3]);
        let pt_b = self.to_spatiotemporal_point(coord_b, 0, [0.0; 3]);
        qualia_core_db::modalities::spatiotemporal_predicates::evaluate_light_travel_offset_nanos(
            &pt_a, &pt_b,
        )
    }

    /// Evaluates the proper time drift in nanoseconds between a coordinate and standard sea-level AHD
    /// over a duration in seconds (combining gravitational potential and kinematic velocity).
    pub fn evaluate_proper_time_drift_nanos(
        &self,
        coord: &GeodeticCoord,
        speed_m_s: f64,
        duration_seconds: f64,
    ) -> f64 {
        let elevation_m = (coord.elevation_ahd_mm as f64) / 1000.0;
        qualia_core_db::modalities::spatiotemporal_predicates::evaluate_proper_time_drift_nanos(
            elevation_m,
            speed_m_s,
            duration_seconds,
        )
    }
}
