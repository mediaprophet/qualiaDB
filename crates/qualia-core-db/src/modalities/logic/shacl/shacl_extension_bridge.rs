//! Bridges typed SHACL extension configs (`*_shacl.rs`) into `SlgOpcode` sequences.

use crate::modalities::logic::core_modalities_shacl::{
    ASPConfiguration, CalculusConfiguration, DialecticalConfiguration, EpistemicConfiguration,
    GraphConfiguration, LTLConfiguration, ParaconsistentConfiguration,
};
use crate::modalities::logic::infrastructure_shacl::SolverConfiguration;
use crate::modalities::logic::specialized_libs_shacl::{
    CryptographicConfiguration, EngineeringSimulationConfiguration, MedicalDataConfiguration,
};
use crate::webizen::SlgOpcode;

/// Merge extension opcodes into a compiled shape opcode list.
pub fn append_extension_opcodes(opcodes: &mut Vec<SlgOpcode>, extension_id: &str) {
    match extension_id {
        "q42:EpistemicConfiguration" => {
            let cfg = EpistemicConfiguration {
                max_certainty: 255,
                max_nesting_depth: 8,
                max_agent_contexts: 64,
                common_knowledge_buffer_size: 128,
            };
            opcodes.extend(cfg.to_opcodes());
            opcodes.push(SlgOpcode::NativeEpistemicEval(128));
        }
        "q42:ParaconsistentConfiguration" => {
            let cfg = ParaconsistentConfiguration {
                max_isolation_severity: 255,
                isolation_buffer_size: 512,
                merge_threshold: 1.0,
                require_context_tracking: true,
            };
            opcodes.extend(cfg.to_opcodes());
            opcodes.push(SlgOpcode::NativeParaconsistentIsolate);
        }
        "q42:LTLConfiguration" => {
            let cfg = LTLConfiguration {
                max_trace_length: 1024,
                max_formula_depth: 16,
                allowed_operators: vec!["G".into(), "F".into(), "X".into(), "U".into(), "R".into()],
                require_well_formedness: true,
            };
            opcodes.extend(cfg.to_opcodes());
            opcodes.push(SlgOpcode::NativeLtlGlobally);
        }
        "q42:GraphConfiguration" => {
            let cfg = GraphConfiguration {
                max_nodes: 4096,
                max_edges: 16384,
                max_node_degree: 256,
                allowed_graph_types: vec!["directed".into()],
            };
            opcodes.extend(cfg.to_opcodes());
            opcodes.push(SlgOpcode::NativeAllenInterval(0));
        }
        "q42:CalculusConfiguration" => {
            let cfg = CalculusConfiguration {
                max_grid_points: 1_000_000,
                max_integration_steps: 100_000,
                allowed_integrators: vec!["simpsons".into(), "rk4".into()],
                require_convergence_check: true,
            };
            opcodes.extend(cfg.to_opcodes());
            opcodes.push(SlgOpcode::NativeCalcSimpsons(0, 0, 0, 0));
        }
        "q42:ASPConfiguration" => {
            let cfg = ASPConfiguration {
                max_variables: 256,
                max_clauses: 1024,
                max_answer_sets: 8,
                allowed_solvers: vec!["clingo".into()],
            };
            opcodes.extend(cfg.to_opcodes());
            opcodes.push(SlgOpcode::NativeAspStableModels);
        }
        "q42:DialecticalConfiguration" => {
            let cfg = DialecticalConfiguration {
                max_thesis_count: 8,
                max_antithesis_count: 8,
                max_synthesis_rounds: 4,
                require_contradiction_detection: true,
            };
            opcodes.extend(cfg.to_opcodes());
            opcodes.push(SlgOpcode::NativeDialecticalSynthesis);
        }
        "q42:SolverConfiguration" => {
            let cfg = SolverConfiguration {
                max_iterations: 10_000,
                convergence_tolerance: 1e-6,
                max_step_size: 1.0,
                min_step_size: 1e-9,
                allowed_solver_types: vec!["calculus".into(), "optimization".into()],
            };
            opcodes.extend(cfg.to_opcodes());
        }
        "q42:CryptographicConfiguration" => {
            let cfg = CryptographicConfiguration {
                min_key_length_bits: 256,
                allowed_algorithms: vec!["ed25519".into(), "aes".into()],
                require_fips_compliance: true,
                max_operation_time_ms: 5000,
            };
            opcodes.extend(cfg.to_opcodes());
        }
        "q42:MedicalDataConfiguration" => {
            let cfg = MedicalDataConfiguration {
                require_hipaa_compliance: true,
                require_de_identification: true,
                allowed_data_types: vec!["fhir".into()],
                max_patient_records: 10_000,
            };
            opcodes.extend(cfg.to_opcodes());
            opcodes.push(SlgOpcode::NativeFhirObservation(0));
        }
        "q42:EngineeringSimulationConfiguration" => {
            let cfg = EngineeringSimulationConfiguration {
                max_mesh_elements: 1_000_000,
                allowed_analysis_types: vec!["structural".into()],
                require_convergence: true,
                max_simulation_time_hours: 24.0,
            };
            opcodes.extend(cfg.to_opcodes());
        }
        _ => {}
    }
    if !matches!(opcodes.last(), Some(SlgOpcode::Halt)) {
        opcodes.push(SlgOpcode::Halt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epistemic_extension_appends_opcodes() {
        let mut ops = Vec::new();
        append_extension_opcodes(&mut ops, "q42:EpistemicConfiguration");
        assert!(ops.len() > 2);
        assert!(ops.contains(&SlgOpcode::NativeEpistemicEval(128)));
    }
}