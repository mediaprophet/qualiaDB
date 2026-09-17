//! Pharmacological contraindication and drug-drug interaction screening.

use super::super::args;
use crate::clinical_engine::{check_contraindications, check_drug_interactions};
use crate::q_hash;
use vibe::{Diagnostic, Span, Value};

pub fn check_drugs(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let drug_a = args::rec_str(args_v, "drug_a").unwrap_or("warfarin");
    let drug_b = args::rec_str(args_v, "drug_b").unwrap_or("ibuprofen");

    let meds = [q_hash(drug_a), q_hash(drug_b)];
    let interactions = check_drug_interactions(&meds);
    let list: Vec<Value> = interactions
        .into_iter()
        .map(|inter| {
            args::record([
                ("severity", Value::String(format!("{:?}", inter.severity))),
                ("mechanism", Value::String(inter.mechanism.to_string())),
            ])
        })
        .collect();

    Ok(Value::List(list))
}

pub fn check_condition(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let drug = args::rec_str(args_v, "drug").unwrap_or("metformin");
    let condition = args::rec_str(args_v, "condition").unwrap_or("709044004");

    let drug_h = q_hash(drug);
    let cond_h = q_hash(condition);
    let contraindications = check_contraindications(drug_h, &[cond_h]);
    let list: Vec<Value> = contraindications
        .into_iter()
        .map(|c| {
            args::record([
                ("severity", Value::String(format!("{:?}", c.severity))),
                ("reason", Value::String(c.reason.to_string())),
            ])
        })
        .collect();

    Ok(Value::List(list))
}
