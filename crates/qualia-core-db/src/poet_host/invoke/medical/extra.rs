//! Additional medical-computing invoke seams — Bayesian differential analysis
//! over a caller-supplied knowledge base.

use super::super::args;
use crate::specialized_libs::medical_computing as med;
use poet_vibe::{Diagnostic, Span, Value};

/// `MedicalComputing.analyze_differential` — transparent naive-Bayes
/// differential over a caller-supplied, non-authoritative knowledge base.
///
/// Args:
///   {
///     findings: [String],
///     unlisted_finding_likelihood: f64,
///     conditions: [
///       { id: String, prior: f64, likelihoods: { finding: f64, ... } }
///     ]
///   }
pub fn analyze_differential(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let findings = args::rec_str_list(args, "findings")
        .ok_or_else(|| args::bad(span, "MedicalComputing.analyze_differential needs findings"))?;
    let unlisted = args::rec_f64(args, "unlisted_finding_likelihood").unwrap_or(0.5);
    let conditions_val = args::rec(args, "conditions").ok_or_else(|| {
        args::bad(
            span,
            "MedicalComputing.analyze_differential needs conditions",
        )
    })?;
    let cond_list = match conditions_val {
        Value::List(l) => l,
        _ => {
            return Err(args::bad(
                span,
                "analyze_differential: conditions must be a list",
            ))
        }
    };

    let mut conditions = Vec::new();
    for c in cond_list {
        let id = args::rec_str(c, "id")
            .ok_or_else(|| args::bad(span, "analyze_differential: each condition needs id"))?
            .to_string();
        let prior = args::rec_f64(c, "prior")
            .ok_or_else(|| args::bad(span, "analyze_differential: each condition needs prior"))?;
        let likelihoods_val = args::rec(c, "likelihoods").ok_or_else(|| {
            args::bad(
                span,
                "analyze_differential: each condition needs likelihoods",
            )
        })?;
        let mut likelihoods = std::collections::HashMap::new();
        if let Value::Record(pairs) = likelihoods_val {
            for (finding, prob_v) in pairs.iter() {
                if let Some(p) = args::as_f64(prob_v) {
                    likelihoods.insert(finding.clone(), p);
                }
            }
        }
        conditions.push(med::ConditionModel {
            condition_id: id,
            prior,
            likelihoods,
        });
    }

    let kb = med::DiagnosticKnowledgeBase {
        conditions,
        unlisted_finding_likelihood: unlisted,
    };

    match med::analyze_differential(&findings, &kb) {
        Ok(proposal) => {
            let ranked: Vec<Value> = proposal
                .ranked
                .iter()
                .map(|p| {
                    args::record([
                        ("condition_id", Value::String(p.condition_id.clone())),
                        ("prior", Value::F64(p.prior)),
                        ("posterior", Value::F64(p.posterior)),
                    ])
                })
                .collect();
            Ok(args::record([
                (
                    "epistemic_status",
                    Value::String(proposal.epistemic_status.to_string()),
                ),
                ("method", Value::String(proposal.method.to_string())),
                (
                    "observed_findings",
                    Value::List(findings.iter().map(|f| Value::String(f.clone())).collect()),
                ),
                ("ranked", Value::List(ranked)),
            ]))
        }
        Err(e) => Err(args::bad(span, format!("analyze_differential: {e:?}"))),
    }
}
