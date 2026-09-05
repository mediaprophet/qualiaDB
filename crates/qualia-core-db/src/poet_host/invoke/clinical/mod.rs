//! Clinical Decision Support & Pharmacological Risk Scoring Subsystem [WASM-Standalone].

pub mod cha2ds2;
pub mod comorbidity;
pub mod contraindication;
pub mod framingham;
pub mod imaging;
mod required;
pub mod score2;

pub use cha2ds2::score as cha2ds2_vasc;
pub use comorbidity::evaluate as evaluate_comorbidity;
pub use contraindication::{
    check_condition as check_contraindication, check_drugs as check_drug_interaction,
};
pub use framingham::score as framingham;
pub use imaging::hu_window;
pub use score2::score as score2;

use super::args;
use crate::clinical_engine::{validate_fhir_observation, FhirObservation};
use vibe::{Diagnostic, Span, Value};

pub fn validate_observation(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let loinc_code = args::rec_str(args_v, "loinc_code")
        .ok_or_else(|| args::bad(span, "FHIR observation needs loinc_code"))?;
    let value = args::rec_f64(args_v, "value")
        .ok_or_else(|| args::bad(span, "FHIR observation needs a numeric value"))?;
    let observation = FhirObservation {
        loinc_code: loinc_code.to_string(),
        value,
        unit_ucum: args::rec_str(args_v, "unit_ucum")
            .unwrap_or_default()
            .to_string(),
        reference_low: args::rec_f64(args_v, "reference_low"),
        reference_high: args::rec_f64(args_v, "reference_high"),
    };
    let result = validate_fhir_observation(&observation);
    Ok(args::record([
        ("valid", Value::Bool(result.is_valid)),
        ("status", Value::String(format!("{:?}", result.status))),
        (
            "interpretation_code",
            Value::String(result.interpretation_code.to_string()),
        ),
        ("loinc_code", Value::String(observation.loinc_code)),
        ("value", Value::F64(observation.value)),
        ("unit_ucum", Value::String(observation.unit_ucum)),
    ]))
}

#[cfg(test)]
mod fhir_tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn validates_real_loinc_observation() {
        let args = Value::Record(BTreeMap::from([
            ("loinc_code".into(), Value::String("4548-4".into())),
            ("value".into(), Value::F64(6.4)),
            ("unit_ucum".into(), Value::String("%".into())),
        ]));
        let Value::Record(result) = validate_observation(&args, Span::new(0, 0)).unwrap() else {
            panic!("expected FHIR result record");
        };
        assert_eq!(result.get("valid"), Some(&Value::Bool(true)));
        assert_eq!(result.get("status"), Some(&Value::String("High".into())));
    }
}
