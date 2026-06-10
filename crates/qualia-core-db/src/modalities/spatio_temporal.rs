// Epic 21: Spatio-Temporal Logics
// Allen's Interval Algebra & RCC8 Spatial Relations

use crate::QualiaQuin;

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
        // Check if boundaries touch externally
        let boundaries_touch = check_boundary_touch(region_a, region_b);
        if boundaries_touch {
            Rcc8Relation::ExternallyConnected
        } else {
            Rcc8Relation::PartiallyOverlapping
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

/// Convert spatial region to QualiaQuin for storage in graph
pub fn region_to_quin(region: &SpatialRegion, context: u64) -> QualiaQuin {
    let subject = crate::q_hash(&format!("region_{}", region.region_id));
    let predicate = crate::q_hash("has_spatial_region");
    
    // Pack centroid and area into object field
    let object = ((region.centroid.0 as u64) << 48) |
                ((region.centroid.1 as u64) << 32) |
                ((region.area as u64) & 0xFFFFFFFF);
    
    let mut quin = QualiaQuin {
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

/// Extract spatial region from QualiaQuin
pub fn quin_to_region(quin: &QualiaQuin) -> Option<SpatialRegion> {
    // Extract centroid and area from object field
    let centroid_x = ((quin.object >> 48) & 0xFFFF) as f64;
    let centroid_y = ((quin.object >> 32) & 0xFFFF) as f64;
    let area = (quin.object & 0xFFFFFFFF) as f64;
    
    // Extract region ID from subject hash (simplified)
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
