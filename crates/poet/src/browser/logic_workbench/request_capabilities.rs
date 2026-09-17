//! Capability requirements advertised by Logic Workbench controls.

pub(super) fn capability_contract_for_button(
    button_id: &str,
) -> Option<(&'static str, &'static str)> {
    Some(match button_id {
        "deontic-evaluate" | "deontic-compile" => ("data-capability-id", "DeonticLogic.evaluate"),
        "epistemic-evaluate" => ("data-capability-id", "EpistemicLogic.evaluate"),
        "paraconsistent-evaluate" | "paraconsistent-saturation" => {
            ("data-capability-id", "ParaconsistentLogic.route")
        }
        "ltl-evaluate" | "ltl-safety" => (
            "data-capability-id",
            "TemporalAndDescriptionLogic.ltl.evaluate",
        ),
        "asp-evaluate" | "asp-optimal" => ("data-capability-id", "SymbolicAndDefeasibleLogic.asp"),
        "dl-evaluate" => (
            "data-capability-id",
            "TemporalAndDescriptionLogic.subsumption",
        ),
        "shacl-validate" => ("data-capability-id", "SHACL.validate"),
        "clinical-risk-evaluate" => ("data-capability-prefix", "ClinicalRisk."),
        "comorbidity-evaluate" => ("data-capability-id", "ClinicalRisk.comorbidity"),
        "dicom-render" => ("data-capability-id", "MedicalImaging.hu_window"),
        "diffusion-evaluate" => ("data-capability-id", "Physics.heat_diffusion_1d"),
        "calculus-evaluate" => ("data-capability-id", "CalculusWorkbench.compute"),
        "n3-evaluate" => ("data-capability-id", "N3Logic.evaluate"),
        "infer-run" | "infer-explain" => ("data-capability-id", "N3Logic.evaluate"),
        "rdfstar-resolve" | "onto-compile" | "onto-validate" => {
            ("data-capability-id", "GraphAuthoring.process")
        }
        "rdfstar-extract" => ("data-capability-id", "NLP.relation_extract"),
        "onto-import" => ("data-capability-id", "GraphAuthoring.process"),
        "abductive-evaluate"
        | "fuzzy-evaluate"
        | "probabilistic-evaluate"
        | "graph-theory-evaluate"
        | "interval-evaluate"
        | "manifold-10d-evaluate"
        | "epistemic-boundaries-evaluate"
        | "modal-evaluate" => ("data-capability-id", "AdvancedLogic.compute"),
        "ctl-evaluate"
        | "defeasible-evaluate"
        | "linear-evaluate"
        | "dialectical-evaluate"
        | "dialectical-counter" => ("data-capability-id", "FormalLogic.compute"),
        "jural-analyze"
        | "stit-evaluate"
        | "stit-joint"
        | "causal-evaluate"
        | "causal-overdetermine"
        | "resp-evaluate"
        | "resp-vacuum"
        | "capacity-evaluate"
        | "deleg-evaluate"
        | "deleg-revoke"
        | "contract-evaluate"
        | "consensus-evaluate"
        | "meta-deontic-evaluate"
        | "meta-deontic-endorse"
        | "arg-evaluate"
        | "arg-visualize" => ("data-capability-id", "LegalLogic.compute"),
        "value-flow-evaluate"
        | "value-flow-royalty"
        | "interaction-gov-evaluate"
        | "identity-fabric-evaluate"
        | "identity-fabric-survive"
        | "capability-gap-evaluate"
        | "legal-compose-evaluate"
        | "legal-compose-zk"
        | "deontic-compose-evaluate"
        | "deontic-compose-mens" => ("data-capability-id", "GovernanceLogic.compute"),
        "allen-rcc8-evaluate" | "manifold-logic-evaluate" => {
            ("data-capability-id", "SpatialLogic.compute")
        }
        "bytecode-vm-trace"
        | "bytecode-vm-stats"
        | "slg-arena-inspect"
        | "forge-compute-evaluate"
        | "compute-profile-evaluate"
        | "privacy-evaluate"
        | "model-lifecycle-evaluate"
        | "model-lifecycle-evict"
        | "inference-monitor-evaluate"
        | "gguf-tokenizer-evaluate"
        | "p64-weight-evaluate" => ("data-capability-id", "InfraLogic.compute"),
        "crdt-sync-evaluate"
        | "agency-merkle-evaluate"
        | "key-vault-evaluate"
        | "policy-evaluator-evaluate"
        | "consent-evaluate"
        | "carrier-evaluate"
        | "control-feedback-evaluate"
        | "likeliness-evaluate"
        | "qubo-evaluate"
        | "owl-evaluate" => ("data-capability-id", "InfraExtLogic.compute"),
        "chemistry-evaluate" => ("data-capability-prefix", "OrganicChemistry."),
        "bioinformatics-evaluate" => ("data-capability-prefix", "Bioinformatics."),
        "physics-evaluate" | "ode-evaluate" => ("data-capability-id", "PhysicsWorkbench.compute"),
        "gbm-var-evaluate" => ("data-capability-id", "FinancialModeling.gbm_var"),
        _ => return None,
    })
}
