//! Engineering analysis invoke seams — survival/structural functions.
//!
//! Exposes `specialized_libs::engineering_analysis::survival` functions
//! through VibeScript invoke IDs.

use super::super::args;
use poet_vibe::{Diagnostic, Span, Value};

/// `EngineeringAnalysis.cauchy_stress` — analyse a 3×3 Cauchy stress tensor.
/// Args: { tensor: [f64] (9 values, row-major 3×3) }
pub fn cauchy_stress(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let tensor_flat = args::rec_f64_list(args, "tensor").ok_or_else(|| {
        args::bad(
            span,
            "EngineeringAnalysis.cauchy_stress needs tensor (9 values)",
        )
    })?;
    if tensor_flat.len() < 9 {
        return Err(args::bad(
            span,
            "EngineeringAnalysis.cauchy_stress: tensor must have 9 values (3×3 row-major)",
        ));
    }
    let mut tensor = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            tensor[i][j] = tensor_flat[i * 3 + j];
        }
    }
    let state = crate::specialized_libs::engineering_analysis::cauchy_stress_analysis(&tensor);
    Ok(args::record([
        ("von_mises", Value::F64(state.von_mises)),
        ("principal_1", Value::F64(state.principal[0])),
        ("principal_2", Value::F64(state.principal[1])),
        ("principal_3", Value::F64(state.principal[2])),
        ("max_shear", Value::F64(state.max_shear)),
        ("hydrostatic", Value::F64(state.hydrostatic)),
    ]))
}

/// `EngineeringAnalysis.drag_force` — aerodynamic drag force.
/// Args: { air_density, velocity, drag_coefficient, area }
pub fn drag_force(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let rho = args::rec_f64(args, "air_density")
        .ok_or_else(|| args::bad(span, "EngineeringAnalysis.drag_force needs air_density"))?;
    let v = args::rec_f64(args, "velocity")
        .ok_or_else(|| args::bad(span, "EngineeringAnalysis.drag_force needs velocity"))?;
    let cd = args::rec_f64(args, "drag_coefficient").ok_or_else(|| {
        args::bad(
            span,
            "EngineeringAnalysis.drag_force needs drag_coefficient",
        )
    })?;
    let area = args::rec_f64(args, "area")
        .ok_or_else(|| args::bad(span, "EngineeringAnalysis.drag_force needs area"))?;
    let force = crate::specialized_libs::engineering_analysis::drag_force(rho, v, cd, area);
    Ok(args::record([("drag_force", Value::F64(force))]))
}

/// `EngineeringAnalysis.reynolds_number` — Reynolds number for flow regime.
/// Args: { density, velocity, char_length, dynamic_viscosity }
pub fn reynolds_number(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let density = args::rec_f64(args, "density")
        .ok_or_else(|| args::bad(span, "EngineeringAnalysis.reynolds_number needs density"))?;
    let velocity = args::rec_f64(args, "velocity")
        .ok_or_else(|| args::bad(span, "EngineeringAnalysis.reynolds_number needs velocity"))?;
    let char_length = args::rec_f64(args, "char_length").ok_or_else(|| {
        args::bad(
            span,
            "EngineeringAnalysis.reynolds_number needs char_length",
        )
    })?;
    let viscosity = args::rec_f64(args, "dynamic_viscosity").ok_or_else(|| {
        args::bad(
            span,
            "EngineeringAnalysis.reynolds_number needs dynamic_viscosity",
        )
    })?;
    let re = crate::specialized_libs::engineering_analysis::reynolds_number(
        density,
        velocity,
        char_length,
        viscosity,
    );
    Ok(args::record([("reynolds_number", Value::F64(re))]))
}

/// `EngineeringAnalysis.fatigue_cycles` — Basquin's law cycles-to-failure.
/// Args: { stress_amplitude, fatigue_strength_coeff, fatigue_strength_exponent }
pub fn fatigue_cycles(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let sa = args::rec_f64(args, "stress_amplitude").ok_or_else(|| {
        args::bad(
            span,
            "EngineeringAnalysis.fatigue_cycles needs stress_amplitude",
        )
    })?;
    let sf = args::rec_f64(args, "fatigue_strength_coeff").ok_or_else(|| {
        args::bad(
            span,
            "EngineeringAnalysis.fatigue_cycles needs fatigue_strength_coeff",
        )
    })?;
    let b = args::rec_f64(args, "fatigue_strength_exponent").ok_or_else(|| {
        args::bad(
            span,
            "EngineeringAnalysis.fatigue_cycles needs fatigue_strength_exponent",
        )
    })?;
    let n = crate::specialized_libs::engineering_analysis::fatigue_cycles_basquin(sa, sf, b);
    Ok(args::record([("cycles_to_failure", Value::F64(n))]))
}

/// `EngineeringAnalysis.miner_damage` — Miner's cumulative damage rule.
/// Args: { blocks: [{stress_amplitude, cycles}] } or { blocks: [f64] (pairs) }
pub fn miner_damage(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let blocks_flat = args::rec_f64_list(args, "blocks").ok_or_else(|| {
        args::bad(
            span,
            "EngineeringAnalysis.miner_damage needs blocks (flat pairs)",
        )
    })?;
    if blocks_flat.len() % 2 != 0 {
        return Err(args::bad(
            span,
            "EngineeringAnalysis.miner_damage: blocks must be pairs [stress, cycles]",
        ));
    }
    let blocks: Vec<(f64, f64)> = blocks_flat.chunks(2).map(|c| (c[0], c[1])).collect();
    let damage = crate::specialized_libs::engineering_analysis::miner_cumulative_damage(&blocks);
    Ok(args::record([("cumulative_damage", Value::F64(damage))]))
}
