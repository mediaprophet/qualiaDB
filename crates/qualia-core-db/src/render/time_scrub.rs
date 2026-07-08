//! Time-Scrub Replay Layer
//! Takes a timestamp `T` and materializes the world state at that time.
//! Uses `nodes_as_of` from the DAG and applies continuous onset/decay alpha ramps
//! to assets based on their temporal boundaries, allowing smooth scrub transitions.

use crate::NQuin;

/// A materialized scene frame at a specific temporal slice.
#[derive(Debug, Clone)]
pub struct TemporalSceneFrame {
    pub time_t: u64,
    pub visible_assets: Vec<RenderAsset>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderAsset {
    pub asset_id: u64,
    /// Temporal alpha visibility: 0.0 (invisible) to 1.0 (fully opaque)
    pub alpha: f32,
}

/// Scrub to a specific point in time and build the visible scene frame.
/// 
/// `nodes` should be the active set of NQuins as of `target_time`.
/// `ramp_window` dictates how many temporal units the fade in/out lasts.
pub fn scrub_to_time(nodes: &[NQuin], target_time: u64, ramp_window: u64) -> TemporalSceneFrame {
    let mut visible = Vec::new();
    
    for quin in nodes {
        // In a real integration, the bounds [start, end] come from the PROV-O quins.
        // We simulate reading the temporal interval from the quin metadata fields.
        if let Some((start, end)) = extract_temporal_bounds(quin) {
            if target_time >= start && target_time <= end {
                let mut alpha = 1.0;
                
                // Onset ramp (fade in)
                if target_time < start + ramp_window {
                    alpha = (target_time - start) as f32 / ramp_window as f32;
                }
                
                // Decay ramp (fade out)
                if target_time > end.saturating_sub(ramp_window) {
                    let decay = end.saturating_sub(target_time);
                    alpha = decay as f32 / ramp_window as f32;
                }
                
                visible.push(RenderAsset {
                    asset_id: quin.subject,
                    alpha,
                });
            }
        }
    }
    
    TemporalSceneFrame {
        time_t: target_time,
        visible_assets: visible,
    }
}

/// Extracts temporal validity [start, end] bounds from an NQuin.
/// Currently assumes reserved metadata bits [55:32] pack this information.
fn extract_temporal_bounds(quin: &NQuin) -> Option<(u64, u64)> {
    // For demonstration/scaffolding, we treat `metadata` as the start time, 
    // and `context` as the duration.
    let start = quin.metadata;
    let duration = quin.context;
    
    if duration > 0 {
        Some((start, start.saturating_add(duration)))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_quin(subject: u64, start: u64, duration: u64) -> NQuin {
        NQuin {
            subject,
            predicate: 0,
            object: 0,
            context: duration,
            metadata: start,
            parity: 0,
        }
    }

    #[test]
    fn test_time_scrub_alpha_ramps() {
        let nodes = vec![
            mock_quin(1, 1000, 1000), // Valid [1000, 2000]
        ];

        let ramp_window = 100;

        // Before onset
        let frame1 = scrub_to_time(&nodes, 500, ramp_window);
        assert!(frame1.visible_assets.is_empty());

        // Middle of onset ramp
        let frame2 = scrub_to_time(&nodes, 1050, ramp_window);
        assert_eq!(frame2.visible_assets.len(), 1);
        assert!((frame2.visible_assets[0].alpha - 0.5).abs() < f32::EPSILON);

        // Fully opaque
        let frame3 = scrub_to_time(&nodes, 1500, ramp_window);
        assert_eq!(frame3.visible_assets.len(), 1);
        assert_eq!(frame3.visible_assets[0].alpha, 1.0);

        // Middle of decay ramp
        let frame4 = scrub_to_time(&nodes, 1950, ramp_window);
        assert_eq!(frame4.visible_assets.len(), 1);
        assert!((frame4.visible_assets[0].alpha - 0.5).abs() < f32::EPSILON);

        // After decay
        let frame5 = scrub_to_time(&nodes, 2500, ramp_window);
        assert!(frame5.visible_assets.is_empty());
    }
}
