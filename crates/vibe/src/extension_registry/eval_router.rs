//! Evaluator, Bytecode VM, and Task Scheduler Call-Site Cutover Router (`Stage S6`).
//!
//! Replaces direct string-based `crate::bind::dispatch` calls across
//! `crates/vibe/src/{eval, bytecode, task_scheduler}` with unified
//! `ExtensionRegistry` resolution, $O(1)$ `PreparedCallCache` lookup,
//! zero-heap slice marshaling, `SentinelPolicyGate` enforcement, and budget charging.

use super::dispatch_adapter::{dispatch_prepared_call, CallExecutionContext, DispatchOutcome};
#[path = "native_budget.rs"]
mod native_budget;
use super::domain_providers::DeterministicMechanicsProvider;
use super::gpu_inference_providers::{EtalonGpuComputeProvider, GovernedInferenceProvider};
use super::graph_logic_providers::{DeonticTemporalProvider, StagedGraphMutationProvider};
use super::prepare::prepare_extension_call;
use super::registry::ExtensionRegistry;
use crate::bind::Host;
use crate::budget::Budget;
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;
use etalon_vibe::{
    EtalonForgeProvider, EtalonModelProvider, EtalonNQuinProvider, EtalonRenderProvider,
    EtalonSenseProvider, EtalonSignalProvider,
};
use vibescript_abi::{
    q_hash_str, BorrowedSliceView, CallerOutputSlice, ExecutionMode, FuncDescriptor, ParamSpec,
    PreparedCallHandle, PrimitiveKind, RegistryGeneration, SemVerReq, SentinelPolicyGate,
    TargetProfileMask, VibeTrap,
};

static MECH_PROVIDER: DeterministicMechanicsProvider = DeterministicMechanicsProvider;
static GPU_PROVIDER: EtalonGpuComputeProvider = EtalonGpuComputeProvider;
static FORGE_PROVIDER: EtalonForgeProvider = EtalonForgeProvider;
static RENDER_PROVIDER: EtalonRenderProvider = EtalonRenderProvider;
static SIGNAL_PROVIDER: EtalonSignalProvider = EtalonSignalProvider;
static SENSE_PROVIDER: EtalonSenseProvider = EtalonSenseProvider;
static MODEL_PROVIDER: EtalonModelProvider = EtalonModelProvider;
static NQUIN_PROVIDER: EtalonNQuinProvider = EtalonNQuinProvider;
static INFER_PROVIDER: GovernedInferenceProvider = GovernedInferenceProvider;
static GRAPH_PROVIDER: StagedGraphMutationProvider = StagedGraphMutationProvider;
static MODALITY_PROVIDER: DeonticTemporalProvider = DeonticTemporalProvider;

/// Short aliases of all built-in extension providers registered by default.
pub const BUILTIN_EXTENSION_ALIASES: &[&str] = &[
    "mech", "gpu", "forge", "render", "signal", "sense", "p64", "nquin", "infer", "qgraph",
    "modality",
];

/// Canonical namespace IRIs of all built-in extension providers registered by default.
pub const BUILTIN_EXTENSION_IRIS: &[&str] = &[
    "https://qualiadb.org/ext/etalon/mechanics",
    "https://qualiadb.org/ext/etalon/gpu",
    "https://qualiadb.org/ext/etalon/forge",
    "https://qualiadb.org/ext/etalon/render",
    "https://qualiadb.org/ext/etalon/signal",
    "https://qualiadb.org/ext/etalon/sense",
    "https://qualiadb.org/ext/etalon/model",
    "https://qualiadb.org/ext/etalon/nquin",
    "https://qualiadb.org/ext/qualia/inference",
    "https://qualiadb.org/ext/qualia/graph",
    "https://qualiadb.org/ext/qualia/modality",
];

impl ExtensionRegistry<'static> {
    /// Constructs an `ExtensionRegistry` pre-populated with all 11 built-in Etalon and Qualia providers:
    /// - `mech` (`DeterministicMechanicsProvider`)
    /// - `gpu` (`EtalonGpuComputeProvider`)
    /// - `forge` (`EtalonForgeProvider`)
    /// - `render` (`EtalonRenderProvider`)
    /// - `signal` (`EtalonSignalProvider`)
    /// - `sense` (`EtalonSenseProvider`)
    /// - `p64` (`EtalonModelProvider`)
    /// - `nquin` (`EtalonNQuinProvider`)
    /// - `infer` (`GovernedInferenceProvider`)
    /// - `qgraph` (`StagedGraphMutationProvider`)
    /// - `modality` (`DeonticTemporalProvider`)
    pub fn with_builtin_providers() -> Self {
        let mut reg = Self::new();
        let _ = reg.register(&MECH_PROVIDER);
        let _ = reg.register(&GPU_PROVIDER);
        let _ = reg.register(&FORGE_PROVIDER);
        let _ = reg.register(&RENDER_PROVIDER);
        let _ = reg.register(&SIGNAL_PROVIDER);
        let _ = reg.register(&SENSE_PROVIDER);
        let _ = reg.register(&MODEL_PROVIDER);
        let _ = reg.register(&NQUIN_PROVIDER);
        let _ = reg.register(&INFER_PROVIDER);
        let _ = reg.register(&GRAPH_PROVIDER);
        let _ = reg.register(&MODALITY_PROVIDER);
        reg
    }
}

/// Returns `true` if `ns` is a known built-in extension alias or canonical IRI.
#[inline]
pub fn is_extension_namespace(ns: &str) -> bool {
    let stripped = ns.strip_prefix("vibe:0.3/").unwrap_or(ns);
    BUILTIN_EXTENSION_ALIASES.contains(&stripped) || BUILTIN_EXTENSION_IRIS.contains(&stripped)
}

/// Returns `true` if `path` starts with a registered built-in extension alias or canonical IRI.
#[inline]
pub fn is_extension_call_path(path: &str) -> bool {
    if let Some((head, _)) = path.split_once('.') {
        if is_extension_namespace(head) {
            return true;
        }
    }
    BUILTIN_EXTENSION_IRIS
        .iter()
        .any(|iri| path.starts_with(iri))
}

/// Fixed-capacity, zero-heap cache mapping call-path FNV-1a hashes to `PreparedCallHandle`s.
#[derive(Clone, Copy)]
pub struct PreparedCallCache {
    entries: [Option<(u64, PreparedCallHandle)>; 32],
    cursor: usize,
}

impl Default for PreparedCallCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PreparedCallCache {
    /// Creates an empty `PreparedCallCache`.
    #[inline]
    pub const fn new() -> Self {
        Self {
            entries: [None; 32],
            cursor: 0,
        }
    }

    /// Looks up a cached `PreparedCallHandle` for `(path_hash, current_gen)`.
    #[inline]
    pub fn get(
        &self,
        path_hash: u64,
        current_gen: RegistryGeneration,
    ) -> Option<PreparedCallHandle> {
        let mut i = 0;
        while i < self.entries.len() {
            if let Some((h, handle)) = self.entries[i] {
                if h == path_hash && handle.registry_generation == current_gen {
                    return Some(handle);
                }
            }
            i += 1;
        }
        None
    }

    /// Inserts a `PreparedCallHandle` into the ring cache.
    #[inline]
    pub fn insert(&mut self, path_hash: u64, handle: PreparedCallHandle) {
        self.entries[self.cursor % self.entries.len()] = Some((path_hash, handle));
        self.cursor = self.cursor.wrapping_add(1);
    }
}

/// Maps a deterministic `VibeTrap` to a VibeScript `Diagnostic`.
pub fn trap_to_diagnostic(trap: VibeTrap, path: &str, span: Span) -> Diagnostic {
    let code = match trap {
        VibeTrap::SentinelUnavailable
        | VibeTrap::PolicyDenied
        | VibeTrap::ExplicitDenyRule
        | VibeTrap::AuthorityExpired
        | VibeTrap::DelegationScopeMismatch
        | VibeTrap::RevokedPolicyGeneration
        | VibeTrap::EffectNotPermittedInMode
        | VibeTrap::MidExecutionPolicyRevoked
        | VibeTrap::ReceiptPersistenceFailed
        | VibeTrap::StagingAbortRequired => DiagCode::E700,
        VibeTrap::TickBudgetExhausted | VibeTrap::ResourceEnvelopeExceeded => DiagCode::E400,
        VibeTrap::NamespaceNotFound | VibeTrap::FunctionNotFound => DiagCode::E300,
        _ => DiagCode::E100,
    };
    Diagnostic::new(
        code,
        span,
        format!(
            "extension trap {:?} (code {}) on `{path}`",
            trap, trap as u16
        ),
    )
}

/// Resolves a dotted or canonical IRI call path against `registry` into a `PreparedCallHandle`.
///
/// Supports:
/// - 3-part dotted path: `"mech.kinematics.verlet_step"`, `"https://.../mechanics.kinematics.verlet_step"`
/// - 2-part dotted path: `"mech.verlet_step"` (searches sub-namespaces of provider `"mech"`)
/// - Full canonical function IRI: `"https://qualiadb.org/ext/etalon/mechanics/kinematics/verlet_step"`
pub fn try_prepare_path(
    registry: &ExtensionRegistry<'_>,
    cache: &mut PreparedCallCache,
    path: &str,
    target_profile: TargetProfileMask,
) -> Result<Option<PreparedCallHandle>, VibeTrap> {
    let path_hash = q_hash_str(path) ^ ((target_profile.0 as u64) << 48);
    if let Some(cached) = cache.get(path_hash, registry.generation()) {
        return Ok(Some(cached));
    }

    // Case 1: Full canonical function IRI (`https://...`)
    if path.starts_with("https://") || path.starts_with("http://") {
        // Check if it has `.sub.func` appended after an alias-resolved IRI
        if let Some((ns_iri, rest)) = split_iri_and_member_suffix(registry, path) {
            if let Some((sub, func)) = rest.split_once('.') {
                let handle = prepare_extension_call(
                    registry,
                    ns_iri,
                    sub,
                    func,
                    SemVerReq::AnyStable,
                    target_profile,
                )?;
                cache.insert(path_hash, handle);
                return Ok(Some(handle));
            } else if let Some((_slot, _prov, desc)) = registry.find_by_namespace_or_alias(ns_iri) {
                for sub_desc in desc.sub_namespaces {
                    if sub_desc.functions.iter().any(|f| f.name == rest) {
                        let handle = prepare_extension_call(
                            registry,
                            ns_iri,
                            sub_desc.name,
                            rest,
                            SemVerReq::AnyStable,
                            target_profile,
                        )?;
                        cache.insert(path_hash, handle);
                        return Ok(Some(handle));
                    }
                }
                return Err(VibeTrap::FunctionNotFound);
            }
        }
        // Check if `path` is an exact function canonical_iri in any registered provider
        let mut slot_idx = 0u32;
        while (slot_idx as usize) < super::registry::MAX_EXTENSION_PROVIDERS {
            if let Some(prov) = registry.provider_at(slot_idx) {
                let desc = prov.descriptor();
                for sub in desc.sub_namespaces {
                    for func in sub.functions {
                        if func.canonical_iri.iri == path {
                            let handle = prepare_extension_call(
                                registry,
                                desc.canonical_iri.iri,
                                sub.name,
                                func.name,
                                SemVerReq::AnyStable,
                                target_profile,
                            )?;
                            cache.insert(path_hash, handle);
                            return Ok(Some(handle));
                        }
                    }
                }
            }
            slot_idx += 1;
        }
        return Ok(None);
    }

    // Case 2: Dotted identifier path (`alias.sub.func` or `alias.func`)
    let Some((first, tail)) = path.split_once('.') else {
        return Ok(None);
    };
    let ns_key = first.strip_prefix("vibe:0.3/").unwrap_or(first);
    let Some((_slot, _prov, desc)) = registry.find_by_namespace_or_alias(ns_key) else {
        return Ok(None);
    };

    if let Some((sub, func)) = tail.split_once('.') {
        let handle = prepare_extension_call(
            registry,
            ns_key,
            sub,
            func,
            SemVerReq::AnyStable,
            target_profile,
        )?;
        cache.insert(path_hash, handle);
        return Ok(Some(handle));
    }

    // 2-segment shorthand `alias.func`: search sub-namespaces for `func`
    for sub in desc.sub_namespaces {
        if sub.functions.iter().any(|f| f.name == tail) {
            let handle = prepare_extension_call(
                registry,
                ns_key,
                sub.name,
                tail,
                SemVerReq::AnyStable,
                target_profile,
            )?;
            cache.insert(path_hash, handle);
            return Ok(Some(handle));
        }
    }
    Err(VibeTrap::FunctionNotFound)
}

fn split_iri_and_member_suffix<'a>(
    registry: &ExtensionRegistry<'_>,
    path: &'a str,
) -> Option<(&'a str, &'a str)> {
    let mut slot_idx = 0u32;
    while (slot_idx as usize) < super::registry::MAX_EXTENSION_PROVIDERS {
        if let Some(prov) = registry.provider_at(slot_idx) {
            let ns_iri = prov.descriptor().canonical_iri.iri;
            if let Some(rest) = path.strip_prefix(ns_iri) {
                if let Some(dot_rest) = rest.strip_prefix('.') {
                    return Some((ns_iri, dot_rest));
                }
            }
        }
        slot_idx += 1;
    }
    None
}

/// Executes a resolved `PreparedCallHandle` by marshaling `pos_args` into stack-allocated
/// `BorrowedSliceView`s, invoking `dispatch_prepared_call`, charging `budget`, and decoding
/// `CallerOutputSlice` back to a `Value`.
pub fn execute_prepared_value_call<'g>(
    registry: &ExtensionRegistry<'_>,
    handle: &PreparedCallHandle,
    ctx: &CallExecutionContext,
    gate: Option<&mut (dyn SentinelPolicyGate + 'g)>,
    pos_args: &[Value],
    budget: &mut Budget,
    span: Span,
    path_label: &str,
) -> Result<(Value, DispatchOutcome), Diagnostic> {
    let (_provider, func) = registry
        .resolve_opcode(handle.opcode)
        .map_err(|t| trap_to_diagnostic(t, path_label, span))?;

    // Unpack record-style payload if called via `capability.invoke(id, { ... })`
    let extracted_storage: Vec<Value>;
    let effective_args: &[Value] = if pos_args.len() == 1 && func.params.len() > 1 {
        if let Value::Record(rec) = &pos_args[0] {
            let mut list = Vec::with_capacity(func.params.len());
            for p in func.params {
                if let Some(v) = rec.get(p.name) {
                    list.push(v.clone());
                }
            }
            extracted_storage = list;
            &extracted_storage
        } else {
            pos_args
        }
    } else if pos_args.len() == 1 && func.params.len() == 1 {
        if let Value::Record(rec) = &pos_args[0] {
            if let Some(v) = rec.get(func.params[0].name) {
                extracted_storage = vec![v.clone()];
                &extracted_storage
            } else {
                pos_args
            }
        } else {
            pos_args
        }
    } else {
        pos_args
    };

    // Stack-allocated input scratch (zero heap on hot prepared path)
    let mut input_scratch = [0u8; 512];
    let mut param_offsets = [(0usize, 0usize, PrimitiveKind::U8, 0usize); 8];
    super::scalar_compat::marshal_args_into_scratch(
        func,
        effective_args,
        &mut input_scratch,
        &mut param_offsets,
    )
    .map_err(|t| trap_to_diagnostic(t, path_label, span))?;

    let mut view_storage =
        [BorrowedSliceView::from_raw_bytes(&[], PrimitiveKind::U8, 0).expect("empty view"); 8];
    for idx in 0..func.params.len() {
        let (start, end, kind, elems) = param_offsets[idx];
        view_storage[idx] =
            BorrowedSliceView::from_raw_bytes(&input_scratch[start..end], kind, elems)
                .map_err(|t| trap_to_diagnostic(t, path_label, span))?;
    }

    let mut out_scratch = [0u8; 256];
    let out_elem_size = func.output_kind.elem_size();
    let out_byte_cap = (func.required_output_elems as usize)
        .saturating_mul(out_elem_size)
        .min(out_scratch.len());
    let mut out_slice = CallerOutputSlice::new(&mut out_scratch[..out_byte_cap], func.output_kind)
        .map_err(|t| trap_to_diagnostic(t, path_label, span))?;

    let mut fallback_sim_gate = super::dispatch_adapter::SimulatedPolicyGate::new_for_actor(0);
    fallback_sim_gate.budget.max_scratch_bytes = 1_048_576;
    fallback_sim_gate.budget.max_vram_bytes = 1_048_576;
    let mut sim_ctx = *ctx;
    let (eff_ctx, eff_gate): (
        &CallExecutionContext,
        Option<&mut (dyn SentinelPolicyGate + '_)>,
    ) = if ctx.mode == ExecutionMode::StandaloneDeveloper
        && gate.is_none()
        && !func.effects.is_pure()
    {
        sim_ctx.mode = ExecutionMode::SimulatedGovernanceTest;
        (&sim_ctx, Some(&mut fallback_sim_gate))
    } else {
        (ctx, gate)
    };

    let outcome = dispatch_prepared_call(
        registry,
        handle,
        eff_ctx,
        eff_gate,
        &view_storage[..func.params.len()],
        &mut out_slice,
    )
    .map_err(|t| trap_to_diagnostic(t, path_label, span))?;

    budget.charge(outcome.ticks_consumed, span)?;
    let result_val = decode_output_slice(func, &out_slice)
        .map_err(|t| trap_to_diagnostic(t, path_label, span))?;

    Ok((result_val, outcome))
}

fn decode_output_slice(
    func: &FuncDescriptor,
    out: &CallerOutputSlice<'_>,
) -> Result<Value, VibeTrap> {
    let count = out.written_elems();
    let view =
        BorrowedSliceView::from_raw_bytes(out.written_byte_slice(), func.output_kind, count)?;
    if count == 1 {
        return decode_single_elem(&view, 0);
    }
    let mut items = Vec::with_capacity(count);
    for i in 0..count {
        items.push(decode_single_elem(&view, i)?);
    }
    Ok(Value::List(items))
}

fn decode_single_elem(view: &BorrowedSliceView<'_>, idx: usize) -> Result<Value, VibeTrap> {
    match view.kind() {
        PrimitiveKind::F64 => Ok(Value::F64(view.read_f64_le(idx)?)),
        PrimitiveKind::F32 => Ok(Value::F64(view.read_f32_le(idx)? as f64)),
        PrimitiveKind::I64 => Ok(Value::I64(view.read_i64_le(idx)?)),
        PrimitiveKind::U64 => Ok(Value::U64(view.read_u64_le(idx)?)),
        PrimitiveKind::U32 => Ok(Value::U64(view.read_u32_le(idx)? as u64)),
        PrimitiveKind::Bool => Ok(Value::Bool(view.as_bytes()[idx] != 0)),
        PrimitiveKind::U8 => Ok(Value::U64(view.as_bytes()[idx] as u64)),
        _ => Err(VibeTrap::TypeMismatch),
    }
}

/// Unified host and extension call router used by `eval`, `bytecode`, and `task_scheduler`.
#[allow(clippy::too_many_arguments)]
pub fn route_call<'g, H: Host>(
    host: &mut H,
    registry: &ExtensionRegistry<'_>,
    cache: &mut PreparedCallCache,
    ctx: &CallExecutionContext,
    gate: Option<&mut (dyn SentinelPolicyGate + 'g)>,
    last_outcome: &mut Option<DispatchOutcome>,
    budget: &mut Budget,
    path: &str,
    pos_args: &[Value],
    named_args: &[(String, Value)],
    span: Span,
) -> Result<Value, Diagnostic> {
    let target_profile = match host.environment() {
        crate::bind::HostEnvironment::NativeDesktop => TargetProfileMask::NATIVE_DESKTOP,
        crate::bind::HostEnvironment::WasmSandbox => TargetProfileMask::WASM_SANDBOX,
    };

    // 1. Intercept `capability.invoke` when its target ID is a registered extension path/IRI
    if path == "capability.invoke" {
        if let Some(Value::String(target_id) | Value::Iri(target_id)) = pos_args.first() {
            if let Some(handle) = try_prepare_path(registry, cache, target_id, target_profile)
                .map_err(|t| trap_to_diagnostic(t, target_id, span))?
            {
                let payload_slice = if pos_args.len() > 1 {
                    &pos_args[1..]
                } else {
                    &[]
                };
                let (val, outcome) = execute_prepared_value_call(
                    registry,
                    &handle,
                    ctx,
                    gate,
                    payload_slice,
                    budget,
                    span,
                    target_id,
                )?;
                *last_outcome = Some(outcome);
                return Ok(val);
            }
        }
    }

    // 2. Check if `path` resolves directly to a registered `VibeNamespaceProvider` function
    match try_prepare_path(registry, cache, path, target_profile) {
        Ok(Some(handle)) => {
            let (val, outcome) = execute_prepared_value_call(
                registry, &handle, ctx, gate, pos_args, budget, span, path,
            )?;
            *last_outcome = Some(outcome);
            return Ok(val);
        }
        Ok(None) => {}
        Err(VibeTrap::FunctionNotFound) => {
            if let Some(val) =
                super::scalar_compat::try_eval_convenience_kernel(path, pos_args, budget, span)?
            {
                return Ok(val);
            }
            return Err(trap_to_diagnostic(VibeTrap::FunctionNotFound, path, span));
        }
        Err(t) => return Err(trap_to_diagnostic(t, path, span)),
    }

    // 3. Otherwise route to built-in host capabilities (`math.*`, `graph.*`, `time.*`, etc.).
    // In `GovernedApplication` mode, verify that an attached Sentinel policy gate is alive.
    if ctx.mode == ExecutionMode::GovernedApplication {
        match gate {
            Some(ref g) if g.is_controller_alive() => {}
            _ => {
                return Err(trap_to_diagnostic(
                    VibeTrap::SentinelUnavailable,
                    path,
                    span,
                ))
            }
        }
    }

    native_budget::dispatch(host, path, pos_args, named_args, span, budget)
}
