//! Bounded OSM vector geometry to local render meshes.
//!
//! This is a per-render-tile compiler, not a national GIS store. It projects
//! source WGS84 coordinates into a local ENU frame, preserves source identity
//! beside the resulting mesh, and lets callers drape geometry against a
//! separately sourced elevation field. A larger Australian place is compiled
//! as many independently streamed tiles.

use crate::domains::geospatial::adapters::osm_adapter::OsmVectorFeature;
use crate::render::assets::Mesh;

mod geometry;

const MAX_COORDINATES: usize = 250_000;
const MAX_TILE_RADIUS_M: f64 = 250_000.0;

/// Local WGS84 tangent frame and cold-build limits for one render tile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VectorSceneFrame {
    pub origin_lat: f64,
    pub origin_lon: f64,
    pub origin_alt_m: f64,
    /// Maximum horizontal distance from the origin for any source vertex.
    pub max_radius_m: f64,
    /// Optional `[min_east, min_north, max_east, max_north]` clip rectangle
    /// in metres relative to this origin. Adjacent tiles should share exact
    /// edge coordinates so independently compiled meshes meet without gaps.
    pub tile_bounds_enu: Option<[f64; 4]>,
    /// Maximum input vertices accepted for one feature.
    pub max_coordinates: usize,
}

impl VectorSceneFrame {
    pub fn new(origin_lat: f64, origin_lon: f64, origin_alt_m: f64) -> Self {
        Self {
            origin_lat,
            origin_lon,
            origin_alt_m,
            max_radius_m: 50_000.0,
            tile_bounds_enu: None,
            max_coordinates: 50_000,
        }
    }

    /// Clip emitted geometry to a tile rectangle in the frame's local ENU plane.
    pub fn with_tile_bounds(mut self, bounds_enu: [f64; 4]) -> Self {
        self.tile_bounds_enu = Some(bounds_enu);
        self
    }

    fn validate(self) -> Result<(), String> {
        if !self.origin_lat.is_finite()
            || !(-90.0..=90.0).contains(&self.origin_lat)
            || !self.origin_lon.is_finite()
            || !(-180.0..=180.0).contains(&self.origin_lon)
            || !self.origin_alt_m.is_finite()
        {
            return Err("vector scene origin is not a finite WGS84 location".into());
        }
        if !self.max_radius_m.is_finite()
            || self.max_radius_m <= 0.0
            || self.max_radius_m > MAX_TILE_RADIUS_M
        {
            return Err(format!(
                "vector scene radius must be greater than zero and at most {MAX_TILE_RADIUS_M} m"
            ));
        }
        if let Some([min_east, min_north, max_east, max_north]) = self.tile_bounds_enu {
            if ![min_east, min_north, max_east, max_north]
                .iter()
                .all(|value| value.is_finite())
                || min_east >= max_east
                || min_north >= max_north
            {
                return Err("vector tile bounds must be finite and have positive area".into());
            }
            if [
                (min_east, min_north),
                (min_east, max_north),
                (max_east, min_north),
                (max_east, max_north),
            ]
            .iter()
            .any(|(east, north)| east.hypot(*north) > self.max_radius_m)
            {
                return Err("vector tile bounds exceed the configured local radius".into());
            }
        }
        if self.max_coordinates == 0 || self.max_coordinates > MAX_COORDINATES {
            return Err(format!(
                "vector scene coordinate budget must be from 1 to {MAX_COORDINATES}"
            ));
        }
        Ok(())
    }
}

/// Game-authored presentation intent for one OSM geometry.
///
/// OSM facts and tags remain source data; these dimensions and offsets are
/// renderer policy and must not be written back as geographic facts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OsmVectorStyle {
    /// A flat circular marker centred on a mapped point.
    Marker {
        radius_m: f32,
        vertical_offset_m: f32,
    },
    /// A camera-facing-free ground ribbon for roads, streams, paths or borders.
    Line {
        width_m: f32,
        vertical_offset_m: f32,
    },
    /// A simple polygon fill. Polygons with interior rings are rejected until
    /// a hole-preserving triangulator is selected by the caller's profile.
    Area { vertical_offset_m: f32 },
}

impl OsmVectorStyle {
    fn validate(self) -> Result<(), String> {
        match self {
            Self::Marker {
                radius_m,
                vertical_offset_m,
            } if !radius_m.is_finite()
                || radius_m <= 0.0
                || radius_m > 1_000.0
                || !vertical_offset_m.is_finite() =>
            {
                Err("marker radius/offset is outside the supported finite range".into())
            }
            Self::Line {
                width_m,
                vertical_offset_m,
            } if !width_m.is_finite()
                || width_m <= 0.0
                || width_m > 2_000.0
                || !vertical_offset_m.is_finite() =>
            {
                Err("line width/offset is outside the supported finite range".into())
            }
            Self::Area { vertical_offset_m } if !vertical_offset_m.is_finite() => {
                Err("area vertical offset must be finite".into())
            }
            _ => Ok(()),
        }
    }

    fn vertical_offset_m(self) -> f32 {
        match self {
            Self::Marker {
                vertical_offset_m, ..
            }
            | Self::Line {
                vertical_offset_m, ..
            }
            | Self::Area { vertical_offset_m } => vertical_offset_m,
        }
    }
}

/// A per-feature render mesh with its OSM source facts retained alongside it.
#[derive(Debug, Clone, PartialEq)]
pub struct OsmRenderedFeature {
    pub source_iri: String,
    pub source_version: Option<u64>,
    pub source_timestamp: Option<String>,
    pub tags: std::collections::BTreeMap<String, String>,
    pub style: OsmVectorStyle,
    pub mesh: Mesh,
}

/// Compile one validated feature in a local ENU frame, using its WGS84
/// elevations as the fallback surface. Geometry with parser diagnostics is
/// refused so incomplete ways or relations cannot silently become art.
pub fn compile_osm_feature_mesh(
    feature: &OsmVectorFeature,
    frame: VectorSceneFrame,
    style: OsmVectorStyle,
) -> Result<OsmRenderedFeature, String> {
    compile_osm_feature_mesh_draped(feature, frame, style, None)
}

/// Compile one feature and sample a caller-owned local terrain surface at
/// every emitted vertex. The sampler returns metres above `frame.origin_alt_m`;
/// returning `None` rejects the feature instead of leaving an undraped gap.
///
/// The sampler is borrowed only during cold mesh construction. Terrain tile
/// ownership, CRS, datum, caching and source provenance remain with the caller.
pub fn compile_osm_feature_mesh_draped(
    feature: &OsmVectorFeature,
    frame: VectorSceneFrame,
    style: OsmVectorStyle,
    ground_height: Option<&dyn Fn(f64, f64) -> Option<f32>>,
) -> Result<OsmRenderedFeature, String> {
    frame.validate()?;
    style.validate()?;
    if let Some(issue) = feature.issues.first() {
        return Err(format!(
            "OSM feature {}/{} has geometry diagnostics: {issue}",
            feature.element_type, feature.id
        ));
    }
    let geometry = feature.geometry.as_ref().ok_or_else(|| {
        format!(
            "OSM feature {}/{} has no geometry",
            feature.element_type, feature.id
        )
    })?;
    let mesh = geometry::compile(geometry, frame, style, ground_height).map_err(|error| {
        format!(
            "OSM feature {}/{}: {error}",
            feature.element_type, feature.id
        )
    })?;
    Ok(OsmRenderedFeature {
        source_iri: feature.source_iri(),
        source_version: feature.version,
        source_timestamp: feature.timestamp.clone(),
        tags: feature.tags.clone(),
        style,
        mesh,
    })
}
