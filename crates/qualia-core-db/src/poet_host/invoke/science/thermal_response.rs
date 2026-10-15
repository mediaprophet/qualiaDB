//! Vibe value adapter over the canonical native response kernel.
use super::super::args;
use crate::render::physics::response::{step_thermal, ThermalTargets};
use vibe::{Diagnostic, Span, Value};
pub fn compute(payload: &Value, span: Span) -> Result<Value, Diagnostic> {
    let field = |key: &str| {
        args::rec_f64(payload, key)
            .filter(|n| n.is_finite())
            .ok_or_else(|| args::bad(span, format!("thermal_response needs finite `{key}`")))
    };
    let result = step_thermal(
        [field("oil_temp")?, field("coolant_temp")?],
        ThermalTargets {
            temperatures: [field("oil_target")?, field("coolant_target")?],
            rates: [field("oil_rate")?, field("coolant_rate")?],
        },
        field("dt")?,
    )
    .map_err(|_| args::bad(span, "thermal_response refused interval or rates"))?;
    Ok(args::record([
        ("oil_temp", Value::F64(result[0])),
        ("coolant_temp", Value::F64(result[1])),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vibe_invocation_reaches_the_native_thermal_kernel() {
        let source = r#"
requires [ capability("capability.invoke"), capability("PhysicsWorkbench.compute") ];
effect fn go() {
    return capability.invoke("PhysicsWorkbench.compute", {
        operation: "thermal_response", dt: 0.05,
        oil_temp: 140.0, coolant_temp: 112.0,
        oil_target: 85.0, coolant_target: 75.0,
        oil_rate: 0.028, coolant_rate: 0.04
    });
}
"#;
        let mut host = crate::poet_host::PoetSnapshot::default();
        let value = host.eval_fn(source, "go", vec![]).unwrap();
        let Value::Record(record) = value else {
            panic!("thermal result must be a record");
        };
        let Value::F64(oil) = record["oil_temp"] else {
            panic!("oil must be numeric");
        };
        assert!(oil < 140.0 && oil > 85.0);
    }
}
