//! Geodetic Frame, Coordinate Systems, Datums & Discrete Global Grid Systems (DGGS).
//!
//! Provides coordinate semantics:
//! - Horizontal: WGS84 (`EPSG:4326`), GDA2020 (`EPSG:7844`), Web Mercator (`EPSG:3857`).
//! - Vertical: AHD (`EPSG:5711`) vs Ellipsoidal (`EPSG:4979`).
//! - DGGS Cell vocabulary and Tier-1 zero-heap geoid datum transformation.

pub const SRID_WGS84: u32 = 4326;
pub const SRID_GDA2020: u32 = 7844;
pub const SRID_WEB_MERCATOR: u32 = 3857;
pub const SRID_MGA_ZONE_55: u32 = 7855;
pub const SRID_MGA_ZONE_56: u32 = 7856;

pub const VERTICAL_DATUM_AHD: u16 = 5711;
pub const VERTICAL_DATUM_ELLIPSOIDAL: u16 = 4979;

/// Kind/provenance of observation epoch for a geodetic sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataEpochKind {
    MeasuredObservation,
    HistoricalReanalysis,
    ForecastProjection,
    SyntheticScenario,
}

/// Geodetic Coordinate in decimal degrees and altitude in millimeters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeodeticCoord {
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub elevation_ahd_mm: i32,
}

impl GeodeticCoord {
    pub const fn new(latitude_deg: f64, longitude_deg: f64, elevation_ahd_mm: i32) -> Self {
        Self { latitude_deg, longitude_deg, elevation_ahd_mm }
    }
}

/// Timestamped geodetic sample with CRS, vertical datum, and epoch metadata.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimestampedGeodeticSample {
    pub coord: GeodeticCoord,
    pub srid: u32,
    pub vertical_datum: u16,
    pub epoch_year: f32,
    pub unix_time_nanos: u64,
    pub epoch_kind: DataEpochKind,
}

/// Vertical datum transformation specification with geoid undulation N in millimeters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerticalDatumTransform {
    pub from_datum: u16,
    pub to_datum: u16,
    pub geoid_undulation_n_mm: i32,
}

/// Frame validation errors for fail-closed geodetic operations.
#[derive(Debug, Clone, PartialEq)]
pub enum FrameValidationError {
    UnsupportedSrid(u32),
    VerticalDatumMismatch { expected: u16, found: u16 },
    InvalidLatitude(f64),
    InvalidLongitude(f64),
    EpochDriftExceeded { frame_epoch: f32, sample_epoch: f32 },
    UnmarkedForecastInObservationStream,
}

impl core::fmt::Display for FrameValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedSrid(s) => write!(f, "Unsupported SRID: {s}"),
            Self::VerticalDatumMismatch { expected, found } => write!(f, "Datum mismatch: exp {expected}, got {found}"),
            Self::InvalidLatitude(lat) => write!(f, "Invalid latitude [-90, +90]: {lat}"),
            Self::InvalidLongitude(lon) => write!(f, "Invalid longitude [-180, +180]: {lon}"),
            Self::EpochDriftExceeded { frame_epoch, sample_epoch } => write!(f, "Epoch drift: frame={frame_epoch}, sample={sample_epoch}"),
            Self::UnmarkedForecastInObservationStream => write!(f, "Unmarked forecast projection in observation stream"),
        }
    }
}

impl std::error::Error for FrameValidationError {}

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
    /// Validates sample coordinate bounds, CRS, vertical datum, and epoch consistency.
    pub fn validate_sample(
        &self,
        sample: &TimestampedGeodeticSample,
        allow_forecast: bool,
    ) -> Result<(), FrameValidationError> {
        let lat = sample.coord.latitude_deg;
        if !(-90.0..=90.0).contains(&lat) || lat.is_nan() {
            return Err(FrameValidationError::InvalidLatitude(lat));
        }
        let lon = sample.coord.longitude_deg;
        if !(-180.0..=180.0).contains(&lon) || lon.is_nan() {
            return Err(FrameValidationError::InvalidLongitude(lon));
        }
        if sample.srid != self.srid {
            return Err(FrameValidationError::UnsupportedSrid(sample.srid));
        }
        if sample.vertical_datum != self.vertical_datum {
            return Err(FrameValidationError::VerticalDatumMismatch {
                expected: self.vertical_datum,
                found: sample.vertical_datum,
            });
        }
        if (sample.epoch_year - self.epoch_year).abs() > 5.0 {
            return Err(FrameValidationError::EpochDriftExceeded {
                frame_epoch: self.epoch_year,
                sample_epoch: sample.epoch_year,
            });
        }
        if !allow_forecast && sample.epoch_kind == DataEpochKind::ForecastProjection {
            return Err(FrameValidationError::UnmarkedForecastInObservationStream);
        }
        Ok(())
    }

    /// Transforms sample elevation using optional geoid undulation and projects into local tangent mm.
    pub fn transform_and_project_local_mm(
        &self,
        sample: &TimestampedGeodeticSample,
        vertical_transform: Option<&VerticalDatumTransform>,
    ) -> Result<(i64, i64, i32), FrameValidationError> {
        let lat = sample.coord.latitude_deg;
        if !(-90.0..=90.0).contains(&lat) || lat.is_nan() {
            return Err(FrameValidationError::InvalidLatitude(lat));
        }
        let lon = sample.coord.longitude_deg;
        if !(-180.0..=180.0).contains(&lon) || lon.is_nan() {
            return Err(FrameValidationError::InvalidLongitude(lon));
        }
        if sample.srid != self.srid {
            return Err(FrameValidationError::UnsupportedSrid(sample.srid));
        }
        if (sample.epoch_year - self.epoch_year).abs() > 5.0 {
            return Err(FrameValidationError::EpochDriftExceeded {
                frame_epoch: self.epoch_year,
                sample_epoch: sample.epoch_year,
            });
        }

        let target_elevation_mm = if sample.vertical_datum == self.vertical_datum {
            sample.coord.elevation_ahd_mm
        } else {
            let tx = vertical_transform.ok_or(FrameValidationError::VerticalDatumMismatch {
                expected: self.vertical_datum,
                found: sample.vertical_datum,
            })?;
            if tx.from_datum != sample.vertical_datum || tx.to_datum != self.vertical_datum {
                return Err(FrameValidationError::VerticalDatumMismatch {
                    expected: self.vertical_datum,
                    found: sample.vertical_datum,
                });
            }
            if tx.from_datum == VERTICAL_DATUM_ELLIPSOIDAL && tx.to_datum == VERTICAL_DATUM_AHD {
                sample.coord.elevation_ahd_mm - tx.geoid_undulation_n_mm
            } else if tx.from_datum == VERTICAL_DATUM_AHD && tx.to_datum == VERTICAL_DATUM_ELLIPSOIDAL {
                sample.coord.elevation_ahd_mm + tx.geoid_undulation_n_mm
            } else {
                sample.coord.elevation_ahd_mm - tx.geoid_undulation_n_mm
            }
        };

        let adjusted = GeodeticCoord { latitude_deg: lat, longitude_deg: lon, elevation_ahd_mm: target_elevation_mm };
        Ok(self.project_to_local_mm(&adjusted))
    }

    /// Projects a geodetic coordinate into local tangent plane coordinates (X, Y in mm) relative to origin.
    #[inline]
    pub fn project_to_local_mm(&self, coord: &GeodeticCoord) -> (i64, i64, i32) {
        const EARTH_RADIUS_MM: f64 = 6_378_137_000.0;
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
    pub fn to_dggs_cell_hash(&self, coord: &GeodeticCoord, resolution_level: u8) -> u64 {
        let res = resolution_level.min(15) as u32;
        let lat_norm = ((coord.latitude_deg + 90.0) / 180.0).clamp(0.0, 1.0);
        let lon_norm = ((coord.longitude_deg + 180.0) / 360.0).clamp(0.0, 1.0);
        let grid_size = 1u64 << res;
        let grid_x = (lon_norm * (grid_size as f64)).floor() as u64;
        let grid_y = (lat_norm * (grid_size as f64)).floor() as u64;

        let mut hash = 0xcbf29ce484222325u64;
        hash = (hash ^ (res as u64)).wrapping_mul(0x100000001b3);
        hash = (hash ^ grid_x).wrapping_mul(0x100000001b3);
        hash = (hash ^ grid_y).wrapping_mul(0x100000001b3);
        hash
    }

    /// Converts a geodetic coordinate to a QualiaDB `SpatiotemporalPoint`.
    pub fn to_spatiotemporal_point(
        &self,
        coord: &GeodeticCoord,
        unix_time_nanos: u64,
        velocity_m_s: [f64; 3],
    ) -> qualia_core_db::modalities::spatiotemporal_predicates::SpatiotemporalPoint {
        let (x_mm, y_mm, z_mm) = self.project_to_local_mm(coord);
        let mut pt = qualia_core_db::modalities::spatiotemporal_predicates::SpatiotemporalPoint::new(
            (x_mm as f64) / 1000.0,
            (y_mm as f64) / 1000.0,
            (z_mm as f64) / 1000.0,
            self.srid,
            unix_time_nanos,
            (coord.elevation_ahd_mm as f64) / 1000.0,
        );
        pt.velocity_m_s = velocity_m_s;
        pt.reference_frame_id = (self.srid as u64) | ((self.vertical_datum as u64) << 32);
        pt
    }

    /// Evaluates the relativistic light-travel offset in nanoseconds between two coordinates.
    pub fn evaluate_light_travel_offset_nanos(&self, coord_a: &GeodeticCoord, coord_b: &GeodeticCoord) -> u64 {
        let pt_a = self.to_spatiotemporal_point(coord_a, 0, [0.0; 3]);
        let pt_b = self.to_spatiotemporal_point(coord_b, 0, [0.0; 3]);
        qualia_core_db::modalities::spatiotemporal_predicates::evaluate_light_travel_offset_nanos(&pt_a, &pt_b)
    }

    /// Evaluates proper time drift in nanoseconds between a coordinate and sea-level AHD.
    pub fn evaluate_proper_time_drift_nanos(&self, coord: &GeodeticCoord, speed_m_s: f64, duration_seconds: f64) -> f64 {
        qualia_core_db::modalities::spatiotemporal_predicates::evaluate_proper_time_drift_nanos(
            (coord.elevation_ahd_mm as f64) / 1000.0,
            speed_m_s,
            duration_seconds,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_gda2020_ahd_projection() {
        let frame = GeodeticFrame::default();
        let sample = TimestampedGeodeticSample {
            coord: GeodeticCoord::new(-35.795, 148.315, 385_000),
            srid: SRID_GDA2020,
            vertical_datum: VERTICAL_DATUM_AHD,
            epoch_year: 2020.0,
            unix_time_nanos: 1_700_000_000_000_000_000,
            epoch_kind: DataEpochKind::MeasuredObservation,
        };
        assert!(frame.validate_sample(&sample, false).is_ok());
        let (x_mm, y_mm, z_mm) = frame.transform_and_project_local_mm(&sample, None).unwrap();
        assert!(x_mm > 0);
        assert!(y_mm < 0);
        assert_eq!(z_mm, 35_000);
    }

    #[test]
    fn test_explicit_ellipsoidal_to_ahd_transform() {
        let frame = GeodeticFrame::default();
        let sample = TimestampedGeodeticSample {
            coord: GeodeticCoord::new(-35.78, 148.29, 405_000),
            srid: SRID_GDA2020,
            vertical_datum: VERTICAL_DATUM_ELLIPSOIDAL,
            epoch_year: 2020.0,
            unix_time_nanos: 1_700_000_000_000_000_000,
            epoch_kind: DataEpochKind::MeasuredObservation,
        };
        let tx = VerticalDatumTransform {
            from_datum: VERTICAL_DATUM_ELLIPSOIDAL,
            to_datum: VERTICAL_DATUM_AHD,
            geoid_undulation_n_mm: 20_000,
        };
        let (x_mm, y_mm, z_mm) = frame.transform_and_project_local_mm(&sample, Some(&tx)).unwrap();
        assert_eq!(x_mm, 0);
        assert_eq!(y_mm, 0);
        assert_eq!(z_mm, 35_000);
    }

    #[test]
    fn test_negative_rejects_mixed_vertical_datums() {
        let frame = GeodeticFrame::default();
        let sample = TimestampedGeodeticSample {
            coord: GeodeticCoord::new(-35.78, 148.29, 405_000),
            srid: SRID_GDA2020,
            vertical_datum: VERTICAL_DATUM_ELLIPSOIDAL,
            epoch_year: 2020.0,
            unix_time_nanos: 1_700_000_000_000_000_000,
            epoch_kind: DataEpochKind::MeasuredObservation,
        };
        let err = frame.transform_and_project_local_mm(&sample, None).unwrap_err();
        assert_eq!(err, FrameValidationError::VerticalDatumMismatch {
            expected: VERTICAL_DATUM_AHD,
            found: VERTICAL_DATUM_ELLIPSOIDAL,
        });
    }

    #[test]
    fn test_negative_rejects_invalid_coordinates() {
        let frame = GeodeticFrame::default();
        let invalid_lat = TimestampedGeodeticSample {
            coord: GeodeticCoord::new(-95.0, 148.29, 350_000),
            srid: SRID_GDA2020,
            vertical_datum: VERTICAL_DATUM_AHD,
            epoch_year: 2020.0,
            unix_time_nanos: 0,
            epoch_kind: DataEpochKind::MeasuredObservation,
        };
        assert!(matches!(frame.validate_sample(&invalid_lat, false), Err(FrameValidationError::InvalidLatitude(_))));

        let invalid_lon = TimestampedGeodeticSample {
            coord: GeodeticCoord::new(-35.78, 195.0, 350_000),
            srid: SRID_GDA2020,
            vertical_datum: VERTICAL_DATUM_AHD,
            epoch_year: 2020.0,
            unix_time_nanos: 0,
            epoch_kind: DataEpochKind::MeasuredObservation,
        };
        assert!(matches!(frame.validate_sample(&invalid_lon, false), Err(FrameValidationError::InvalidLongitude(_))));
    }

    #[test]
    fn test_negative_rejects_unmarked_forecast_in_observation_stream() {
        let frame = GeodeticFrame::default();
        let forecast_sample = TimestampedGeodeticSample {
            coord: GeodeticCoord::new(-35.78, 148.29, 350_000),
            srid: SRID_GDA2020,
            vertical_datum: VERTICAL_DATUM_AHD,
            epoch_year: 2020.0,
            unix_time_nanos: 1_700_000_000_000_000_000,
            epoch_kind: DataEpochKind::ForecastProjection,
        };
        assert_eq!(frame.validate_sample(&forecast_sample, false), Err(FrameValidationError::UnmarkedForecastInObservationStream));
        assert!(frame.validate_sample(&forecast_sample, true).is_ok());
    }

    #[test]
    fn test_epoch_drift_validation() {
        let frame = GeodeticFrame::default();
        let drifted = TimestampedGeodeticSample {
            coord: GeodeticCoord::new(-35.78, 148.29, 350_000),
            srid: SRID_GDA2020,
            vertical_datum: VERTICAL_DATUM_AHD,
            epoch_year: 2028.5,
            unix_time_nanos: 0,
            epoch_kind: DataEpochKind::MeasuredObservation,
        };
        assert!(matches!(frame.validate_sample(&drifted, false), Err(FrameValidationError::EpochDriftExceeded { .. })));
    }
}
