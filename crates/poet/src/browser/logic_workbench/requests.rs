//! Validated browser-field to native-capability request mapping.

use super::helpers::field_value;
use super::request_advanced::advanced_request;
use super::request_authoring::authoring_request;
use super::request_calculus::calculus_request;
use super::request_formal::formal_request;
use super::request_governance::governance_request;
use super::request_infra::infra_request;
use super::request_infra_ext::infra_ext_request;
use super::request_legal::legal_request;
use super::request_parse::{
    assignment, bool_assignment, call_arguments, optional_f64, optional_f64_aliases,
    optional_f64_list, optional_u64, required_assignment, required_f64, required_f64_list,
    required_string_list, required_u64,
};
use super::request_physics::{ode_request, physics_request};
use super::request_reasoning::reasoning_request;
use super::request_spatial::spatial_request;
use web_sys::Document;

pub(super) use super::request_capabilities::capability_contract_for_button;

fn clinical_request(
    document: &Document,
    model: &str,
) -> Result<(&'static str, serde_json::Value), String> {
    let source = field_value(document, "clinical-risk-input");
    match model {
        "framingham" => Ok((
            "ClinicalRisk.framingham",
            serde_json::json!({
                "age": required_f64(&source, "age")? as u64,
                "sex_male": required_assignment(&source, "sex")?.eq_ignore_ascii_case("male"),
                "total_cholesterol_mmol": required_f64(&source, "total_chol")?,
                "hdl_cholesterol_mmol": required_f64(&source, "hdl")?,
                "systolic_bp": required_f64(&source, "sys_bp")?,
                "bp_treated": assignment(&source, "bp_treated").map(|_| bool_assignment(&source, "bp_treated")).transpose()?.unwrap_or(false),
                "current_smoker": bool_assignment(&source, "smoker")?,
                "diabetic": bool_assignment(&source, "diabetes")?
            }),
        )),
        "cha2ds2_vasc" => Ok((
            "ClinicalRisk.cha2ds2_vasc",
            serde_json::json!({
                "age": required_f64(&source, "age")? as u64,
                "congestive_heart_failure": bool_assignment(&source, "chf")?,
                "hypertension": bool_assignment(&source, "hypertension")?,
                "diabetes": bool_assignment(&source, "diabetes")?,
                "stroke_tia_history": bool_assignment(&source, "stroke")?,
                "vascular_disease": bool_assignment(&source, "vascular")?,
                "sex_female": assignment(&source, "sex").is_some_and(|value| value.eq_ignore_ascii_case("female"))
            }),
        )),
        "score2" => Ok((
            "ClinicalRisk.score2",
            serde_json::json!({
                "age": required_f64(&source, "age")? as u64,
                "sex_male": required_assignment(&source, "sex")?.eq_ignore_ascii_case("male"),
                "systolic_bp": required_f64(&source, "sys_bp")?,
                "total_cholesterol_mmol": required_f64(&source, "total_chol")?,
                "hdl_cholesterol_mmol": required_f64(&source, "hdl")?,
                "current_smoker": bool_assignment(&source, "smoker")?
            }),
        )),
        "drug_interaction" => Ok((
            "ClinicalRisk.drug_interaction",
            serde_json::json!({
                "drug_a": required_assignment(&source, "drug_a")?,
                "drug_b": required_assignment(&source, "drug_b")?
            }),
        )),
        "contraindication" => Ok((
            "ClinicalRisk.contraindication",
            serde_json::json!({
                "drug": required_assignment(&source, "drug")?,
                "condition": required_assignment(&source, "condition")?
            }),
        )),
        "fhir_observation" => Ok((
            "ClinicalRisk.fhir_observation",
            serde_json::json!({
                "loinc_code": required_assignment(&source, "loinc_code")?,
                "value": required_f64(&source, "value")?,
                "unit_ucum": required_assignment(&source, "unit_ucum")?,
                "reference_low": optional_f64(&source, "reference_low")?,
                "reference_high": optional_f64(&source, "reference_high")?
            }),
        )),
        _ => Err(format!("Unknown clinical model `{model}`.")),
    }
}

fn chemistry_request(
    document: &Document,
    operation: &str,
) -> Result<(&'static str, serde_json::Value), String> {
    const MOLECULAR_OPERATIONS: &[&str] = &[
        "smiles_validate",
        "mw",
        "logp",
        "tpsa",
        "lipinski",
        "veber",
        "ghose",
        "egan",
        "functional_groups",
        "pka",
        "chiral",
        "fingerprint",
    ];
    let smiles = field_value(document, "chemistry-smiles");
    if MOLECULAR_OPERATIONS.contains(&operation) && smiles.trim().is_empty() {
        return Err("Enter a SMILES string for this molecular operation.".into());
    }
    let params = field_value(document, "chemistry-params");
    let mut arguments = serde_json::Map::new();
    arguments.insert("operation".into(), serde_json::json!(operation));
    arguments.insert("smiles".into(), serde_json::json!(smiles));
    let numeric = [
        ("a", &["a", "A"][..]),
        ("ea", &["ea", "Ea"][..]),
        ("temperature", &["temperature", "T"][..]),
        ("delta_h", &["delta_h", "dH"][..]),
        ("delta_s", &["delta_s", "dS"][..]),
        ("delta_g", &["delta_g", "dG"][..]),
        ("pka", &["pka", "pKa"][..]),
        ("base_concentration", &["base_concentration", "base"][..]),
        ("acid_concentration", &["acid_concentration", "acid"][..]),
        ("product_mw", &["product_mw"][..]),
        ("waste_kg", &["waste_kg"][..]),
        ("product_kg", &["product_kg"][..]),
        ("yield_fraction", &["yield_fraction"][..]),
        ("solvent_kg", &["solvent_kg"][..]),
    ];
    for (target, aliases) in numeric {
        if let Some(value) = optional_f64_aliases(&params, aliases)? {
            arguments.insert(target.into(), serde_json::json!(value));
        }
    }
    for key in ["radius", "reactant_c_atoms", "product_c_atoms"] {
        if let Some(value) = optional_u64(&params, key)? {
            arguments.insert(key.into(), serde_json::json!(value));
        }
    }
    for key in ["reactant_mws", "byproduct_mws"] {
        if let Some(values) = optional_f64_list(&params, key)? {
            arguments.insert(key.into(), serde_json::json!(values));
        }
    }
    Ok((
        if operation == "smiles_validate" {
            "OrganicChemistry.validate_smiles"
        } else {
            "OrganicChemistry.compute"
        },
        serde_json::Value::Object(arguments),
    ))
}

fn bioinformatics_request(
    document: &Document,
    operation: &str,
) -> Result<(&'static str, serde_json::Value), String> {
    let source = field_value(document, "bioinformatics-input");
    let mut arguments = serde_json::Map::new();
    arguments.insert("operation".into(), serde_json::json!(operation));
    match operation {
        "nucleotide_align" | "protein_align" | "needleman_wunsch" => {
            arguments.insert(
                "seq1".into(),
                serde_json::json!(required_assignment(&source, "seq1")?),
            );
            arguments.insert(
                "seq2".into(),
                serde_json::json!(required_assignment(&source, "seq2")?),
            );
        }
        "kmer_frequency" => {
            arguments.insert(
                "sequence".into(),
                serde_json::json!(required_assignment(&source, "sequence")?),
            );
            arguments.insert("k".into(), serde_json::json!(required_u64(&source, "k")?));
        }
        "fasta_validate" => {
            arguments.insert(
                "header".into(),
                serde_json::json!(required_assignment(&source, "header")?),
            );
            arguments.insert(
                "sequence".into(),
                serde_json::json!(required_assignment(&source, "sequence")?),
            );
        }
        "gene_expression" => {
            arguments.insert(
                "gene".into(),
                serde_json::json!(required_assignment(&source, "gene")?),
            );
            arguments.insert(
                "baseline".into(),
                serde_json::json!(required_f64(&source, "baseline")?),
            );
            arguments.insert(
                "treatment".into(),
                serde_json::json!(required_f64(&source, "treatment")?),
            );
            arguments.insert(
                "threshold".into(),
                serde_json::json!(optional_f64(&source, "threshold")?.unwrap_or(2.0)),
            );
        }
        "metabolite_similarity" => {
            arguments.insert(
                "fingerprint1".into(),
                serde_json::json!(required_f64_list(&source, "fingerprint1")?),
            );
            arguments.insert(
                "fingerprint2".into(),
                serde_json::json!(required_f64_list(&source, "fingerprint2")?),
            );
        }
        "minhash" => {
            arguments.insert(
                "sequence".into(),
                serde_json::json!(required_assignment(&source, "sequence")?),
            );
            arguments.insert("k".into(), serde_json::json!(required_u64(&source, "k")?));
            arguments.insert(
                "sketch_size".into(),
                serde_json::json!(optional_u64(&source, "sketch_size")?.unwrap_or(64)),
            );
        }
        "upgma_tree" => {
            arguments.insert(
                "distances".into(),
                serde_json::json!(required_f64_list(&source, "distances")?),
            );
            arguments.insert("n".into(), serde_json::json!(required_u64(&source, "n")?));
        }
        _ => return Err(format!("Unknown bioinformatics operation `{operation}`.")),
    }
    Ok((
        "Bioinformatics.compute",
        serde_json::Value::Object(arguments),
    ))
}

fn gbm_var_request(document: &Document) -> Result<(&'static str, serde_json::Value), String> {
    let source = field_value(document, "gbm-var-input");
    Ok((
        "FinancialModeling.gbm_var",
        serde_json::json!({
            "s0": required_f64(&source, "s0")?,
            "mu": required_f64(&source, "mu")?,
            "sigma": required_f64(&source, "sigma")?,
            "time_horizon": required_f64(&source, "T")?,
            "dt": required_f64(&source, "dt")?,
            "portfolio_value": required_f64(&source, "portfolio")?,
            "confidence": required_f64(&source, "confidence")?,
            "paths": optional_u64(&source, "paths")?.unwrap_or(2048),
            "seed": optional_u64(&source, "seed")?.unwrap_or(42)
        }),
    ))
}

pub(super) fn logic_request(
    document: &Document,
    tool_name: &str,
) -> Result<(&'static str, serde_json::Value), String> {
    let normalized = tool_name.to_ascii_lowercase();
    if let Some(model) = normalized.strip_prefix("clinical-") {
        return clinical_request(document, model);
    }
    if let Some(operation) = normalized.strip_prefix("chemistry-") {
        return chemistry_request(document, operation);
    }
    if let Some(operation) = normalized.strip_prefix("bio-") {
        return bioinformatics_request(document, operation);
    }
    if let Some(operation) = normalized.strip_prefix("physics-") {
        return physics_request(document, operation);
    }
    if let Some(operation) = normalized.strip_prefix("calculus-") {
        return calculus_request(document, operation);
    }
    if normalized == "ode-solver" {
        return ode_request(document);
    }
    if normalized == "gbm-var" {
        return gbm_var_request(document);
    }
    if matches!(
        normalized.as_str(),
        "rdf-star" | "rdfstar-extract" | "ontology-compile" | "ontology-validate"
    ) {
        return authoring_request(document, normalized.as_str());
    }
    if matches!(
        normalized.as_str(),
        "abductive"
            | "fuzzy"
            | "probabilistic"
            | "graph-theory"
            | "interval"
            | "manifold-10d"
            | "epistemic-boundaries"
            | "modal"
    ) {
        let mode = match normalized.as_str() {
            "graph-theory" => "graph",
            other => other,
        };
        return advanced_request(document, mode);
    }
    if matches!(
        normalized.as_str(),
        "ctl" | "defeasible" | "linear" | "dialectical" | "dialectical-counterfactual"
    ) {
        return formal_request(document, normalized.as_str());
    }
    if matches!(
        normalized.as_str(),
        "ltl"
            | "ltl-safety"
            | "asp"
            | "asp-optimal"
            | "paraconsistent-saturation"
            | "inference"
            | "inference-explain"
    ) {
        return reasoning_request(document, normalized.as_str());
    }
    if matches!(
        normalized.as_str(),
        "jural"
            | "stit"
            | "stit-joint"
            | "causal"
            | "causal-overdetermination"
            | "responsibility"
            | "responsibility-vacuum"
            | "capacity"
            | "delegation"
            | "delegation-revocation"
            | "contract"
            | "consensus"
            | "meta-deontic"
            | "meta-deontic-endorse"
            | "grounded-extension"
            | "preferred-extension"
            | "stable-extension"
            | "complete-extension"
            | "argumentation-visualize"
    ) {
        return legal_request(document, normalized.as_str());
    }
    if matches!(
        normalized.as_str(),
        "value-flow"
            | "value-flow-royalty"
            | "interaction-governance"
            | "identity-fabric"
            | "identity-fabric-survive"
            | "capability-gap"
            | "legal-compose"
            | "legal-compose-zk"
            | "deontic-compose"
            | "deontic-compose-mens"
    ) {
        return governance_request(document, normalized.as_str());
    }
    if let Some(operation) = normalized.strip_prefix("allen-rcc8-") {
        return spatial_request(document, operation);
    }
    if normalized == "manifold-logic" {
        return spatial_request(document, "manifold-logic");
    }
    if matches!(
        normalized.as_str(),
        "bytecode-vm"
            | "bytecode-vm-stats"
            | "slg-arena"
            | "compute-profile"
            | "model-lifecycle"
            | "model-lifecycle-evict"
            | "inference-monitor"
            | "gguf-tokenizer"
            | "p64-weight"
    ) || normalized.starts_with("forge-compute")
        || normalized.starts_with("privacy")
    {
        return infra_request(document, normalized.as_str());
    }
    if matches!(
        normalized.as_str(),
        "crdt-sync"
            | "agency-merkle"
            | "key-vault"
            | "policy-evaluator"
            | "consent-manager"
            | "carrier"
            | "control-feedback"
            | "likeliness"
            | "qubo"
            | "owl-converter"
    ) {
        return infra_ext_request(document, normalized.as_str());
    }
    if normalized == "comorbidity" {
        let source = field_value(document, "comorbidity-input");
        return Ok((
            "ClinicalRisk.comorbidity",
            serde_json::json!({
                "patient": required_assignment(&source, "patient")?,
                "target_organ": required_assignment(&source, "target_organ")?,
                "conditions": required_string_list(&source, "conditions")?,
                "antecedent": assignment(&source, "antecedent"),
                "consequent": assignment(&source, "consequent"),
                "severity": optional_f64(&source, "severity")?
            }),
        ));
    }
    if normalized == "dicom-render" {
        let source = field_value(document, "dicom-metadata");
        let study_uid = field_value(document, "dicom-study-uid");
        if study_uid.trim().is_empty() {
            return Err("Enter the DICOM Study UID before rendering.".into());
        }
        return Ok((
            "MedicalImaging.hu_window",
            serde_json::json!({
                "study_uid": study_uid,
                "width": required_u64(&source, "width")?,
                "height": required_u64(&source, "height")?,
                "pixels": required_f64_list(&source, "pixels")?,
                "window": required_f64(&source, "window")?,
                "level": required_f64(&source, "level")?
            }),
        ));
    }
    if normalized == "diffusion" {
        let source = field_value(document, "diffusion-input");
        return Ok((
            "Physics.heat_diffusion_1d",
            serde_json::json!({
                "initial": required_f64_list(&source, "initial")?,
                "alpha": required_f64(&source, "alpha")?,
                "dx": required_f64(&source, "dx")?,
                "total_time": required_f64(&source, "total_time")?,
                "samples": required_u64(&source, "samples")?
            }),
        ));
    }
    match normalized.as_str() {
        "deontic-compile" => {
            let modality = field_value(document, "deontic-operator").to_ascii_lowercase();
            if modality == "waive" {
                return Err("WAIVE has no registered native compilation contract yet.".into());
            }
            Ok((
                "DeonticLogic.evaluate",
                serde_json::json!({
                    "operation": "compile",
                    "modality": modality,
                    "target": field_value(document, "deontic-target"),
                    "body": field_value(document, "deontic-body")
                }),
            ))
        }
        "deontic" => {
            let modality = field_value(document, "deontic-operator").to_ascii_lowercase();
            if modality == "waive" {
                return Err("WAIVE has no registered native evaluation contract yet.".into());
            }
            Ok((
                "DeonticLogic.evaluate",
                serde_json::json!({
                    "modality": modality,
                    "target": field_value(document, "deontic-target"),
                    "body": field_value(document, "deontic-body")
                }),
            ))
        }
        "n3" | "n3-evaluate" => Ok((
            "N3Logic.evaluate",
            serde_json::json!({
                "source": field_value(document, "n3-editor"),
                "mode": "evaluate",
                "context": "urn:poet:n3:workbench"
            }),
        )),
        "n3-parse" => Ok((
            "N3Logic.evaluate",
            serde_json::json!({
                "source": field_value(document, "n3-editor"),
                "mode": "parse",
                "context": "urn:poet:n3:workbench"
            }),
        )),
        "epistemic" => {
            let modality = match field_value(document, "epistemic-op").as_str() {
                "affirms" => "believes",
                "knows" => "knows",
                _ => return Err("This epistemic operator is not exposed yet.".into()),
            };
            Ok((
                "EpistemicLogic.evaluate",
                serde_json::json!({
                    "modality": modality,
                    "agent": field_value(document, "epistemic-agent"),
                    "body": field_value(document, "epistemic-editor")
                }),
            ))
        }
        "paraconsistent" => Ok(("ParaconsistentLogic.route", serde_json::Value::Null)),
        "shacl" => {
            let subject = field_value(document, "shacl-target-class");
            if subject.trim().is_empty() {
                return Err("Enter a SHACL target class before validating the live graph.".into());
            }
            Ok(("SHACL.validate", serde_json::json!({ "subject": subject })))
        }
        "description-logic" => {
            let source = field_value(document, "dl-editor");
            let values = call_arguments(&source, "subsumes").ok_or_else(|| {
                "Enter a subsumes(SubClass, SuperClass) query before evaluation.".to_string()
            })?;
            if values.len() != 2 {
                return Err("Subsumption evaluation requires exactly two classes.".into());
            }
            Ok((
                "TemporalAndDescriptionLogic.subsumption",
                serde_json::json!([values[0], values[1]]),
            ))
        }
        _ => Err(format!(
            "No registered native capability contract currently matches the {tool_name} panel."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::capability_contract_for_button;

    #[test]
    fn only_completed_button_bindings_are_advertised() {
        assert_eq!(
            capability_contract_for_button("clinical-risk-evaluate"),
            Some(("data-capability-prefix", "ClinicalRisk."))
        );
        assert_eq!(
            capability_contract_for_button("chemistry-evaluate"),
            Some(("data-capability-prefix", "OrganicChemistry."))
        );
        assert_eq!(
            capability_contract_for_button("bioinformatics-evaluate"),
            Some(("data-capability-prefix", "Bioinformatics."))
        );
        assert_eq!(
            capability_contract_for_button("gbm-var-evaluate"),
            Some(("data-capability-id", "FinancialModeling.gbm_var"))
        );
        assert_eq!(
            capability_contract_for_button("modal-evaluate"),
            Some(("data-capability-id", "AdvancedLogic.compute"))
        );
        assert_eq!(
            capability_contract_for_button("value-flow-evaluate"),
            Some(("data-capability-id", "GovernanceLogic.compute"))
        );
        assert_eq!(
            capability_contract_for_button("allen-rcc8-evaluate"),
            Some(("data-capability-id", "SpatialLogic.compute"))
        );
        assert_eq!(
            capability_contract_for_button("bytecode-vm-trace"),
            Some(("data-capability-id", "InfraLogic.compute"))
        );
        assert_eq!(
            capability_contract_for_button("owl-evaluate"),
            Some(("data-capability-id", "InfraExtLogic.compute"))
        );
    }
}
