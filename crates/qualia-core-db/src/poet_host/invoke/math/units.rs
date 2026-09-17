//! Physical unit conversion — `solvers::units::conversion`.
//!
//! Wraps the engine's dimensionally-checked `convert`. The solver owns the
//! affine scale math (linear factor + offset for Celsius/Fahrenheit) and the
//! fail-closed dimension check; this seam maps unit names to `Unit` consts and
//! marshals `Value`. The name→`Unit` table mirrors
//! `wasm_bridge/engine/units.rs::unit_by_name` exactly (same aliases), so the
//! native invoke path and the WASM export accept the same vocabulary.

use super::super::args;
use crate::solvers::units::conversion::{convert, Unit};
use crate::solvers::units::UnitsError;
use vibe::{Diagnostic, Span, Value};

/// Resolve a unit name (canonical symbol or ASCII alias) to its `Unit` const.
/// Replicates `wasm_bridge/engine/units.rs::unit_by_name`.
fn unit_by_name(name: &str) -> Option<Unit> {
    let u = match name {
        // length
        "m" | "metre" | "meter" => Unit::METRE,
        "km" | "kilometre" | "kilometer" => Unit::KILOMETRE,
        "cm" | "centimetre" | "centimeter" => Unit::CENTIMETRE,
        "mm" | "millimetre" | "millimeter" => Unit::MILLIMETRE,
        "in" | "inch" => Unit::INCH,
        "ft" | "foot" | "feet" => Unit::FOOT,
        "mi" | "mile" => Unit::MILE,
        // mass
        "kg" | "kilogram" => Unit::KILOGRAM,
        "g" | "gram" => Unit::GRAM,
        "lb" | "pound" => Unit::POUND,
        // time
        "s" | "sec" | "second" => Unit::SECOND,
        "min" | "minute" => Unit::MINUTE,
        "h" | "hr" | "hour" => Unit::HOUR,
        // force / energy / pressure
        "N" | "newton" => Unit::NEWTON,
        "J" | "joule" => Unit::JOULE,
        "kWh" | "kwh" => Unit::KILOWATT_HOUR,
        "Pa" | "pascal" => Unit::PASCAL,
        "bar" => Unit::BAR,
        // temperature
        "K" | "kelvin" => Unit::KELVIN,
        "°C" | "C" | "degC" | "celsius" => Unit::CELSIUS,
        "°F" | "F" | "degF" | "fahrenheit" => Unit::FAHRENHEIT,
        _ => return None,
    };
    Some(u)
}

fn map_err(span: Span, e: UnitsError) -> Diagnostic {
    args::bad(span, format!("{e}"))
}

/// Convert a magnitude between two named units of the same physical dimension.
/// Affine (Celsius/Fahrenheit) and linear scales are both handled. Fails closed
/// on an unknown unit name or incompatible dimensions (e.g. `m` → `s`).
///
/// Input: record `{ value: f64, from: string, to: string }`.
/// Output: record `{ value: f64, from: string, to: string }` (canonical symbols).
pub fn convert_unit(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let value = args::rec_f64(args_v, "value")
        .ok_or_else(|| args::bad(span, "convert_unit needs value: a number"))?;
    let from_name = args::rec_str(args_v, "from")
        .ok_or_else(|| args::bad(span, "convert_unit needs from: a unit name"))?;
    let to_name = args::rec_str(args_v, "to")
        .ok_or_else(|| args::bad(span, "convert_unit needs to: a unit name"))?;
    let from = unit_by_name(from_name)
        .ok_or_else(|| args::bad(span, format!("unknown unit: {:?}", from_name)))?;
    let to = unit_by_name(to_name)
        .ok_or_else(|| args::bad(span, format!("unknown unit: {:?}", to_name)))?;
    let converted = convert(value, &from, &to).map_err(|e| map_err(span, e))?;
    Ok(args::record([
        ("value", Value::F64(converted)),
        ("from", Value::String(from.name.to_string())),
        ("to", Value::String(to.name.to_string())),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn case(value: f64, from: &str, to: &str) -> Value {
        let mut m = BTreeMap::new();
        m.insert("value".into(), Value::F64(value));
        m.insert("from".into(), Value::String(from.into()));
        m.insert("to".into(), Value::String(to.into()));
        Value::Record(m)
    }

    #[test]
    fn inches_to_centimetres() {
        let v = convert_unit(&case(1.0, "in", "cm"), Span { start: 0, end: 0 }).unwrap();
        match v {
            Value::Record(r) => match r.get("value") {
                Some(Value::F64(x)) => assert!((x - 2.54).abs() < 1e-6),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn celsius_to_fahrenheit_affine() {
        // 100 °C = 212 °F
        let v = convert_unit(&case(100.0, "C", "F"), Span { start: 0, end: 0 }).unwrap();
        match v {
            Value::Record(r) => match r.get("value") {
                Some(Value::F64(x)) => assert!((x - 212.0).abs() < 1e-3),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn unknown_unit_fails_closed() {
        assert!(convert_unit(&case(1.0, "smoot", "m"), Span { start: 0, end: 0 }).is_err());
    }

    #[test]
    fn incompatible_dimensions_fail_closed() {
        // m → s: length vs time.
        assert!(convert_unit(&case(1.0, "m", "s"), Span { start: 0, end: 0 }).is_err());
    }
}
