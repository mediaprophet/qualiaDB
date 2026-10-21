//! Fiduciary tool dispatch routing for Qualia Model Context Protocol (MCP).
//!
//! Enforces zero-allocation boundaries and dispatches incoming tool actions
//! to their specialized domain handlers.
//! Follows `AGENTS.md` §0-B (<500 lines).

use crate::tools::*;
use crate::types::{parse_tool_args, McpIntentFrame, McpSystemError, RawToolPayload};

/// Dispatches incoming tool actions without dynamic heap allocation in inner matching paths.
pub fn enforce_fiduciary_tool_dispatch(
    payload: RawToolPayload,
    intent_frame: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    match payload.tool_name {
        // ── Graph & Codebase Audit ──────────────────────────────────────────
        b"query_sparql" => query_sparql(payload.arguments_raw),
        b"query_graph" => {
            if intent_frame.sanctuary_override.is_none() {
                return Err(McpSystemError::SanctuaryGateTriggered);
            }
            query_sparql(payload.arguments_raw)
        }
        b"codebase_audit" => codebase_audit(payload.arguments_raw),
        b"get_graph_stats" => get_graph_stats(payload.arguments_raw),
        b"list_ontologies" => list_ontologies(payload.arguments_raw),
        b"ingest_ontology" => ingest_ontology(payload.arguments_raw),
        b"get_did_info" => get_did_info(payload.arguments_raw),
        b"get_wallet_status" => get_wallet_status(payload.arguments_raw),

        // ── VibeScript DSL ──────────────────────────────────────────────────
        b"vibe_check" => vibe_check(payload.arguments_raw),
        b"vibe_compile" => vibe_compile(payload.arguments_raw),
        b"vibe_format" => vibe_format(payload.arguments_raw),
        b"vibe_invoke" => vibe_invoke(payload.arguments_raw),
        b"vibe_eval" => vibe_eval(payload.arguments_raw),
        b"vibe_catalog" => vibe_catalog(payload.arguments_raw),
        b"wgsl_forge" => wgsl_forge(payload.arguments_raw),
        b"poet_authoring" => poet_authoring(payload.arguments_raw),
        b"poet_studio_domain" => poet_studio_domain(payload.arguments_raw),
        b"poet_artifact_lifecycle" => poet_artifact_lifecycle(payload.arguments_raw),

        // ── Simulation: Geomechanics, Space Weather, Fire Ecology ───────────
        b"sim_soil_compaction" => sim_soil_compaction(payload.arguments_raw),
        b"sim_trench_excavation" => sim_trench_excavation(payload.arguments_raw),
        b"sim_space_weather_eval" => sim_space_weather_eval(payload.arguments_raw),
        b"sim_fire_ecology_eval" => sim_fire_ecology_eval(payload.arguments_raw),

        // ── Geospatial & Coordinates ────────────────────────────────────────
        b"geospatial_transform" => geospatial_transform(payload.arguments_raw),
        b"geospatial_hilbert" => geospatial_hilbert(payload.arguments_raw),
        b"geospatial_strata" => geospatial_strata(payload.arguments_raw),

        // ── Vision, Biosense & Media ────────────────────────────────────────
        b"computer_vision" => computer_vision(payload.arguments_raw),
        b"vision_rppg" => vision_rppg(payload.arguments_raw),
        b"vision_respiration" => vision_respiration(payload.arguments_raw),
        b"vision_liveness_pad" => vision_liveness_pad(payload.arguments_raw),

        // ── Audio Intelligence ──────────────────────────────────────────────
        b"audio_features" => audio_features(payload.arguments_raw),
        b"audio_spatial_hrtf" => audio_spatial_hrtf(payload.arguments_raw),

        // ── Logic Modalities & N3 Reasoning ─────────────────────────────────
        b"evaluate_modality" => evaluate_modality(payload.arguments_raw),
        b"evaluate_logic_rules" => evaluate_logic_rules(payload.arguments_raw),
        b"symbolic_logic_infer" => symbolic_logic_infer(payload.arguments_raw),
        b"inject_test_quin" => inject_test_quin(payload.arguments_raw),

        // ── SHACL Constraints & Verification ────────────────────────────────
        b"validate_shacl" => validate_shacl(payload.arguments_raw),
        b"validate_enumerated_identity" => validate_enumerated_identity_tool(payload.arguments_raw),
        b"shacl_credential_gate" => shacl_credential_gate(payload.arguments_raw),
        b"shacl_degrade_violations" => shacl_degrade_violations(payload.arguments_raw),
        b"shacl_route" => shacl_route(payload.arguments_raw),

        // ── QApps Host Runtime ──────────────────────────────────────────────
        b"list_qapps" => list_qapps(payload.arguments_raw),
        b"get_qapp_manifest" => {
            let v = parse_tool_args(payload.arguments_raw)?;
            let name = v.get("qapp_name").and_then(serde_json::Value::as_str).unwrap_or("");
            get_qapp_manifest(name)
        }
        b"inspect_qapp_readiness" => {
            let v = parse_tool_args(payload.arguments_raw)?;
            let name = v.get("qapp_name").and_then(serde_json::Value::as_str).unwrap_or("");
            inspect_qapp_readiness(name)
        }
        b"list_qapp_updates" => list_qapp_updates(payload.arguments_raw),
        b"describe_qapp_surface_schema" => crate::resources::describe_qapp_surface_schema(payload.arguments_raw),

        // ── Scientific & Engineering Compute ────────────────────────────────
        b"matrix_operation" => matrix_operation(payload.arguments_raw),
        b"algebra_solve_polynomial" => algebra_solve_polynomial(payload.arguments_raw),
        b"algebra_matrix_analyze" => algebra_matrix_analyze(payload.arguments_raw),
        b"cas" => cas(payload.arguments_raw),
        b"ode_solve" => ode_solve(payload.arguments_raw),
        b"chemical_analysis" => chemical_analysis(payload.arguments_raw),
        b"chemical_descriptors" => chemical_descriptors(payload.arguments_raw),
        b"statistical_analysis" => statistical_analysis(payload.arguments_raw),
        b"financial_model" => financial_model(payload.arguments_raw),
        b"medical_score" => medical_score(payload.arguments_raw),
        b"clinical_risk" => clinical_risk(payload.arguments_raw),
        b"engineering_analysis_op" => engineering_analysis_op(payload.arguments_raw),
        b"computational_geometry" => computational_geometry(payload.arguments_raw),
        b"geometry_manifests" => geometry_manifests(payload.arguments_raw),
        b"geometric_algebra_op" => geometric_algebra_op(payload.arguments_raw),
        b"bioinformatics_align" => bioinformatics_align(payload.arguments_raw),
        b"conditioning_compile" => conditioning_compile(payload.arguments_raw),
        b"conditioning_validate" => conditioning_validate(payload.arguments_raw),

        // ── Data Formats: Zero-Heap RDF, JSON, CSV ──────────────────────────
        b"parse_rdf" => parse_rdf(payload.arguments_raw),
        b"parse_json" => parse_json(payload.arguments_raw),
        b"parse_csv" => parse_csv(payload.arguments_raw),
        b"serialize_rdf" => serialize_rdf(payload.arguments_raw),
        b"serialize_json" => serialize_json(payload.arguments_raw),
        b"serialize_csv" => serialize_csv(payload.arguments_raw),

        // ── Cooperative Governance ──────────────────────────────────────────
        b"values_check" => values_check(payload.arguments_raw),
        b"values_evaluate" => values_evaluate(payload.arguments_raw),
        b"jural_correlate" => jural_correlate(payload.arguments_raw),
        b"deontic_govern" => deontic_govern(payload.arguments_raw),
        b"mcp_cooperate" => mcp_cooperate(payload.arguments_raw),

        // ── Quantum Computing & Optimization ────────────────────────────────
        b"qpu_optimize" => {
            if !intent_frame.qpu_enabled {
                return Err(McpSystemError::FeatureNotEnabled);
            }
            qpu_optimize(payload.arguments_raw)
        }
        b"qpu_dft" => qpu_dft(payload.arguments_raw),
        b"qpu_status" => qpu_status(payload.arguments_raw),

        // ── Local LLM Intelligence ─────────────────────────────────────────
        b"llm_infer" => {
            if !intent_frame.llm_enabled {
                return Err(McpSystemError::FeatureNotEnabled);
            }
            llm_infer(payload.arguments_raw)
        }
        b"llm_chat" => {
            if !intent_frame.llm_enabled {
                return Err(McpSystemError::FeatureNotEnabled);
            }
            llm_chat(payload.arguments_raw)
        }
        b"list_models" => list_models(payload.arguments_raw),

        // ── System Status & Tasks ───────────────────────────────────────────
        b"get_system_status" => get_system_status(payload.arguments_raw, intent_frame),
        b"list_capabilities" => list_capabilities(payload.arguments_raw),
        b"get_pending_tasks" => get_pending_tasks(payload.arguments_raw),
        b"run_docs_tests" => run_docs_tests(payload.arguments_raw),

        _ => Err(McpSystemError::ToolNotFound),
    }
}
