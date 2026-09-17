//! Presentation morphology for a 3D/4D scene node (document → spatial desk → world).

use super::super::args;
use crate::entity_view::projection::PresentationLevel;
use vibe::{Diagnostic, Span, Value};

// STUB (T27): real presentation morphism not yet implemented.
// Currently echoes x,y,z,t. See docs/vibescript-full-impl-PLAN.md §8.4 T27.
pub fn project(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args_v, "x").unwrap_or(0.0);
    let y = args::rec_f64(args_v, "y").unwrap_or(0.0);
    let z = args::rec_f64(args_v, "z").unwrap_or(0.0);
    let t = args::rec_f64(args_v, "t").unwrap_or(0.0);
    let level = PresentationLevel::from_u8(args::rec_u64(args_v, "level").unwrap_or(2) as u8);
    let id = args::rec_u64(args_v, "entity").unwrap_or(0);
    Ok(args::record([
        ("entity_id", Value::U64(id)),
        ("x", Value::F64(x)),
        ("y", Value::F64(y)),
        ("z", Value::F64(z)),
        ("t", Value::F64(t)),
        ("level", Value::U64(level.as_u8() as u64)),
        ("level_name", Value::String(format!("{level:?}"))),
        (
            "spatial",
            Value::Bool(level.as_u8() >= PresentationLevel::SpatialDesk.as_u8()),
        ),
        ("honesty", Value::String("stub".into())),
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
    fn manifold_project_honesty_is_stub() {
        let m = BTreeMap::new();
        match project(&Value::Record(m), Span { start: 0, end: 0 }).unwrap() {
            Value::Record(r) => assert_eq!(r.get("honesty"), Some(&Value::String("stub".into()))),
            other => panic!("{other:?}"),
        }
    }
}
