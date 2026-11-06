//! Stage S6 (`VS03-12`) GPU compute (`EtalonGpuComputeProvider`) and governed
//! inference (`GovernedInferenceProvider`) extension providers with canonical
//! contract digests and direct-call reference oracles.

use vibescript_abi::{
    q_hash, q_hash_str, BorrowedSliceView, CallerOutputSlice, CanonicalIri, ContractVersionAxes,
    DeterminismGuarantee, EffectSet, FuncDescriptor, NQuin48, OntologicalContract, ParamSpec,
    PrimitiveKind, ProviderDescriptor, ResourceEnvelope, ReversibilityKind, SemVer64, StableFuncId,
    SubNamespaceDescriptor, TargetProfileMask, VibeNamespaceProvider, VibeTrap,
};

const GPU_TARGETS: TargetProfileMask = TargetProfileMask(
    TargetProfileMask::NATIVE_DESKTOP.0
        | TargetProfileMask::WASM_SANDBOX.0
        | TargetProfileMask::GPU_COMPUTE.0,
);

const INFER_TARGETS: TargetProfileMask = TargetProfileMask(
    TargetProfileMask::ALL_CPU.0 | TargetProfileMask::GPU_COMPUTE.0,
);

const AFFINE_PARAMS: &[ParamSpec] = &[
    ParamSpec::slice("input_vec", PrimitiveKind::F32, 4, 4),
    ParamSpec::slice("scale_bias", PrimitiveKind::F32, 2, 2),
];

const GEMV4_PARAMS: &[ParamSpec] = &[
    ParamSpec::slice("mat4x4", PrimitiveKind::F32, 16, 16),
    ParamSpec::slice("vec4", PrimitiveKind::F32, 4, 4),
];

const TOPK2_PARAMS: &[ParamSpec] = &[ParamSpec::slice("scores", PrimitiveKind::F32, 4, 16)];

const GPU_EFFECTS: EffectSet = EffectSet::PURE_COMPUTE.union(EffectSet::HARDWARE_IO);

const AFFINE_ENV: ResourceEnvelope = ResourceEnvelope {
    max_ticks: 64,
    max_scratch_bytes: 0,
    max_vram_bytes: 256,
    max_output_bytes: 16,
    max_spawned_tasks: 0,
    flags: 0,
};

const GEMV4_ENV: ResourceEnvelope = ResourceEnvelope {
    max_ticks: 128,
    max_scratch_bytes: 0,
    max_vram_bytes: 512,
    max_output_bytes: 16,
    max_spawned_tasks: 0,
    flags: 0,
};

const TOPK2_ENV: ResourceEnvelope = ResourceEnvelope {
    max_ticks: 96,
    max_scratch_bytes: 0,
    max_vram_bytes: 256,
    max_output_bytes: 8,
    max_spawned_tasks: 0,
    flags: 0,
};

const GPU_COMPUTE_FUNCS: &[FuncDescriptor] = &[
    FuncDescriptor {
        name: "affine_f32",
        canonical_iri: CanonicalIri::new(
            "https://qualiadb.org/ext/etalon/gpu/compute/affine_f32",
        ),
        stable_id: StableFuncId::new(0, 0),
        params: AFFINE_PARAMS,
        output_kind: PrimitiveKind::F32,
        required_output_elems: 4,
        effects: GPU_EFFECTS,
        determinism: DeterminismGuarantee::DeterministicTolerance,
        reversibility: ReversibilityKind::PureNoState,
        envelope: AFFINE_ENV,
        contract: OntologicalContract {
            contract_digest: OntologicalContract::canonical_digest(
                q_hash_str("https://qualiadb.org/ext/etalon/gpu/compute/affine_f32"),
                StableFuncId::new(0, 0),
                q_hash_str("(f32[4],f32[2])->f32[4]"),
                GPU_EFFECTS,
                DeterminismGuarantee::DeterministicTolerance,
                ReversibilityKind::PureNoState,
                AFFINE_ENV,
                (0, 1),
                (1, 1),
                (2, 1),
            ),
            schema_hash: q_hash_str("(f32[4],f32[2])->f32[4]"),
            precondition_quin_span: (0, 1),
            postcondition_quin_span: (1, 1),
            invariant_quin_span: (2, 1),
        },
    },
    FuncDescriptor {
        name: "gemv4_f32",
        canonical_iri: CanonicalIri::new(
            "https://qualiadb.org/ext/etalon/gpu/compute/gemv4_f32",
        ),
        stable_id: StableFuncId::new(0, 1),
        params: GEMV4_PARAMS,
        output_kind: PrimitiveKind::F32,
        required_output_elems: 4,
        effects: GPU_EFFECTS,
        determinism: DeterminismGuarantee::DeterministicTolerance,
        reversibility: ReversibilityKind::PureNoState,
        envelope: GEMV4_ENV,
        contract: OntologicalContract {
            contract_digest: OntologicalContract::canonical_digest(
                q_hash_str("https://qualiadb.org/ext/etalon/gpu/compute/gemv4_f32"),
                StableFuncId::new(0, 1),
                q_hash_str("(f32[16],f32[4])->f32[4]"),
                GPU_EFFECTS,
                DeterminismGuarantee::DeterministicTolerance,
                ReversibilityKind::PureNoState,
                GEMV4_ENV,
                (0, 1),
                (1, 1),
                (2, 1),
            ),
            schema_hash: q_hash_str("(f32[16],f32[4])->f32[4]"),
            precondition_quin_span: (0, 1),
            postcondition_quin_span: (1, 1),
            invariant_quin_span: (2, 1),
        },
    },
    FuncDescriptor {
        name: "topk2_f32",
        canonical_iri: CanonicalIri::new(
            "https://qualiadb.org/ext/etalon/gpu/compute/topk2_f32",
        ),
        stable_id: StableFuncId::new(0, 2),
        params: TOPK2_PARAMS,
        output_kind: PrimitiveKind::F32,
        required_output_elems: 2,
        effects: GPU_EFFECTS,
        determinism: DeterminismGuarantee::DeterministicTolerance,
        reversibility: ReversibilityKind::PureNoState,
        envelope: TOPK2_ENV,
        contract: OntologicalContract {
            contract_digest: OntologicalContract::canonical_digest(
                q_hash_str("https://qualiadb.org/ext/etalon/gpu/compute/topk2_f32"),
                StableFuncId::new(0, 2),
                q_hash_str("(f32[4..16])->f32[2]"),
                GPU_EFFECTS,
                DeterminismGuarantee::DeterministicTolerance,
                ReversibilityKind::PureNoState,
                TOPK2_ENV,
                (0, 1),
                (1, 1),
                (2, 1),
            ),
            schema_hash: q_hash_str("(f32[4..16])->f32[2]"),
            precondition_quin_span: (0, 1),
            postcondition_quin_span: (1, 1),
            invariant_quin_span: (2, 1),
        },
    },
];

const GPU_SUB_NAMESPACES: &[SubNamespaceDescriptor] = &[SubNamespaceDescriptor {
    name: "compute",
    sub_id: 0,
    functions: GPU_COMPUTE_FUNCS,
}];

const GPU_DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    canonical_iri: CanonicalIri::new("https://qualiadb.org/ext/etalon/gpu"),
    short_alias: "gpu",
    versions: ContractVersionAxes::v0_3_default(
        SemVer64::release(0, 3, 0),
        SemVer64::release(0, 1, 0),
    ),
    supported_targets: GPU_TARGETS,
    sub_namespaces: GPU_SUB_NAMESPACES,
    max_ontology_quins: 16,
};

const GPU_ONTOLOGY_QUINS: &[NQuin48] = &[
    NQuin48::from_parts(
        q_hash_str("https://qualiadb.org/ext/etalon/gpu"),
        q_hash_str("https://qualiadb.org/schema/vibe#hasPrecondition"),
        q_hash_str("https://qualiadb.org/schema/vibe#AlignedDeviceBufferSlice"),
        q_hash_str("https://qualiadb.org/context/public"),
        1,
    ),
    NQuin48::from_parts(
        q_hash_str("https://qualiadb.org/ext/etalon/gpu"),
        q_hash_str("https://qualiadb.org/schema/vibe#hasPostcondition"),
        q_hash_str("https://qualiadb.org/schema/vibe#ReadbackCompletedAndSlabReleased"),
        q_hash_str("https://qualiadb.org/context/public"),
        2,
    ),
    NQuin48::from_parts(
        q_hash_str("https://qualiadb.org/ext/etalon/gpu"),
        q_hash_str("https://qualiadb.org/schema/vibe#preservesInvariant"),
        q_hash_str("https://qualiadb.org/schema/vibe#ZeroLeakedTransientSlabs"),
        q_hash_str("https://qualiadb.org/context/public"),
        3,
    ),
];

/// Portable GPU compute provider (`VS03-12`).
#[derive(Debug, Clone, Copy, Default)]
pub struct EtalonGpuComputeProvider;

impl VibeNamespaceProvider for EtalonGpuComputeProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &GPU_DESCRIPTOR
    }

    fn static_ontology_nquins(&self) -> &'static [NQuin48] {
        GPU_ONTOLOGY_QUINS
    }

    fn call(
        &self,
        stable_id: StableFuncId,
        inputs: &[BorrowedSliceView<'_>],
        output: &mut CallerOutputSlice<'_>,
    ) -> Result<u64, VibeTrap> {
        match (stable_id.sub_namespace_id, stable_id.func_id) {
            (0, 0) => {
                // compute::affine_f32
                let mut x = [0.0f32; 4];
                for (i, slot) in x.iter_mut().enumerate() {
                    let v = inputs[0].read_f32_le(i)?;
                    if !v.is_finite() {
                        return Err(VibeTrap::NumericalDomainError);
                    }
                    *slot = v;
                }
                let scale = inputs[1].read_f32_le(0)?;
                let bias = inputs[1].read_f32_le(1)?;
                if !scale.is_finite() || !bias.is_finite() {
                    return Err(VibeTrap::NumericalDomainError);
                }
                let y = oracle_gpu_affine_f32(x, scale, bias);
                for &val in &y {
                    output.push_f32_le(val)?;
                }
                Ok(24)
            }
            (0, 1) => {
                // compute::gemv4_f32
                let mut mat = [0.0f32; 16];
                for (i, slot) in mat.iter_mut().enumerate() {
                    let v = inputs[0].read_f32_le(i)?;
                    if !v.is_finite() {
                        return Err(VibeTrap::NumericalDomainError);
                    }
                    *slot = v;
                }
                let mut vec4 = [0.0f32; 4];
                for (i, slot) in vec4.iter_mut().enumerate() {
                    let v = inputs[1].read_f32_le(i)?;
                    if !v.is_finite() {
                        return Err(VibeTrap::NumericalDomainError);
                    }
                    *slot = v;
                }
                let y = oracle_gpu_gemv4_f32(mat, vec4);
                for &val in &y {
                    output.push_f32_le(val)?;
                }
                Ok(48)
            }
            (0, 2) => {
                // compute::topk2_f32
                let count = inputs[0].elem_count();
                let mut scores = [0.0f32; 16];
                for (i, slot) in scores.iter_mut().take(count).enumerate() {
                    let v = inputs[0].read_f32_le(i)?;
                    if !v.is_finite() {
                        return Err(VibeTrap::NumericalDomainError);
                    }
                    *slot = v;
                }
                let top2 = oracle_gpu_topk2_f32(&scores[..count]);
                output.push_f32_le(top2[0])?;
                output.push_f32_le(top2[1])?;
                Ok(32)
            }
            _ => Err(VibeTrap::FunctionNotFound),
        }
    }
}

/// Direct-call oracle for `gpu::compute::affine_f32`.
#[inline]
pub fn oracle_gpu_affine_f32(x: [f32; 4], scale: f32, bias: f32) -> [f32; 4] {
    [
        x[0] * scale + bias,
        x[1] * scale + bias,
        x[2] * scale + bias,
        x[3] * scale + bias,
    ]
}

/// Direct-call oracle for `gpu::compute::gemv4_f32`.
#[inline]
pub fn oracle_gpu_gemv4_f32(mat: [f32; 16], vec4: [f32; 4]) -> [f32; 4] {
    let mut out = [0.0f32; 4];
    for r in 0..4 {
        let base = r * 4;
        out[r] = mat[base] * vec4[0]
            + mat[base + 1] * vec4[1]
            + mat[base + 2] * vec4[2]
            + mat[base + 3] * vec4[3];
    }
    out
}

/// Direct-call oracle for `gpu::compute::topk2_f32`.
#[inline]
pub fn oracle_gpu_topk2_f32(scores: &[f32]) -> [f32; 2] {
    let mut best1 = f32::NEG_INFINITY;
    let mut best2 = f32::NEG_INFINITY;
    for &s in scores {
        if s >= best1 {
            best2 = best1;
            best1 = s;
        } else if s > best2 {
            best2 = s;
        }
    }
    [best1, best2]
}

// ---------------------------------------------------------------------------
// GovernedInferenceProvider (VS03-12)
// ---------------------------------------------------------------------------

const RMS_NORM_PARAMS: &[ParamSpec] = &[
    ParamSpec::slice("x", PrimitiveKind::F64, 4, 4),
    ParamSpec::slice("weight", PrimitiveKind::F64, 4, 4),
    ParamSpec::scalar("eps", PrimitiveKind::F64),
];

const SOFTMAX_PARAMS: &[ParamSpec] = &[ParamSpec::slice("logits", PrimitiveKind::F64, 4, 4)];

const TOKEN_STEP_PARAMS: &[ParamSpec] = &[
    ParamSpec::slice("logits", PrimitiveKind::F64, 4, 4),
    ParamSpec::scalar("kv_state_hash", PrimitiveKind::U64),
    ParamSpec::scalar("step_index", PrimitiveKind::U64),
];

const RMS_ENV: ResourceEnvelope = ResourceEnvelope::hot_zero_heap(48, 32);
const SOFTMAX_ENV: ResourceEnvelope = ResourceEnvelope::hot_zero_heap(48, 32);
const TOKEN_STEP_EFFECTS: EffectSet =
    EffectSet::HARDWARE_IO.union(EffectSet::GRAPH_MUTATE);
const TOKEN_STEP_ENV: ResourceEnvelope = ResourceEnvelope {
    max_ticks: 64,
    max_scratch_bytes: 0,
    max_vram_bytes: 128,
    max_output_bytes: 24,
    max_spawned_tasks: 0,
    flags: 0,
};

const INFER_ACTIVATION_FUNCS: &[FuncDescriptor] = &[
    FuncDescriptor {
        name: "rms_norm_f64",
        canonical_iri: CanonicalIri::new(
            "https://qualiadb.org/ext/qualia/inference/activations/rms_norm_f64",
        ),
        stable_id: StableFuncId::new(0, 0),
        params: RMS_NORM_PARAMS,
        output_kind: PrimitiveKind::F64,
        required_output_elems: 4,
        effects: EffectSet::PURE_COMPUTE,
        determinism: DeterminismGuarantee::DeterministicTolerance,
        reversibility: ReversibilityKind::PureNoState,
        envelope: RMS_ENV,
        contract: OntologicalContract {
            contract_digest: OntologicalContract::canonical_digest(
                q_hash_str("https://qualiadb.org/ext/qualia/inference/activations/rms_norm_f64"),
                StableFuncId::new(0, 0),
                q_hash_str("(f64[4],f64[4],f64)->f64[4]"),
                EffectSet::PURE_COMPUTE,
                DeterminismGuarantee::DeterministicTolerance,
                ReversibilityKind::PureNoState,
                RMS_ENV,
                (0, 1),
                (1, 1),
                (2, 1),
            ),
            schema_hash: q_hash_str("(f64[4],f64[4],f64)->f64[4]"),
            precondition_quin_span: (0, 1),
            postcondition_quin_span: (1, 1),
            invariant_quin_span: (2, 1),
        },
    },
    FuncDescriptor {
        name: "softmax_f64",
        canonical_iri: CanonicalIri::new(
            "https://qualiadb.org/ext/qualia/inference/activations/softmax_f64",
        ),
        stable_id: StableFuncId::new(0, 1),
        params: SOFTMAX_PARAMS,
        output_kind: PrimitiveKind::F64,
        required_output_elems: 4,
        effects: EffectSet::PURE_COMPUTE,
        determinism: DeterminismGuarantee::DeterministicTolerance,
        reversibility: ReversibilityKind::PureNoState,
        envelope: SOFTMAX_ENV,
        contract: OntologicalContract {
            contract_digest: OntologicalContract::canonical_digest(
                q_hash_str("https://qualiadb.org/ext/qualia/inference/activations/softmax_f64"),
                StableFuncId::new(0, 1),
                q_hash_str("(f64[4])->f64[4]"),
                EffectSet::PURE_COMPUTE,
                DeterminismGuarantee::DeterministicTolerance,
                ReversibilityKind::PureNoState,
                SOFTMAX_ENV,
                (0, 1),
                (1, 1),
                (2, 1),
            ),
            schema_hash: q_hash_str("(f64[4])->f64[4]"),
            precondition_quin_span: (0, 1),
            postcondition_quin_span: (1, 1),
            invariant_quin_span: (2, 1),
        },
    },
];

const INFER_DECODE_FUNCS: &[FuncDescriptor] = &[FuncDescriptor {
    name: "governed_token_step",
    canonical_iri: CanonicalIri::new(
        "https://qualiadb.org/ext/qualia/inference/decode/governed_token_step",
    ),
    stable_id: StableFuncId::new(1, 0),
    params: TOKEN_STEP_PARAMS,
    output_kind: PrimitiveKind::U64,
    required_output_elems: 3,
    effects: TOKEN_STEP_EFFECTS,
    determinism: DeterminismGuarantee::DeterministicExact,
    reversibility: ReversibilityKind::StagedReversible,
    envelope: TOKEN_STEP_ENV,
    contract: OntologicalContract {
        contract_digest: OntologicalContract::canonical_digest(
            q_hash_str("https://qualiadb.org/ext/qualia/inference/decode/governed_token_step"),
            StableFuncId::new(1, 0),
            q_hash_str("(f64[4],u64,u64)->u64[3]"),
            TOKEN_STEP_EFFECTS,
            DeterminismGuarantee::DeterministicExact,
            ReversibilityKind::StagedReversible,
            TOKEN_STEP_ENV,
            (0, 1),
            (1, 1),
            (2, 1),
        ),
        schema_hash: q_hash_str("(f64[4],u64,u64)->u64[3]"),
        precondition_quin_span: (0, 1),
        postcondition_quin_span: (1, 1),
        invariant_quin_span: (2, 1),
    },
}];

const INFER_SUB_NAMESPACES: &[SubNamespaceDescriptor] = &[
    SubNamespaceDescriptor {
        name: "activations",
        sub_id: 0,
        functions: INFER_ACTIVATION_FUNCS,
    },
    SubNamespaceDescriptor {
        name: "decode",
        sub_id: 1,
        functions: INFER_DECODE_FUNCS,
    },
];

const INFER_DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    canonical_iri: CanonicalIri::new("https://qualiadb.org/ext/qualia/inference"),
    short_alias: "infer",
    versions: ContractVersionAxes::v0_3_default(
        SemVer64::release(0, 3, 0),
        SemVer64::release(0, 1, 0),
    ),
    supported_targets: INFER_TARGETS,
    sub_namespaces: INFER_SUB_NAMESPACES,
    max_ontology_quins: 16,
};

const INFER_ONTOLOGY_QUINS: &[NQuin48] = &[
    NQuin48::from_parts(
        q_hash_str("https://qualiadb.org/ext/qualia/inference"),
        q_hash_str("https://qualiadb.org/schema/vibe#hasPrecondition"),
        q_hash_str("https://qualiadb.org/schema/vibe#PerTokenSentinelTicketActive"),
        q_hash_str("https://qualiadb.org/context/public"),
        1,
    ),
    NQuin48::from_parts(
        q_hash_str("https://qualiadb.org/ext/qualia/inference"),
        q_hash_str("https://qualiadb.org/schema/vibe#hasPostcondition"),
        q_hash_str("https://qualiadb.org/schema/vibe#NormalizedDistributionOrRolledBackKv"),
        q_hash_str("https://qualiadb.org/context/public"),
        2,
    ),
    NQuin48::from_parts(
        q_hash_str("https://qualiadb.org/ext/qualia/inference"),
        q_hash_str("https://qualiadb.org/schema/vibe#preservesInvariant"),
        q_hash_str("https://qualiadb.org/schema/vibe#ZeroUncommittedTokenLeakage"),
        q_hash_str("https://qualiadb.org/context/public"),
        3,
    ),
];

/// Governed inference extension provider (`VS03-12`).
#[derive(Debug, Clone, Copy, Default)]
pub struct GovernedInferenceProvider;

impl VibeNamespaceProvider for GovernedInferenceProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &INFER_DESCRIPTOR
    }

    fn static_ontology_nquins(&self) -> &'static [NQuin48] {
        INFER_ONTOLOGY_QUINS
    }

    fn call(
        &self,
        stable_id: StableFuncId,
        inputs: &[BorrowedSliceView<'_>],
        output: &mut CallerOutputSlice<'_>,
    ) -> Result<u64, VibeTrap> {
        match (stable_id.sub_namespace_id, stable_id.func_id) {
            (0, 0) => {
                // activations::rms_norm_f64
                let mut x = [0.0f64; 4];
                let mut w = [0.0f64; 4];
                for i in 0..4 {
                    x[i] = inputs[0].read_f64_le(i)?;
                    w[i] = inputs[1].read_f64_le(i)?;
                    if !x[i].is_finite() || !w[i].is_finite() {
                        return Err(VibeTrap::NumericalDomainError);
                    }
                }
                let eps = inputs[2].read_f64_le(0)?;
                if !eps.is_finite() || eps < 0.0 {
                    return Err(VibeTrap::NumericalDomainError);
                }
                let normed = oracle_rms_norm_f64(x, w, eps);
                for &val in &normed {
                    output.push_f64_le(val)?;
                }
                Ok(28)
            }
            (0, 1) => {
                // activations::softmax_f64
                let mut logits = [0.0f64; 4];
                for (i, slot) in logits.iter_mut().enumerate() {
                    let v = inputs[0].read_f64_le(i)?;
                    if !v.is_finite() {
                        return Err(VibeTrap::NumericalDomainError);
                    }
                    *slot = v;
                }
                let probs = oracle_softmax_f64(logits);
                for &p in &probs {
                    output.push_f64_le(p)?;
                }
                Ok(24)
            }
            (1, 0) => {
                // decode::governed_token_step
                let mut logits = [0.0f64; 4];
                for (i, slot) in logits.iter_mut().enumerate() {
                    let v = inputs[0].read_f64_le(i)?;
                    if !v.is_finite() {
                        return Err(VibeTrap::NumericalDomainError);
                    }
                    *slot = v;
                }
                let kv_state_hash = inputs[1].read_u64_le(0)?;
                let step_index = inputs[2].read_u64_le(0)?;
                let (token_id, next_kv_hash, next_step) =
                    oracle_governed_token_step(logits, kv_state_hash, step_index);
                output.push_u64_le(token_id)?;
                output.push_u64_le(next_kv_hash)?;
                output.push_u64_le(next_step)?;
                Ok(36)
            }
            _ => Err(VibeTrap::FunctionNotFound),
        }
    }
}

/// Direct-call oracle for `infer::activations::rms_norm_f64`.
#[inline]
pub fn oracle_rms_norm_f64(x: [f64; 4], weight: [f64; 4], eps: f64) -> [f64; 4] {
    let mean_sq = 0.25 * (x[0] * x[0] + x[1] * x[1] + x[2] * x[2] + x[3] * x[3]);
    let inv_rms = 1.0 / (mean_sq + eps).sqrt();
    [
        x[0] * inv_rms * weight[0],
        x[1] * inv_rms * weight[1],
        x[2] * inv_rms * weight[2],
        x[3] * inv_rms * weight[3],
    ]
}

/// Direct-call oracle for `infer::activations::softmax_f64`.
#[inline]
pub fn oracle_softmax_f64(logits: [f64; 4]) -> [f64; 4] {
    let max_l = logits[0].max(logits[1]).max(logits[2]).max(logits[3]);
    let e0 = (logits[0] - max_l).exp();
    let e1 = (logits[1] - max_l).exp();
    let e2 = (logits[2] - max_l).exp();
    let e3 = (logits[3] - max_l).exp();
    let inv_sum = 1.0 / (e0 + e1 + e2 + e3);
    [e0 * inv_sum, e1 * inv_sum, e2 * inv_sum, e3 * inv_sum]
}

/// Direct-call oracle for `infer::decode::governed_token_step`.
#[inline]
pub fn oracle_governed_token_step(
    logits: [f64; 4],
    kv_state_hash: u64,
    step_index: u64,
) -> (u64, u64, u64) {
    let mut best_idx = 0usize;
    let mut best_val = logits[0];
    let mut i = 1;
    while i < 4 {
        if logits[i] > best_val {
            best_val = logits[i];
            best_idx = i;
        }
        i += 1;
    }
    let token_id = best_idx as u64;
    let mut step_bytes = [0u8; 16];
    step_bytes[..8].copy_from_slice(&kv_state_hash.to_le_bytes());
    step_bytes[8..16].copy_from_slice(&token_id.to_le_bytes());
    let next_kv_hash = q_hash(&step_bytes) ^ step_index.rotate_left(7);
    (token_id, next_kv_hash, step_index.saturating_add(1))
}
