//! Qualia-native **compute universe fabric** (Track B2).
//!
//! This is not a generic GPU scheduler. Universes U0/U1/U2 map to QualiaDB primitives:
//!
//! 1. **Graph–tensor duality** — `NQuin` graph and `Tensor10D` SOA share one resident substrate (U1 pin).
//! 2. **Phase-8 bifurcation** — lock-free SPSC rings between U0 (LLM), Sentinel, and U1 (tensor).
//! 3. **Sentinel governance** — `DenyRollback` on the control ring before the next token matmul.
//! 4. **VramLedger pins** — Full / Eco / Reserve per universe; U0 KV protected in Reserve.
//!
//! One physical `wgpu::Device` (`gpu_context::shared_gpu`); logical parallelism via queues + rings.

use core::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

use crate::gpu_context::{
    ComputeUniverse, OperationalMode, QueueLane, UniverseOrchestrator, VramLedger, VramLedgerSlot,
};
use crate::tensor::buffer_export::{TensorBufferHeader, TENSOR_STRIDE};
use crate::tensor::resident_substrate::{global_resident_substrate, MAX_KNN_HITS};
use crate::tensor::Tensor10D;

/// Qualia engine primitive exploited by a compute universe (not portable DB semantics).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualiaPrimitive {
    /// U0 autoregressive forward + KV (Phase-8 LLM thread).
    Phase8LlmForward = 0,
    /// U1/U2 shared SOA: `NQuin` ↔ `Tensor10D` zero-copy substrate.
    GraphTensorSubstrate = 1,
    /// Sentinel on logit stream; `DenyRollback` on control ring.
    Phase8Sentinel = 2,
    /// Pinned ledger slots per universe (Full / Eco / Reserve).
    VramLedgerPin = 3,
}

impl ComputeUniverse {
    /// Native primitives this universe must use (documentation + static dispatch hooks).
    #[inline]
    pub fn qualia_primitives(self) -> &'static [QualiaPrimitive] {
        match self {
            ComputeUniverse::LlmInference => {
                &[
                    QualiaPrimitive::Phase8LlmForward,
                    QualiaPrimitive::Phase8Sentinel,
                    QualiaPrimitive::VramLedgerPin,
                ]
            }
            ComputeUniverse::Tensor10D => {
                &[
                    QualiaPrimitive::GraphTensorSubstrate,
                    QualiaPrimitive::VramLedgerPin,
                ]
            }
            ComputeUniverse::Viewport => {
                &[
                    QualiaPrimitive::GraphTensorSubstrate,
                    QualiaPrimitive::VramLedgerPin,
                ]
            }
            ComputeUniverse::AcousticPlane => {
                &[
                    QualiaPrimitive::GraphTensorSubstrate,
                    QualiaPrimitive::VramLedgerPin,
                ]
            }
        }
    }

    #[inline]
    pub fn phase8_channels(self) -> &'static [Phase8Channel] {
        match self {
            ComputeUniverse::LlmInference => {
                &[
                    Phase8Channel::LogitUpstream,
                    Phase8Channel::ControlDownstream,
                    Phase8Channel::ContextInject,
                ]
            }
            ComputeUniverse::Tensor10D => &[Phase8Channel::ContextInject],
            ComputeUniverse::Viewport => &[],
            ComputeUniverse::AcousticPlane => &[],
        }
    }
}

/// Phase-8 bifurcation ring (see `llm_agent.rs` LogitStream / ControlStream + U1 inject).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase8Channel {
    /// U0 → Sentinel: per-step logit summary (anomaly / governance).
    LogitUpstream = 0,
    /// Sentinel → U0: `DenyRollback` before next matmul.
    ControlDownstream = 1,
    /// U1 → U0: continuous context from `visit_tensor_search_into` (no stop-and-RAG).
    ContextInject = 2,
}

pub const CONTEXT_INJECT_RING_CAP: usize = 64;
pub const ATTENTION_MASK_WORDS: usize = 256;
/// KV cache bitmask for `fused_attention.wgsl` (1024 context slots).
pub const KV_ATTENTION_MASK_WORDS: usize = 32;
pub const MAX_DRAFT_LEN: usize = 8;
pub const TOPOLOGY_DRAFT_RING_CAP: usize = 4;

/// Lightweight pointer pushed U1→U0 (48 B `NQuin` stays in SOA; ring carries index + hash).
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContextInjectToken {
    pub tensor_index: u32,
    pub subject_hash: u64,
    pub distance: f32,
    pub manifold_w: f32,
}

impl ContextInjectToken {
    pub const fn empty() -> Self {
        Self {
            tensor_index: 0,
            subject_hash: 0,
            distance: 0.0,
            manifold_w: 0.0,
        }
    }
}

impl Default for ContextInjectToken {
    fn default() -> Self {
        Self::empty()
    }
}

/// Fixed-capacity SPSC context ring (producer U1, consumer U0).
pub struct ContextInjectRing {
    slots: UnsafeCell<[ContextInjectToken; CONTEXT_INJECT_RING_CAP]>,
    write_seq: AtomicU32,
    read_seq: AtomicU32,
}

// SAFETY: SPSC — one producer (U1), one consumer (U0); slot published via write_seq Release.
unsafe impl Sync for ContextInjectRing {}

impl ContextInjectRing {
    pub const fn new() -> Self {
        Self {
            slots: UnsafeCell::new([ContextInjectToken::empty(); CONTEXT_INJECT_RING_CAP]),
            write_seq: AtomicU32::new(0),
            read_seq: AtomicU32::new(0),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        let w = self.write_seq.load(Ordering::Acquire);
        let r = self.read_seq.load(Ordering::Acquire);
        (w.wrapping_sub(r)) as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Producer (U1 tensor search). Returns false when full.
    pub fn try_push(&self, token: ContextInjectToken) -> bool {
        let w = self.write_seq.load(Ordering::Relaxed);
        let r = self.read_seq.load(Ordering::Acquire);
        if w.wrapping_sub(r) >= CONTEXT_INJECT_RING_CAP as u32 {
            return false;
        }
        let slot = (w % CONTEXT_INJECT_RING_CAP as u32) as usize;
        unsafe {
            (*self.slots.get())[slot] = token;
        }
        self.write_seq.store(w.wrapping_add(1), Ordering::Release);
        true
    }

    /// Consumer (U0 decode loop). Returns None when empty.
    pub fn try_pop(&self) -> Option<ContextInjectToken> {
        let r = self.read_seq.load(Ordering::Relaxed);
        let w = self.write_seq.load(Ordering::Acquire);
        if r == w {
            return None;
        }
        let slot = (r % CONTEXT_INJECT_RING_CAP as u32) as usize;
        let token = unsafe { (*self.slots.get())[slot] };
        self.read_seq.store(r.wrapping_add(1), Ordering::Release);
        Some(token)
    }
}

/// Resident graph↔tensor SOA visible to U1 (write/search), U0 (read/inject), U2 (render).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GraphTensorSubstrate {
    pub byte_len: u64,
    pub node_count: u32,
    pub stride_bytes: u32,
    pub header_bytes: u32,
}

impl GraphTensorSubstrate {
    #[inline]
    pub fn from_tensor_bytes(resident_bytes: u64) -> Self {
        let header_bytes = std::mem::size_of::<TensorBufferHeader>() as u32;
        let stride = TENSOR_STRIDE as u32;
        if resident_bytes <= header_bytes as u64 {
            return Self {
                byte_len: resident_bytes,
                header_bytes,
                stride_bytes: stride,
                node_count: 0,
            };
        }
        let payload = resident_bytes - header_bytes as u64;
        let node_count = (payload / stride as u64).min(u32::MAX as u64) as u32;
        Self {
            byte_len: resident_bytes,
            node_count,
            stride_bytes: stride,
            header_bytes,
        }
    }

    #[inline]
    pub fn is_resident(&self) -> bool {
        self.node_count > 0
    }
}

/// U1 output: sparse attention routing bitmask for U0 matmul (10D kNN filter).
#[derive(Debug, Clone, Copy)]
pub struct AttentionRouteMask {
    pub words: [u64; ATTENTION_MASK_WORDS],
    pub active_bits: u32,
}

impl Default for AttentionRouteMask {
    fn default() -> Self {
        Self {
            words: [0u64; ATTENTION_MASK_WORDS],
            active_bits: 0,
        }
    }
}

impl AttentionRouteMask {
    #[inline]
    pub fn set_index(&mut self, index: u32) {
        let bit = index as usize;
        let word = bit / 64;
        let offset = bit % 64;
        if word < ATTENTION_MASK_WORDS {
            self.words[word] |= 1u64 << offset;
            self.active_bits = self.active_bits.saturating_add(1);
        }
    }

    #[inline]
    pub fn is_set(&self, index: u32) -> bool {
        let bit = index as usize;
        let word = bit / 64;
        let offset = bit % 64;
        word < ATTENTION_MASK_WORDS && (self.words[word] & (1u64 << offset)) != 0
    }
}

/// Qualia fabric: ledger orchestration + shared substrate + Phase-8 context ring.
pub struct UniverseFabric {
    pub orchestrator: UniverseOrchestrator,
    pub substrate: GraphTensorSubstrate,
}

impl UniverseFabric {
    #[inline]
    pub fn current(ledger: &VramLedger) -> Self {
        let tensor_bytes = ledger.used_in_slot(VramLedgerSlot::Tensor10D);
        Self {
            orchestrator: crate::gpu_context::universe_orchestrator(),
            substrate: GraphTensorSubstrate::from_tensor_bytes(tensor_bytes),
        }
    }

    /// Reserve mode: U2 capped, U0 KV stays Full (LLM wins scheduling).
    #[inline]
    pub fn reserve_protects_llm_kv(&self, global: OperationalMode) -> bool {
        global == OperationalMode::Reserve
            && self.orchestrator.effective_mode(ComputeUniverse::LlmInference, global)
                == OperationalMode::Full
            && self.orchestrator.effective_mode(ComputeUniverse::Viewport, global)
                == OperationalMode::Reserve
    }

    #[inline]
    pub fn queue_for(&self, universe: ComputeUniverse) -> QueueLane {
        self.orchestrator.partition(universe).queue_lane
    }

    #[inline]
    pub fn can_pin_tensor(&self, ledger: &VramLedger, extra_bytes: u64) -> bool {
        ledger.can_allocate_in_universe(
            &self.orchestrator,
            ComputeUniverse::Tensor10D,
            extra_bytes,
        )
    }
}

static CONTEXT_INJECT_RING: ContextInjectRing = ContextInjectRing::new();

#[inline]
pub fn context_inject_ring() -> &'static ContextInjectRing {
    &CONTEXT_INJECT_RING
}

/// Record a tensor kNN hit for async U0 consumption (continuous RAG path).
#[inline]
pub fn push_tensor_context(token: ContextInjectToken) -> bool {
    let ok = context_inject_ring().try_push(token);
    if ok {
        crate::gpu_context::record_logic_flash();
    }
    ok
}

/// U0 decode loop pulls injected graph context (non-blocking).
#[inline]
pub fn pop_tensor_context() -> Option<ContextInjectToken> {
    context_inject_ring().try_pop()
}

// ── U0 decode hints (atomics) ───────────────────────────────────────────────

static DECODE_TOKEN_ID: AtomicU32 = AtomicU32::new(0);
static DECODE_STEP: AtomicU32 = AtomicU32::new(0);
static QUERY_SUBJECT_HASH: AtomicU32 = AtomicU32::new(0); // lower 32 bits; upper in QUERY_SUBJECT_HI
static QUERY_SUBJECT_HI: AtomicU32 = AtomicU32::new(0);
struct SyncUnsafeCell<T>(UnsafeCell<T>);
unsafe impl<T> Sync for SyncUnsafeCell<T> {}

const ZERO_TENSOR10D: Tensor10D = Tensor10D {
    q: 0.0,
    v: 0.0,
    w: 0.0,
    x: 0.0,
    y: 0.0,
    z: 0.0,
    t: 0.0,
    alpha: 0.0,
    mu: 0.0,
    sigma: 0.0,
};

static QUERY_TENSOR_CELL: SyncUnsafeCell<Tensor10D> = SyncUnsafeCell(UnsafeCell::new(ZERO_TENSOR10D));
static QUERY_SEQ: AtomicU32 = AtomicU32::new(0);

static ATTENTION_MASK_CELL: SyncUnsafeCell<AttentionRouteMask> =
    SyncUnsafeCell(UnsafeCell::new(AttentionRouteMask {
        words: [0u64; ATTENTION_MASK_WORDS],
        active_bits: 0,
    }));
static ATTENTION_MASK_SEQ: AtomicU32 = AtomicU32::new(0);

#[cfg(not(target_arch = "wasm32"))]
static PRODUCER_STARTED: AtomicBool = AtomicBool::new(false);
#[cfg(not(target_arch = "wasm32"))]
static PRODUCER_STOP: AtomicBool = AtomicBool::new(false);

/// U0 publishes the active decode position for the U1 producer (non-blocking).
#[inline]
pub fn publish_decode_hint(token_id: u32, step: u32) {
    DECODE_TOKEN_ID.store(token_id, Ordering::Relaxed);
    DECODE_STEP.store(step, Ordering::Release);
}

/// U0 publishes the 10D query anchor U1 should search around.
#[inline]
pub fn publish_query_tensor(tensor: Tensor10D, subject_hash: u64) {
    unsafe {
        *QUERY_TENSOR_CELL.0.get() = tensor;
    }
    QUERY_SUBJECT_HASH.store(subject_hash as u32, Ordering::Relaxed);
    QUERY_SUBJECT_HI.store((subject_hash >> 32) as u32, Ordering::Relaxed);
    QUERY_SEQ.fetch_add(1, Ordering::Release);
}

#[inline]
pub fn query_subject_hash() -> u64 {
    let lo = QUERY_SUBJECT_HASH.load(Ordering::Relaxed) as u64;
    let hi = QUERY_SUBJECT_HI.load(Ordering::Relaxed) as u64;
    (hi << 32) | lo
}

#[inline]
pub fn load_query_tensor() -> Tensor10D {
    unsafe { *QUERY_TENSOR_CELL.0.get() }
}

#[inline]
pub fn attention_route_mask() -> AttentionRouteMask {
    unsafe { *ATTENTION_MASK_CELL.0.get() }
}

#[inline]
pub fn attention_mask_seq() -> u32 {
    ATTENTION_MASK_SEQ.load(Ordering::Acquire)
}

/// Publish U1 route mask snapshot (producer / tests).
#[inline]
pub fn publish_attention_route_mask(mask: AttentionRouteMask) {
    unsafe {
        *ATTENTION_MASK_CELL.0.get() = mask;
    }
    ATTENTION_MASK_SEQ.fetch_add(1, Ordering::Release);
}

#[inline]
fn set_kv_mask_bit(words: &mut [u32; KV_ATTENTION_MASK_WORDS], slot: u32) {
    let bit = slot as usize;
    let word = bit / 32;
    let offset = bit % 32;
    if word < KV_ATTENTION_MASK_WORDS {
        words[word] |= 1u32 << offset;
    }
}

/// Map U1 route mask (tensor indices) → KV slot mask for attention (B3.2b light).
#[inline]
pub fn attention_kv_mask_u32(
    token_idx: u32,
    max_context: u32,
) -> ([u32; KV_ATTENTION_MASK_WORDS], u32) {
    let route = attention_route_mask();
    let mut words = [0u32; KV_ATTENTION_MASK_WORDS];
    let cap = max_context.min((KV_ATTENTION_MASK_WORDS as u32) * 32);
    set_kv_mask_bit(&mut words, token_idx.min(cap.saturating_sub(1)));

    let provenance = crate::tensor::kv_provenance::global_kv_provenance();
    let mut mapped = 1u32;
    for (word_idx, w) in route.words.iter().enumerate() {
        for bit in 0..64 {
            if (*w & (1u64 << bit)) == 0 {
                continue;
            }
            let tensor_idx = (word_idx * 64 + bit) as u32;
            let kv_slot = provenance
                .kv_slot_for_tensor(tensor_idx)
                .unwrap_or(tensor_idx);
            if kv_slot < cap {
                set_kv_mask_bit(&mut words, kv_slot);
                mapped += 1;
            }
        }
    }

    let mask_active = if mapped > 1 && route.active_bits > 0 {
        1u32
    } else {
        0u32
    };
    (words, mask_active)
}

/// B3.2a — kNN hits → sparse attention bitmask (zero-heap).
#[inline]
pub fn build_attention_route_mask(
    query: &Tensor10D,
    max_distance: f32,
    hits: &[usize],
    hit_count: usize,
) -> AttentionRouteMask {
    let mut mask = AttentionRouteMask::default();
    for i in 0..hit_count.min(hits.len()) {
        mask.set_index(hits[i] as u32);
    }
    let _ = (query, max_distance);
    mask
}

/// One U1 producer cycle: search resident SOA, push ring, refresh mask, draft topology.
pub fn run_tensor_search_producer_cycle(max_distance: f32, vocab_len: u32) -> usize {
    let substrate = global_resident_substrate();
    if substrate.node_count() == 0 {
        return 0;
    }

    let query = load_query_tensor();
    let subject_hash = query_subject_hash();
    let mut hits = [0usize; MAX_KNN_HITS];
    // METRIC PARITY (ALGEBRA_MANIFOLD_PLAN.md Phase 4.1, now unified): the GPU shader
    // (shaders/tensor_volume.wgsl) ports `Tensor10D::full_distance` exactly — the metric is
    // chosen by the QUERY's `v` topology class (euclidean / cyclic / hyperbolic / boundary).
    // So this GPU path and the CPU fallback below (which also uses `full_distance`) agree
    // for ALL `v`, not only `v == 0`. `volume_gpu::cpu_tensor_search_into` is the shared,
    // GPU-independent reference for the same metric.
    let hit_count = crate::tensor::volume_gpu::try_gpu_tensor_search_into(&query, max_distance, &mut hits)
        .unwrap_or_else(|| {
            substrate
                .tensor_search_into(&query, max_distance, &mut hits)
                .unwrap_or(0)
        });

    let mask = build_attention_route_mask(&query, max_distance, &hits, hit_count);
    publish_attention_route_mask(mask);

    let pushed = push_tensor_search_hits(&hits[..hit_count], subject_hash, query.w, max_distance);
    let _ = extrapolate_topology_draft(&query, subject_hash, 4, vocab_len);
    pushed
}

#[cfg(not(target_arch = "wasm32"))]
fn tensor_search_producer_loop() {
    crate::platform_scheduler::bind_background_thread();
    let mut idle_spins = 0u32;
    while !PRODUCER_STOP.load(Ordering::Relaxed) {
        let pushed = run_tensor_search_producer_cycle(3.0, 32_000);
        if pushed > 0 {
            crate::gpu_context::record_producer_cycle(pushed as u32);
            idle_spins = 0;
        } else if global_resident_substrate().node_count() == 0 {
            thread::sleep(Duration::from_millis(4));
        } else {
            idle_spins = idle_spins.saturating_add(1);
            if idle_spins > 32 {
                thread::sleep(Duration::from_millis(1));
                idle_spins = 0;
            } else {
                thread::yield_now();
            }
        }
    }
}

/// B3.3a — spawn background U1 tensor search producer (idempotent).
#[cfg(not(target_arch = "wasm32"))]
pub fn start_tensor_search_producer() -> bool {
    if PRODUCER_STARTED.swap(true, Ordering::AcqRel) {
        return false;
    }
    PRODUCER_STOP.store(false, Ordering::Release);
    thread::Builder::new()
        .name("qualia-u1-tensor-producer".into())
        .spawn(tensor_search_producer_loop)
        .expect("U1 tensor search producer thread");
    true
}

#[cfg(not(target_arch = "wasm32"))]
pub fn stop_tensor_search_producer() {
    PRODUCER_STOP.store(true, Ordering::Release);
}

#[cfg(target_arch = "wasm32")]
pub fn start_tensor_search_producer() -> bool {
    false
}

#[cfg(target_arch = "wasm32")]
pub fn stop_tensor_search_producer() {}

/// B2.4 producer hook: push kNN indices from `visit_tensor_search` / `tensor_search_into`.
#[inline]
pub fn push_tensor_search_hits(
    indices: &[usize],
    subject_hash: u64,
    manifold_w: f32,
    max_distance: f32,
) -> usize {
    let mut pushed = 0usize;
    for (rank, &index) in indices.iter().enumerate() {
        if !push_tensor_context(ContextInjectToken {
            tensor_index: index as u32,
            subject_hash,
            distance: max_distance * (1.0 + rank as f32 * 0.01),
            manifold_w,
        }) {
            crate::gpu_context::record_context_ring_drop();
            break;
        }
        pushed += 1;
    }
    pushed
}

// ── B3.1 topological speculative decoding (draft ring scaffold) ───────────────

/// Draft batch pushed U1→U0 for parallel verify (no draft-model weights).
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TopologyDraftBatch {
    pub draft_len: u8,
    pub _pad: [u8; 7],
    pub draft_ids: [u32; MAX_DRAFT_LEN],
    pub concept_hashes: [u64; MAX_DRAFT_LEN],
}

impl TopologyDraftBatch {
    pub const fn empty() -> Self {
        Self {
            draft_len: 0,
            _pad: [0; 7],
            draft_ids: [0; MAX_DRAFT_LEN],
            concept_hashes: [0; MAX_DRAFT_LEN],
        }
    }
}

pub struct TopologyDraftRing {
    slots: UnsafeCell<[TopologyDraftBatch; TOPOLOGY_DRAFT_RING_CAP]>,
    write_seq: AtomicU32,
    read_seq: AtomicU32,
}

unsafe impl Sync for TopologyDraftRing {}

impl TopologyDraftRing {
    pub const fn new() -> Self {
        Self {
            slots: UnsafeCell::new([TopologyDraftBatch::empty(); TOPOLOGY_DRAFT_RING_CAP]),
            write_seq: AtomicU32::new(0),
            read_seq: AtomicU32::new(0),
        }
    }

    pub fn try_push(&self, batch: TopologyDraftBatch) -> bool {
        if batch.draft_len == 0 {
            return false;
        }
        let w = self.write_seq.load(Ordering::Relaxed);
        let r = self.read_seq.load(Ordering::Acquire);
        if w.wrapping_sub(r) >= TOPOLOGY_DRAFT_RING_CAP as u32 {
            return false;
        }
        let slot = (w % TOPOLOGY_DRAFT_RING_CAP as u32) as usize;
        unsafe {
            (*self.slots.get())[slot] = batch;
        }
        self.write_seq.store(w.wrapping_add(1), Ordering::Release);
        true
    }

    pub fn try_pop(&self) -> Option<TopologyDraftBatch> {
        let r = self.read_seq.load(Ordering::Relaxed);
        let w = self.write_seq.load(Ordering::Acquire);
        if r == w {
            return None;
        }
        let slot = (r % TOPOLOGY_DRAFT_RING_CAP as u32) as usize;
        let batch = unsafe { (*self.slots.get())[slot] };
        self.read_seq.store(r.wrapping_add(1), Ordering::Release);
        Some(batch)
    }
}

static TOPOLOGY_DRAFT_RING: TopologyDraftRing = TopologyDraftRing::new();

#[inline]
pub fn topology_draft_ring() -> &'static TopologyDraftRing {
    &TOPOLOGY_DRAFT_RING
}

#[inline]
pub fn pop_topology_draft() -> Option<TopologyDraftBatch> {
    topology_draft_ring().try_pop()
}

/// Phase-8 Sentinel gate for U1→U0 topology drafts (B3.1d polish).
///
/// Mirrors the logit-stream anachronism check: any proposed draft token whose
/// little-endian first byte is `0x99` is rejected before `verify_topology_draft_batch`.
#[inline]
pub fn sentinel_allows_topology_draft(batch: &TopologyDraftBatch) -> bool {
    if batch.draft_len == 0 {
        return false;
    }
    for i in 0..batch.draft_len as usize {
        if batch.draft_ids[i].to_le_bytes()[0] == 0x99 {
            return false;
        }
        if (batch.concept_hashes[i] as u8) == 0x99 {
            return false;
        }
    }
    true
}

/// Extrapolate γ concept hashes from kNN trajectory (B3.1b); optional `TopologyDraftMapper` (B3.1c).
pub fn extrapolate_topology_draft(
    query: &Tensor10D,
    subject_hash: u64,
    gamma: usize,
    vocab_len: u32,
) -> Option<TopologyDraftBatch> {
    #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
    {
        extrapolate_topology_draft_mapped(query, subject_hash, gamma, vocab_len, None)
    }
    #[cfg(all(target_arch = "wasm32", not(feature = "wasm-llm")))]
    {
        extrapolate_topology_draft_mapped(query, subject_hash, gamma, vocab_len)
    }
}

pub fn extrapolate_topology_draft_mapped(
    query: &Tensor10D,
    subject_hash: u64,
    gamma: usize,
    vocab_len: u32,
    #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
    mapper: Option<&crate::topology_draft::TopologyDraftMapper<'_>>,
) -> Option<TopologyDraftBatch> {
    let gamma = gamma.clamp(1, MAX_DRAFT_LEN);
    let substrate = global_resident_substrate();
    if substrate.node_count() == 0 || vocab_len == 0 {
        return None;
    }
    let mut hits = [0usize; MAX_KNN_HITS];
    let hit_count = {
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
        {
            crate::tensor::volume_gpu::try_gpu_tensor_search_into(query, 4.0, &mut hits)
                .unwrap_or_else(|| {
                    substrate
                        .tensor_search_into(query, 4.0, &mut hits)
                        .unwrap_or(0)
                })
        }
        #[cfg(all(target_arch = "wasm32", not(feature = "wasm-llm")))]
        {
            substrate
                .tensor_search_into(query, 4.0, &mut hits)
                .unwrap_or(0)
        }
    };
    if hit_count == 0 {
        return None;
    }

    let mut concepts = [0u64; MAX_DRAFT_LEN];
    for i in 0..gamma.min(hit_count) {
        concepts[i] = substrate.subject_hash_at(hits[i] as u32) ^ subject_hash;
    }
    let batch = {
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
        if let Some(m) = mapper {
            m.fill_draft_batch(&concepts[..gamma.min(hit_count)], gamma.min(hit_count))
        } else {
            let mut batch = TopologyDraftBatch::empty();
            for i in 0..gamma.min(hit_count) {
                batch.concept_hashes[i] = concepts[i];
                batch.draft_ids[i] = (concepts[i] as u32) % vocab_len.max(1);
            }
            batch.draft_len = gamma.min(hit_count) as u8;
            batch
        }
        #[cfg(all(target_arch = "wasm32", not(feature = "wasm-llm")))]
        {
            let mut batch = TopologyDraftBatch::empty();
            for i in 0..gamma.min(hit_count) {
                batch.concept_hashes[i] = concepts[i];
                batch.draft_ids[i] = (concepts[i] as u32) % vocab_len.max(1);
            }
            batch.draft_len = gamma.min(hit_count) as u8;
            batch
        }
    };
    let _ = topology_draft_ring().try_push(batch);
    Some(batch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_ring_push_pop() {
        let ring = ContextInjectRing::new();
        assert!(ring.try_push(ContextInjectToken {
            tensor_index: 1,
            subject_hash: 42,
            distance: 0.5,
            manifold_w: 0.0,
        }));
        let t = ring.try_pop().expect("token");
        assert_eq!(t.tensor_index, 1);
        assert!(ring.is_empty());
    }

    #[test]
    fn substrate_node_count_from_bytes() {
        let header = std::mem::size_of::<TensorBufferHeader>() as u64;
        let sub = GraphTensorSubstrate::from_tensor_bytes(header + 40 * 10);
        assert_eq!(sub.node_count, 10);
    }

    #[test]
    fn u0_has_sentinel_primitive() {
        let prims = ComputeUniverse::LlmInference.qualia_primitives();
        assert!(prims.contains(&QualiaPrimitive::Phase8Sentinel));
    }

    #[test]
    fn attention_mask_sets_bits() {
        let mut mask = AttentionRouteMask::default();
        mask.set_index(100);
        assert!(mask.is_set(100));
        assert!(!mask.is_set(101));
    }

    #[test]
    fn push_tensor_search_hits_drains_to_ring() {
        let pushed = push_tensor_search_hits(&[3, 7], 99, 0.5, 1.0);
        assert_eq!(pushed, 2);
        assert_eq!(pop_tensor_context().unwrap().tensor_index, 3);
        assert_eq!(pop_tensor_context().unwrap().tensor_index, 7);
    }

    #[test]
    fn build_attention_mask_from_hits() {
        let query = Tensor10D::default();
        let mask = build_attention_route_mask(&query, 1.0, &[2, 5, 9], 3);
        assert!(mask.is_set(2));
        assert!(mask.is_set(5));
        assert_eq!(mask.active_bits, 3);
    }

    #[test]
    fn attention_kv_mask_maps_tensor_bits() {
        let mut route = AttentionRouteMask::default();
        route.set_index(5);
        route.set_index(12);
        publish_attention_route_mask(route);
        let (words, active) = attention_kv_mask_u32(20, 1024);
        assert_eq!(active, 1);
        assert_ne!(words[0] & (1u32 << 5), 0);
        assert_ne!(words[0] & (1u32 << 12), 0);
        assert_ne!(words[0] & (1u32 << 20), 0);
    }

    #[test]
    fn topology_draft_ring_roundtrip() {
        let ring = TopologyDraftRing::new();
        let batch = TopologyDraftBatch {
            draft_len: 2,
            _pad: [0; 7],
            draft_ids: [10, 20, 0, 0, 0, 0, 0, 0],
            concept_hashes: [1, 2, 0, 0, 0, 0, 0, 0],
        };
        assert!(ring.try_push(batch));
        let popped = ring.try_pop().unwrap();
        assert_eq!(popped.draft_len, 2);
        assert_eq!(popped.draft_ids[0], 10);
    }

    #[test]
    fn sentinel_rejects_anachronistic_draft_token() {
        let clean = TopologyDraftBatch {
            draft_len: 1,
            _pad: [0; 7],
            draft_ids: [42, 0, 0, 0, 0, 0, 0, 0],
            concept_hashes: [7, 0, 0, 0, 0, 0, 0, 0],
        };
        assert!(sentinel_allows_topology_draft(&clean));

        let bad_id = TopologyDraftBatch {
            draft_len: 1,
            _pad: [0; 7],
            draft_ids: [0x99, 0, 0, 0, 0, 0, 0, 0],
            concept_hashes: [7, 0, 0, 0, 0, 0, 0, 0],
        };
        assert!(!sentinel_allows_topology_draft(&bad_id));

        let bad_hash = TopologyDraftBatch {
            draft_len: 1,
            _pad: [0; 7],
            draft_ids: [42, 0, 0, 0, 0, 0, 0, 0],
            concept_hashes: [0x99, 0, 0, 0, 0, 0, 0, 0],
        };
        assert!(!sentinel_allows_topology_draft(&bad_hash));
    }

    #[test]
    fn producer_cycle_with_global_substrate() {
        let tensors = [
            Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            Tensor10D::new(0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        ];
        global_resident_substrate()
            .load_from_tensors(&tensors, 77)
            .unwrap();
        publish_query_tensor(tensors[0], 77);
        let pushed = run_tensor_search_producer_cycle(2.0, 32_000);
        assert!(pushed >= 1);
        assert!(attention_route_mask().active_bits >= 1);
        while pop_tensor_context().is_some() {}
    }
}