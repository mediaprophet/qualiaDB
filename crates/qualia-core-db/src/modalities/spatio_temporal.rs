// Epic 21: Spatio-Temporal Logics
// Allen's Interval Algebra & RCC8 Spatial Relations

use crate::NQuin;

/// Allen's Interval Algebra operations for temporal reasoning
pub enum TemporalOp {
    Before,
    Meets,
    Overlaps,
    Starts,
    During,
    Finishes,
    Equals,
}

/// RCC8 spatial relations for topological reasoning
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rcc8Relation {
    /// Region A is disconnected from Region B
    Disconnected,
    /// Region A is externally connected to Region B (touches at boundary)
    ExternallyConnected,
    /// Region A is partially overlapping with Region B
    PartiallyOverlapping,
    /// Region A is tangentially proper part of Region B (touches boundary)
    TangentiallyProperPart,
    /// Region A is tangentially proper part inverse of Region B
    TangentiallyProperPartInverse,
    /// Region A is non-tangential proper part of Region B (completely inside)
    NonTangentialProperPart,
    /// Region A is non-tangential proper part inverse of Region B
    NonTangentialProperPartInverse,
    /// Region A is equal to Region B
    Equal,
}

/// Spatial region representation for RCC8 reasoning
#[derive(Debug, Clone)]
pub struct SpatialRegion {
    pub region_id: u64,
    pub boundary_points: Vec<(f64, f64)>, // Simplified boundary representation
    pub centroid: (f64, f64),
    pub area: f64,
}

impl SpatialRegion {
    /// Create a new spatial region from boundary points
    pub fn new(region_id: u64, boundary_points: Vec<(f64, f64)>) -> Self {
        let centroid = Self::compute_centroid(&boundary_points);
        let area = Self::compute_area(&boundary_points);
        
        Self {
            region_id,
            boundary_points,
            centroid,
            area,
        }
    }
    
    /// Compute centroid of polygon (simplified)
    fn compute_centroid(points: &[(f64, f64)]) -> (f64, f64) {
        if points.is_empty() {
            return (0.0, 0.0);
        }
        
        let (sum_x, sum_y) = points.iter().fold((0.0, 0.0), |(sx, sy), (x, y)| {
            (sx + x, sy + y)
        });
        
        (sum_x / points.len() as f64, sum_y / points.len() as f64)
    }
    
    /// Compute area using shoelace formula (simplified)
    fn compute_area(points: &[(f64, f64)]) -> f64 {
        if points.len() < 3 {
            return 0.0;
        }
        
        let mut area = 0.0;
        for i in 0..points.len() {
            let j = (i + 1) % points.len();
            area += points[i].0 * points[j].1;
            area -= points[j].0 * points[i].1;
        }
        
        area.abs() / 2.0
    }
    
    /// Check if point is inside region (ray casting algorithm)
    pub fn contains_point(&self, point: (f64, f64)) -> bool {
        if self.boundary_points.len() < 3 {
            return false;
        }
        
        let mut inside = false;
        let (x, y) = point;
        let n = self.boundary_points.len();
        
        for i in 0..n {
            let j = (i + 1) % n;
            let (xi, yi) = self.boundary_points[i];
            let (xj, yj) = self.boundary_points[j];
            
            if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
                inside = !inside;
            }
        }
        
        inside
    }
    
    /// Check if this region intersects with another region
    pub fn intersects(&self, other: &SpatialRegion) -> bool {
        // Simplified intersection check using bounding boxes
        let self_bounds = self.get_bounding_box();
        let other_bounds = other.get_bounding_box();
        
        !(self_bounds.0 > other_bounds.1 || self_bounds.1 < other_bounds.0 ||
          self_bounds.2 > other_bounds.3 || self_bounds.3 < other_bounds.2)
    }
    
    /// Get bounding box (min_x, max_x, min_y, max_y)
    fn get_bounding_box(&self) -> (f64, f64, f64, f64) {
        if self.boundary_points.is_empty() {
            return (0.0, 0.0, 0.0, 0.0);
        }
        
        let (min_x, max_x) = self.boundary_points.iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), (x, _)| {
                (min.min(*x), max.max(*x))
            });
            
        let (min_y, max_y) = self.boundary_points.iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), (_, y)| {
                (min.min(*y), max.max(*y))
            });
        
        (min_x, max_x, min_y, max_y)
    }
}

/// Evaluate RCC8 spatial relation between two regions
pub fn evaluate_rcc8(region_a: &SpatialRegion, region_b: &SpatialRegion) -> Rcc8Relation {
    // Check for equality first
    if region_a.region_id == region_b.region_id {
        return Rcc8Relation::Equal;
    }
    
    // Check if regions intersect
    let intersects = region_a.intersects(region_b);
    
    if !intersects {
        return Rcc8Relation::Disconnected;
    }
    
    // Check if one region is completely inside the other
    let a_inside_b = region_a.boundary_points.iter()
        .all(|&point| region_b.contains_point(point));
    let b_inside_a = region_b.boundary_points.iter()
        .all(|&point| region_a.contains_point(point));
    
    if a_inside_b && b_inside_a {
        Rcc8Relation::Equal
    } else if a_inside_b {
        // Check if boundaries touch
        let boundaries_touch = check_boundary_touch(region_a, region_b);
        if boundaries_touch {
            Rcc8Relation::TangentiallyProperPart
        } else {
            Rcc8Relation::NonTangentialProperPart
        }
    } else if b_inside_a {
        let boundaries_touch = check_boundary_touch(region_a, region_b);
        if boundaries_touch {
            Rcc8Relation::TangentiallyProperPartInverse
        } else {
            Rcc8Relation::NonTangentialProperPartInverse
        }
    } else {
        // Neither region is fully inside the other; they do intersect (bounding
        // boxes overlap per the check above).  check_boundary_touch returns true
        // when a boundary point of one region lies in the *interior* of the other,
        // which means the interiors genuinely overlap → PartiallyOverlapping.
        // If no boundary point penetrates the other's interior the regions can
        // only share a boundary curve without interior overlap → ExternallyConnected.
        let interior_overlap = check_boundary_touch(region_a, region_b);
        if interior_overlap {
            Rcc8Relation::PartiallyOverlapping
        } else {
            Rcc8Relation::ExternallyConnected
        }
    }
}

/// Check if two regions touch at their boundaries
fn check_boundary_touch(region_a: &SpatialRegion, region_b: &SpatialRegion) -> bool {
    // Simplified boundary touch check
    // In practice, this would use more sophisticated geometric algorithms
    for &point_a in &region_a.boundary_points {
        if region_b.contains_point(point_a) {
            return true;
        }
    }
    
    for &point_b in &region_b.boundary_points {
        if region_a.contains_point(point_b) {
            return true;
        }
    }
    
    false
}

// ── Zero-heap RCC-8 over bounded boundary-point slices ───────────────────────────
//
// A region is represented as N boundary-point quins
//   (region_id, q_hash("spatial:boundary"), pack_point(x, y))   [metadata = vertex seq]
// so full-polygon RCC-8 runs allocation-free over caller-supplied stack slices,
// fitting the 48-byte NQuin model (no Vec, no SpatialRegion on this path).

/// Max vertices per region on the zero-heap RCC-8 path.
pub const MAX_BOUNDARY_POINTS: usize = 64;

/// Fixed-point scale for packing a vertex coordinate (6 decimal places).
const POINT_SCALE: f64 = 1_000_000.0;
const GEO_EPS: f64 = 1e-9;

/// Pack a 2-D vertex into a u64 object field: signed fixed-point x in the high 32
/// bits, y in the low 32 bits. Handles negative coordinates (lat/long).
pub fn pack_point(x: f64, y: f64) -> u64 {
    let xi = (x * POINT_SCALE).round() as i32 as u32 as u64;
    let yi = (y * POINT_SCALE).round() as i32 as u32 as u64;
    (xi << 32) | yi
}

/// Inverse of [`pack_point`].
pub fn unpack_point(packed: u64) -> (f64, f64) {
    let xi = (packed >> 32) as u32 as i32;
    let yi = (packed & 0xFFFF_FFFF) as u32 as i32;
    (xi as f64 / POINT_SCALE, yi as f64 / POINT_SCALE)
}

fn bbox(poly: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    let mut mnx = f64::INFINITY;
    let mut mxx = f64::NEG_INFINITY;
    let mut mny = f64::INFINITY;
    let mut mxy = f64::NEG_INFINITY;
    for &(x, y) in poly {
        mnx = mnx.min(x);
        mxx = mxx.max(x);
        mny = mny.min(y);
        mxy = mxy.max(y);
    }
    (mnx, mxx, mny, mxy)
}

fn bbox_overlap(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
    let (amnx, amxx, amny, amxy) = bbox(a);
    let (bmnx, bmxx, bmny, bmxy) = bbox(b);
    !(amnx > bmxx || amxx < bmnx || amny > bmxy || amxy < bmny)
}

/// Ray-casting point-in-polygon (interior, exclusive of the boundary) over a slice.
fn point_in_interior(p: (f64, f64), poly: &[(f64, f64)]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let (x, y) = p;
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let j = (i + 1) % n;
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
    }
    inside
}

/// True if `p` lies on an edge of `poly` (within epsilon) — i.e. on the boundary.
fn point_on_boundary(p: (f64, f64), poly: &[(f64, f64)]) -> bool {
    let n = poly.len();
    if n < 2 {
        return false;
    }
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        // collinear (cross product ~ 0) and within the segment's bounding box.
        let cross = (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
        if cross.abs() <= GEO_EPS
            && p.0 >= a.0.min(b.0) - GEO_EPS
            && p.0 <= a.0.max(b.0) + GEO_EPS
            && p.1 >= a.1.min(b.1) - GEO_EPS
            && p.1 <= a.1.max(b.1) + GEO_EPS
        {
            return true;
        }
    }
    false
}

#[inline]
fn inside_or_on(p: (f64, f64), poly: &[(f64, f64)]) -> bool {
    point_in_interior(p, poly) || point_on_boundary(p, poly)
}

fn all_inside_or_on(pts: &[(f64, f64)], poly: &[(f64, f64)]) -> bool {
    !pts.is_empty() && pts.iter().all(|&p| inside_or_on(p, poly))
}

fn any_on_boundary(pts: &[(f64, f64)], poly: &[(f64, f64)]) -> bool {
    pts.iter().any(|&p| point_on_boundary(p, poly))
}

fn interiors_overlap(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
    a.iter().any(|&p| point_in_interior(p, b)) || b.iter().any(|&p| point_in_interior(p, a))
}

/// Zero-heap, full-polygon RCC-8 over two boundary-vertex slices (+ region ids).
/// Correctly distinguishes tangential (TPP/TPPi) from non-tangential (NTPP/NTPPi)
/// proper parts via a point-on-boundary test — unlike `evaluate_rcc8`, which
/// conflated them. Allocation-free.
pub fn evaluate_rcc8_points(
    id_a: u64,
    a: &[(f64, f64)],
    id_b: u64,
    b: &[(f64, f64)],
) -> Rcc8Relation {
    if id_a == id_b {
        return Rcc8Relation::Equal;
    }
    if !bbox_overlap(a, b) {
        return Rcc8Relation::Disconnected;
    }
    let a_in_b = all_inside_or_on(a, b);
    let b_in_a = all_inside_or_on(b, a);
    if a_in_b && b_in_a {
        return Rcc8Relation::Equal;
    }
    if a_in_b {
        return if any_on_boundary(a, b) {
            Rcc8Relation::TangentiallyProperPart
        } else {
            Rcc8Relation::NonTangentialProperPart
        };
    }
    if b_in_a {
        return if any_on_boundary(b, a) {
            Rcc8Relation::TangentiallyProperPartInverse
        } else {
            Rcc8Relation::NonTangentialProperPartInverse
        };
    }
    if interiors_overlap(a, b) {
        Rcc8Relation::PartiallyOverlapping
    } else {
        // bounding boxes overlap and a boundary point touches, but interiors do not.
        Rcc8Relation::ExternallyConnected
    }
}

/// Evaluate temporal relation using Allen's Interval Algebra
pub fn evaluate_temporal(
    op: TemporalOp,
    t1_start: i64,
    t1_end: i64,
    t2_start: i64,
    t2_end: i64,
) -> bool {
    match op {
        TemporalOp::Before => t1_end < t2_start,
        TemporalOp::Meets => t1_end == t2_start,
        TemporalOp::Overlaps => t1_start < t2_start && t1_end > t2_start && t1_end < t2_end,
        TemporalOp::Starts => t1_start == t2_start && t1_end < t2_end,
        TemporalOp::During => t1_start > t2_start && t1_end < t2_end,
        TemporalOp::Finishes => t1_end == t2_end && t1_start > t2_start,
        TemporalOp::Equals => t1_start == t2_start && t1_end == t2_end,
    }
}

/// Fixed-point scale for encoding centroid and area into the 64-bit object field.
/// Centroid components use bits [63:48] and [47:32] (16 bits each, ×SPATIAL_SCALE).
/// Area uses bits [31:0] (32 bits, ×SPATIAL_SCALE).
/// This preserves three decimal places of precision for component values < 65.535.
const SPATIAL_SCALE: f64 = 1_000.0;

/// Convert spatial region to NQuin for storage in graph.
///
/// The `region_id` is stored directly as the `subject` so that `quin_to_region`
/// can recover it exactly.  The predicate carries the semantic type stamp.
pub fn region_to_quin(region: &SpatialRegion, context: u64) -> NQuin {
    let subject = region.region_id;
    let predicate = crate::q_hash("has_spatial_region");

    // Pack centroid and area using fixed-point encoding (×SPATIAL_SCALE)
    // so that fractional values survive the integer round-trip.
    let cx = (region.centroid.0 * SPATIAL_SCALE).round() as u64;
    let cy = (region.centroid.1 * SPATIAL_SCALE).round() as u64;
    let ar = (region.area       * SPATIAL_SCALE).round() as u64;

    let object = ((cx & 0xFFFF) << 48) | ((cy & 0xFFFF) << 32) | (ar & 0xFFFF_FFFF);

    let mut quin = NQuin {
        subject,
        predicate,
        object,
        context,
        metadata: 0,
        parity: 0,
    };

    // Set parity for validation
    quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context ^ quin.metadata;

    quin
}

/// Extract spatial region from NQuin.
pub fn quin_to_region(quin: &NQuin) -> Option<SpatialRegion> {
    // Decode fixed-point fields (÷SPATIAL_SCALE) to recover fractional values.
    let centroid_x = ((quin.object >> 48) & 0xFFFF) as f64 / SPATIAL_SCALE;
    let centroid_y = ((quin.object >> 32) & 0xFFFF) as f64 / SPATIAL_SCALE;
    let area       = ( quin.object        & 0xFFFF_FFFF) as f64 / SPATIAL_SCALE;

    // region_id is stored directly in the subject field.
    let region_id = quin.subject;

    Some(SpatialRegion {
        region_id,
        boundary_points: vec![], // Boundary points stored separately in practice
        centroid: (centroid_x, centroid_y),
        area,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rcc8_basic_relations() {
        let region_a = SpatialRegion::new(1, vec![(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)]);
        let region_b = SpatialRegion::new(2, vec![(1.0, 1.0), (3.0, 1.0), (3.0, 3.0), (1.0, 3.0)]);

        let relation = evaluate_rcc8(&region_a, &region_b);
        assert_eq!(relation, Rcc8Relation::PartiallyOverlapping);
    }

    #[test]
    fn test_rcc8_points_zero_heap() {
        // Big square A = [0,10]^2; small square B = [3,7]^2 strictly inside A.
        let big = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let small = [(3.0, 3.0), (7.0, 3.0), (7.0, 7.0), (3.0, 7.0)];
        // B is a NON-tangential proper part of A (strictly inside, no shared boundary).
        assert_eq!(evaluate_rcc8_points(1, &small, 2, &big), Rcc8Relation::NonTangentialProperPart);
        assert_eq!(evaluate_rcc8_points(2, &big, 1, &small), Rcc8Relation::NonTangentialProperPartInverse);

        // Disjoint squares → Disconnected.
        let far = [(20.0, 20.0), (22.0, 20.0), (22.0, 22.0), (20.0, 22.0)];
        assert_eq!(evaluate_rcc8_points(1, &big, 3, &far), Rcc8Relation::Disconnected);

        // Partially overlapping squares.
        let a = [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)];
        let b = [(2.0, 2.0), (6.0, 2.0), (6.0, 6.0), (2.0, 6.0)];
        assert_eq!(evaluate_rcc8_points(1, &a, 2, &b), Rcc8Relation::PartiallyOverlapping);

        // Same region id → Equal.
        assert_eq!(evaluate_rcc8_points(5, &a, 5, &b), Rcc8Relation::Equal);

        // pack/unpack round-trips (incl. negative coords).
        let (x, y) = unpack_point(pack_point(-45.123456, 170.654321));
        assert!((x + 45.123456).abs() < 1e-5 && (y - 170.654321).abs() < 1e-5);
    }
    
    #[test]
    fn test_temporal_relations() {
        assert!(evaluate_temporal(TemporalOp::Before, 0, 10, 15, 25));
        assert!(evaluate_temporal(TemporalOp::Meets, 0, 10, 10, 20));
        assert!(evaluate_temporal(TemporalOp::Overlaps, 0, 15, 10, 25));
        assert!(evaluate_temporal(TemporalOp::During, 5, 15, 0, 25));
    }
    
    #[test]
    fn test_region_quin_conversion() {
        let region = SpatialRegion::new(42, vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);
        let quin = region_to_quin(&region, 123);
        let extracted = quin_to_region(&quin).unwrap();
        
        assert_eq!(extracted.region_id, region.region_id);
        assert_eq!(extracted.centroid, region.centroid);
        assert_eq!(extracted.area, region.area);
    }
}
