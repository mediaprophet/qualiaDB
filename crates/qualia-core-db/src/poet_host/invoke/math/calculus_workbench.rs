//! Bounded numerical-calculus adapter for POET's Calculus panel.

use super::super::args;
use super::closure_solvers::{calc_adaptive_simpson, ode_rk4_integrate};
use crate::solvers::calculus::grid::{
    detect_simd_width, integrate_simpsons_kahan, integrate_trapezoidal_chunked, ContinuousGrid,
};
use crate::specialized_libs::symbolic_algebra;
use std::collections::HashMap;
use vibe::{Diagnostic, Span, Value};

const MAX_PANELS: usize = 100_000;

fn sampled_integral(args_v: &Value, span: Span, simpson: bool) -> Result<Value, Diagnostic> {
    let expression = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "calculus integration needs `expr`"))?;
    let a = args::rec_f64(args_v, "a")
        .ok_or_else(|| args::bad(span, "calculus integration needs `a`"))?;
    let b = args::rec_f64(args_v, "b")
        .ok_or_else(|| args::bad(span, "calculus integration needs `b`"))?;
    let mut panels = args::rec_u64(args_v, "panels").unwrap_or(1_000) as usize;
    if !a.is_finite() || !b.is_finite() || b <= a || panels < 2 || panels > MAX_PANELS {
        return Err(args::bad(
            span,
            "integration requires finite a<b and 2..100,000 panels",
        ));
    }
    if simpson && panels & 1 == 1 {
        panels += 1;
    }
    let expression = symbolic_algebra::parse(expression)
        .map_err(|error| args::bad(span, format!("invalid expression: {error}")))?;
    let step = (b - a) / panels as f64;
    let mut environment = HashMap::new();
    let mut samples = Vec::with_capacity(panels + 1);
    for index in 0..=panels {
        environment.insert("x".to_string(), a + index as f64 * step);
        let value = expression
            .eval(&environment)
            .ok_or_else(|| args::bad(span, "expression produced an unbound or non-finite value"))?;
        samples.push(value);
    }
    let bytes = bytemuck::cast_slice(&samples);
    let grid = ContinuousGrid::new(bytes, samples.len())
        .map_err(|error| args::bad(span, format!("continuous grid: {error:?}")))?;
    let (value, compensation, method) = if simpson {
        let (value, compensation) = integrate_simpsons_kahan(&grid, step as f32)
            .map_err(|error| args::bad(span, format!("Simpson integration: {error:?}")))?;
        (value, compensation as f64, "Simpson-Kahan")
    } else {
        let value = integrate_trapezoidal_chunked(&grid, step)
            .map_err(|error| args::bad(span, format!("trapezoidal integration: {error:?}")))?;
        (value, 0.0, "Trapezoidal-SIMD")
    };
    Ok(args::record([
        ("method", Value::String(method.into())),
        ("value", Value::F64(value)),
        ("panels", Value::U64(panels as u64)),
        ("step", Value::F64(step)),
        ("kahan_compensation", Value::F64(compensation)),
    ]))
}

pub fn compute(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    match args::rec_str(args_v, "operation")
        .ok_or_else(|| args::bad(span, "CalculusWorkbench.compute needs `operation`"))?
    {
        "rk4_step" => ode_rk4_integrate(args_v, span),
        "simpsons" => sampled_integral(args_v, span, true),
        "trapezoidal" | "large_grid" => sampled_integral(args_v, span, false),
        "adaptive" => calc_adaptive_simpson(args_v, span),
        "simd_width" => Ok(args::record([
            (
                "simd_width",
                Value::String(format!("{:?}", detect_simd_width())),
            ),
            ("cache_line_bytes", Value::U64(64)),
        ])),
        operation => Err(args::bad(
            span,
            format!("unknown calculus operation `{operation}`"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrates_x_squared_with_simpson_grid() {
        let input = args::record([
            ("operation", Value::String("simpsons".into())),
            ("expr", Value::String("x^2".into())),
            ("a", Value::F64(0.0)),
            ("b", Value::F64(1.0)),
            ("panels", Value::U64(100)),
        ]);
        let Value::Record(result) = compute(&input, Span::new(0, 0)).unwrap() else {
            panic!("expected record")
        };
        let value = args::as_f64(result.get("value").unwrap()).unwrap();
        assert!((value - 1.0 / 3.0).abs() < 1e-5);
    }
}
