/// Minkowski Spatial Sieve for Android Asset Apportionment
///
/// This module provides the structural foundation for passing spatial logs
/// and bounding boxes to a GPU Compute Shader (e.g., via wgpu/Vulkan).
/// It calculates whether a specific financial transaction occurred within the
/// Minkowski spatial overlap of a tracked Asset (e.g., a Work Vehicle or Business Location).

#[derive(Clone, Copy)]
pub struct GeoCoordinate {
    pub lat: f64,
    pub lng: f64,
    pub timestamp_ms: u64,
}

#[derive(Clone, Copy)]
pub struct BoundingBox {
    pub min_lat: f64,
    pub min_lng: f64,
    pub max_lat: f64,
    pub max_lng: f64,
}

/// A mock vectorised representation of the GPU-accelerated Minkowski overlap check.
/// In a production environment with `wgpu`, this would dispatch a compute shader
/// that performs matrix multiplications over thousands of GeoCoordinates instantly.
pub fn compute_spatial_overlap_gpu_mock(
    route_log: &[GeoCoordinate],
    asset_bounds: BoundingBox,
) -> f64 {
    if route_log.is_empty() {
        return 0.0;
    }

    let mut overlap_count = 0;

    // Vectorized check representation
    for point in route_log {
        if point.lat >= asset_bounds.min_lat
            && point.lat <= asset_bounds.max_lat
            && point.lng >= asset_bounds.min_lng
            && point.lng <= asset_bounds.max_lng
        {
            overlap_count += 1;
        }
    }

    // Return the apportionment ratio (e.g. 50% of the trip was inside the asset bounds)
    overlap_count as f64 / route_log.len() as f64
}

/// Encode a GPS fix into two `SPATIAL_CONTEXT` quins and return them.
///
/// Quin 1: `subject=geohash64 | predicate=P_HAS_GEOMETRY | object=geohash64`
/// Quin 2: `subject=geohash64 | predicate=P_GENERATED_AT | object=ts`
///
/// The GeoHash-64 value is bit-interleaved lon/lat (32 bits each) computed
/// by `kml_bridge::encode_geohash_64`.  Callers write the returned quins to
/// storage; this function never allocates a graph writer itself.
pub fn log_spatial_coordinate(lat: f64, lng: f64, ts: u64) -> [crate::NQuin; 2] {
    use crate::kml_bridge::{encode_geohash_64, SPATIAL_CONTEXT, P_HAS_GEOMETRY, P_GENERATED_AT};
    let geohash = encode_geohash_64(lng, lat);
    [
        crate::NQuin {
            subject:   geohash,
            predicate: P_HAS_GEOMETRY,
            object:    geohash,
            context:   SPATIAL_CONTEXT,
            metadata:  ts & 0xFFFF_FFFF,
            parity:    0,
        },
        crate::NQuin {
            subject:   geohash,
            predicate: P_GENERATED_AT,
            object:    ts,
            context:   SPATIAL_CONTEXT,
            metadata:  ts & 0xFFFF_FFFF,
            parity:    0,
        },
    ]
}

/// Evaluates a recent spatial log against a claimed jurisdictional boundary
/// (e.g., verifying the user is actually physically in Australia to prevent
/// foreign scammer activity on a specific project or transaction).
pub fn verify_proof_of_location(
    recent_log: GeoCoordinate,
    jurisdiction_bounds: BoundingBox,
) -> bool {
    recent_log.lat >= jurisdiction_bounds.min_lat
        && recent_log.lat <= jurisdiction_bounds.max_lat
        && recent_log.lng >= jurisdiction_bounds.min_lng
        && recent_log.lng <= jurisdiction_bounds.max_lng
}
