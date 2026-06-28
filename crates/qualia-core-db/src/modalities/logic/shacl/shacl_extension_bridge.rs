//! Bridges typed SHACL extension configs (`*_shacl.rs`) into `SlgOpcode` sequences.

use crate::modalities::logic::computational_maths_shacl::{
    AssumptionConfiguration, ExactArithmeticConfiguration, IntegralTransformConfiguration,
    InterpolationConfiguration, NumberTheoryConfiguration, NumericalMethodConfiguration,
    SpecialFunctionConfiguration, SymbolicCalculusConfiguration, UnitsConfiguration,
    VectorCalculusConfiguration,
};
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
        // ── Computational-mathematics engine ───────────────────────────────────
        "q42:UnitsConfiguration" => {
            let cfg = UnitsConfiguration {
                dimension_components: 7,
                require_dimensional_consistency: true,
                allowed_unit_systems: vec!["si".into()],
            };
            opcodes.extend(cfg.to_opcodes());
        }
        "q42:NumberTheoryConfiguration" => {
            let cfg = NumberTheoryConfiguration {
                max_input_bits: 4096,
                max_factorization_iterations: 1_000_000,
                allowed_operations: vec!["primality".into(), "factorization".into()],
            };
            opcodes.extend(cfg.to_opcodes());
        }
        "q42:SpecialFunctionConfiguration" => {
            let cfg = SpecialFunctionConfiguration {
                max_series_terms: 100_000,
                convergence_tolerance: 1e-12,
                allowed_families: vec!["bessel".into(), "zeta".into(), "airy".into()],
            };
            opcodes.extend(cfg.to_opcodes());
        }
        "q42:InterpolationConfiguration" => {
            let cfg = InterpolationConfiguration {
                max_nodes: 1_000_000,
                require_distinct_nodes: true,
                allowed_methods: vec!["lagrange".into(), "cubic_spline".into()],
            };
            opcodes.extend(cfg.to_opcodes());
        }
        "q42:IntegralTransformConfiguration" => {
            let cfg = IntegralTransformConfiguration {
                max_samples: 16_777_216,
                require_invertibility_check: true,
                allowed_transforms: vec!["dft".into(), "laplace".into(), "ztransform".into()],
            };
            opcodes.extend(cfg.to_opcodes());
        }
        "q42:VectorCalculusConfiguration" => {
            let cfg = VectorCalculusConfiguration {
                max_spatial_dimension: 3,
                require_field_smoothness: true,
                allowed_operators: vec!["gradient".into(), "divergence".into(), "curl".into()],
            };
            opcodes.extend(cfg.to_opcodes());
        }
        "q42:ExactArithmeticConfiguration" => {
            let cfg = ExactArithmeticConfiguration {
                max_digits: 1_000_000,
                require_exact: true,
                allowed_types: vec!["bigint".into(), "bigrational".into()],
            };
            opcodes.extend(cfg.to_opcodes());
        }
        "q42:SymbolicCalculusConfiguration" => {
            let cfg = SymbolicCalculusConfiguration {
                max_order: 1024,
                require_roundtrip_verification: true,
                allowed_operations: vec!["integrate".into(), "ode_solve".into(), "gradient".into()],
            };
            opcodes.extend(cfg.to_opcodes());
        }
        "q42:AssumptionConfiguration" => {
            let cfg = AssumptionConfiguration {
                require_sound_rewrite: true,
                allowed_signs: vec!["positive".into(), "nonnegative".into(), "nonzero".into()],
            };
            opcodes.extend(cfg.to_opcodes());
        }
        "q42:NumericalMethodConfiguration" => {
            let cfg = NumericalMethodConfiguration {
                max_state_dimension: 100_000,
                max_steps: 1_000_000,
                convergence_tolerance: 1e-9,
                allowed_integrators: vec!["rk4".into(), "simpson".into(), "shooting_bvp".into()],
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

    #[test]
    fn computational_maths_extensions_append_opcodes() {
        // Each new computational-maths extension id compiles to a non-empty, Halt-terminated
        // opcode sequence.
        for id in [
            "q42:UnitsConfiguration",
            "q42:NumberTheoryConfiguration",
            "q42:SpecialFunctionConfiguration",
            "q42:InterpolationConfiguration",
            "q42:IntegralTransformConfiguration",
            "q42:VectorCalculusConfiguration",
            "q42:ExactArithmeticConfiguration",
            "q42:SymbolicCalculusConfiguration",
            "q42:AssumptionConfiguration",
            "q42:NumericalMethodConfiguration",
        ] {
            let mut ops = Vec::new();
            append_extension_opcodes(&mut ops, id);
            assert!(ops.len() >= 2, "{id} produced too few opcodes");
            assert_eq!(
                ops.last(),
                Some(&SlgOpcode::Halt),
                "{id} not Halt-terminated"
            );
        }
    }
}
