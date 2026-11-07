//! VibeScript 0.03 Extension Registry, Cold Preparation, Zero-Heap Hot Dispatch,
//! and Deterministic Domain Provider Adapters (`VS03-06`, `VS03-07`, `VS03-08`).

pub mod dispatch_adapter;
pub mod domain_providers;
pub mod eval_router;
pub mod gpu_inference_providers;
pub mod graph_logic_providers;
pub mod prepare;
pub mod registry;

pub use dispatch_adapter::{
    dispatch_prepared_call, CallExecutionContext, DispatchOutcome, SimulatedPolicyGate,
};
pub use domain_providers::{
    oracle_aabb_volume, oracle_excavation_prism_volume, oracle_kinetic_energy,
    oracle_spring_damper_force, oracle_verlet_step, DeterministicMechanicsProvider,
};
pub use etalon_vibe::{
    EtalonForgeProvider, EtalonModelProvider, EtalonNQuinProvider, EtalonRenderProvider,
    EtalonSenseProvider, EtalonSignalProvider, ETALON_VIBE_PROVIDERS,
};
pub use eval_router::{
    execute_prepared_value_call, is_extension_call_path, is_extension_namespace, route_call,
    trap_to_diagnostic, try_prepare_path, PreparedCallCache, BUILTIN_EXTENSION_ALIASES,
    BUILTIN_EXTENSION_IRIS,
};
pub use gpu_inference_providers::{
    oracle_governed_token_step, oracle_gpu_affine_f32, oracle_gpu_gemv4_f32,
    oracle_gpu_rmsnorm_f32, oracle_gpu_rope_f32, oracle_gpu_topk2_f32, oracle_rms_norm_f64,
    oracle_softmax_f64, EtalonGpuComputeProvider, GovernedInferenceProvider,
};
pub use graph_logic_providers::{
    oracle_allen_interval_relation, oracle_pack_deontic_predicate, oracle_seal_quin,
    oracle_verify_quin_parity, DeonticTemporalProvider, StagedGraphMutationProvider,
};
pub use prepare::prepare_extension_call;
pub use registry::{ExtensionRegistry, MAX_EXTENSION_PROVIDERS};

#[cfg(test)]
mod tests {
    use super::*;
    use vibescript_abi::{
        q_hash_str, BorrowedSliceView, CallerOutputSlice, CompletionStatus, ExecutionMode,
        PrimitiveKind, SemVer64, SemVerReq, TargetProfileMask, VibeTrap,
    };

    fn f64_1_to_bytes(v: f64) -> [u8; 8] {
        v.to_le_bytes()
    }

    fn f64_3_to_bytes(vals: [f64; 3]) -> [u8; 24] {
        let mut out = [0u8; 24];
        for (i, v) in vals.iter().enumerate() {
            out[i * 8..(i + 1) * 8].copy_from_slice(&v.to_le_bytes());
        }
        out
    }

    #[test]
    fn vs03_registration_atomicity_and_duplicate_rejection() {
        static PROVIDER: DeterministicMechanicsProvider = DeterministicMechanicsProvider;
        let mut reg = ExtensionRegistry::new();
        let gen0 = reg.generation();
        let slot = reg.register(&PROVIDER).expect("first registration succeeds");
        assert_eq!(slot, 0);
        assert!(reg.generation() > gen0);

        let gen1 = reg.generation();
        // Registering the same provider again must fail atomically without changing generation
        let dup_err = reg.register(&PROVIDER).unwrap_err();
        assert_eq!(dup_err, VibeTrap::DuplicateNamespaceIri);
        assert_eq!(reg.generation(), gen1);
        assert_eq!(reg.active_count(), 1);
    }

    #[test]
    fn vs03_prepare_checks_zero_major_semver_and_target_profile() {
        static PROVIDER: DeterministicMechanicsProvider = DeterministicMechanicsProvider;
        let mut reg = ExtensionRegistry::new();
        reg.register(&PROVIDER).unwrap();

        // Provider version is 0.4.2 -> matches ^0.4.0, fails ^0.5.0 and ^0.3.0
        let ok_handle = prepare_extension_call(
            &reg, "mech", "kinematics", "verlet_step",
            SemVerReq::Caret(SemVer64::release(0, 4, 0)), TargetProfileMask::NATIVE_DESKTOP,
        ).expect("^0.4.0 matches 0.4.2");
        assert_eq!(ok_handle.opcode.ns_slot(), 0);

        let bad_ver = prepare_extension_call(
            &reg, "mech", "kinematics", "verlet_step",
            SemVerReq::Caret(SemVer64::release(0, 5, 0)), TargetProfileMask::NATIVE_DESKTOP,
        ).unwrap_err();
        assert_eq!(bad_ver, VibeTrap::VersionRequirementUnsatisfied);

        // GPU_COMPUTE is not in ALL_CPU
        let bad_target = prepare_extension_call(
            &reg, "mech", "kinematics", "verlet_step",
            SemVerReq::AnyStable, TargetProfileMask::GPU_COMPUTE,
        ).unwrap_err();
        assert_eq!(bad_target, VibeTrap::TargetProfileUnsupported);
    }

    #[test]
    fn vs03_stale_registry_generation_fails_closed_after_unregister() {
        static PROVIDER: DeterministicMechanicsProvider = DeterministicMechanicsProvider;
        let mut reg = ExtensionRegistry::new();
        let slot = reg.register(&PROVIDER).unwrap();
        let handle = prepare_extension_call(
            &reg, "mech", "kinematics", "kinetic_energy",
            SemVerReq::AnyStable, TargetProfileMask::NATIVE_DESKTOP,
        ).unwrap();

        // Unregister advances generation
        reg.unregister(slot).unwrap();

        let mass_b = f64_1_to_bytes(2.0);
        let vel_b = f64_3_to_bytes([3.0, 4.0, 0.0]);
        let inputs = [
            BorrowedSliceView::from_raw_bytes(&mass_b, PrimitiveKind::F64, 1).unwrap(),
            BorrowedSliceView::from_raw_bytes(&vel_b, PrimitiveKind::F64, 3).unwrap(),
        ];
        let mut out_raw = [0u8; 8];
        let mut out = CallerOutputSlice::new(&mut out_raw, PrimitiveKind::F64).unwrap();
        let ctx = CallExecutionContext {
            actor_did_hash: q_hash_str("did:q42:dev"),
            context_world_hash: 1,
            lamport_timestamp: 10,
            mode: ExecutionMode::StandaloneDeveloper,
        };

        let res = dispatch_prepared_call(&reg, &handle, &ctx, None, &inputs, &mut out);
        assert_eq!(res, Err(VibeTrap::StaleRegistryGeneration));
    }

    #[test]
    fn vs03_standalone_and_simulated_modes_match_direct_oracle_and_check_output_capacity() {
        static PROVIDER: DeterministicMechanicsProvider = DeterministicMechanicsProvider;
        let mut reg = ExtensionRegistry::new();
        reg.register(&PROVIDER).unwrap();

        let handle = prepare_extension_call(
            &reg, "https://qualiadb.org/ext/etalon/mechanics", "kinematics", "verlet_step",
            SemVerReq::Caret(SemVer64::release(0, 4, 0)), TargetProfileMask::WASM_SANDBOX,
        ).unwrap();

        let (pos, vel, acc, dt) = ([1.0, 2.0, 3.0], [0.5, -1.0, 2.0], [0.0, -9.81, 0.0], 0.1);
        let (pos_b, vel_b, acc_b, dt_b) = (
            f64_3_to_bytes(pos), f64_3_to_bytes(vel), f64_3_to_bytes(acc), f64_1_to_bytes(dt),
        );
        let inputs = [
            BorrowedSliceView::from_raw_bytes(&pos_b, PrimitiveKind::F64, 3).unwrap(),
            BorrowedSliceView::from_raw_bytes(&vel_b, PrimitiveKind::F64, 3).unwrap(),
            BorrowedSliceView::from_raw_bytes(&acc_b, PrimitiveKind::F64, 3).unwrap(),
            BorrowedSliceView::from_raw_bytes(&dt_b, PrimitiveKind::F64, 1).unwrap(),
        ];

        // 1. Pre-dispatch capacity check: 5 elements (< required 6) fails closed before execution
        let mut small_raw = [0u8; 5 * 8];
        let mut small_out = CallerOutputSlice::new(&mut small_raw, PrimitiveKind::F64).unwrap();
        let standalone_ctx = CallExecutionContext {
            actor_did_hash: q_hash_str("did:q42:alice"),
            context_world_hash: 1,
            lamport_timestamp: 42,
            mode: ExecutionMode::StandaloneDeveloper,
        };
        assert_eq!(
            dispatch_prepared_call(&reg, &handle, &standalone_ctx, None, &inputs, &mut small_out),
            Err(VibeTrap::OutputBufferTooSmall)
        );

        // 2. StandaloneDeveloper mode with 6-element output buffer matches direct-call oracle
        let mut out_raw = [0u8; 6 * 8];
        let mut out = CallerOutputSlice::new(&mut out_raw, PrimitiveKind::F64).unwrap();
        let outcome = dispatch_prepared_call(&reg, &handle, &standalone_ctx, None, &inputs, &mut out)
            .expect("standalone call succeeds");
        assert_eq!(outcome.receipt.status, CompletionStatus::StandaloneCompleted);
        assert!(!outcome.receipt.simulated);

        let out_view =
            BorrowedSliceView::from_raw_bytes(out.written_byte_slice(), PrimitiveKind::F64, 6)
                .unwrap();
        let exp = oracle_verlet_step(pos, vel, acc, dt).unwrap();
        for i in 0..3 {
            assert!((out_view.read_f64_le(i).unwrap() - exp[i]).abs() < 1e-12);
            assert!((out_view.read_f64_le(i + 3).unwrap() - exp[i + 3]).abs() < 1e-12);
        }

        // 3. GovernedApplication mode without a Sentinel gate fails closed
        let governed_ctx = CallExecutionContext {
            mode: ExecutionMode::GovernedApplication,
            ..standalone_ctx
        };
        let mut out_raw2 = [0u8; 6 * 8];
        let mut out2 = CallerOutputSlice::new(&mut out_raw2, PrimitiveKind::F64).unwrap();
        assert_eq!(
            dispatch_prepared_call(&reg, &handle, &governed_ctx, None, &inputs, &mut out2),
            Err(VibeTrap::SentinelUnavailable)
        );

        // 4. SimulatedGovernanceTest mode marks receipt `simulated = true` and enforces receipt failure
        let sim_ctx = CallExecutionContext {
            mode: ExecutionMode::SimulatedGovernanceTest,
            ..standalone_ctx
        };
        let mut sim_gate = SimulatedPolicyGate::new_for_actor(standalone_ctx.actor_did_hash);
        let mut out_raw3 = [0u8; 6 * 8];
        let mut out3 = CallerOutputSlice::new(&mut out_raw3, PrimitiveKind::F64).unwrap();
        let sim_outcome =
            dispatch_prepared_call(&reg, &handle, &sim_ctx, Some(&mut sim_gate), &inputs, &mut out3)
                .expect("simulated governance succeeds");
        assert!(sim_outcome.receipt.simulated);
        assert_eq!(sim_outcome.receipt.status, CompletionStatus::SimulatedCommitted);

        // Simulated receipt persistence failure must fail closed
        sim_gate.fail_receipt_commit = true;
        let mut out_raw4 = [0u8; 6 * 8];
        let mut out4 = CallerOutputSlice::new(&mut out_raw4, PrimitiveKind::F64).unwrap();
        assert_eq!(
            dispatch_prepared_call(&reg, &handle, &sim_ctx, Some(&mut sim_gate), &inputs, &mut out4),
            Err(VibeTrap::ReceiptPersistenceFailed)
        );
    }

    #[test]
    fn vs03_intra_provider_duplicate_stable_id_with_distinct_names_rejected() {
        use vibescript_abi::{
            CanonicalIri, ContractVersionAxes, DeterminismGuarantee, EffectSet, FuncDescriptor,
            NQuin48, OntologicalContract, ParamSpec, ProviderDescriptor, ResourceEnvelope,
            ReversibilityKind, StableFuncId, SubNamespaceDescriptor, VibeNamespaceProvider,
        };

        const PARAMS: &[ParamSpec] = &[ParamSpec::scalar("x", PrimitiveKind::F64)];
        const ENV: ResourceEnvelope = ResourceEnvelope::hot_zero_heap(8, 8);
        const CONTRACT: OntologicalContract = OntologicalContract {
            contract_digest: [0x44; 32],
            schema_hash: 1,
            precondition_quin_span: (0, 0),
            postcondition_quin_span: (0, 0),
            invariant_quin_span: (0, 0),
        };
        const FUNCS: &[FuncDescriptor] = &[
            FuncDescriptor {
                name: "alpha",
                canonical_iri: CanonicalIri::new("https://qualiadb.org/ext/test/dup/sub/alpha"),
                stable_id: StableFuncId::new(0, 7),
                params: PARAMS,
                output_kind: PrimitiveKind::F64,
                required_output_elems: 1,
                effects: EffectSet::PURE_COMPUTE,
                determinism: DeterminismGuarantee::DeterministicExact,
                reversibility: ReversibilityKind::PureNoState,
                envelope: ENV,
                contract: CONTRACT,
            },
            FuncDescriptor {
                name: "beta",
                canonical_iri: CanonicalIri::new("https://qualiadb.org/ext/test/dup/sub/beta"),
                stable_id: StableFuncId::new(0, 7), // duplicate StableFuncId!
                params: PARAMS,
                output_kind: PrimitiveKind::F64,
                required_output_elems: 1,
                effects: EffectSet::PURE_COMPUTE,
                determinism: DeterminismGuarantee::DeterministicExact,
                reversibility: ReversibilityKind::PureNoState,
                envelope: ENV,
                contract: CONTRACT,
            },
        ];
        const SUBS: &[SubNamespaceDescriptor] = &[SubNamespaceDescriptor {
            name: "sub",
            sub_id: 0,
            functions: FUNCS,
        }];
        const DESC: ProviderDescriptor = ProviderDescriptor {
            canonical_iri: CanonicalIri::new("https://qualiadb.org/ext/test/dup"),
            short_alias: "dup",
            versions: ContractVersionAxes::v0_3_default(
                SemVer64::release(0, 3, 0),
                SemVer64::release(0, 1, 0),
            ),
            supported_targets: TargetProfileMask::ALL_CPU,
            sub_namespaces: SUBS,
            max_ontology_quins: 4,
        };
        struct BadDupProvider;
        impl VibeNamespaceProvider for BadDupProvider {
            fn descriptor(&self) -> &'static ProviderDescriptor {
                &DESC
            }
            fn static_ontology_nquins(&self) -> &'static [NQuin48] {
                &[]
            }
            fn call(
                &self,
                _stable_id: StableFuncId,
                _inputs: &[BorrowedSliceView<'_>],
                _output: &mut CallerOutputSlice<'_>,
            ) -> Result<u64, VibeTrap> {
                Ok(1)
            }
        }
        static BAD: BadDupProvider = BadDupProvider;
        let mut reg = ExtensionRegistry::new();
        assert_eq!(reg.register(&BAD), Err(VibeTrap::DuplicateStableFuncId));
        assert_eq!(reg.active_count(), 0);
    }

    #[test]
    fn vs03_stage_s6_ast_bytecode_and_task_scheduler_cutover_parity() {
        use crate::bind::LocalHost;
        use crate::budget::Budget;
        use crate::bytecode::{compile, Vm};
        use crate::eval::{Engine, Env};
        use crate::value::Value;
        use vibescript_abi::ResourceEnvelope;

        // 1. AST Evaluator 3-level & 2-level extension calls across mech, gpu, infer, qgraph, modality
        let src = r#"
            import "vibe:0.3/mech" as mech;
            import "vibe:0.3/gpu" as gpu;
            import "vibe:0.3/infer" as infer;
            import "vibe:0.3/qgraph" as qgraph;
            import "vibe:0.3/modality" as modality;

            fn run_suite() {
                let ke = mech.kinematics.kinetic_energy(4.0, [3.0, 0.0, 4.0]);
                let aff = gpu.compute.affine_f32([1.0, 2.0, -1.0, 0.5], [2.0, 1.0]);
                let probs = infer.activations.softmax_f64([0.0, 0.0, 0.0, 0.0]);
                let sealed = qgraph.staging.seal_quin([10, 20, 30, 40, 50]);
                let ok_parity = qgraph.staging.verify_quin_parity(sealed);
                let allen_rel = modality.allen.interval_relation([0, 10], [10, 20]);
                return [ke, aff[0], probs[0], ok_parity, allen_rel];
            }
        "#;
        let prog = crate::parse_program(src).expect("parses");
        crate::check_program(&prog).expect("checks");

        let actor = q_hash_str("did:q42:local_engine");
        let mut sim_gate = SimulatedPolicyGate::new_for_actor(actor);
        sim_gate.budget = ResourceEnvelope {
            max_ticks: 1024,
            max_scratch_bytes: 1024,
            max_vram_bytes: 2048,
            max_output_bytes: 256,
            max_spawned_tasks: 0,
            flags: 0,
        };
        let sim_ctx = CallExecutionContext {
            actor_did_hash: actor,
            context_world_hash: 1,
            lamport_timestamp: 10,
            mode: ExecutionMode::SimulatedGovernanceTest,
        };

        let mut host = LocalHost::default();
        let mut env = Env::default();
        crate::eval::populate_import_aliases(&mut env, &prog.imports).unwrap();
        let mut engine = Engine::with_program(&mut host, Budget::default(), &prog);
        engine.set_extension_governance(sim_ctx, Some(&mut sim_gate));
        let res = engine
            .call_function(&prog, "run_suite", vec![], &mut env)
            .expect("AST evaluates extension calls");

        let exp_ke = oracle_kinetic_energy(4.0, [3.0, 0.0, 4.0]).unwrap();
        let exp_aff = oracle_gpu_affine_f32([1.0, 2.0, -1.0, 0.5], 2.0, 1.0);
        let exp_probs = oracle_softmax_f64([0.0, 0.0, 0.0, 0.0]);
        let exp_allen = oracle_allen_interval_relation([0, 10], [10, 20]).unwrap();
        assert_eq!(
            res,
            Value::List(vec![
                Value::F64(exp_ke),
                Value::F64(exp_aff[0] as f64),
                Value::F64(exp_probs[0]),
                Value::U64(1),
                Value::U64(exp_allen),
            ])
        );
        assert!(engine.last_extension_outcome().is_some());

        // 2. Bytecode compiler + VM executes extension calls through the same unified router
        let bc_src = r#"
            fn compute_ke() -> f64 {
                return mech.kinematics.kinetic_energy(4.0, [3.0, 0.0, 4.0]);
            }
        "#;
        let bc_prog = crate::parse_program(bc_src).expect("parses bc_prog");
        let chunk = compile(&bc_prog).expect("compiles to bytecode");
        let mut bc_host = LocalHost::default();
        let mut vm = Vm::new(&chunk, &mut bc_host, Budget::default());
        let bc_val = vm.call_function(0, &[]).expect("VM executes extension call");
        assert_eq!(bc_val, Value::F64(exp_ke));
        assert!(vm.last_extension_outcome().is_some());

        // 3. Task scheduler (`task.spawn` + `task.await`) executes extension calls and enforces
        // policy generation freshness
        let task_src = r#"
            fn run_task() {
                let h = task.spawn("= mech.kinematics.kinetic_energy(2.0, [3.0, 4.0, 0.0])");
                return task.await(h);
            }
        "#;
        let task_prog = crate::parse_program(task_src).expect("parses task_prog");
        let mut task_host = LocalHost::default();
        let mut task_env = Env::default();
        let mut task_engine = Engine::with_program(&mut task_host, Budget::default(), &task_prog);
        let task_res = task_engine
            .call_function(&task_prog, "run_task", vec![], &mut task_env)
            .expect("task.await evaluates extension call");
        if let Value::Record(rec) = task_res {
            assert_eq!(rec.get("status"), Some(&Value::String("completed".into())));
            assert_eq!(
                rec.get("result"),
                Some(&Value::F64(
                    oracle_kinetic_energy(2.0, [3.0, 4.0, 0.0]).unwrap()
                ))
            );
        } else {
            panic!("expected task record, got {task_res:?}");
        }

        // 4. All 11 built-in providers (8 Etalon + 3 Qualia) register and execute across VibeScript
        let builtin = ExtensionRegistry::with_builtin_providers();
        assert_eq!(builtin.active_count(), 11);
        assert_eq!(builtin.iter_providers().count(), 11);
        let etalon_src = r#"
            import "vibe:0.3/forge" as forge;
            import "vibe:0.3/render" as render;
            import "vibe:0.3/signal" as signal;
            import "vibe:0.3/sense" as sense;
            import "vibe:0.3/p64" as p64;
            import "vibe:0.3/nquin" as nquin;
            import "vibe:0.3/mech" as mech;
            import "vibe:0.3/gpu" as gpu;
            fn run_etalon() {
                let tile = forge.shader.tile_schedule_gemm([64, 64, 64], [16, 16, 16]);
                let sdf = render.spatial.sdf_torus_eval([3.0, 0.0, 0.0], [2.0, 0.5]);
                let rms = signal.dsp.signal_rms_peak([1.0, -1.0, 1.0, -1.0]);
                let drift = sense.telemetry.clock_stamp_drift_ns([1050, 1000], 100);
                let align = p64.runtime.p64_align_offset([100, 64]);
                let par = nquin.wire.compute_wire_parity([1, 2, 4, 8, 16]);
                let f = mech.kinematics.spring_damper_force([1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [10.0, 1.0]);
                let rn = gpu.compute.rmsnorm_f32([0.0, 0.0, 0.0, 0.0], [1.0, 1.0, 1.0, 1.0], 1.0);
                return [tile[0], sdf, rms[0], drift[1], align[0], par, f[0], rn[0]];
            }
        "#;
        let etalon_prog = crate::parse_program(etalon_src).expect("parses etalon_src");
        crate::check_program(&etalon_prog).expect("checks etalon_src");
        let mut etalon_env = Env::default();
        crate::eval::populate_import_aliases(&mut etalon_env, &etalon_prog.imports).unwrap();
        let mut etalon_engine = Engine::with_program(&mut host, Budget::default(), &etalon_prog);
        let etalon_res = etalon_engine
            .call_function(&etalon_prog, "run_etalon", vec![], &mut etalon_env)
            .expect("evaluates all 8 Etalon providers");
        assert_eq!(
            etalon_res,
            Value::List(vec![
                Value::U64(4),
                Value::F64(0.5),
                Value::F64(1.0),
                Value::U64(1),
                Value::U64(128),
                Value::U64(31),
                Value::F64(-10.0),
                Value::F64(0.0),
            ])
        );
    }
}
