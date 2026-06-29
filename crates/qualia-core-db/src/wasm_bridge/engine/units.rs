//! SI units, dimensional analysis & CODATA constants exports.
//!
//! Wraps the engine's wasm-clean solver math (`crate::solvers::units::*`). Same code
//! the native MCP tools and the solver unit tests exercise: a value carries its physical
//! [`Dimension`] (the 7-vector of SI base-exponents) and arithmetic is *dimensionally
//! checked* — converting metres to seconds, or multiplying/dividing quantities, all fail
//! closed on incompatible dimensions rather than fabricating a number.
#![cfg(target_arch = "wasm32")]

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use super::jserr;

use crate::solvers::units::conversion::{convert, Unit};
use crate::solvers::units::dimension::Dimension;
use crate::solvers::units::quantity::Quantity;
use crate::solvers::units::{constants, UnitsError};

// ── Unit table ──────────────────────────────────────────────────────────────
// The `Unit` consts the conversion layer defines. Looked up by the unit's own
// canonical symbol *and* a few common ASCII aliases (so callers need not type the
// degree sign). This is the exhaustive set of units the solver layer implements.
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

/// Every unit the solver layer defines, as `(canonical symbol, human label)` — drives
/// the demo dropdowns and `units_list_units`.
const UNIT_CATALOG: &[(&str, &str)] = &[
    ("m", "metre (length)"),
    ("km", "kilometre (length)"),
    ("cm", "centimetre (length)"),
    ("mm", "millimetre (length)"),
    ("in", "inch (length)"),
    ("ft", "foot (length)"),
    ("mi", "mile (length)"),
    ("kg", "kilogram (mass)"),
    ("g", "gram (mass)"),
    ("lb", "pound (mass)"),
    ("s", "second (time)"),
    ("min", "minute (time)"),
    ("h", "hour (time)"),
    ("N", "newton (force)"),
    ("J", "joule (energy)"),
    ("kWh", "kilowatt-hour (energy)"),
    ("Pa", "pascal (pressure)"),
    ("bar", "bar (pressure)"),
    ("K", "kelvin (temperature)"),
    ("°C", "degree Celsius (temperature)"),
    ("°F", "degree Fahrenheit (temperature)"),
];

// ── CODATA constants table ──────────────────────────────────────────────────
// Maps a stable name to the dimensioned `Quantity` const in `solvers::units::constants`.
fn constant_by_name(name: &str) -> Option<(Quantity, &'static str, &'static str)> {
    // (quantity, canonical symbol, human description)
    let c = match name {
        "speed_of_light" | "c" => (constants::SPEED_OF_LIGHT, "c", "speed of light in vacuum"),
        "gravitational" | "G" => (
            constants::GRAVITATIONAL,
            "G",
            "Newtonian constant of gravitation",
        ),
        "planck" | "h" => (constants::PLANCK, "h", "Planck constant"),
        "reduced_planck" | "hbar" | "ħ" => {
            (constants::REDUCED_PLANCK, "ħ", "reduced Planck constant h/2π")
        }
        "boltzmann" | "k_B" | "kB" => (constants::BOLTZMANN, "k_B", "Boltzmann constant"),
        "avogadro" | "N_A" | "NA" => (constants::AVOGADRO, "N_A", "Avogadro constant"),
        "elementary_charge" | "e" => {
            (constants::ELEMENTARY_CHARGE, "e", "elementary charge")
        }
        "gas_constant" | "R" => (constants::GAS_CONSTANT, "R", "molar gas constant N_A·k_B"),
        "stefan_boltzmann" | "sigma" | "σ" => (
            constants::STEFAN_BOLTZMANN,
            "σ",
            "Stefan–Boltzmann constant",
        ),
        "standard_gravity" | "g0" => {
            (constants::STANDARD_GRAVITY, "g₀", "standard gravity")
        }
        "standard_atmosphere" | "atm" => (
            constants::STANDARD_ATMOSPHERE,
            "atm",
            "standard atmosphere",
        ),
        "electron_mass" | "m_e" | "me" => (constants::ELECTRON_MASS, "m_e", "electron mass"),
        "proton_mass" | "m_p" | "mp" => (constants::PROTON_MASS, "m_p", "proton mass"),
        _ => return None,
    };
    Some(c)
}

/// Stable names for every CODATA constant exposed, with symbol + description — drives
/// `units_list_constants` and the demo picker.
const CONSTANT_CATALOG: &[(&str, &str, &str)] = &[
    ("speed_of_light", "c", "speed of light in vacuum"),
    ("gravitational", "G", "Newtonian constant of gravitation"),
    ("planck", "h", "Planck constant"),
    ("reduced_planck", "ħ", "reduced Planck constant h/2π"),
    ("boltzmann", "k_B", "Boltzmann constant"),
    ("avogadro", "N_A", "Avogadro constant"),
    ("elementary_charge", "e", "elementary charge"),
    ("gas_constant", "R", "molar gas constant N_A·k_B"),
    ("stefan_boltzmann", "σ", "Stefan–Boltzmann constant"),
    ("standard_gravity", "g0", "standard gravity"),
    ("standard_atmosphere", "atm", "standard atmosphere"),
    ("electron_mass", "m_e", "electron mass"),
    ("proton_mass", "m_p", "proton mass"),
];

// ── shared serialized shapes ────────────────────────────────────────────────

/// A physical dimension serialized as the SI 7-vector plus a couple of conveniences.
#[derive(Serialize)]
struct DimensionOut {
    /// `[length, mass, time, current, temperature, amount, luminosity]` exponents.
    exponents: [i8; 7],
    /// Labelled exponents, for readability in the demo.
    length: i8,
    mass: i8,
    time: i8,
    current: i8,
    temperature: i8,
    amount: i8,
    luminosity: i8,
    /// `true` when every exponent is zero (a pure number).
    dimensionless: bool,
}

impl From<Dimension> for DimensionOut {
    fn from(d: Dimension) -> Self {
        let e = d.exponents;
        DimensionOut {
            exponents: e,
            length: e[0],
            mass: e[1],
            time: e[2],
            current: e[3],
            temperature: e[4],
            amount: e[5],
            luminosity: e[6],
            dimensionless: d.is_dimensionless(),
        }
    }
}

fn map_units_err(e: UnitsError) -> JsValue {
    JsValue::from_str(&format!("{e}"))
}

// ── exports ─────────────────────────────────────────────────────────────────

/// Convert a magnitude between two named units of the **same** physical dimension.
/// Affine (Celsius/Fahrenheit) and linear scales are both handled. Fails closed if the
/// units have different dimensions (e.g. `m` → `s`).
///
/// Input `{ value, from, to }` → `{ value, from, to, dimension:{..} }`.
#[wasm_bindgen]
pub fn units_convert(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        value: f64,
        from: String,
        to: String,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let from = unit_by_name(&p.from)
        .ok_or_else(|| JsValue::from_str(&format!("unknown unit: {:?}", p.from)))?;
    let to = unit_by_name(&p.to)
        .ok_or_else(|| JsValue::from_str(&format!("unknown unit: {:?}", p.to)))?;
    let converted = convert(p.value, &from, &to).map_err(map_units_err)?;
    #[derive(Serialize)]
    struct Out {
        value: f64,
        from: String,
        to: String,
        dimension: DimensionOut,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        value: converted,
        from: from.name.to_string(),
        to: to.name.to_string(),
        dimension: from.dimension.into(),
    })?)
}

/// Multiply or divide two dimensioned quantities, composing their dimensions. Each
/// quantity is `{ value, unit }`; the unit string is resolved to its SI factor so the
/// result value is in coherent SI base units, and the result dimension is returned as the
/// 7-vector. `divide` fails closed on a zero divisor.
///
/// Input `{ a:{value,unit}, b:{value,unit}, op:"multiply"|"divide" }`
/// → `{ value, dimension:{..} }`.
#[wasm_bindgen]
pub fn units_quantity_op(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct QtyIn {
        value: f64,
        unit: String,
    }
    #[derive(Deserialize)]
    struct In {
        a: QtyIn,
        b: QtyIn,
        op: String,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;

    let ua = unit_by_name(&p.a.unit)
        .ok_or_else(|| JsValue::from_str(&format!("unknown unit: {:?}", p.a.unit)))?;
    let ub = unit_by_name(&p.b.unit)
        .ok_or_else(|| JsValue::from_str(&format!("unknown unit: {:?}", p.b.unit)))?;

    // `Unit::quantity` applies the affine map to SI, so e.g. 0 °C becomes 273.15 K
    // before any dimensional arithmetic — products/quotients are only meaningful in
    // coherent SI.
    let qa: Quantity = ua.quantity(p.a.value);
    let qb: Quantity = ub.quantity(p.b.value);

    let result: Quantity = match p.op.as_str() {
        "multiply" | "mul" | "*" => qa.mul(&qb),
        "divide" | "div" | "/" => qa
            .div(&qb)
            .ok_or_else(|| JsValue::from_str("divide: divisor value is zero"))?,
        other => {
            return Err(JsValue::from_str(&format!(
                "unknown op: {other:?} (expected \"multiply\" or \"divide\")"
            )))
        }
    };

    #[derive(Serialize)]
    struct Out {
        value: f64,
        dimension: DimensionOut,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        value: result.value,
        dimension: result.dimension.into(),
    })?)
}

/// Look up a CODATA / SI-2019 physical constant by name, returning its value (in coherent
/// SI base units) and its physical dimension as the 7-vector.
///
/// Input `{ name }` → `{ name, symbol, description, value, dimension:{..} }`.
/// Accepted names are those from `units_list_constants` (canonical name or symbol alias).
#[wasm_bindgen]
pub fn units_constant(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        name: String,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let (q, symbol, description) = constant_by_name(&p.name)
        .ok_or_else(|| JsValue::from_str(&format!("unknown constant: {:?}", p.name)))?;
    #[derive(Serialize)]
    struct Out {
        name: String,
        symbol: String,
        description: String,
        value: f64,
        dimension: DimensionOut,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        name: p.name,
        symbol: symbol.to_string(),
        description: description.to_string(),
        value: q.value,
        dimension: q.dimension.into(),
    })?)
}

/// List every unit the engine can convert between, with a human label and its dimension
/// 7-vector. Takes an empty object `{}`. Input `{}` → `{ units:[{symbol,label,dimension}] }`.
#[wasm_bindgen]
pub fn units_list_units(_val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Serialize)]
    struct UnitInfo {
        symbol: String,
        label: String,
        dimension: DimensionOut,
    }
    let units: Vec<UnitInfo> = UNIT_CATALOG
        .iter()
        .filter_map(|(sym, label)| {
            unit_by_name(sym).map(|u| UnitInfo {
                symbol: (*sym).to_string(),
                label: (*label).to_string(),
                dimension: u.dimension.into(),
            })
        })
        .collect();
    #[derive(Serialize)]
    struct Out {
        units: Vec<UnitInfo>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { units })?)
}

/// List every CODATA constant available to `units_constant`, with value, symbol,
/// description and dimension. Takes an empty object `{}`.
/// Input `{}` → `{ constants:[{name,symbol,description,value,dimension}] }`.
#[wasm_bindgen]
pub fn units_list_constants(_val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Serialize)]
    struct ConstInfo {
        name: String,
        symbol: String,
        description: String,
        value: f64,
        dimension: DimensionOut,
    }
    let constants_out: Vec<ConstInfo> = CONSTANT_CATALOG
        .iter()
        .filter_map(|(name, _sym, _desc)| {
            constant_by_name(name).map(|(q, symbol, description)| ConstInfo {
                name: (*name).to_string(),
                symbol: symbol.to_string(),
                description: description.to_string(),
                value: q.value,
                dimension: q.dimension.into(),
            })
        })
        .collect();
    #[derive(Serialize)]
    struct Out {
        constants: Vec<ConstInfo>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        constants: constants_out,
    })?)
}
