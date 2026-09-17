//! SMILES validation — `domains::chemical::organic_chemistry`.

use super::super::args;
use crate::domains::chemical::organic_chemistry::validate_smiles;
use crate::domains::chemical::organic_chemistry::{
    arrhenius_rate, atom_economy, circular_fingerprint, compute_descriptors, e_factor,
    equilibrium_constant, estimate_pka, evaluate_egan, evaluate_ghose, evaluate_lipinski,
    evaluate_veber, gibbs_free_energy, green_metrics, henderson_hasselbalch, parse_smiles,
};
use vibe::{Diagnostic, Span, Value};

pub fn smiles(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let s = args::as_str(args_v)
        .or_else(|| args::rec_str(args_v, "smiles"))
        .ok_or_else(|| args::bad(span, "validate_smiles needs a SMILES string"))?;
    let r = validate_smiles(s);
    Ok(args::record([
        ("valid", Value::Bool(r.is_valid)),
        ("atom_count", Value::U64(r.atom_count as u64)),
        ("error", r.error.map(Value::String).unwrap_or(Value::Null)),
    ]))
}

fn need_f64(args_v: &Value, key: &str, span: Span) -> Result<f64, Diagnostic> {
    args::rec_f64(args_v, key).ok_or_else(|| args::bad(span, format!("compute needs {key}")))
}

fn parsed_molecule(
    args_v: &Value,
    span: Span,
) -> Result<crate::domains::chemical::organic_chemistry::Molecule, Diagnostic> {
    let smiles = args::rec_str(args_v, "smiles")
        .ok_or_else(|| args::bad(span, "molecular operation needs a SMILES string"))?;
    let molecule = parse_smiles(smiles);
    if molecule.is_valid {
        Ok(molecule)
    } else {
        Err(args::bad(
            span,
            molecule
                .error
                .clone()
                .unwrap_or_else(|| "invalid SMILES".into()),
        ))
    }
}

pub fn organic_compute(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let operation = args::rec_str(args_v, "operation")
        .ok_or_else(|| args::bad(span, "OrganicChemistry.compute needs operation"))?;
    match operation {
        "smiles_validate" => smiles(args_v, span),
        "mw" | "logp" | "tpsa" | "lipinski" | "veber" | "ghose" | "egan" | "functional_groups"
        | "pka" | "chiral" | "fingerprint" => {
            let molecule = parsed_molecule(args_v, span)?;
            let descriptors = compute_descriptors(&molecule);
            Ok(match operation {
                "mw" => args::record([
                    ("molecular_weight", Value::F64(descriptors.molecular_weight)),
                    ("formula", Value::String(descriptors.formula)),
                ]),
                "logp" => Value::F64(descriptors.logp_crippen),
                "tpsa" => Value::F64(descriptors.tpsa_ertl),
                "lipinski" => {
                    let result = evaluate_lipinski(&descriptors);
                    args::record([
                        ("passes", Value::Bool(result.passes)),
                        ("violations", Value::U64(result.violations as u64)),
                        ("mw_ok", Value::Bool(result.mw_ok)),
                        ("logp_ok", Value::Bool(result.logp_ok)),
                        ("hbd_ok", Value::Bool(result.hbd_ok)),
                        ("hba_ok", Value::Bool(result.hba_ok)),
                    ])
                }
                "veber" => {
                    let result = evaluate_veber(&descriptors);
                    args::record([
                        ("passes", Value::Bool(result.passes)),
                        ("rotatable_bonds_ok", Value::Bool(result.rot_bonds_ok)),
                        ("tpsa_ok", Value::Bool(result.tpsa_ok)),
                    ])
                }
                "ghose" => {
                    let result = evaluate_ghose(&descriptors);
                    args::record([
                        ("passes", Value::Bool(result.passes)),
                        ("mw_ok", Value::Bool(result.mw_ok)),
                        ("logp_ok", Value::Bool(result.logp_ok)),
                        ("atoms_ok", Value::Bool(result.atoms_ok)),
                        ("molar_refractivity_ok", Value::Bool(result.mr_ok)),
                    ])
                }
                "egan" => {
                    let result = evaluate_egan(&descriptors);
                    args::record([
                        ("passes", Value::Bool(result.passes)),
                        ("tpsa_ok", Value::Bool(result.tpsa_ok)),
                        ("logp_ok", Value::Bool(result.logp_ok)),
                    ])
                }
                "functional_groups" => Value::List(
                    crate::domains::chemical::organic_chemistry::detect_functional_groups(
                        &molecule,
                    )
                    .into_iter()
                    .map(|group| Value::String(format!("{group:?}")))
                    .collect(),
                ),
                "pka" => Value::List(
                    estimate_pka(&molecule)
                        .into_iter()
                        .map(|estimate| {
                            args::record([
                                ("group", Value::String(format!("{:?}", estimate.group))),
                                ("pka", Value::F64(estimate.pka)),
                                ("is_acid", Value::Bool(estimate.is_acid)),
                            ])
                        })
                        .collect(),
                ),
                "chiral" => Value::U64(descriptors.chiral_centers as u64),
                "fingerprint" => {
                    let radius = args::rec_u64(args_v, "radius").unwrap_or(2).min(8) as usize;
                    Value::List(
                        circular_fingerprint(&molecule, radius)
                            .into_iter()
                            .take(512)
                            .map(Value::U64)
                            .collect(),
                    )
                }
                _ => unreachable!(),
            })
        }
        "arrhenius" => {
            let a = need_f64(args_v, "a", span)?;
            let ea = need_f64(args_v, "ea", span)?;
            let temperature = need_f64(args_v, "temperature", span)?;
            if temperature <= 0.0 {
                return Err(args::bad(span, "temperature must be above 0 K"));
            }
            Ok(Value::F64(arrhenius_rate(a, ea, temperature)))
        }
        "gibbs" => Ok(Value::F64(gibbs_free_energy(
            need_f64(args_v, "delta_h", span)?,
            need_f64(args_v, "delta_s", span)?,
            need_f64(args_v, "temperature", span)?,
        ))),
        "equilibrium" => Ok(Value::F64(equilibrium_constant(
            need_f64(args_v, "delta_g", span)?,
            need_f64(args_v, "temperature", span)?,
        ))),
        "henderson" => Ok(Value::F64(henderson_hasselbalch(
            need_f64(args_v, "pka", span)?,
            need_f64(args_v, "base_concentration", span)?,
            need_f64(args_v, "acid_concentration", span)?,
        ))),
        "atom_economy" => {
            let reactants = args::rec_f64_list(args_v, "reactant_mws")
                .ok_or_else(|| args::bad(span, "atom_economy needs reactant_mws"))?;
            Ok(Value::F64(atom_economy(
                &reactants,
                need_f64(args_v, "product_mw", span)?,
            )))
        }
        "e_factor" => Ok(Value::F64(e_factor(
            need_f64(args_v, "waste_kg", span)?,
            need_f64(args_v, "product_kg", span)?,
        ))),
        "green_metrics" => {
            let reactants = args::rec_f64_list(args_v, "reactant_mws")
                .ok_or_else(|| args::bad(span, "green_metrics needs reactant_mws"))?;
            let byproducts = args::rec_f64_list(args_v, "byproduct_mws").unwrap_or_default();
            let result = green_metrics(
                &reactants,
                need_f64(args_v, "product_mw", span)?,
                &byproducts,
                need_f64(args_v, "yield_fraction", span)?,
                need_f64(args_v, "solvent_kg", span)?,
                need_f64(args_v, "product_kg", span)?,
                args::rec_u64(args_v, "reactant_c_atoms").unwrap_or(0) as u32,
                args::rec_u64(args_v, "product_c_atoms").unwrap_or(0) as u32,
            );
            Ok(args::record([
                ("atom_economy_pct", Value::F64(result.atom_economy_pct)),
                (
                    "yield_corrected_ae_pct",
                    Value::F64(result.yield_corrected_ae_pct),
                ),
                ("e_factor", Value::F64(result.e_factor)),
                (
                    "process_mass_intensity",
                    Value::F64(result.process_mass_intensity),
                ),
                (
                    "reaction_mass_efficiency_pct",
                    Value::F64(result.reaction_mass_efficiency_pct),
                ),
                (
                    "carbon_efficiency_pct",
                    Value::F64(result.carbon_efficiency_pct),
                ),
            ]))
        }
        _ => Err(args::bad(
            span,
            format!("unknown organic chemistry operation `{operation}`"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ethanol_is_valid() {
        match smiles(&Value::String("CCO".into()), Span { start: 0, end: 0 }).unwrap() {
            Value::Record(r) => assert_eq!(r.get("valid"), Some(&Value::Bool(true))),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn aspirin_descriptors_are_computed_by_the_native_engine() {
        let mut record = std::collections::BTreeMap::new();
        record.insert("operation".into(), Value::String("mw".into()));
        record.insert(
            "smiles".into(),
            Value::String("CC(=O)Oc1ccccc1C(=O)O".into()),
        );
        let Value::Record(result) =
            organic_compute(&Value::Record(record), Span::new(0, 0)).unwrap()
        else {
            panic!("expected descriptor record");
        };
        let Value::F64(weight) = result["molecular_weight"] else {
            panic!("expected molecular weight");
        };
        assert!(weight > 170.0 && weight < 190.0);
    }
}
