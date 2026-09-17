//! Command dispatch for the logic workbench — maps command palette labels
//! to tool IDs and opens the workbench.

use web_sys::Document;

pub(super) fn dispatch_command(document: &Document, label: &str) -> bool {
    let tool = match label {
        "Logic Workbench" => {
            super::toggle_logic_workbench(document);
            return true;
        }
        // P0 core
        "Deontic Rule Editor" => "deontic",
        "N3 Logic Studio" => "n3",
        "SHACL Validator" => "shacl",
        "RDF-Star Editor" => "rdfstar",
        "Ontology Builder" => "ontology",
        "Evaluate Modality" => "modality",
        "Symbolic Logic Inference" => "infer",
        "Jural Relations" => "jural",
        "Argumentation Framework" => "argumentation",
        // P1 legal
        "STIT Agency" => "stit",
        "Causal Liability" => "causal",
        "Responsibility / Meta-Guard" => "responsibility",
        "Capacity Evaluator" => "capacity",
        "Delegation Tracker" => "delegation",
        "Contract Formation" => "contract",
        "Consensus / Partition" => "consensus",
        "Meta-Deontic Breach" => "meta_deontic",
        // P1 governance
        "Value Flow / Commons" => "value_flow",
        "Interaction Governance" => "interaction_gov",
        "Identity Fabric" => "identity_fabric",
        "Capability Gap Analyzer" => "capability_gap",
        "Legal Compose" => "legal_compose",
        "Deontic Compose" => "deontic_compose",
        // P1 logic
        "Epistemic Logic" => "epistemic",
        "Paraconsistent Logic" => "paraconsistent",
        "Linear Temporal Logic" => "ltl",
        "Computation Tree Logic" => "ctl",
        "Answer Set Programming" => "asp",
        "Defeasible Logic" => "defeasible",
        "Linear Logic" => "linear",
        "Description Logic" => "description",
        "Dialectical Logic" => "dialectical",
        // P1 advanced
        "Abductive Reasoning" => "abductive",
        "Fuzzy Logic" => "fuzzy",
        "Probabilistic Reasoning" => "probabilistic",
        "Graph Theory" => "graph_theory",
        "Interval Logic" => "interval",
        "Manifold 10D Logic" => "manifold_10d",
        "Epistemic Boundaries" => "epistemic_boundaries",
        "Modal Logic" => "modal",
        // P2 domain computational
        "Clinical Risk Scorer" => "clinical_risk",
        "DICOM Viewer" => "dicom_viewer",
        "Comorbidity Analyzer" => "comorbidity",
        "Chemistry Modeler" => "chemistry",
        "Physics Simulator" => "physics",
        "ODE Solver" => "ode_solver",
        "Bioinformatics Lab" => "bioinformatics",
        "GBM / VaR Simulator" => "gbm_var",
        "Diffusion Controller" => "diffusion",
        // P2 infrastructure
        "Bytecode / VM Inspector" => "bytecode_vm",
        "SLG Arena Inspector" => "slg_arena",
        "Forge Compute Probe" => "forge_compute",
        "Compute Profile" => "compute_profile",
        "Privacy / HE / DP" => "privacy",
        "Model Lifecycle" => "model_lifecycle",
        "Inference Monitor" => "inference_monitor",
        "GGUF Tokenizer Inspector" => "gguf_tokenizer",
        "P64 Weight Inspector" => "p64_weight",
        // P2 infrastructure extensions
        "CRDT / Sync Dashboard" => "crdt_sync",
        "Agency / Merkle Inspector" => "agency_merkle",
        "Key Vault Manager" => "key_vault",
        "Policy Evaluator" => "policy_evaluator",
        "Consent Manager" => "consent_manager",
        "Carrier / Media Binding" => "carrier",
        "Control Feedback" => "control_feedback",
        "Likeliness" => "likeliness",
        "QUBO Compiler" => "qubo",
        "OWL Converter" => "owl_converter",
        // P2 extras
        "Allen / RCC8" => "allen_rcc8",
        "Manifold Logic" => "manifold_logic",
        "Calculus" => "calculus",
        _ => return false,
    };
    super::open_to_tool(document, tool);
    true
}
