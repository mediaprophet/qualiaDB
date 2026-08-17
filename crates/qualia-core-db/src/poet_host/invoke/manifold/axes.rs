//! `.10d` axis-role taxonomy — what a 3D/4D UI may treat as distance vs selector.

use super::super::args;
use crate::container_10d::axis_role::{AxisRole, AXIS_ORDER, COORDINATE_AXES, SELECTOR_AXES};
use poet_vibe::{Diagnostic, Span, Value};

pub fn taxonomy(_args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let roles: Vec<Value> = AXIS_ORDER
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let role = if COORDINATE_AXES.contains(&i) {
                if i == crate::container_10d::axis_role::MU_AXIS {
                    AxisRole::CoordinateCarrier
                } else {
                    AxisRole::Coordinate
                }
            } else if SELECTOR_AXES.contains(&i) {
                AxisRole::Selector
            } else {
                AxisRole::Undefined
            };
            args::record([
                ("axis", Value::String((*name).into())),
                ("index", Value::U64(i as u64)),
                ("role", Value::String(format!("{role:?}"))),
                ("in_distance", Value::Bool(role.is_coordinate())),
            ])
        })
        .collect();
    Ok(Value::List(roles))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_axes() {
        match taxonomy(&Value::Null, Span { start: 0, end: 0 }).unwrap() {
            Value::List(xs) => assert_eq!(xs.len(), 10),
            other => panic!("{other:?}"),
        }
    }
}
