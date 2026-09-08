//! Wave 38 leftover Live chains — Medical / Manifold / crypto / gemm / Privacy / Sentinel / discovery / DAG / FinancialModeling.

use super::*;

fn live_tool(
    id: &'static str,
    label: &'static str,
    scope: &'static str,
    description: &'static str,
    icon: &'static str,
    ontology_prefix: &'static str,
) -> Box<dyn crate::tool_chest::core::tool::Tool> {
    Box::new(SimpleTool::new(
        ToolMetadata {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            kind: ToolKind::RunAction,
            capability_scope: Some(scope.into()),
            ontology_prefix: ontology_prefix.into(),
            description: description.into(),
        },
        ActionType::Invoke,
    ))
}

pub(super) fn med_live_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        live_tool("health:med_live_tanimoto", "Tanimoto", "Medical.tanimoto", "Fingerprint Tanimoto via Medical.tanimoto.", "health", "health"),
        live_tool("health:med_live_fingerprint", "Structural fingerprint", "Medical.structural_fingerprint", "Morgan-style bits via Medical.structural_fingerprint.", "health", "health"),
        live_tool("health:med_live_intensity", "Intensity grid", "Medical.analyze_intensity_grid", "Histogram via Medical.analyze_intensity_grid.", "health", "health"),
        live_tool("health:med_live_differential", "Differential analysis", "MedicalComputing.analyze_differential", "Bayes update via MedicalComputing.analyze_differential.", "health", "health"),
        live_tool("health:med_live_screen", "Screen compounds", "MedicalComputing.screen_compounds", "Screen via MedicalComputing.screen_compounds.", "health", "health"),
    ]
}

pub(super) fn manifold_live_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        live_tool("spatial:manifold_live_axes", "Manifold axes", "Manifold.axes", "Axis taxonomy via Manifold.axes.", "3d", "geo"),
        live_tool("spatial:manifold_live_distance", "Manifold distance", "Manifold.distance", "10-D distance via Manifold.distance.", "3d", "geo"),
        live_tool("spatial:manifold_live_project", "Manifold project", "Manifold.project", "Desk projection via Manifold.project.", "3d", "geo"),
    ]
}

pub(super) fn crypto_priv_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        live_tool("scientific:crypto_live_sha256", "SHA-256", "QuantumAndCryptographic.sha256", "SHA-256 of a string via QuantumAndCryptographic.sha256.", "lab", "sci"),
        live_tool("scientific:crypto_live_sha512", "SHA-512", "QuantumAndCryptographic.sha512", "SHA-512 via QuantumAndCryptographic.sha512.", "lab", "sci"),
        live_tool("scientific:crypto_live_blake3", "BLAKE3", "QuantumAndCryptographic.blake3", "BLAKE3 via QuantumAndCryptographic.blake3.", "lab", "sci"),
        live_tool("scientific:gemm_live", "GEMM", "LinearAlgebra.gemm", "BLAS GEMM via LinearAlgebra.gemm (solver CPU floor, GPU when present).", "lab", "sci"),
        live_tool("scientific:privacy_live_gaussian", "Gaussian sigma", "Privacy.gaussian_sigma", "(ε, δ)-DP sigma via Privacy.gaussian_sigma.", "lab", "sci"),
        live_tool("scientific:sentinel_live_gate", "Sentinel gate", "Sentinel.gate", "Privilege gate via Sentinel.gate.", "lab", "sci"),
    ]
}

pub(super) fn disc_dag_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        live_tool("ai:disc_live_catalog", "Capability catalog", "CapabilityDiscovery.catalog", "TTL catalog via CapabilityDiscovery.catalog.", "ai", "ai"),
        live_tool("ai:disc_live_coverage", "Capability coverage", "CapabilityDiscovery.coverage", "Coverage matrix via CapabilityDiscovery.coverage.", "ai", "ai"),
        live_tool("ai:dag_live_execute", "DAG execute", "agent.dag.execute", "Execute a DAG via agent.dag.execute.", "ai", "ai"),
        live_tool("ai:dag_live_validate", "DAG validate", "agent.dag.validate", "Validate a DAG via agent.dag.validate.", "ai", "ai"),
        live_tool("ai:dag_live_status", "DAG status", "agent.dag.status", "Executor status via agent.dag.status.", "ai", "ai"),
    ]
}

pub(super) fn fm_live_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        live_tool("econ:fm_live_black_scholes", "Black–Scholes", "FinancialModeling.black_scholes", "ATM call via FinancialModeling.black_scholes.", "finance", "econ"),
        live_tool("econ:fm_live_portfolio_risk", "Portfolio risk", "FinancialModeling.portfolio_risk", "Risk from prices via FinancialModeling.portfolio_risk.", "finance", "econ"),
    ]
}
