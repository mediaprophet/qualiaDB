//! Bounded adapter for the zero-allocation comorbidity evaluator.

use super::super::args;
use crate::medical::comorbidity_eval::{
    compile_exacerbation_quins, eval_comorbidity, ComorbidityStatus, ComorbidityVerdict,
    MAX_COMORBIDITY_VERDICTS, MAX_CONDITION_SLOTS, PRED_HAS_CONDITION,
};
use crate::{q_hash, NQuin};
use vibe::{Diagnostic, Span, Value};

pub fn evaluate(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let patient = args::rec_str(args_v, "patient")
        .ok_or_else(|| args::bad(span, "comorbidity evaluation needs `patient`"))?;
    let conditions = args::rec_str_list(args_v, "conditions")
        .ok_or_else(|| args::bad(span, "comorbidity evaluation needs `conditions`"))?;
    if conditions.is_empty() || conditions.len() > MAX_CONDITION_SLOTS {
        return Err(args::bad(span, "`conditions` must contain 1..32 entries"));
    }
    let patient_hash = q_hash(patient);
    let target_hash = args::rec_str(args_v, "target_organ")
        .map(q_hash)
        .unwrap_or(0);
    let mut graph = [NQuin::default(); 36];
    let mut graph_count = 0usize;
    for condition in &conditions {
        let condition_hash = q_hash(condition);
        let quin = &mut graph[graph_count];
        quin.subject = patient_hash;
        quin.predicate = PRED_HAS_CONDITION;
        quin.object = condition_hash;
        quin.context = patient_hash;
        quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;
        graph_count += 1;
    }
    if let (Some(ante), Some(consequent)) = (
        args::rec_str(args_v, "antecedent"),
        args::rec_str(args_v, "consequent"),
    ) {
        let severity = args::rec_f64(args_v, "severity").unwrap_or(0.5);
        if !severity.is_finite() || !(0.0..=1.0).contains(&severity) {
            return Err(args::bad(span, "`severity` must be between 0 and 1"));
        }
        let mut edge = [NQuin::default(); 2];
        compile_exacerbation_quins(
            q_hash(ante),
            q_hash(consequent),
            patient_hash,
            severity as f32,
            &mut edge,
        );
        graph[graph_count] = edge[0];
        graph[graph_count + 1] = edge[1];
        graph_count += 2;
    }
    let empty = ComorbidityVerdict {
        condition_hash: 0,
        compounded_risk_milli: 0,
        status: ComorbidityStatus::Active,
        _pad: [0; 3],
    };
    let mut verdicts = [empty; MAX_COMORBIDITY_VERDICTS];
    let count = eval_comorbidity(
        patient_hash,
        target_hash,
        &graph[..graph_count],
        &mut verdicts,
    )
    .map_err(|error| args::bad(span, format!("comorbidity evaluation failed: {error:?}")))?;
    let rows = verdicts[..count]
        .iter()
        .map(|verdict| {
            let name = conditions
                .iter()
                .find(|condition| q_hash(condition) == verdict.condition_hash)
                .cloned()
                .unwrap_or_else(|| format!("hash:{:016x}", verdict.condition_hash));
            args::record([
                ("condition", Value::String(name)),
                (
                    "compounded_risk",
                    Value::F64(verdict.compounded_risk_milli as f64 / 1_000.0),
                ),
                ("status", Value::String(format!("{:?}", verdict.status))),
            ])
        })
        .collect();
    Ok(args::record([
        ("patient", Value::String(patient.to_string())),
        ("verdict_count", Value::U64(count as u64)),
        ("verdicts", Value::List(rows)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_compounding_edge() {
        let input = args::record([
            ("patient", Value::String("did:patient:1".into())),
            (
                "conditions",
                Value::List(vec![
                    Value::String("Type 2 Diabetes Mellitus".into()),
                    Value::String("Heart".into()),
                ]),
            ),
            ("target_organ", Value::String("Heart".into())),
            (
                "antecedent",
                Value::String("Type 2 Diabetes Mellitus".into()),
            ),
            ("consequent", Value::String("Heart".into())),
            ("severity", Value::F64(0.8)),
        ]);
        let Value::Record(result) = evaluate(&input, Span::new(0, 0)).unwrap() else {
            panic!("expected record")
        };
        assert!(args::as_u64(result.get("verdict_count").unwrap()).unwrap() >= 1);
    }
}
