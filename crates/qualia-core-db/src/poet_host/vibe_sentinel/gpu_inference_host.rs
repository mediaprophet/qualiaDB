//! Real WGSL Forge / wgpu Compute Provider Adapter and Governed Per-Token Inference Session (`VS03-12`).
//!
//! Connects VibeScript 0.03 extension dispatch to:
//! 1. `crate::wgsl_forge::execute::WgpuComputeContext` + `BuiltinKernel::AffineF32` with mandatory
//!    transient GPU buffer reclamation (`clear_transient_allocations()`), and
//! 2. Per-token governed inference (`GovernedInferenceProvider` / `decode::governed_token_step`)
//!    with full staged `NQuin` and KV-state rollback if any token step is denied, revoked mid-stream,
//!    or fails receipt persistence.

use super::policy_gate::WebizenSentinelGate;
use super::receipt_commit::execute_governed_staged_call;
use crate::NQuin;
use vibe::extension_registry::{
    oracle_gpu_affine_f32, prepare_extension_call, CallExecutionContext, ExtensionRegistry,
};
use vibescript_abi::{
    q_hash_str, BorrowedSliceView, CallerOutputSlice, CanonicalIri, ContractVersionAxes,
    DeterminismGuarantee, EffectSet, FuncDescriptor, NQuin48, OntologicalContract, ParamSpec,
    PrimitiveKind, ProviderDescriptor, ResourceEnvelope, ReversibilityKind, SemVer64, SemVerReq,
    StableFuncId, SubNamespaceDescriptor, TargetProfileMask, VibeNamespaceProvider, VibeTrap,
};

const WGPU_FORGE_TARGETS: TargetProfileMask = TargetProfileMask(
    TargetProfileMask::NATIVE_DESKTOP.0
        | TargetProfileMask::WASM_SANDBOX.0
        | TargetProfileMask::GPU_COMPUTE.0,
);

const WGPU_AFFINE_PARAMS: &[ParamSpec] = &[
    ParamSpec::slice("input_vec", PrimitiveKind::F32, 4, 4),
    ParamSpec::slice("scale_bias", PrimitiveKind::F32, 2, 2),
];

const WGPU_EFFECTS: EffectSet = EffectSet::PURE_COMPUTE.union(EffectSet::HARDWARE_IO);

const WGPU_AFFINE_ENVELOPE: ResourceEnvelope = ResourceEnvelope {
    max_ticks: 64,
    max_scratch_bytes: 0,
    max_vram_bytes: 256,
    max_output_bytes: 16,
    max_spawned_tasks: 0,
    flags: 0,
};

const WGPU_FORGE_FUNCS: &[FuncDescriptor] = &[FuncDescriptor {
    name: "affine_f32",
    canonical_iri: CanonicalIri::new(
        "https://qualiadb.org/ext/etalon/gpu/wgpu/compute/affine_f32",
    ),
    stable_id: StableFuncId::new(0, 0),
    params: WGPU_AFFINE_PARAMS,
    output_kind: PrimitiveKind::F32,
    required_output_elems: 4,
    effects: WGPU_EFFECTS,
    determinism: DeterminismGuarantee::DeterministicTolerance,
    reversibility: ReversibilityKind::PureNoState,
    envelope: WGPU_AFFINE_ENVELOPE,
    contract: OntologicalContract {
        contract_digest: OntologicalContract::canonical_digest(
            q_hash_str("https://qualiadb.org/ext/etalon/gpu/wgpu/compute/affine_f32"),
            StableFuncId::new(0, 0),
            q_hash_str("(f32[4],f32[2])->f32[4]"),
            WGPU_EFFECTS,
            DeterminismGuarantee::DeterministicTolerance,
            ReversibilityKind::PureNoState,
            WGPU_AFFINE_ENVELOPE,
            (0, 1),
            (1, 1),
            (2, 1),
        ),
        schema_hash: q_hash_str("(f32[4],f32[2])->f32[4]"),
        precondition_quin_span: (0, 1),
        postcondition_quin_span: (1, 1),
        invariant_quin_span: (2, 1),
    },
}];

const WGPU_FORGE_SUB_NAMESPACES: &[SubNamespaceDescriptor] = &[SubNamespaceDescriptor {
    name: "compute",
    sub_id: 0,
    functions: WGPU_FORGE_FUNCS,
}];

const WGPU_FORGE_DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    canonical_iri: CanonicalIri::new("https://qualiadb.org/ext/etalon/gpu/wgpu"),
    short_alias: "wgpu_forge",
    versions: ContractVersionAxes::v0_3_default(
        SemVer64::release(0, 3, 0),
        SemVer64::release(0, 1, 0),
    ),
    supported_targets: WGPU_FORGE_TARGETS,
    sub_namespaces: WGPU_FORGE_SUB_NAMESPACES,
    max_ontology_quins: 16,
};

const WGPU_FORGE_ONTOLOGY_QUINS: &[NQuin48] = &[
    NQuin48::from_parts(
        q_hash_str("https://qualiadb.org/ext/etalon/gpu/wgpu"),
        q_hash_str("https://qualiadb.org/schema/vibe#hasPrecondition"),
        q_hash_str("https://qualiadb.org/schema/vibe#FiniteF32InputSlice"),
        q_hash_str("https://qualiadb.org/context/public"),
        1,
    ),
    NQuin48::from_parts(
        q_hash_str("https://qualiadb.org/ext/etalon/gpu/wgpu"),
        q_hash_str("https://qualiadb.org/schema/vibe#hasPostcondition"),
        q_hash_str("https://qualiadb.org/schema/vibe#TransientGpuAllocationsReclaimed"),
        q_hash_str("https://qualiadb.org/context/public"),
        2,
    ),
    NQuin48::from_parts(
        q_hash_str("https://qualiadb.org/ext/etalon/gpu/wgpu"),
        q_hash_str("https://qualiadb.org/schema/vibe#preservesInvariant"),
        q_hash_str("https://qualiadb.org/schema/vibe#ZeroGpuMemoryLeakage"),
        q_hash_str("https://qualiadb.org/context/public"),
        3,
    ),
];

/// Executes `BuiltinKernel::AffineF32` on `WgpuComputeContext` when a native wgpu adapter is
/// available, reading back `[f32; 4]` and unconditionally reclaiming transient GPU allocations
/// (`clear_transient_allocations()`). Returns `(output, used_hardware_wgpu, post_cleanup_bytes)`.
pub fn execute_wgpu_forge_affine_f32_with_cleanup(
    x: [f32; 4],
    scale: f32,
    bias: f32,
) -> Result<([f32; 4], bool, usize), VibeTrap> {
    if !scale.is_finite() || !bias.is_finite() || x.iter().any(|v| !v.is_finite()) {
        return Err(VibeTrap::NumericalDomainError);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use crate::wgsl_forge::execute::{
            BindingUsage, QualiaCompute, WgpuComputeContext, WgpuPipeline,
        };
        use crate::wgsl_forge::{
            emit_shader, validate_wgsl, AffineParams, BuiltinKernel, Schedule, TargetBackend,
        };

        let kernel = BuiltinKernel::AffineF32.spec();
        let schedule = Schedule::default();
        let generated = emit_shader(&kernel, schedule, TargetBackend::Wgsl)
            .map_err(|_| VibeTrap::NumericalDomainError)?;
        let report = validate_wgsl(&generated.source).map_err(|_| VibeTrap::NumericalDomainError)?;
        if !report.naga_validated {
            return Err(VibeTrap::NumericalDomainError);
        }

        if let Ok(mut ctx) = WgpuComputeContext::new(64 * 1024) {
            let run_result = (|| -> Result<[f32; 4], VibeTrap> {
                let in_bytes = bytemuck::cast_slice(&x);
                let view_in = ctx
                    .allocate_and_write(in_bytes, 0, 0, BindingUsage::StorageRead)
                    .map_err(|_| VibeTrap::ResourceEnvelopeExceeded)?;
                let view_out = ctx
                    .allocate_transient(16, 1, 0, BindingUsage::StorageReadWrite)
                    .map_err(|_| VibeTrap::ResourceEnvelopeExceeded)?;
                let params = AffineParams {
                    length: 4,
                    scale,
                    bias,
                    _pad: 0,
                };
                let view_params = ctx
                    .allocate_and_write(bytemuck::bytes_of(&params), 2, 0, BindingUsage::Uniform)
                    .map_err(|_| VibeTrap::ResourceEnvelopeExceeded)?;
                let buffers = [view_in, view_out, view_params];
                let pipeline =
                    WgpuPipeline::compile(&ctx, &generated.source, &kernel.entry_point)
                        .map_err(|_| VibeTrap::NumericalDomainError)?;
                pipeline
                    .dispatch(&buffers, &schedule, 4)
                    .map_err(|_| VibeTrap::NumericalDomainError)?;
                let mut arr = [0.0f32; 4];
                ctx.read_buffer_f32_into(&view_out, &mut arr)
                    .map_err(|_| VibeTrap::NumericalDomainError)?;
                Ok(arr)
            })();

            // Always reclaim transient GPU buffers regardless of success or failure.
            ctx.clear_transient_allocations();
            ctx.allocator.restore_checkpoint(0);
            let post_bytes = ctx.allocator.write_checkpoint() as usize;
            let out = run_result?;
            return Ok((out, true, post_bytes));
        }
    }

    let out = oracle_gpu_affine_f32(x, scale, bias);
    Ok((out, false, 0))
}

/// Real QualiaDB WGSL Forge / wgpu Compute Extension Provider (`VS03-12`).
#[derive(Debug, Clone, Copy, Default)]
pub struct WgpuForgeExtensionProvider;

impl VibeNamespaceProvider for WgpuForgeExtensionProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &WGPU_FORGE_DESCRIPTOR
    }

    fn static_ontology_nquins(&self) -> &'static [NQuin48] {
        WGPU_FORGE_ONTOLOGY_QUINS
    }

    fn call(
        &self,
        stable_id: StableFuncId,
        inputs: &[BorrowedSliceView<'_>],
        output: &mut CallerOutputSlice<'_>,
    ) -> Result<u64, VibeTrap> {
        if stable_id != StableFuncId::new(0, 0) {
            return Err(VibeTrap::FunctionNotFound);
        }
        let mut x = [0.0f32; 4];
        for (i, slot) in x.iter_mut().enumerate() {
            *slot = inputs[0].read_f32_le(i)?;
        }
        let scale = inputs[1].read_f32_le(0)?;
        let bias = inputs[1].read_f32_le(1)?;

        let (res, _used_hw, post_bytes) =
            execute_wgpu_forge_affine_f32_with_cleanup(x, scale, bias)?;
        if post_bytes != 0 {
            return Err(VibeTrap::StagingAbortRequired);
        }
        for &val in &res {
            output.push_f32_le(val)?;
        }
        Ok(24)
    }
}

/// Outcome of a governed multi-token inference stream (`VS03-12`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GovernedInferenceStreamSummary {
    /// Number of token steps committed.
    pub committed_steps: usize,
    /// Final rolling KV state hash after all token steps committed.
    pub final_kv_state_hash: u64,
}

/// Runs a multi-step governed token decoding loop (`infer.decode.governed_token_step`)
/// where every token step is admitted by `WebizenSentinelGate`, audited with a receipt `NQuin` pair,
/// and stages an `emittedToken` `NQuin` into `published_graph`.
///
/// If **any** token step fails admission, experiences mid-stream policy revocation (`on_step_completed`),
/// or fails receipt persistence, **all** staged token `NQuin`s and `decoded_tokens_out` entries from the
/// entire stream are rolled back and `*kv_state_hash` is restored to its pre-stream value.
#[allow(clippy::too_many_arguments)]
pub fn execute_governed_per_token_inference<F>(
    registry: &ExtensionRegistry<'_>,
    base_ctx: &CallExecutionContext,
    gate: &mut WebizenSentinelGate<'_>,
    step_logits: &[[f64; 4]],
    kv_state_hash: &mut u64,
    published_graph: &mut [NQuin],
    published_len: &mut usize,
    decoded_tokens_out: &mut [(u64, u64)],
    mut on_step_completed: F,
) -> Result<GovernedInferenceStreamSummary, VibeTrap>
where
    F: FnMut(usize, &mut WebizenSentinelGate<'_>),
{
    if step_logits.len() > decoded_tokens_out.len() {
        return Err(VibeTrap::OutputBufferTooSmall);
    }

    let handle = prepare_extension_call(
        registry,
        "infer",
        "decode",
        "governed_token_step",
        SemVerReq::AnyStable,
        TargetProfileMask::NATIVE_DESKTOP,
    )?;

    let saved_published_len = *published_len;
    let saved_kv_hash = *kv_state_hash;
    let emitted_pred = q_hash_str("https://qualiadb.org/schema/vibe#emittedToken");

    let mut step_idx = 0;
    while step_idx < step_logits.len() {
        let mut logits_bytes = [0u8; 32];
        let mut k = 0;
        while k < 4 {
            logits_bytes[k * 8..(k + 1) * 8]
                .copy_from_slice(&step_logits[step_idx][k].to_le_bytes());
            k += 1;
        }
        let kv_bytes = (*kv_state_hash).to_le_bytes();
        let step_u64 = step_idx as u64;
        let step_bytes = step_u64.to_le_bytes();

        let inputs = [
            BorrowedSliceView::from_raw_bytes(&logits_bytes, PrimitiveKind::F64, 4)?,
            BorrowedSliceView::from_raw_bytes(&kv_bytes, PrimitiveKind::U64, 1)?,
            BorrowedSliceView::from_raw_bytes(&step_bytes, PrimitiveKind::U64, 1)?,
        ];

        let mut out_raw = [0u8; 24];
        let mut out_slice = CallerOutputSlice::new(&mut out_raw, PrimitiveKind::U64)?;

        let step_ctx = CallExecutionContext {
            actor_did_hash: base_ctx.actor_did_hash,
            context_world_hash: base_ctx.context_world_hash,
            lamport_timestamp: base_ctx.lamport_timestamp.saturating_add(step_u64),
            mode: base_ctx.mode,
        };

        // Pre-compute candidate token NQuin using deterministic argmax for staging parity
        let mut best_id = 0u64;
        let mut best_val = step_logits[step_idx][0];
        let mut m = 1;
        while m < 4 {
            if step_logits[step_idx][m] > best_val {
                best_val = step_logits[step_idx][m];
                best_id = m as u64;
            }
            m += 1;
        }

        let staged_quin = NQuin {
            subject: step_ctx.actor_did_hash,
            predicate: emitted_pred,
            object: best_id,
            context: step_ctx.context_world_hash,
            metadata: step_ctx.lamport_timestamp,
            parity: step_ctx.actor_did_hash ^ emitted_pred ^ best_id ^ step_ctx.context_world_hash,
        };

        match execute_governed_staged_call(
            registry,
            &handle,
            &step_ctx,
            gate,
            &inputs,
            &mut out_slice,
            &[staged_quin],
            published_graph,
            published_len,
        ) {
            Ok(_) => {
                let mut tok_b = [0u8; 8];
                tok_b.copy_from_slice(&out_raw[0..8]);
                let tok_id = u64::from_le_bytes(tok_b);
                let mut next_kv_b = [0u8; 8];
                next_kv_b.copy_from_slice(&out_raw[8..16]);
                let next_kv = u64::from_le_bytes(next_kv_b);

                *kv_state_hash = next_kv;
                decoded_tokens_out[step_idx] = (tok_id, next_kv);
                on_step_completed(step_idx, gate);
                step_idx += 1;
            }
            Err(trap) => {
                // Roll back all staged NQuins and decoded token outputs from this stream
                let mut r = saved_published_len;
                while r < *published_len {
                    published_graph[r] = NQuin::default();
                    r += 1;
                }
                *published_len = saved_published_len;
                *kv_state_hash = saved_kv_hash;
                let mut z = 0;
                while z <= step_idx && z < decoded_tokens_out.len() {
                    decoded_tokens_out[z] = (0, 0);
                    z += 1;
                }
                return Err(trap);
            }
        }
    }

    Ok(GovernedInferenceStreamSummary {
        committed_steps: step_logits.len(),
        final_kv_state_hash: *kv_state_hash,
    })
}
