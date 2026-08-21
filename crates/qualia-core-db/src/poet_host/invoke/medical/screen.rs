//! Additional medical-computing invoke seams — compound screening.

use super::super::args;
use crate::specialized_libs::medical_computing as med;
use poet_vibe::{Diagnostic, Span, Value};

/// `MedicalComputing.screen_compounds` — rule-based compound screening with
/// Lipinski/Veber filters and optional Tanimoto similarity ranking.
///
/// Args:
///   {
///     compounds: [
///       {
///         id: String, name: String, smiles: String,
///         molecular_weight: f64, logp: f64, solubility: f64,
///         acute_toxicity: f64, chronic_toxicity: f64,
///         mutagenicity: bool, carcinogenicity: bool
///       }
///     ],
///     target: {
///       id: String, name: String, target_type: String,
///       biological_function: String, disease_association: [String]
///     },
///     query_smiles: String?
///   }
pub fn screen_compounds(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let compounds_val = args::rec(args, "compounds")
        .ok_or_else(|| args::bad(span, "MedicalComputing.screen_compounds needs compounds"))?;
    let comp_list = match compounds_val {
        Value::List(l) => l,
        _ => {
            return Err(args::bad(
                span,
                "screen_compounds: compounds must be a list",
            ))
        }
    };

    let mut compounds = Vec::new();
    for c in comp_list {
        let compound_id = args::rec_str(c, "id")
            .ok_or_else(|| args::bad(span, "screen_compounds: each compound needs id"))?
            .to_string();
        let compound_name = args::rec_str(c, "name")
            .ok_or_else(|| args::bad(span, "screen_compounds: each compound needs name"))?
            .to_string();
        let chemical_structure = args::rec_str(c, "smiles").unwrap_or("").to_string();
        let molecular_weight = args::rec_f64(c, "molecular_weight").unwrap_or(0.0);
        let logp = args::rec_f64(c, "logp").unwrap_or(0.0);
        let solubility = args::rec_f64(c, "solubility").unwrap_or(0.0);
        let acute_toxicity = args::rec_f64(c, "acute_toxicity").unwrap_or(0.0);
        let chronic_toxicity = args::rec_f64(c, "chronic_toxicity").unwrap_or(0.0);
        let mutagenicity = args::rec_bool(c, "mutagenicity").unwrap_or(false);
        let carcinogenicity = args::rec_bool(c, "carcinogenicity").unwrap_or(false);

        compounds.push(med::Compound {
            compound_id,
            compound_name,
            chemical_structure,
            properties: med::CompoundProperties {
                molecular_weight,
                logp,
                solubility,
                toxicity: med::ToxicityProfile {
                    acute_toxicity,
                    chronic_toxicity,
                    mutagenicity,
                    carcinogenicity,
                },
            },
        });
    }

    let target_val = args::rec(args, "target")
        .ok_or_else(|| args::bad(span, "MedicalComputing.screen_compounds needs target"))?;
    let target_id = args::rec_str(target_val, "id")
        .ok_or_else(|| args::bad(span, "screen_compounds: target needs id"))?
        .to_string();
    let target_name = args::rec_str(target_val, "name")
        .ok_or_else(|| args::bad(span, "screen_compounds: target needs name"))?
        .to_string();
    let target_type_str = args::rec_str(target_val, "target_type").unwrap_or("Enzyme");
    let target_type = match target_type_str {
        "Receptor" => med::TargetType::Receptor,
        "IonChannel" => med::TargetType::IonChannel,
        "Transporter" => med::TargetType::Transporter,
        "NuclearReceptor" => med::TargetType::NuclearReceptor,
        _ => med::TargetType::Enzyme,
    };
    let biological_function = args::rec_str(target_val, "biological_function")
        .unwrap_or("")
        .to_string();
    let disease_association =
        args::rec_str_list(target_val, "disease_association").unwrap_or_default();

    let target = med::DrugTarget {
        target_id,
        target_name,
        target_type,
        properties: med::TargetProperties {
            binding_sites: Vec::new(),
            biological_function,
            disease_association,
        },
    };

    let query_smiles = args::rec_str(args, "query_smiles").map(|s| s.to_string());

    match med::screen_compounds_rulebased(&compounds, &target, query_smiles.as_deref()) {
        Ok(proposal) => {
            let ranked: Vec<Value> = proposal
                .ranked
                .iter()
                .map(|row| {
                    args::record([
                        ("compound_id", Value::String(row.compound_id.clone())),
                        ("molecular_weight", Value::F64(row.molecular_weight)),
                        ("logp_estimate", Value::F64(row.logp_estimate)),
                        ("hb_donors", Value::U64(row.hb_donors as u64)),
                        ("hb_acceptors", Value::U64(row.hb_acceptors as u64)),
                        ("rotatable_bonds", Value::U64(row.rotatable_bonds as u64)),
                        ("tpsa", Value::F64(row.tpsa)),
                        (
                            "lipinski_violations",
                            Value::U64(row.lipinski_violations as u64),
                        ),
                        ("passes_lipinski", Value::Bool(row.passes_lipinski)),
                        ("passes_veber", Value::Bool(row.passes_veber)),
                        ("tanimoto_to_query", Value::F64(row.tanimoto_to_query)),
                        (
                            "descriptors_from_structure",
                            Value::Bool(row.descriptors_from_structure),
                        ),
                    ])
                })
                .collect();
            Ok(args::record([
                (
                    "epistemic_status",
                    Value::String(proposal.epistemic_status.to_string()),
                ),
                ("method", Value::String(proposal.method.to_string())),
                ("target_id", Value::String(proposal.target_id.clone())),
                (
                    "query_smiles",
                    proposal
                        .query_smiles
                        .as_ref()
                        .map(|s| Value::String(s.clone()))
                        .unwrap_or(Value::Null),
                ),
                ("ranked", Value::List(ranked)),
            ]))
        }
        Err(e) => Err(args::bad(span, format!("screen_compounds: {e:?}"))),
    }
}
