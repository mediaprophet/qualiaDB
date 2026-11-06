//! QualiaDB Governed VibeScript Extension Host (`vibe_sentinel`, `VS03-09`, `VS03-10`, `VS03-11`, `VS03-12`, `VS03-13`).
//!
//! Integrates `vibescript-abi` and `vibe::extension_registry` with QualiaDB's
//! zero-heap deontic contract evaluator (`crate::modalities::logic::deontic`),
//! reversible `NQuin` staging, fail-closed receipt persistence, bounded
//! ontology `NQuin` mounting, WGSL Forge / wgpu compute dispatch, and per-token
//! governed inference sessions.

pub mod gpu_inference_host;
pub mod ontology_mount;
pub mod policy_gate;
pub mod receipt_commit;

pub use gpu_inference_host::{
    execute_governed_per_token_inference, execute_wgpu_forge_affine_f32_with_cleanup,
    GovernedInferenceStreamSummary, WgpuForgeExtensionProvider,
};
pub use ontology_mount::{
    BoundedOntologyMount, MountedProviderSpan, PermittedFunctionSummary,
    MAX_MOUNTED_ONTOLOGY_PROVIDERS,
};
pub use policy_gate::{
    make_deontic_norm_quin, norm_property_hash55, nquin_to_quin48, quin48_to_nquin,
    WebizenSentinelGate, MAX_SENTINEL_VERDICTS,
};
pub use receipt_commit::execute_governed_staged_call;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modalities::logic::deontic::{OP_FORBID, OP_PERMIT};
    use crate::NQuin;
    use vibe::extension_registry::{
        prepare_extension_call, CallExecutionContext, DeterministicMechanicsProvider,
        ExtensionRegistry,
    };
    use vibescript_abi::{
        q_hash_str, BorrowedSliceView, CallerOutputSlice, CompletionStatus, EffectSet,
        ExecutionMode, PrimitiveKind, ResourceEnvelope, SemVerReq, TargetProfileMask, VibeTrap,
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
    fn vs03_sentinel_permit_forbid_defeater_and_expiry_precedence() {
        let actor = q_hash_str("did:q42:alice");
        let world = q_hash_str("did:q42:world:lab-1");
        let prov_hash = q_hash_str("https://qualiadb.org/ext/etalon/mechanics");
        let verlet_hash =
            q_hash_str("https://qualiadb.org/ext/etalon/mechanics/kinematics/verlet_step");
        let ke_hash =
            q_hash_str("https://qualiadb.org/ext/etalon/mechanics/kinematics/kinetic_energy");

        // Norm 1: Permit provider-wide in `world` until t = 1000
        let n_permit = make_deontic_norm_quin(
            actor,
            OP_PERMIT,
            false,
            prov_hash,
            prov_hash,
            world,
            1000,
        );
        // Norm 2: Explicitly forbid `kinetic_energy` in `world`
        let n_forbid_ke =
            make_deontic_norm_quin(actor, OP_FORBID, false, ke_hash, prov_hash, world, 1000);

        let norms = [n_permit, n_forbid_ke];
        let mut receipt_sink = [NQuin::default(); 8];
        let gate = WebizenSentinelGate::new(
            &norms,
            &mut receipt_sink,
            1,
            EffectSet::PURE_COMPUTE,
            ResourceEnvelope::hot_zero_heap(100, 128),
        );

        // 1. `verlet_step` is permitted at t = 500
        assert_eq!(
            gate.evaluate_access(actor, verlet_hash, prov_hash, world, 500),
            Ok(())
        );
        // 2. `kinetic_energy` is explicitly denied by `OP_FORBID`
        assert_eq!(
            gate.evaluate_access(actor, ke_hash, prov_hash, world, 500),
            Err(VibeTrap::ExplicitDenyRule)
        );
        // 3. Wrong world context -> DelegationScopeMismatch
        assert_eq!(
            gate.evaluate_access(actor, verlet_hash, prov_hash, world ^ 1, 500),
            Err(VibeTrap::DelegationScopeMismatch)
        );
        // 4. After t = 1000 -> AuthorityExpired
        assert_eq!(
            gate.evaluate_access(actor, verlet_hash, prov_hash, world, 1001),
            Err(VibeTrap::AuthorityExpired)
        );

        // 5. Adding a DEFEATER_BIT (`q42:unless`) exception norm defeats `n_forbid_ke`
        let n_defeat_ke =
            make_deontic_norm_quin(actor, OP_PERMIT, true, ke_hash, prov_hash, world, 1000);
        let norms_with_defeater = [n_permit, n_forbid_ke, n_defeat_ke];
        let gate2 = WebizenSentinelGate::new(
            &norms_with_defeater,
            &mut receipt_sink,
            2,
            EffectSet::PURE_COMPUTE,
            ResourceEnvelope::hot_zero_heap(100, 128),
        );
        assert_eq!(
            gate2.evaluate_access(actor, ke_hash, prov_hash, world, 500),
            Ok(())
        );
    }

    #[test]
    fn vs03_staged_commit_and_fail_closed_receipt_rollback() {
        static PROVIDER: DeterministicMechanicsProvider = DeterministicMechanicsProvider;
        let mut reg = ExtensionRegistry::new();
        reg.register(&PROVIDER).unwrap();

        let handle = prepare_extension_call(
            &reg,
            "mech",
            "kinematics",
            "kinetic_energy",
            SemVerReq::AnyStable,
            TargetProfileMask::NATIVE_DESKTOP,
        )
        .unwrap();

        let actor = q_hash_str("did:q42:alice");
        let world = q_hash_str("did:q42:world:lab-1");
        let prov_hash = handle.provider_iri_hash;
        let norms = [make_deontic_norm_quin(
            actor,
            OP_PERMIT,
            false,
            prov_hash,
            prov_hash,
            world,
            2000,
        )];

        // Receipt sink has space for exactly 1 call (2 NQuins)
        let mut receipt_sink = [NQuin::default(); 2];
        let mut gate = WebizenSentinelGate::new(
            &norms,
            &mut receipt_sink,
            7,
            EffectSet::PURE_COMPUTE,
            ResourceEnvelope::hot_zero_heap(100, 128),
        );

        let mass_b = f64_1_to_bytes(4.0);
        let vel_b = f64_3_to_bytes([3.0, 0.0, 4.0]); // |v|^2 = 25 -> KE = 0.5 * 4 * 25 = 50
        let inputs = [
            BorrowedSliceView::from_raw_bytes(&mass_b, PrimitiveKind::F64, 1).unwrap(),
            BorrowedSliceView::from_raw_bytes(&vel_b, PrimitiveKind::F64, 3).unwrap(),
        ];

        let staged_mutation = NQuin {
            subject: actor,
            predicate: q_hash_str("https://qualiadb.org/schema/vibe#recordedEnergy"),
            object: 50,
            context: world,
            metadata: 100,
            parity: actor
                ^ q_hash_str("https://qualiadb.org/schema/vibe#recordedEnergy")
                ^ 50
                ^ world,
        };
        let mut published_graph = [NQuin::default(); 4];
        let mut published_len = 0usize;

        let ctx = CallExecutionContext {
            actor_did_hash: actor,
            context_world_hash: world,
            lamport_timestamp: 100,
            mode: ExecutionMode::IntegratedGovernanceTest,
        };

        // Call 1 succeeds: receipt persisted (2 NQuins) and staged mutation published
        let mut out_raw1 = [0u8; 8];
        let mut out1 = CallerOutputSlice::new(&mut out_raw1, PrimitiveKind::F64).unwrap();
        let outcome = execute_governed_staged_call(
            &reg,
            &handle,
            &ctx,
            &mut gate,
            &inputs,
            &mut out1,
            &[staged_mutation],
            &mut published_graph,
            &mut published_len,
        )
        .expect("first governed call commits");
        assert_eq!(outcome.receipt.status, CompletionStatus::Committed);
        assert_eq!(gate.persisted_receipt_quin_count(), 2);
        assert_eq!(published_len, 1);

        // Call 2: receipt_sink is now full (2/2 used) -> commit_receipt fails with ReceiptPersistenceFailed,
        // and `published_len` MUST remain 1 (staged mutation rolled back / withheld!).
        let mut out_raw2 = [0u8; 8];
        let mut out2 = CallerOutputSlice::new(&mut out_raw2, PrimitiveKind::F64).unwrap();
        let err = execute_governed_staged_call(
            &reg,
            &handle,
            &ctx,
            &mut gate,
            &inputs,
            &mut out2,
            &[staged_mutation],
            &mut published_graph,
            &mut published_len,
        )
        .unwrap_err();
        assert_eq!(err, VibeTrap::ReceiptPersistenceFailed);
        assert_eq!(
            published_len, 1,
            "staged state must not be published when receipt persistence fails"
        );
    }

    #[test]
    fn vs03_bounded_ontology_mount_discovery_and_unmount() {
        static PROVIDER: DeterministicMechanicsProvider = DeterministicMechanicsProvider;
        let mut store_buf = [NQuin::default(); 8];
        let mut mount = BoundedOntologyMount::new(&mut store_buf, 8);

        let span = mount.mount_provider(&PROVIDER).expect("mounts cleanly");
        assert_eq!(span.len, 3);
        assert_eq!(mount.mounted_quin_count(), 3);

        // Mounting only permits `verlet_step` for `actor`
        let actor = q_hash_str("did:q42:bob");
        let world = q_hash_str("did:q42:world:2");
        let prov_hash = q_hash_str("https://qualiadb.org/ext/etalon/mechanics");
        let verlet_hash =
            q_hash_str("https://qualiadb.org/ext/etalon/mechanics/kinematics/verlet_step");
        let norms = [make_deontic_norm_quin(
            actor,
            OP_PERMIT,
            false,
            verlet_hash,
            prov_hash,
            world,
            900,
        )];
        let mut receipt_sink = [NQuin::default(); 4];
        let gate = WebizenSentinelGate::new(
            &norms,
            &mut receipt_sink,
            1,
            EffectSet::PURE_COMPUTE,
            ResourceEnvelope::hot_zero_heap(100, 128),
        );

        let mut discovered = [PermittedFunctionSummary {
            provider_iri_hash: 0,
            func_iri_hash: 0,
            stable_id: vibescript_abi::StableFuncId::new(0, 0),
        }; 4];
        let count = mount
            .discover_permitted_functions(&PROVIDER, &gate, actor, world, 100, &mut discovered)
            .unwrap();
        // Only `verlet_step` (1 of the 3 functions) is permitted for Bob!
        assert_eq!(count, 1);
        assert_eq!(discovered[0].func_iri_hash, verlet_hash);

        // Unmount provider cleans up mounted ontology records
        mount.unmount_provider(prov_hash).unwrap();
        assert_eq!(mount.mounted_quin_count(), 0);
        assert_eq!(
            mount.discover_permitted_functions(
                &PROVIDER,
                &gate,
                actor,
                world,
                100,
                &mut discovered
            ),
            Err(VibeTrap::NamespaceNotFound)
        );
    }

    #[test]
    fn vs03_12_wgpu_forge_provider_dispatch_and_transient_cleanup() {
        static WGPU_PROV: WgpuForgeExtensionProvider = WgpuForgeExtensionProvider;
        let mut reg = ExtensionRegistry::new();
        reg.register(&WGPU_PROV).unwrap();

        let handle = prepare_extension_call(
            &reg,
            "wgpu_forge",
            "compute",
            "affine_f32",
            SemVerReq::AnyStable,
            TargetProfileMask::NATIVE_DESKTOP,
        )
        .unwrap();

        let actor = q_hash_str("did:q42:alice");
        let world = q_hash_str("did:q42:world:gpu-lab");
        let prov_hash = handle.provider_iri_hash;
        let norms = [make_deontic_norm_quin(
            actor,
            OP_PERMIT,
            false,
            prov_hash,
            prov_hash,
            world,
            5000,
        )];
        let mut receipt_sink = [NQuin::default(); 4];
        let mut gate = WebizenSentinelGate::new(
            &norms,
            &mut receipt_sink,
            1,
            EffectSet::PURE_COMPUTE.union(EffectSet::HARDWARE_IO),
            ResourceEnvelope {
                max_ticks: 128,
                max_scratch_bytes: 0,
                max_vram_bytes: 1024,
                max_output_bytes: 64,
                max_spawned_tasks: 0,
                flags: 0,
            },
        );

        let x = [1.0f32, 2.0, -3.0, 4.5];
        let sb = [2.0f32, 0.5];
        let mut x_bytes = [0u8; 16];
        for (i, v) in x.iter().enumerate() {
            x_bytes[i * 4..(i + 1) * 4].copy_from_slice(&v.to_le_bytes());
        }
        let mut sb_bytes = [0u8; 8];
        sb_bytes[0..4].copy_from_slice(&sb[0].to_le_bytes());
        sb_bytes[4..8].copy_from_slice(&sb[1].to_le_bytes());

        let inputs = [
            BorrowedSliceView::from_raw_bytes(&x_bytes, PrimitiveKind::F32, 4).unwrap(),
            BorrowedSliceView::from_raw_bytes(&sb_bytes, PrimitiveKind::F32, 2).unwrap(),
        ];
        let mut out_raw = [0u8; 16];
        let mut out = CallerOutputSlice::new(&mut out_raw, PrimitiveKind::F32).unwrap();

        let ctx = CallExecutionContext {
            actor_did_hash: actor,
            context_world_hash: world,
            lamport_timestamp: 200,
            mode: ExecutionMode::IntegratedGovernanceTest,
        };
        let mut published_graph = [NQuin::default(); 2];
        let mut published_len = 0usize;

        let outcome = execute_governed_staged_call(
            &reg,
            &handle,
            &ctx,
            &mut gate,
            &inputs,
            &mut out,
            &[],
            &mut published_graph,
            &mut published_len,
        )
        .expect("governed wgpu_forge affine_f32 succeeds");
        assert_eq!(outcome.receipt.status, CompletionStatus::Committed);

        let view = BorrowedSliceView::from_raw_bytes(out.written_byte_slice(), PrimitiveKind::F32, 4)
            .unwrap();
        let expected = vibe::extension_registry::oracle_gpu_affine_f32(x, sb[0], sb[1]);
        for (i, &exp) in expected.iter().enumerate() {
            let got = view.read_f32_le(i).unwrap();
            assert!(
                (got - exp).abs() <= 1e-5,
                "element {i}: got {got}, expected {exp}"
            );
        }

        // Verify direct cleanup invariant (`post_cleanup_bytes == 0`)
        let (direct_out, _used_hw, post_bytes) =
            execute_wgpu_forge_affine_f32_with_cleanup(x, sb[0], sb[1]).unwrap();
        assert_eq!(post_bytes, 0, "transient GPU allocations must be reclaimed");
        for (i, &exp) in expected.iter().enumerate() {
            assert!((direct_out[i] - exp).abs() <= 1e-5);
        }
    }

    #[test]
    fn vs03_12_governed_per_token_inference_commit_and_midstream_rollback() {
        let reg = ExtensionRegistry::with_builtin_providers();
        let actor = q_hash_str("did:q42:alice");
        let world = q_hash_str("did:q42:world:infer-1");
        let infer_prov_hash = q_hash_str("https://qualiadb.org/ext/qualia/inference");

        let norms = [make_deontic_norm_quin(
            actor,
            OP_PERMIT,
            false,
            infer_prov_hash,
            infer_prov_hash,
            world,
            5000,
        )];

        let steps = [
            [0.1, 2.4, 0.5, -1.0], // token 1
            [3.2, 0.1, -0.5, 0.0], // token 0
            [-0.2, 0.4, 1.1, 4.8], // token 3
        ];

        // 1. Full 3-step stream succeeds when policy remains valid and receipt sink has capacity (6 NQuins)
        let mut receipt_sink = [NQuin::default(); 8];
        let mut gate = WebizenSentinelGate::new(
            &norms,
            &mut receipt_sink,
            10,
            EffectSet::PURE_COMPUTE
                .union(EffectSet::HARDWARE_IO)
                .union(EffectSet::GRAPH_MUTATE),
            ResourceEnvelope {
                max_ticks: 256,
                max_scratch_bytes: 256,
                max_vram_bytes: 256,
                max_output_bytes: 64,
                max_spawned_tasks: 0,
                flags: 0,
            },
        );
        let ctx = CallExecutionContext {
            actor_did_hash: actor,
            context_world_hash: world,
            lamport_timestamp: 300,
            mode: ExecutionMode::IntegratedGovernanceTest,
        };
        let mut kv_hash = 0xCAFE_BABE_0000_0001u64;
        let mut published_graph = [NQuin::default(); 8];
        let mut published_len = 0usize;
        let mut decoded = [(0u64, 0u64); 3];

        let summary = execute_governed_per_token_inference(
            &reg,
            &ctx,
            &mut gate,
            &steps,
            &mut kv_hash,
            &mut published_graph,
            &mut published_len,
            &mut decoded,
            |_step, _gate| {},
        )
        .expect("3-step governed token stream commits");
        assert_eq!(summary.committed_steps, 3);
        assert_eq!(published_len, 3);
        assert_eq!(decoded[0].0, 1);
        assert_eq!(decoded[1].0, 0);
        assert_eq!(decoded[2].0, 3);
        assert_eq!(gate.persisted_receipt_quin_count(), 6);

        // 2. Mid-stream revocation (receipt sink exhausted on step 2 -> ReceiptPersistenceFailed)
        // must roll back ALL staged token NQuins and restore initial `kv_hash`!
        let mut short_receipt_sink = [NQuin::default(); 4]; // only enough for 2 steps, fails on 3rd!
        let mut gate_revoke = WebizenSentinelGate::new(
            &norms,
            &mut short_receipt_sink,
            11,
            EffectSet::PURE_COMPUTE
                .union(EffectSet::HARDWARE_IO)
                .union(EffectSet::GRAPH_MUTATE),
            ResourceEnvelope {
                max_ticks: 256,
                max_scratch_bytes: 256,
                max_vram_bytes: 256,
                max_output_bytes: 64,
                max_spawned_tasks: 0,
                flags: 0,
            },
        );
        let initial_kv = 0xDEAD_BEEF_1234_5678u64;
        let mut kv_hash_rollback = initial_kv;
        let mut published_rollback = [NQuin::default(); 8];
        let mut published_rollback_len = 1usize; // 1 pre-existing committed record
        let mut decoded_rollback = [(0u64, 0u64); 3];

        let err = execute_governed_per_token_inference(
            &reg,
            &ctx,
            &mut gate_revoke,
            &steps,
            &mut kv_hash_rollback,
            &mut published_rollback,
            &mut published_rollback_len,
            &mut decoded_rollback,
            |_step, _gate| {},
        )
        .unwrap_err();
        assert_eq!(err, VibeTrap::ReceiptPersistenceFailed);
        assert_eq!(
            published_rollback_len, 1,
            "all staged token NQuins from aborted stream must be rolled back"
        );
        assert_eq!(
            kv_hash_rollback, initial_kv,
            "KV state hash must be restored on mid-stream abort"
        );
        assert_eq!(decoded_rollback, [(0, 0); 3]);
    }
}
