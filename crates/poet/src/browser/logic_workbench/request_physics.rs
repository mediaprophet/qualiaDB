//! Physics Simulator and ODE-panel request construction.

use super::helpers::field_value;
use super::request_parse::{
    optional_f64, optional_u64, required_f64, required_f64_list, required_u64,
};
use web_sys::Document;

fn number(
    arguments: &mut serde_json::Map<String, serde_json::Value>,
    source: &str,
    key: &str,
) -> Result<(), String> {
    arguments.insert(key.into(), serde_json::json!(required_f64(source, key)?));
    Ok(())
}

fn count(
    arguments: &mut serde_json::Map<String, serde_json::Value>,
    source: &str,
    key: &str,
) -> Result<(), String> {
    arguments.insert(key.into(), serde_json::json!(required_u64(source, key)?));
    Ok(())
}

fn optional_number(
    arguments: &mut serde_json::Map<String, serde_json::Value>,
    source: &str,
    key: &str,
) -> Result<(), String> {
    if let Some(value) = optional_f64(source, key)? {
        arguments.insert(key.into(), serde_json::json!(value));
    }
    Ok(())
}

fn battery_arguments(
    arguments: &mut serde_json::Map<String, serde_json::Value>,
    source: &str,
    needs_load: bool,
) -> Result<(), String> {
    for key in ["soc", "cell_resistance", "cell_capacity_ah"] {
        number(arguments, source, key)?;
    }
    for key in ["cells_series", "cells_parallel"] {
        count(arguments, source, key)?;
    }
    if needs_load {
        number(arguments, source, "load_current")?;
    }
    Ok(())
}

fn solar_arguments(
    arguments: &mut serde_json::Map<String, serde_json::Value>,
    source: &str,
    array: bool,
) -> Result<(), String> {
    for key in [
        "short_circuit_current",
        "open_circuit_voltage",
        "fill_factor",
    ] {
        number(arguments, source, key)?;
    }
    if let Some(value) = optional_u64(source, "scan_steps")? {
        arguments.insert("scan_steps".into(), serde_json::json!(value));
    }
    if array {
        count(arguments, source, "panel_count")?;
    }
    Ok(())
}

fn build_request(source: &str, operation: &str) -> Result<serde_json::Value, String> {
    let mut arguments = serde_json::Map::new();
    arguments.insert("operation".into(), serde_json::json!(operation));
    match operation {
        "metropolis" => {
            number(&mut arguments, source, "temperature")?;
            for key in ["ensemble_size", "steps"] {
                count(&mut arguments, source, key)?;
            }
            optional_number(&mut arguments, source, "proposal_scale")?;
            if let Some(value) = optional_u64(source, "seed")? {
                arguments.insert("seed".into(), serde_json::json!(value));
            }
        }
        "ode_solver" => {
            for key in ["y1", "y2", "dt"] {
                number(&mut arguments, source, key)?;
            }
            count(&mut arguments, source, "steps")?;
            for key in ["k1", "k2", "coupling"] {
                optional_number(&mut arguments, source, key)?;
            }
        }
        "dft" => {
            count(&mut arguments, source, "electron_count")?;
            count(&mut arguments, source, "resolution")?;
        }
        "pinn" => {
            arguments.insert(
                "molecule_features".into(),
                serde_json::json!(required_f64_list(source, "molecule_features")?),
            );
            arguments.insert(
                "receptor_features".into(),
                serde_json::json!(required_f64_list(source, "receptor_features")?),
            );
        }
        "gibbs" => {
            for key in ["temperature", "enthalpy", "entropy"] {
                number(&mut arguments, source, key)?;
            }
        }
        "cell_ocv" | "pack_ocv" => battery_arguments(&mut arguments, source, false)?,
        "terminal_voltage" | "deliverable_power" => {
            battery_arguments(&mut arguments, source, true)?
        }
        "max_power_point" => solar_arguments(&mut arguments, source, false)?,
        "array_mppt" => solar_arguments(&mut arguments, source, true)?,
        "heat_loss" => {
            for key in ["u_value", "area", "delta_t"] {
                number(&mut arguments, source, key)?;
            }
        }
        "phase_change" => {
            for key in ["mass", "latent_heat"] {
                number(&mut arguments, source, key)?;
            }
        }
        "thermal_efficiency" => {
            for key in ["useful_power", "u_value", "area", "delta_t"] {
                number(&mut arguments, source, key)?;
            }
        }
        _ => return Err(format!("Unknown physics operation `{operation}`.")),
    }
    Ok(serde_json::Value::Object(arguments))
}

pub(super) fn physics_request(
    document: &Document,
    operation: &str,
) -> Result<(&'static str, serde_json::Value), String> {
    let source = field_value(document, "physics-input");
    Ok((
        "PhysicsWorkbench.compute",
        build_request(&source, operation)?,
    ))
}

pub(super) fn ode_request(
    document: &Document,
) -> Result<(&'static str, serde_json::Value), String> {
    let source = field_value(document, "ode-input");
    Ok((
        "PhysicsWorkbench.compute",
        build_request(&source, "ode_solver")?,
    ))
}
