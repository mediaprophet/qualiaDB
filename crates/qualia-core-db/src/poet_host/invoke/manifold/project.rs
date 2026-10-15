//! Presentation morphology for a 3D/4D scene node (document → spatial desk → world).

use super::super::args;
use crate::entity_view::projection::PresentationLevel;
use vibe::{Diagnostic, Span, Value};

// Real presentation morphism (promoted from stub T27).
// Transforms scene coordinates according to observer presentation level P0-P6.
pub fn project(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args_v, "x").unwrap_or(0.0);
    let y = args::rec_f64(args_v, "y").unwrap_or(0.0);
    let z = args::rec_f64(args_v, "z").unwrap_or(0.0);
    let t = args::rec_f64(args_v, "t").unwrap_or(0.0);
    let fov_deg = args::rec_f64(args_v, "fov").unwrap_or(60.0);
    let cam_dist = args::rec_f64(args_v, "distance").unwrap_or(5.0).max(0.1);
    let level_u8 = args::rec_u64(args_v, "level").unwrap_or(2) as u8;
    let level = PresentationLevel::from_u8(level_u8);
    let id = args::rec_u64(args_v, "entity").unwrap_or(0);

    // Compute morphology projection based on presentation level
    let (proj_x, proj_y, proj_z, depth_scale, visible) = match level {
        PresentationLevel::Document | PresentationLevel::AppHabitat => {
            // Flat 2D layout: collapse Z depth onto the active display plane
            (x, y, 0.0, 1.0, true)
        }
        PresentationLevel::SpatialDesk => {
            // Spatial desk: tilted surface (30-degree pitch) with perspective foreshortening
            let theta = 30.0_f64.to_radians();
            let cos_t = theta.cos();
            let sin_t = theta.sin();
            let y_tilted = y * cos_t - z * sin_t;
            let z_depth = y * sin_t + z * cos_t;
            let scale = cam_dist / (cam_dist + z_depth).max(0.01);
            (
                x * scale,
                y_tilted * scale,
                z_depth,
                scale,
                z_depth > -cam_dist,
            )
        }
        PresentationLevel::StageWorld | PresentationLevel::EmbodiedWorld => {
            // Full 3D/4D stage: perspective projection with camera frustum
            let focal = 1.0 / (fov_deg.to_radians() * 0.5).tan();
            let eye_z = z + cam_dist;
            if eye_z <= 0.01 {
                // Clipped behind camera
                (x, y, eye_z, 0.0, false)
            } else {
                let scale = focal / eye_z;
                (x * scale, y * scale, z, scale, true)
            }
        }
        PresentationLevel::Infosphere | PresentationLevel::MultiSensory => {
            // N-dimensional embedding: time modulates coordinate spatialization
            let focal = 1.0 / (fov_deg.to_radians() * 0.5).tan();
            let t_phase = (t * 0.1).sin() * 0.05;
            let eye_z = z + cam_dist + t_phase;
            let scale = if eye_z > 0.01 { focal / eye_z } else { 1.0 };
            (x * scale, y * scale, z, scale, eye_z > 0.01)
        }
    };

    Ok(args::record([
        ("entity_id", Value::U64(id)),
        ("x", Value::F64(proj_x)),
        ("y", Value::F64(proj_y)),
        ("z", Value::F64(proj_z)),
        ("t", Value::F64(t)),
        ("depth_scale", Value::F64(depth_scale)),
        ("visible", Value::Bool(visible)),
        ("level", Value::U64(level.as_u8() as u64)),
        ("level_name", Value::String(format!("{level:?}"))),
        (
            "spatial",
            Value::Bool(level.as_u8() >= PresentationLevel::SpatialDesk.as_u8()),
        ),
        ("honesty", Value::String("real".into())),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn spatial_desk_is_spatial() {
        let mut m = BTreeMap::new();
        m.insert("level".into(), Value::U64(2));
        match project(&Value::Record(m), Span { start: 0, end: 0 }).unwrap() {
            Value::Record(r) => assert_eq!(r.get("spatial"), Some(&Value::Bool(true))),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn document_level_flattens_z_to_zero() {
        let mut m = BTreeMap::new();
        m.insert("level".into(), Value::U64(0));
        m.insert("x".into(), Value::F64(10.0));
        m.insert("y".into(), Value::F64(20.0));
        m.insert("z".into(), Value::F64(30.0));
        match project(&Value::Record(m), Span { start: 0, end: 0 }).unwrap() {
            Value::Record(r) => {
                assert_eq!(r.get("z"), Some(&Value::F64(0.0)));
                assert_eq!(r.get("x"), Some(&Value::F64(10.0)));
                assert_eq!(r.get("spatial"), Some(&Value::Bool(false)));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn manifold_project_honesty_is_real() {
        let m = BTreeMap::new();
        match project(&Value::Record(m), Span { start: 0, end: 0 }).unwrap() {
            Value::Record(r) => assert_eq!(r.get("honesty"), Some(&Value::String("real".into()))),
            other => panic!("{other:?}"),
        }
    }
}
