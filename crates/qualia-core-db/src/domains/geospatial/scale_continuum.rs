//! Scale-Continuum Engine (Phase 1.5)
//! 
//! Manages scale-driven LOD transitions across the macro-verse (Earth) and
//! micro-verse (anatomy/cellular), leveraging the hierarchical ReferenceFrame.

use crate::domains::geospatial::reference_frame::ReferenceFrame;

/// Determines if a scale transition is required based on the current camera scale
/// factor and the active reference frame. 
/// 
/// Threshold heuristic: If the observer zooms in by more than 1000x relative
/// to the frame's base scale, we should transition to a child frame (micro-verse).
pub fn requires_micro_transition(camera_scale: f64, active_frame: &ReferenceFrame) -> bool {
    let scale_ratio = camera_scale / active_frame.scale;
    scale_ratio > 1000.0
}

/// Evaluates cross-scale RCC-8 (Region Connection Calculus) containment.
/// 
/// True if a target coordinate in a given reference frame is structurally
/// "Inside" or "CoveredBy" the region defined by the parent frame.
/// 
/// Zero-heap constraint: Uses stack allocations and bounded iterations.
pub fn is_contained_in_parent(
    local_pt: [f64; 3], 
    frame: &ReferenceFrame, 
    parent_bounds_min: [f64; 3], 
    parent_bounds_max: [f64; 3]
) -> bool {
    let global_pt = frame.transform_to_parent(local_pt);
    
    global_pt[0] >= parent_bounds_min[0] && global_pt[0] <= parent_bounds_max[0] &&
    global_pt[1] >= parent_bounds_min[1] && global_pt[1] <= parent_bounds_max[1] &&
    global_pt[2] >= parent_bounds_min[2] && global_pt[2] <= parent_bounds_max[2]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::geospatial::reference_frame::ReferenceFrame;

    #[test]
    fn test_requires_micro_transition() {
        let frame = ReferenceFrame::new(1, None); // default scale is 1.0
        
        assert!(!requires_micro_transition(100.0, &frame));
        assert!(requires_micro_transition(1001.0, &frame));
    }

    #[test]
    fn test_is_contained_in_parent() {
        let mut frame = ReferenceFrame::new(2, Some(1));
        frame.translation = [10.0, 10.0, 10.0];
        frame.scale = 0.1;

        let local_pt = [5.0, 0.0, 0.0];
        // global pt = [10.5, 10.0, 10.0]

        let min_bounds = [0.0, 0.0, 0.0];
        let max_bounds = [20.0, 20.0, 20.0];

        assert!(is_contained_in_parent(local_pt, &frame, min_bounds, max_bounds));

        // Outside parent bounds
        let out_bounds = [0.0, 0.0, 1.0];
        assert!(!is_contained_in_parent(local_pt, &frame, min_bounds, out_bounds));
    }
}
