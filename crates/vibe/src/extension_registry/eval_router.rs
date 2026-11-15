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
    q_hash_str, AdmissionRequest, BorrowedSliceView, CallerOutputSlice, CompletionStatus,
    EffectSet, ExecutionMode, ExecutionReceiptHeader, FuncDescriptor, PreparedCallHandle,
    PrimitiveKind, RegistryGeneration, ResourceEnvelope, SemVerReq, SentinelPolicyGate,
    StableFuncId, TargetProfileMask, VibeOpCode, VibeTrap,
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
        reg.register(&MECH_PROVIDER)
            .expect("built-in mech provider registration failed");
        reg.register(&GPU_PROVIDER)
            .expect("built-in gpu provider registration failed");
        reg.register(&FORGE_PROVIDER)
            .expect("built-in forge provider registration failed");
        reg.register(&RENDER_PROVIDER)
            .expect("built-in render provider registration failed");
        reg.register(&SIGNAL_PROVIDER)
            .expect("built-in signal provider registration failed");
        reg.register(&SENSE_PROVIDER)
            .expect("built-in sense provider registration failed");
        reg.register(&MODEL_PROVIDER)
            .expect("built-in model provider registration failed");
        reg.register(&NQUIN_PROVIDER)
            .expect("built-in nquin provider registration failed");
        reg.register(&INFER_PROVIDER)
            .expect("built-in infer provider registration failed");
        reg.register(&GRAPH_PROVIDER)
            .expect("built-in qgraph provider registration failed");
        reg.register(&MODALITY_PROVIDER)
            .expect("built-in modality provider registration failed");
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

#[repr(C, align(16))]
struct AlignedScratch<const N: usize>([u8; N]);

const STACK_INPUT_SCRATCH_BYTES: usize = 1024;
const STACK_OUTPUT_SCRATCH_BYTES: usize = 512;
const STACK_PARAM_SLOTS: usize = 16;

fn estimate_input_scratch_bytes(func: &FuncDescriptor, args: &[&Value]) -> usize {
    let mut total = 0usize;
    for (idx, p) in func.params.iter().enumerate() {
        let align = p.kind.elem_align();
        total = (total + align - 1) & !(align - 1);
        let arg_bytes = match args.get(idx) {
            Some(Value::String(s) | Value::Iri(s) | Value::Blank(s))
                if p.kind == PrimitiveKind::Utf8Bytes =>
            {
                s.len()
            }
            Some(Value::List(xs)) => xs.len().saturating_mul(p.kind.elem_size()),
            _ => (p.max_elems as usize).saturating_mul(p.kind.elem_size()),
        };
        total = total.saturating_add(arg_bytes.max(p.kind.elem_size()));
    }
    if func.params.len() == 1 && args.len() > 1 {
        let elem = func.params[0].kind.elem_size();
        total = total.max(args.len().saturating_mul(elem));
    }
    total
}

/// Executes a resolved `PreparedCallHandle` by marshaling `pos_args` into stack-allocated
/// `BorrowedSliceView`s (with descriptor-driven overflow sizing when needed), invoking
/// `dispatch_prepared_call`, charging `budget`, and decoding `CallerOutputSlice` back to a `Value`.
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
    // without shifting positional indices when keys are missing.
    let mut stack_refs: [&Value; STACK_PARAM_SLOTS] = [const { &Value::Null }; STACK_PARAM_SLOTS];
    let mut overflow_refs: Vec<&Value> = Vec::new();
    let effective_args: &[&Value] = if pos_args.len() == 1 {
        if let Value::Record(rec) = &pos_args[0] {
            if func.params.len() == 1
                && !rec.contains_key(func.params[0].name)
                && func.params[0].kind == PrimitiveKind::ResourceHandle
            {
                stack_refs[0] = &pos_args[0];
                &stack_refs[..1]
            } else if func.params.len() <= STACK_PARAM_SLOTS {
                for (idx, p) in func.params.iter().enumerate() {
                    let v = rec.get(p.name).ok_or_else(|| {
                        trap_to_diagnostic(VibeTrap::ArityMismatch, path_label, span)
                    })?;
                    stack_refs[idx] = v;
                }
                &stack_refs[..func.params.len()]
            } else {
                overflow_refs.reserve_exact(func.params.len());
                for p in func.params {
                    let v = rec.get(p.name).ok_or_else(|| {
                        trap_to_diagnostic(VibeTrap::ArityMismatch, path_label, span)
                    })?;
                    overflow_refs.push(v);
                }
                &overflow_refs
            }
        } else {
            stack_refs[0] = &pos_args[0];
            &stack_refs[..1]
        }
    } else if pos_args.len() <= STACK_PARAM_SLOTS {
        for (idx, v) in pos_args.iter().enumerate() {
            stack_refs[idx] = v;
        }
        &stack_refs[..pos_args.len()]
    } else {
        overflow_refs = pos_args.iter().collect();
        &overflow_refs
    };

    // Descriptor-driven input scratch sizing (zero-heap stack path for bounded calls)
    let needed_in = estimate_input_scratch_bytes(func, effective_args);
    let mut stack_in = AlignedScratch([0u8; STACK_INPUT_SCRATCH_BYTES]);
    let mut heap_in: Vec<u8> = Vec::new();
    let input_scratch: &mut [u8] = if needed_in <= STACK_INPUT_SCRATCH_BYTES {
        &mut stack_in.0
    } else {
        heap_in.resize(needed_in, 0);
        &mut heap_in
    };

    let mut stack_offsets = [(0usize, 0usize, PrimitiveKind::U8, 0usize); STACK_PARAM_SLOTS];
    let mut heap_offsets = Vec::new();
    let param_offsets: &mut [(usize, usize, PrimitiveKind, usize)] =
        if func.params.len() <= STACK_PARAM_SLOTS {
            &mut stack_offsets[..func.params.len()]
        } else {
            heap_offsets.resize(func.params.len(), (0, 0, PrimitiveKind::U8, 0));
            &mut heap_offsets
        };

    super::scalar_compat::marshal_args_into_scratch(
        func,
        effective_args,
        input_scratch,
        param_offsets,
    )
    .map_err(|t| trap_to_diagnostic(t, path_label, span))?;

    let empty_view = BorrowedSliceView::from_raw_bytes(&[], PrimitiveKind::U8, 0)
        .expect("empty view");
    let mut stack_views = [empty_view; STACK_PARAM_SLOTS];
    let mut heap_views = Vec::new();
    let view_slice: &mut [BorrowedSliceView<'_>] = if func.params.len() <= STACK_PARAM_SLOTS {
        &mut stack_views[..func.params.len()]
    } else {
        heap_views.resize(func.params.len(), empty_view);
        &mut heap_views
    };
    for idx in 0..func.params.len() {
        let (start, end, kind, elems) = param_offsets[idx];
        view_slice[idx] =
            BorrowedSliceView::from_raw_bytes(&input_scratch[start..end], kind, elems)
                .map_err(|t| trap_to_diagnostic(t, path_label, span))?;
    }

    let out_elem_size = func.output_kind.elem_size();
    let out_byte_cap = (func.required_output_elems as usize)
        .saturating_mul(out_elem_size)
        .max(out_elem_size);
    let mut stack_out = AlignedScratch([0u8; STACK_OUTPUT_SCRATCH_BYTES]);
    let mut heap_out: Vec<u8> = Vec::new();
    let out_buf: &mut [u8] = if out_byte_cap <= STACK_OUTPUT_SCRATCH_BYTES {
        &mut stack_out.0[..out_byte_cap]
    } else {
        heap_out.resize(out_byte_cap, 0);
        &mut heap_out
    };
    let mut out_slice = CallerOutputSlice::new(out_buf, func.output_kind)
        .map_err(|t| trap_to_diagnostic(t, path_label, span))?;

    let outcome = dispatch_prepared_call(
        registry,
        handle,
        ctx,
        gate,
        view_slice,
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
    if func.output_kind == PrimitiveKind::Utf8Bytes {
        let s = view.read_utf8_str()?;
        return Ok(Value::String(s.to_owned()));
    }
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
        PrimitiveKind::I32 => Ok(Value::I64(i64::from(view.read_i32_le(idx)?))),
        PrimitiveKind::U64 => Ok(Value::U64(view.read_u64_le(idx)?)),
        PrimitiveKind::U32 => Ok(Value::U64(view.read_u32_le(idx)? as u64)),
        PrimitiveKind::Bool => Ok(Value::Bool(view.read_bool(idx)?)),
        PrimitiveKind::U8 => Ok(Value::U64(u64::from(view.read_u8(idx)?))),
        PrimitiveKind::NQuin48 => {
            let q = view.read_nquin(idx)?;
            Ok(Value::QuinRef(crate::value::QuinRef::from_raw(
                q.0[0], q.0[1], q.0[2], q.0[3], q.0[4], q.0[5],
            )))
        }
        PrimitiveKind::ResourceHandle => {
            let h = view.read_resource_handle(idx)?;
            let mut rec = std::collections::BTreeMap::new();
            rec.insert(
                "owner_provider_hash".to_string(),
                Value::U64(h.owner_provider_hash),
            );
            rec.insert("kind".to_string(), Value::U64(u64::from(h.kind)));
            rec.insert("flags".to_string(), Value::U64(u64::from(h.flags)));
            rec.insert("generation".to_string(), Value::U64(u64::from(h.generation)));
            rec.insert("slot".to_string(), Value::U64(u64::from(h.slot)));
            rec.insert("reserved".to_string(), Value::U64(u64::from(h.reserved)));
            Ok(Value::Record(rec))
        }
        PrimitiveKind::Utf8Bytes => {
            let s = view.read_utf8_str()?;
            Ok(Value::String(s.to_owned()))
        }
    }
}

/// Unified host and extension call router used by `eval`, `bytecode`, and `task_scheduler`.
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
        Err(t) => return Err(trap_to_diagnostic(t, path, span)),
    }

    // 3. Otherwise route to built-in host capabilities (`math.*`, `graph.*`, `time.*`, etc.).
    match ctx.mode {
        ExecutionMode::StandaloneDeveloper => {
            native_budget::dispatch(host, path, pos_args, named_args, span, budget)
        }
        ExecutionMode::GovernedApplication
        | ExecutionMode::SimulatedGovernanceTest
        | ExecutionMode::IntegratedGovernanceTest => {
            let policy_gate = gate.ok_or_else(|| {
                trap_to_diagnostic(VibeTrap::SentinelUnavailable, path, span)
            })?;
            if !policy_gate.is_controller_alive() {
                return Err(trap_to_diagnostic(
                    VibeTrap::SentinelUnavailable,
                    path,
                    span,
                ));
            }
            let func_hash = q_hash_str(path);
            let native_provider_hash = q_hash_str("vibe:native");
            let native_handle = PreparedCallHandle {
                opcode: VibeOpCode(0),
                registry_generation: registry.generation(),
                reserved: 0,
                provider_iri_hash: native_provider_hash,
                func_iri_hash: func_hash,
                contract_digest: [0u8; 32],
            };
            let req_effects = if path.starts_with("math.") {
                EffectSet::PURE_COMPUTE
            } else if path == "graph.stage" || path == "graph.commit" {
                EffectSet::GRAPH_MUTATE
            } else {
                EffectSet::GRAPH_READ
            };
            let req = AdmissionRequest {
                handle: native_handle,
                actor_did_hash: ctx.actor_did_hash,
                context_world_hash: ctx.context_world_hash,
                lamport_timestamp: ctx.lamport_timestamp,
                mode: ctx.mode,
                requested_effects: req_effects,
                declared_envelope: ResourceEnvelope::hot_zero_heap(1000, 4096),
                input_fingerprint: func_hash ^ (pos_args.len() as u64),
            };
            let ticket = policy_gate
                .admit_call(&req)
                .map_err(|t| trap_to_diagnostic(t, path, span))?;
            if ctx.lamport_timestamp > ticket.expiry_lamport {
                return Err(trap_to_diagnostic(VibeTrap::AuthorityExpired, path, span));
            }
            match native_budget::dispatch(host, path, pos_args, named_args, span, budget) {
                Ok(val) => {
                    policy_gate
                        .validate_post_execution(&ticket, native_handle.opcode, 1, 0, func_hash)
                        .map_err(|t| trap_to_diagnostic(t, path, span))?;
                    let (status, simulated) = match ctx.mode {
                        ExecutionMode::SimulatedGovernanceTest => {
                            (CompletionStatus::SimulatedCommitted, true)
                        }
                        _ => (CompletionStatus::Committed, ticket.simulated),
                    };
                    let receipt = ExecutionReceiptHeader {
                        ticket_id: ticket.ticket_id,
                        actor_did_hash: ctx.actor_did_hash,
                        provider_iri_hash: native_provider_hash,
                        func_iri_hash: func_hash,
                        stable_id: StableFuncId {
                            sub_namespace_id: 0,
                            func_id: 0,
                        },
                        mode: ctx.mode,
                        status,
                        simulated,
                        reserved: [0; 5],
                        policy_generation: ticket.policy_generation,
                        registry_generation: registry.generation(),
                        ticks_consumed: 1,
                        output_bytes_written: 0,
                        trap_code: 0,
                        failure_stage: 0,
                        input_output_fold_hash: func_hash,
                        lamport_timestamp: ctx.lamport_timestamp,
                    };
                    let audit_nquins = policy_gate
                        .commit_receipt(&receipt)
                        .map_err(|t| trap_to_diagnostic(t, path, span))?;
                    *last_outcome = Some(DispatchOutcome {
                        ticks_consumed: 1,
                        output_elems_written: 0,
                        output_bytes_written: 0,
                        receipt,
                        audit_nquins,
                    });
                    Ok(val)
                }
                Err(diag) => {
                    let abort_receipt = ExecutionReceiptHeader {
                        ticket_id: ticket.ticket_id,
                        actor_did_hash: ctx.actor_did_hash,
                        provider_iri_hash: native_provider_hash,
                        func_iri_hash: func_hash,
                        stable_id: StableFuncId {
                            sub_namespace_id: 0,
                            func_id: 0,
                        },
                        mode: ctx.mode,
                        status: CompletionStatus::TrappedExecution,
                        simulated: ticket.simulated
                            || ctx.mode == ExecutionMode::SimulatedGovernanceTest,
                        reserved: [0; 5],
                        policy_generation: ticket.policy_generation,
                        registry_generation: registry.generation(),
                        ticks_consumed: 0,
                        output_bytes_written: 0,
                        trap_code: 0,
                        failure_stage: 4,
                        input_output_fold_hash: func_hash,
                        lamport_timestamp: ctx.lamport_timestamp,
                    };
                    policy_gate
                        .commit_receipt(&abort_receipt)
                        .map_err(|t| trap_to_diagnostic(t, path, span))?;
                    Err(diag)
                }
            }
        }
    }
}

#[cfg(test)]
mod up02_eval_router_tests {
    use super::*;
    use crate::bind::LocalHost;
    use crate::budget::Budget;
    use etalon_vibe::{
        oracle_bioelectric_dipole_preview, oracle_decode_container_summary,
        oracle_dicom_modality_check, oracle_evaluate_directivity_sh,
        oracle_radiative_transfer_slice, oracle_render_binaural_frame,
        oracle_sh_irradiance_probe, oracle_sonify_emf_envelope, oracle_verify_evidence_gate,
    };

    fn eval_route(path: &str, pos_args: &[Value]) -> Result<Value, Diagnostic> {
        let mut host = LocalHost::default();
        let registry = ExtensionRegistry::with_builtin_providers();
        let mut cache = PreparedCallCache::new();
        let actor = q_hash_str("did:q42:local_engine");
        let ctx = CallExecutionContext {
            actor_did_hash: actor,
            context_world_hash: 1,
            lamport_timestamp: 1,
            mode: ExecutionMode::SimulatedGovernanceTest,
        };
        let mut sim_gate = super::super::dispatch_adapter::SimulatedPolicyGate::new_for_actor(actor);
        sim_gate.budget.max_scratch_bytes = 1_048_576;
        sim_gate.budget.max_vram_bytes = 1_048_576;
        let mut last_outcome = None;
        let mut budget = Budget::default();
        route_call(
            &mut host,
            &registry,
            &mut cache,
            &ctx,
            Some(&mut sim_gate),
            &mut last_outcome,
            &mut budget,
            path,
            pos_args,
            &[],
            Span::new(0, 0),
        )
    }

    fn f32_list(vals: &[f32]) -> Value {
        Value::List(vals.iter().map(|&v| Value::F64(f64::from(v))).collect())
    }

    fn f64_list(vals: &[f64]) -> Value {
        Value::List(vals.iter().map(|&v| Value::F64(v)).collect())
    }

    fn u32_list(vals: &[u32]) -> Value {
        Value::List(vals.iter().map(|&v| Value::U64(u64::from(v))).collect())
    }

    fn u64_list(vals: &[u64]) -> Value {
        Value::List(vals.iter().map(|&v| Value::U64(v)).collect())
    }

    fn assert_f32_slice_eq(actual: Value, expected: &[f32]) {
        match actual {
            Value::List(xs) => {
                assert_eq!(xs.len(), expected.len());
                for (a, &e) in xs.iter().zip(expected.iter()) {
                    let av = a.as_f64().expect("expected numeric Value") as f32;
                    assert!(
                        (av - e).abs() <= 1e-6,
                        "mismatch: actual={av}, expected={e}"
                    );
                }
            }
            other => panic!("expected Value::List, got {other:?}"),
        }
    }

    #[test]
    fn up01_render_emf_routes_and_traps_on_invalid_args() {
        let dipole = [1.0f32, 2.0, -0.5];
        let probe = [0.1f32, 0.2, 0.3];
        let gain = 2.0f32;
        let res = eval_route(
            "render.emf.bioelectric_dipole_preview",
            &[f32_list(&dipole), f32_list(&probe), Value::F64(f64::from(gain))],
        )
        .expect("bioelectric_dipole_preview evaluates");
        let exp = oracle_bioelectric_dipole_preview(dipole, probe, gain).unwrap();
        assert_f32_slice_eq(res, &exp);
        assert!(eval_route(
            "render.emf.bioelectric_dipole_preview",
            &[f32_list(&dipole), f32_list(&[0.0, 0.0, 0.0]), Value::F64(1.0)],
        )
        .is_err());
        assert!(eval_route(
            "render.emf.bioelectric_dipole_preview",
            &[f32_list(&[1.0, 2.0]), f32_list(&probe), Value::F64(1.0)],
        )
        .is_err());

        let opt = [0.5f32, 0.8, 0.4];
        let inc = 12.0f32;
        let res = eval_route(
            "render.emf.radiative_transfer_slice",
            &[f32_list(&opt), Value::F64(f64::from(inc))],
        )
        .expect("radiative_transfer_slice evaluates");
        let exp = oracle_radiative_transfer_slice(opt, inc).unwrap();
        assert_f32_slice_eq(res, &exp);
        assert!(eval_route(
            "render.emf.radiative_transfer_slice",
            &[f32_list(&[-0.1, 0.5, 0.0]), Value::F64(1.0)],
        )
        .is_err());

        let sh = [1.0f32, 0.2, -0.1, 0.3];
        let norm = [0.0f32, 1.0, 0.0];
        let res = eval_route(
            "render.emf.sh_irradiance_probe",
            &[f32_list(&sh), f32_list(&norm)],
        )
        .expect("sh_irradiance_probe evaluates");
        let exp = oracle_sh_irradiance_probe(sh, norm).unwrap();
        assert_f32_slice_eq(res, &exp);
        assert!(eval_route(
            "render.emf.sh_irradiance_probe",
            &[f32_list(&sh), f32_list(&[0.0, 0.0, 0.0])],
        )
        .is_err());
    }

    #[test]
    fn up01_signal_qac_routes_and_traps_on_invalid_args() {
        let meta = [4u32, 48_000, 1024, 0x05];
        let res = eval_route("signal.qac.decode_container_summary", &[u32_list(&meta)])
            .expect("decode_container_summary evaluates");
        let exp = oracle_decode_container_summary(meta).unwrap();
        assert_eq!(res, Value::List(vec![Value::U64(exp[0]), Value::U64(exp[1])]));
        assert!(eval_route(
            "signal.qac.decode_container_summary",
            &[u32_list(&[0, 48_000, 1024, 0x05])],
        )
        .is_err());
        assert!(eval_route(
            "signal.qac.decode_container_summary",
            &[u32_list(&[4, 48_000, 1024, 0x10])],
        )
        .is_err());

        let pose = [30.0f32, 10.0, 2.0];
        let gain_lin = 0.75f32;
        let res = eval_route(
            "signal.qac.render_binaural_frame",
            &[f32_list(&pose), Value::F64(f64::from(gain_lin))],
        )
        .expect("render_binaural_frame evaluates");
        let exp = oracle_render_binaural_frame(pose, gain_lin).unwrap();
        assert_f32_slice_eq(res, &exp);
        assert!(eval_route(
            "signal.qac.render_binaural_frame",
            &[f32_list(&[30.0, 10.0, 0.01]), Value::F64(1.0)],
        )
        .is_err());

        let angles = [0.4f32, 0.2];
        let weights = [1.0f32, 0.25, -0.1, 0.15];
        let res = eval_route(
            "signal.qac.evaluate_directivity_sh",
            &[f32_list(&angles), f32_list(&weights)],
        )
        .expect("evaluate_directivity_sh evaluates");
        let exp = oracle_evaluate_directivity_sh(angles, weights).unwrap();
        assert_f32_slice_eq(res, &exp);
        assert!(eval_route(
            "signal.qac.evaluate_directivity_sh",
            &[f32_list(&[0.0, 2.5]), f32_list(&weights)],
        )
        .is_err());

        let emf = [550.0f64, 1.5, 0.8];
        let res = eval_route("signal.qac.sonify_emf_envelope", &[f64_list(&emf)])
            .expect("sonify_emf_envelope evaluates");
        let exp = oracle_sonify_emf_envelope(emf).unwrap();
        match res {
            Value::List(xs) => {
                assert_eq!(xs.len(), 2);
                assert!((xs[0].as_f64().unwrap() - exp[0]).abs() <= 1e-9);
                assert!((xs[1].as_f64().unwrap() - exp[1]).abs() <= 1e-9);
            }
            other => panic!("expected Value::List, got {other:?}"),
        }
        assert!(eval_route(
            "signal.qac.sonify_emf_envelope",
            &[f64_list(&[300.0, 1.5, 0.8])],
        )
        .is_err());
    }

    #[test]
    fn up01_sense_authority_routes_and_traps_on_invalid_args() {
        let gate = [0b11u64, 500, 1000, 0x03];
        let res = eval_route("sense.authority.verify_evidence_gate", &[u64_list(&gate)])
            .expect("verify_evidence_gate evaluates");
        let exp = oracle_verify_evidence_gate(gate).unwrap();
        assert_eq!(res, Value::List(vec![Value::U64(exp[0]), Value::U64(exp[1])]));
        assert!(eval_route(
            "sense.authority.verify_evidence_gate",
            &[u64_list(&[0, 500, 1000, 0x03])],
        )
        .is_err());
        assert!(eval_route(
            "sense.authority.verify_evidence_gate",
            &[u64_list(&[1, 500, 1000, 0x04])],
        )
        .is_err());

        let dicom = [2u32, 12, 1];
        let res = eval_route("sense.authority.dicom_modality_check", &[u32_list(&dicom)])
            .expect("dicom_modality_check evaluates");
        let exp = oracle_dicom_modality_check(dicom).unwrap();
        assert_eq!(
            res,
            Value::List(vec![Value::U64(u64::from(exp[0])), Value::U64(u64::from(exp[1]))])
        );
        assert!(eval_route(
            "sense.authority.dicom_modality_check",
            &[u32_list(&[9, 12, 1])],
        )
        .is_err());
    }
}
