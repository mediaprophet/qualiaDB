//! Wave 39 leftover Live chains — remaining curated Host singles (exhausts helper-aware Q2).

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

pub(super) fn longtail_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        live_tool("scientific:longtail_bio_align", "Bioinformatics align", "Bioinformatics.align", "SW align via Bioinformatics.align.", "lab", "sci"),
        live_tool("scientific:longtail_dmp", "Discrete maximum principle", "Calculus.discrete_maximum_principle_holds", "DMP check via Calculus.discrete_maximum_principle_holds.", "lab", "sci"),
        live_tool("scientific:longtail_poisson", "Poisson Dirichlet", "Calculus.solve_poisson_dirichlet", "Poisson solve via Calculus.solve_poisson_dirichlet.", "lab", "sci"),
        live_tool("scientific:longtail_t_norm", "Gödel t-norm", "CausalFuzzyAndControl.t_norm", "Gödel t-norm via CausalFuzzyAndControl.t_norm.", "lab", "sci"),
        live_tool("scientific:longtail_parse_bse", "Parse BSE JSON", "Chemistry.parse_bse_json", "Basis set via Chemistry.parse_bse_json.", "lab", "sci"),
        live_tool("scientific:longtail_conduction", "1-D conduction", "EngineeringAnalysis.analyze_conduction", "Conduction via EngineeringAnalysis.analyze_conduction.", "lab", "sci"),
        live_tool("scientific:longtail_fem", "FEM static", "EngineeringAnalysis.fem_static", "Truss FEM via EngineeringAnalysis.fem_static.", "lab", "sci"),
        live_tool("scientific:longtail_simpson", "Simpson integral", "NumericalCalculus.simpson", "Simpson via NumericalCalculus.simpson.", "lab", "sci"),
        live_tool("scientific:longtail_ontology", "Ontology align", "OntologyAlignment.align", "Align via OntologyAlignment.align.", "lab", "sci"),
        live_tool("scientific:longtail_units", "Convert units", "PhysicalUnits.convert", "Unit convert via PhysicalUnits.convert.", "lab", "sci"),
        live_tool("scientific:longtail_projectile", "Projectile", "PhysicsAndODE.projectile", "Ballistics via PhysicsAndODE.projectile.", "lab", "sci"),
        live_tool("scientific:longtail_poly_coeffs", "Polynomial coeffs", "PolynomialAlgebra.coeffs", "Coeffs via PolynomialAlgebra.coeffs.", "lab", "sci"),
        live_tool("scientific:longtail_bessel", "Bessel J", "SpecialFunctionsAndTransforms.bessel_j", "J_n via SpecialFunctionsAndTransforms.bessel_j.", "lab", "sci"),
        live_tool("scientific:longtail_ltl_finally", "LTL finally", "TemporalAndDescriptionLogic.ltl.finally", "F(φ) via TemporalAndDescriptionLogic.ltl.finally.", "lab", "sci"),
        live_tool("scientific:longtail_ltl_globally", "LTL globally", "TemporalAndDescriptionLogic.ltl.globally", "G(φ) via TemporalAndDescriptionLogic.ltl.globally.", "lab", "sci"),
        live_tool("scientific:longtail_hash_iri", "Hash IRI", "hash.iri", "60-bit FNV via hash.iri.", "lab", "sci"),
    ]
}

pub(super) fn wealth_live_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        live_tool("econ:wealth_live_aggregate", "Aggregate wealth", "Econ.aggregate_wealth", "Availability via Econ.aggregate_wealth.", "finance", "econ"),
        live_tool("econ:wealth_live_cumulative", "Cumulative wealth", "Econ.cumulative_wealth", "Wealth path via Econ.cumulative_wealth.", "finance", "econ"),
        live_tool("econ:wealth_live_narrative", "Econ narrative divergence", "Econ.narrative_divergence", "Availability via Econ.narrative_divergence.", "finance", "econ"),
    ]
}

pub(super) fn net_live_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        live_tool("comm:net_live_peer_hash", "Peer hash", "Net.peer_hash", "DID hash via Net.peer_hash.", "comm", "comm"),
        live_tool("comm:net_live_sonic_pack", "Sonic pack", "Net.sonic_pack", "Packed sonic token via Net.sonic_pack.", "comm", "comm"),
    ]
}

pub(super) fn id_live_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        live_tool("rights:id_live_agency", "Agency evaluate", "Agency.evaluate", "Ed25519 agency via Agency.evaluate.", "evaluate", "rights"),
        live_tool("rights:id_live_parse_did", "Parse did:q42", "ContractsIdentityAndConsensus.parse_did_q42", "Parse via ContractsIdentityAndConsensus.parse_did_q42.", "evaluate", "rights"),
        live_tool("rights:id_live_board_project", "Board project", "CooperativeWork.board_project", "Kanban via CooperativeWork.board_project.", "evaluate", "rights"),
    ]
}
