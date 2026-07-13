//! Shared GPU context and VRAM ledger for LLM + tensor + render coexistence.
//!
//! **Compute universes** (Track B2): logical partitions on one physical adapter —
//! pinned ledger slots and queue lanes, not multiple `GPUDevice` instances.
//!
//! Qualia-native bindings (graph–tensor SOA, Phase-8 SPSC, Sentinel, ledger pins)
//! live in `compute_universe.rs` — this module owns VRAM accounting and `shared_gpu()`.
//!
//! Operational modes: **Full**, **Eco**, **Reserve** (no heap in hot-path accounting).

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;

// `caps` reports wgpu adapter capabilities, so it only compiles where the wgpu
// dependency is present: native always, or wasm with the `gpu-runtime` feature.
// Without this gate the module fails the `wasm-logic` build (no wgpu crate).
#[cfg(any(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
mod caps;
#[cfg(any(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
pub(crate) use caps::{experimental_features_allowed, requested_native_llm_features};
#[cfg(not(target_arch = "wasm32"))]
pub use caps::{
    qualia_backend_override, recommend_inference_backend, GpuAdapterCaps, GpuFeatureCaps,
    GpuLimitCaps,
};

/// Desktop / portal operational mode (thermal + VRAM driven).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OperationalMode {
    #[default]
    Full = 0,
    Eco = 1,
    /// Engine-only: inference + queries; viewport particles/bloom off.
    Reserve = 2,
}

impl OperationalMode {
    #[inline]
    pub fn from_pressure(pressure: f32) -> Self {
        if pressure >= 0.92 {
            Self::Reserve
        } else if pressure >= 0.72 {
            Self::Eco
        } else {
            Self::Full
        }
    }

    #[inline]
    pub fn max_particles(self) -> u32 {
        match self {
            Self::Full => 50_000,
            Self::Eco => 8_000,
            Self::Reserve => 0,
        }
    }

    #[inline]
    pub fn bloom_enabled(self) -> bool {
        matches!(self, Self::Full)
    }

    /// Whether this tier can render the full 3D scene. Only [`Self::Full`] does; `Eco`
    /// (VRAM-conservation) and `Reserve` (engine-only) degrade 3D → 2D — the affordability rail.
    /// Single source of the Phase-5 budget rule, shared by `render::authoring` and the portal.
    #[inline]
    pub fn supports_3d(self) -> bool {
        matches!(self, Self::Full)
    }
}

/// Parallel compute plane on shared silicon (maps to 10D **q** / **w** semantics).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComputeUniverse {
    /// U0 — LLM forward pass, KV cache, weight staging.
    LlmInference = 0,
    /// U1 — baked 10D tensor SOA, kNN / spatial filters.
    Tensor10D = 1,
    /// U2 — projector, ambient, bloom.
    Viewport = 2,
    /// U3 — AcousticPlane; read-only pin on U1 SOA + sonic token SPSC (no extra ledger slice yet).
    AcousticPlane = 3,
}

impl ComputeUniverse {
    pub const ALL: [Self; 4] = [
        Self::LlmInference,
        Self::Tensor10D,
        Self::Viewport,
        Self::AcousticPlane,
    ];

    /// Physical ledger partition index (U3 aliases U1 until acoustic sidecar pins land).
    #[inline]
    pub fn partition_index(self) -> usize {
        match self {
            Self::AcousticPlane => Self::Tensor10D as usize,
            u => u as usize,
        }
    }

    #[inline]
    pub fn label(self) -> &'static str {
        match self {
            Self::LlmInference => "U0 LLM",
            Self::Tensor10D => "U1 Tensor10D",
            Self::Viewport => "U2 Viewport",
            Self::AcousticPlane => "U3 AcousticPlane",
        }
    }

    #[inline]
    pub fn default_queue_lane(self) -> QueueLane {
        match self {
            Self::LlmInference => QueueLane::LlmCompute,
            Self::Tensor10D => QueueLane::TensorCompute,
            Self::Viewport | Self::AcousticPlane => QueueLane::ViewportRender,
        }
    }

    #[inline]
    pub fn ledger_slots(self) -> &'static [VramLedgerSlot] {
        match self {
            Self::LlmInference => &[VramLedgerSlot::LlmKvCache, VramLedgerSlot::LlmWeightStaging],
            Self::Tensor10D | Self::AcousticPlane => &[VramLedgerSlot::Tensor10D],
            Self::Viewport => &[VramLedgerSlot::Viewport],
        }
    }
}

/// Pinned VRAM accounting bucket (zero-copy crossover between universes reads, not writes).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VramLedgerSlot {
    LlmKvCache = 0,
    LlmWeightStaging = 1,
    Tensor10D = 2,
    Viewport = 3,
}

/// Preferred async queue on the single `wgpu::Device` (spatial concurrency, not MIG).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueLane {
    LlmCompute = 0,
    TensorCompute = 1,
    ViewportRender = 2,
}

/// Immutable consecutive byte range in the logical VRAM ledger (no overlap between universes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VramByteRange {
    pub offset: u64,
    pub size: u64,
}

impl VramByteRange {
    #[inline]
    pub const fn empty() -> Self {
        Self { offset: 0, size: 0 }
    }

    #[inline]
    pub fn end(&self) -> u64 {
        self.offset.saturating_add(self.size)
    }

    /// True when `[offset, offset+size)` lies entirely inside this partition pin.
    #[inline]
    pub fn contains(&self, alloc_offset: u64, alloc_size: u64) -> bool {
        alloc_size == 0
            || alloc_offset >= self.offset && alloc_offset.saturating_add(alloc_size) <= self.end()
    }
}

/// Hermetic partition: universe ↔ ledger bounds ↔ queue preference.
#[derive(Debug, Clone, Copy)]
pub struct UniversePartition {
    pub universe: ComputeUniverse,
    pub mode: OperationalMode,
    /// Hard cap for this universe (sum of its ledger slots).
    pub vram_budget_bytes: u64,
    /// Pinned consecutive ledger slice — U2 cannot grow into U0's range.
    pub ledger_range: VramByteRange,
    pub queue_lane: QueueLane,
}

/// Orchestrator enforces pinned boundaries on one adapter (Full / Eco / Reserve degrade per universe).
#[derive(Debug, Clone)]
pub struct UniverseOrchestrator {
    pub active_mode: OperationalMode,
    pub partitions: [UniversePartition; 3],
}

impl UniverseOrchestrator {
    /// VRAM split for the active operational mode (consecutive pins, no thrashing).
    #[inline]
    pub fn budget_triplet(total_bytes: u64, mode: OperationalMode) -> (u64, u64, u64) {
        match mode {
            OperationalMode::Reserve => {
                let u2 = total_bytes / 10;
                let rem = total_bytes.saturating_sub(u2);
                let u0 = rem / 2;
                let u1 = rem.saturating_sub(u0);
                (u0, u1, u2)
            }
            OperationalMode::Eco => {
                let u2 = total_bytes / 4;
                let rem = total_bytes.saturating_sub(u2);
                let u0 = rem.saturating_mul(6) / 10;
                let u1 = rem.saturating_sub(u0);
                (u0, u1, u2)
            }
            OperationalMode::Full => {
                let u0 = total_bytes.saturating_mul(55) / 100;
                let u1 = total_bytes.saturating_mul(25) / 100;
                let u2 = total_bytes.saturating_mul(15) / 100;
                (u0, u1, u2)
            }
        }
    }

    fn partition_at_offset(
        universe: ComputeUniverse,
        mode: OperationalMode,
        offset: u64,
        size: u64,
    ) -> UniversePartition {
        UniversePartition {
            universe,
            mode,
            vram_budget_bytes: size,
            ledger_range: VramByteRange { offset, size },
            queue_lane: universe.default_queue_lane(),
        }
    }

    /// Mode-aware ledger partition (Track B2.2).
    pub fn from_total_budget(total_bytes: u64, mode: OperationalMode) -> Self {
        let (u0, u1, u2) = Self::budget_triplet(total_bytes, mode);
        Self {
            active_mode: mode,
            partitions: [
                Self::partition_at_offset(ComputeUniverse::LlmInference, mode, 0, u0),
                Self::partition_at_offset(ComputeUniverse::Tensor10D, mode, u0, u1),
                Self::partition_at_offset(
                    ComputeUniverse::Viewport,
                    mode,
                    u0.saturating_add(u1),
                    u2,
                ),
            ],
        }
    }

    /// Default split at **Full** fidelity: 55% U0 / 25% U1 / 15% U2 (~5% headroom implicit).
    #[inline]
    pub fn from_total_budget_full(total_bytes: u64) -> Self {
        Self::from_total_budget(total_bytes, OperationalMode::Full)
    }

    #[inline]
    pub fn partition(&self, universe: ComputeUniverse) -> &UniversePartition {
        &self.partitions[universe.partition_index()]
    }

    /// Global mode mapped per universe — LLM (U0) wins under pressure.
    #[inline]
    pub fn effective_mode(
        &self,
        universe: ComputeUniverse,
        global: OperationalMode,
    ) -> OperationalMode {
        match global {
            OperationalMode::Full => OperationalMode::Full,
            OperationalMode::Eco => match universe {
                ComputeUniverse::Viewport | ComputeUniverse::AcousticPlane => OperationalMode::Eco,
                _ => OperationalMode::Eco,
            },
            OperationalMode::Reserve => match universe {
                ComputeUniverse::LlmInference => OperationalMode::Full,
                ComputeUniverse::Tensor10D => OperationalMode::Eco,
                ComputeUniverse::Viewport | ComputeUniverse::AcousticPlane => {
                    OperationalMode::Reserve
                }
            },
        }
    }

    #[inline]
    pub fn max_particles(&self, universe: ComputeUniverse, global: OperationalMode) -> u32 {
        self.effective_mode(universe, global).max_particles()
    }

    #[inline]
    pub fn bloom_enabled(&self, universe: ComputeUniverse, global: OperationalMode) -> bool {
        universe == ComputeUniverse::Viewport
            && self.effective_mode(universe, global).bloom_enabled()
    }
}

/// Universe map derived from adapter budget + live operational mode (recomputed; 3 partitions).
#[inline]
pub fn universe_orchestrator() -> UniverseOrchestrator {
    let ledger = global_vram_ledger();
    UniverseOrchestrator::from_total_budget(ledger.budget().max(1), ledger.mode())
}

/// Alias retained for orchestration call sites.
#[inline]
pub fn global_universe_orchestrator() -> UniverseOrchestrator {
    universe_orchestrator()
}

/// U2-effective operational mode from live `VramLedger` pressure.
#[inline]
pub fn viewport_operational_mode() -> OperationalMode {
    let ledger = global_vram_ledger();
    universe_orchestrator().effective_mode(ComputeUniverse::Viewport, ledger.mode())
}

/// Zero-heap ambient draw throttle — static SSBO, dynamic `instance_count` (instant step-down).
#[inline]
pub fn ambient_draw_instances_for_mode(resident: u32, global: OperationalMode) -> u32 {
    let cap = universe_orchestrator()
        .effective_mode(ComputeUniverse::Viewport, global)
        .max_particles();
    resident.min(cap)
}

/// Live ledger hook for per-frame draw throttling (no buffer resize).
#[inline]
pub fn ambient_draw_instances(resident: u32) -> u32 {
    ambient_draw_instances_for_mode(resident, global_vram_ledger().mode())
}

/// Zero-heap VRAM budget tracker (bytes, atomics).
#[derive(Debug, Default)]
pub struct VramLedger {
    budget_bytes: AtomicU64,
    tensor_bytes: AtomicU64,
    kv_cache_bytes: AtomicU64,
    render_bytes: AtomicU64,
    llm_weight_staging_bytes: AtomicU64,
    mode: AtomicU32,
}

impl VramLedger {
    pub const KV_CACHE_CAP_BYTES: u64 = 448 * 1024 * 1024;

    #[inline]
    fn load_slot(&self, slot: VramLedgerSlot) -> u64 {
        match slot {
            VramLedgerSlot::LlmKvCache => self.kv_cache_bytes.load(Ordering::Relaxed),
            VramLedgerSlot::LlmWeightStaging => {
                self.llm_weight_staging_bytes.load(Ordering::Relaxed)
            }
            VramLedgerSlot::Tensor10D => self.tensor_bytes.load(Ordering::Relaxed),
            VramLedgerSlot::Viewport => self.render_bytes.load(Ordering::Relaxed),
        }
    }

    #[inline]
    fn store_slot(&self, slot: VramLedgerSlot, bytes: u64) {
        let bytes = match slot {
            VramLedgerSlot::LlmKvCache => bytes.min(Self::KV_CACHE_CAP_BYTES),
            _ => bytes,
        };
        match slot {
            VramLedgerSlot::LlmKvCache => self.kv_cache_bytes.store(bytes, Ordering::Relaxed),
            VramLedgerSlot::LlmWeightStaging => self
                .llm_weight_staging_bytes
                .store(bytes, Ordering::Relaxed),
            VramLedgerSlot::Tensor10D => self.tensor_bytes.store(bytes, Ordering::Relaxed),
            VramLedgerSlot::Viewport => self.render_bytes.store(bytes, Ordering::Relaxed),
        }
        self.refresh_mode();
    }

    #[inline]
    pub fn budget(&self) -> u64 {
        self.budget_bytes.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn used_in_slot(&self, slot: VramLedgerSlot) -> u64 {
        self.load_slot(slot)
    }

    #[inline]
    pub fn record_slot(&self, slot: VramLedgerSlot, bytes: u64) {
        self.store_slot(slot, bytes);
    }

    #[inline]
    pub fn universe_used_bytes(&self, universe: ComputeUniverse) -> u64 {
        universe
            .ledger_slots()
            .iter()
            .map(|s| self.load_slot(*s))
            .sum()
    }

    #[inline]
    pub fn universe_pressure(
        &self,
        orchestrator: &UniverseOrchestrator,
        universe: ComputeUniverse,
    ) -> f32 {
        let cap = orchestrator.partition(universe).vram_budget_bytes;
        if cap == 0 {
            return 0.0;
        }
        (self.universe_used_bytes(universe) as f32 / cap as f32).clamp(0.0, 1.25)
    }

    #[inline]
    pub fn can_allocate_in_universe(
        &self,
        orchestrator: &UniverseOrchestrator,
        universe: ComputeUniverse,
        extra_bytes: u64,
    ) -> bool {
        let part = orchestrator.partition(universe);
        let used = self.universe_used_bytes(universe);
        if used.saturating_add(extra_bytes) > part.vram_budget_bytes {
            return false;
        }
        part.ledger_range
            .contains(part.ledger_range.offset.saturating_add(used), extra_bytes)
            && self.can_allocate(extra_bytes)
    }

    /// Map a ledger slot to its pinned byte offset inside the adapter ledger.
    #[inline]
    pub fn slot_byte_offset(orchestrator: &UniverseOrchestrator, slot: VramLedgerSlot) -> u64 {
        let universe = match slot {
            VramLedgerSlot::LlmKvCache | VramLedgerSlot::LlmWeightStaging => {
                ComputeUniverse::LlmInference
            }
            VramLedgerSlot::Tensor10D => ComputeUniverse::Tensor10D,
            VramLedgerSlot::Viewport => ComputeUniverse::Viewport,
        };
        let used_before = match slot {
            VramLedgerSlot::LlmWeightStaging => {
                orchestrator
                    .partition(ComputeUniverse::LlmInference)
                    .ledger_range
                    .offset
                    + global_vram_ledger().used_in_slot(VramLedgerSlot::LlmKvCache)
            }
            _ => orchestrator.partition(universe).ledger_range.offset,
        };
        used_before
    }

    #[inline]
    pub fn record_universe(&self, universe: ComputeUniverse, slot: VramLedgerSlot, bytes: u64) {
        debug_assert!(universe.ledger_slots().contains(&slot));
        self.store_slot(slot, bytes);
    }

    #[inline]
    pub fn new(budget_bytes: u64) -> Self {
        Self {
            budget_bytes: AtomicU64::new(budget_bytes),
            ..Default::default()
        }
    }

    #[inline]
    pub fn set_budget(&self, bytes: u64) {
        self.budget_bytes.store(bytes, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_tensor(&self, bytes: u64) {
        self.record_universe(ComputeUniverse::Tensor10D, VramLedgerSlot::Tensor10D, bytes);
    }

    #[inline]
    pub fn record_kv_cache(&self, bytes: u64) {
        self.record_universe(
            ComputeUniverse::LlmInference,
            VramLedgerSlot::LlmKvCache,
            bytes,
        );
    }

    #[inline]
    pub fn record_render(&self, bytes: u64) {
        self.record_universe(ComputeUniverse::Viewport, VramLedgerSlot::Viewport, bytes);
    }

    #[inline]
    pub fn record_llm_staging(&self, bytes: u64) {
        self.record_universe(
            ComputeUniverse::LlmInference,
            VramLedgerSlot::LlmWeightStaging,
            bytes,
        );
    }

    #[inline]
    pub fn used_bytes(&self) -> u64 {
        self.tensor_bytes.load(Ordering::Relaxed)
            + self.kv_cache_bytes.load(Ordering::Relaxed)
            + self.render_bytes.load(Ordering::Relaxed)
            + self.llm_weight_staging_bytes.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn pressure(&self) -> f32 {
        let budget = self.budget_bytes.load(Ordering::Relaxed);
        if budget == 0 {
            return 0.0;
        }
        (self.used_bytes() as f32 / budget as f32).clamp(0.0, 1.25)
    }

    #[inline]
    pub fn mode(&self) -> OperationalMode {
        match self.mode.load(Ordering::Relaxed) {
            1 => OperationalMode::Eco,
            2 => OperationalMode::Reserve,
            _ => OperationalMode::Full,
        }
    }

    #[inline]
    pub fn refresh_mode(&self) {
        let m = OperationalMode::from_pressure(self.pressure());
        self.mode.store(m as u32, Ordering::Relaxed);
    }

    #[inline]
    pub fn can_allocate(&self, extra_bytes: u64) -> bool {
        let budget = self.budget_bytes.load(Ordering::Relaxed);
        self.used_bytes() + extra_bytes <= budget
    }
}

/// Process-wide ledger (lazy static).
static LEDGER: std::sync::OnceLock<VramLedger> = std::sync::OnceLock::new();

#[inline]
pub fn global_vram_ledger() -> &'static VramLedger {
    LEDGER.get_or_init(|| VramLedger::new(6 * 1024 * 1024 * 1024))
}

// ── Ambient telemetry pulses (atomics, zero-heap) ─────────────────────────────

static LLM_HEAT_MILLI: AtomicU32 = AtomicU32::new(0);
static LOGIC_FLASH_MILLI: AtomicU32 = AtomicU32::new(0);
static BAKE_PULSE_MILLI: AtomicU32 = AtomicU32::new(0);
static NETWORK_RIPPLE_MILLI: AtomicU32 = AtomicU32::new(0);
static PRODUCER_CYCLE_MILLI: AtomicU32 = AtomicU32::new(0);
static CONTEXT_RING_DROP_MILLI: AtomicU32 = AtomicU32::new(0);
static DRAFT_ACCEPT_MILLI: AtomicU32 = AtomicU32::new(0);
static DRAFT_LEN_CUR: AtomicU32 = AtomicU32::new(0);

#[inline]
fn pulse(atom: &AtomicU32, strength_milli: u32) {
    let cur = atom.load(Ordering::Relaxed);
    atom.store(cur.max(strength_milli), Ordering::Relaxed);
}

#[inline]
fn sample_decay(atom: &AtomicU32, decay: u32) -> f32 {
    let v = atom.load(Ordering::Relaxed);
    atom.store(v.saturating_sub(decay), Ordering::Relaxed);
    (v as f32 / 1000.0).clamp(0.0, 1.0)
}

/// Called once per autoregressive decode step (gguf_bridge hot loop).
#[inline]
pub fn record_llm_decode_step() {
    pulse(&LLM_HEAT_MILLI, 1000);
}

/// SPARQL / GeoSPARQL / rule resolution flash.
#[inline]
pub fn record_logic_flash() {
    pulse(&LOGIC_FLASH_MILLI, 900);
}

/// Tensor / Quin bake or encode event.
#[inline]
pub fn record_bake_pulse() {
    pulse(&BAKE_PULSE_MILLI, 850);
    global_vram_ledger().refresh_mode();
}

/// Mesh / network I/O ripple (daemon fetch, torrent, etc.).
#[inline]
pub fn record_network_ripple() {
    pulse(&NETWORK_RIPPLE_MILLI, 700);
}

/// U1 background producer completed a kNN inject cycle (B3.3).
#[inline]
pub fn record_producer_cycle(pushed_tokens: u32) {
    let strength = (500 + pushed_tokens.min(16) * 25).min(1000);
    pulse(&PRODUCER_CYCLE_MILLI, strength);
}

/// Context inject ring full — lossy drop rather than stalling U0.
#[inline]
pub fn record_context_ring_drop() {
    pulse(&CONTEXT_RING_DROP_MILLI, 800);
}

/// Topological speculative decode: accepted draft tokens this step (B3.1e).
#[inline]
pub fn record_draft_acceptance(accepted: u32, draft_len: u32) {
    if draft_len > 0 {
        let rate = ((accepted as u64) * 1000 / draft_len as u64).min(1000) as u32;
        DRAFT_LEN_CUR.store(draft_len, Ordering::Relaxed);
        pulse(&DRAFT_ACCEPT_MILLI, rate);
    }
}

/// Portal + desktop ambient field sampling (48 B contract subset).
#[inline]
pub fn sample_ambient_telemetry() -> [f32; 11] {
    let ledger = global_vram_ledger();
    [
        ledger.pressure(),
        sample_decay(&NETWORK_RIPPLE_MILLI, 25),
        sample_decay(&BAKE_PULSE_MILLI, 20),
        sample_decay(&LOGIC_FLASH_MILLI, 35),
        sample_decay(&LLM_HEAT_MILLI, 30),
        sample_decay(&PRODUCER_CYCLE_MILLI, 18),
        sample_decay(&CONTEXT_RING_DROP_MILLI, 22),
        0.08,
        0.25,
        ledger.pressure() * 0.5,
        ledger.mode() as u32 as f32,
    ]
}

// ── Shared wgpu device (native: one device per process) ───────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub struct SharedGpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// The wgpu instance — needed to create surfaces (must be same instance as adapter).
    pub instance: wgpu::Instance,
    /// The wgpu adapter — needed to create surfaces and query capabilities.
    pub adapter: wgpu::Adapter,
    /// Immutable adapter capability snapshot for diagnostics and feature negotiation.
    pub adapter_caps: GpuAdapterCaps,
    /// Feature subset actually requested on the process-wide device.
    pub enabled_features: GpuFeatureCaps,
    /// Whether `TIMESTAMP_QUERY` was negotiated on this device (adapter-dependent).
    /// When false, the LLM GPU profiler degrades to a no-op (CPU wall-clock only).
    pub timestamps_supported: bool,
    /// Nanoseconds per timestamp tick (`Queue::get_timestamp_period`); 0.0 when unsupported.
    pub timestamp_period_ns: f32,
}

#[cfg(not(target_arch = "wasm32"))]
impl SharedGpuContext {
    /// Logical queue lane for universe-tagged dispatch (B2.3).
    /// Single physical `wgpu::Queue` today; lane tags preserve driver scheduling intent.
    #[inline]
    pub fn queue_for_lane(&self, lane: QueueLane) -> &wgpu::Queue {
        let _ = lane;
        &self.queue
    }

    #[inline]
    pub fn queue_for_universe(&self, universe: ComputeUniverse) -> &wgpu::Queue {
        self.queue_for_lane(universe.default_queue_lane())
    }
}

#[cfg(not(target_arch = "wasm32"))]
static SHARED_GPU: OnceLock<SharedGpuContext> = OnceLock::new();

/// Choose the DX12 shader compiler. DX12's legacy FXC compiler cannot compile our flash-attention
/// shader (`fused_attention.wgsl`, X4026) — DXC (the modern compiler) can. Resolution order:
///   1. `QUALIA_DXC_PATH` → `DynamicDxc` at that explicit `dxcompiler.dll` (bespoke override).
///   2. `dxcompiler.dll` beside the current executable (where `build.rs` copies the vendored
///      `vendor/dxc/` DLLs) → `DynamicDxc` at that path (turnkey — no env var needed).
///   3. Otherwise `Auto` (static-DXC → PATH-DXC → FXC) — graceful fallback (Vulkan stays default).
#[cfg(not(target_arch = "wasm32"))]
fn resolve_dx12_compiler() -> wgpu::Dx12Compiler {
    if let Ok(p) = std::env::var("QUALIA_DXC_PATH") {
        if !p.trim().is_empty() {
            log::info!("shared_gpu|dx12_compiler|DynamicDxc(env)={p}");
            return wgpu::Dx12Compiler::DynamicDxc { dxc_path: p };
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let dll = dir.join("dxcompiler.dll");
            if dll.exists() {
                log::info!(
                    "shared_gpu|dx12_compiler|DynamicDxc(vendored)={}",
                    dll.display()
                );
                return wgpu::Dx12Compiler::DynamicDxc {
                    dxc_path: dll.to_string_lossy().into_owned(),
                };
            }
        }
    }
    wgpu::Dx12Compiler::Auto
}

#[cfg(not(target_arch = "wasm32"))]
async fn init_shared_gpu_async() -> Result<SharedGpuContext, String> {
    // Inference-pipeline (GPU backend) selection. Default = wgpu's own pick; `QUALIA_WGPU_BACKEND`
    // pins it (e.g. =vulkan for the vendor-neutral path). The capability checker then reports what
    // was actually selected + the recommendation, so "which pipeline is this machine on" is visible.
    // DX12 shader compiler: the legacy FXC (D3DCompile) compiler CANNOT compile our flash-attention
    // shader (`fused_attention.wgsl` — barriers after a per-thread varying-length SDPA loop; FXC
    // error X4026), which is what the long-mislabelled "DX12 decode deadlock" actually was. DXC (the
    // modern DirectX Shader Compiler) compiles it correctly. wgpu's own default is `Auto`
    // (static-DXC → DXC-on-PATH → FXC), so DX12 silently falls back to FXC unless `dxcompiler.dll`
    // is discoverable. `QUALIA_DXC_PATH` points wgpu straight at a `dxcompiler.dll` (with `dxil.dll`
    // alongside it, for DXIL signing) so DX12 uses DXC without needing it on PATH. Absent the var we
    // keep `Auto` (Vulkan stays the working default backend regardless).
    let dx12_compiler = resolve_dx12_compiler();
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backend_options.dx12.shader_compiler = dx12_compiler;
    if let Some(backends) = caps::qualia_backend_override() {
        log::info!("shared_gpu|backend_override|QUALIA_WGPU_BACKEND={backends:?}");
        desc.backends = backends;
    } else if cfg!(target_os = "windows") {
        // Windows default = DX12. It is the verified-reliable native path: the DXC compiler fix
        // builds the fused-attention shader, and DX12 decodes Q4_K_M / large models (e.g.
        // llama-3.2-3b) that the Vulkan/SPIR-V path currently HANGS on (tracked bug). Vulkan is
        // still the default off-Windows and remains selectable anywhere via QUALIA_WGPU_BACKEND=vulkan.
        desc.backends = wgpu::Backends::DX12;
        log::info!("shared_gpu|backend_default|windows->dx12 (override with QUALIA_WGPU_BACKEND=vulkan)");
    }
    let instance = wgpu::Instance::new(desc);
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })
        .await
        .map_err(|e| format!("Failed to find wgpu adapter: {e}"))?;
    let adapter_caps = GpuAdapterCaps::from_adapter(&adapter);
    log::info!(
        "shared_gpu|adapter|{}|{}",
        adapter_caps.summary_line(),
        adapter_caps.llm_feature_line()
    );
    log::info!(
        "shared_gpu|inference_backend|{}|recommend: {}",
        adapter_caps.backend_label(),
        caps::recommend_inference_backend(&adapter_caps)
    );
    if adapter_caps.is_integrated_gpu()
        && std::env::var("QUALIA_LLM_ALLOW_IGPU").ok().as_deref() != Some("1")
    {
        log::warn!(
            "shared_gpu|adapter|integrated_gpu_selected|set QUALIA_LLM_ALLOW_IGPU=1 to acknowledge this for native LLM runs"
        );
    }

    #[cfg(target_os = "windows")]
    if let Ok(memory) = crate::directml_bridge::probe_best_adapter_memory() {
        let total_local = memory
            .local_budget_bytes
            .max(memory.dedicated_vram_bytes)
            .max(memory.available_local_bytes());
        if total_local > 0 {
            global_vram_ledger().set_budget(total_local);
        }
    }

    // Request only features the adapter advertises, and keep the selector in the caps module so
    // native feature policy stays visible. Today only timestamps are used by default; f16,
    // subgroup, pipeline-cache/statistics, and cooperative matrix are enabled for the optimized
    // native shader variants that follow.
    let required_features = requested_native_llm_features(adapter.features());
    let enabled_features = GpuFeatureCaps::from_features(required_features);
    log::info!(
        "shared_gpu|enabled_features|{}",
        enabled_features.compact_flags()
    );
    if adapter_caps.features.cooperative_matrix && !enabled_features.cooperative_matrix {
        log::info!(
            "shared_gpu|cooperative_matrix_supported_but_disabled|set QUALIA_WGPU_EXPERIMENTAL_FEATURES=1 to request it"
        );
    }
    let ts_supported = enabled_features.timestamp_query;
    // Modern weight tensors blow past the wgpu DEFAULTS (max_buffer_size = 256 MiB,
    // max_storage_buffer_binding_size = 128 MiB): the all-F16 Llama-3.2-3B tied lm_head
    // (token_embd, 3072×128256×2 = 751 MiB) is a single resident buffer that the defaults reject
    // ("Buffer size 788004864 > maximum buffer size 268435456"). Raise both caps to the adapter's
    // reported maximum — always valid for request_device, so this never fails on weaker GPUs (they
    // simply get their own, smaller, max). Vendor-neutral: pure wgpu limits, no CUDA / no extra
    // device feature. Other limits stay at the conservative defaults.
    let adapter_limits = adapter.limits();
    let required_limits = wgpu::Limits {
        max_buffer_size: adapter_limits.max_buffer_size,
        max_storage_buffer_binding_size: adapter_limits.max_storage_buffer_binding_size,
        ..wgpu::Limits::default()
    };
    let experimental_features = if required_features.intersects(
        wgpu::Features::EXPERIMENTAL_COOPERATIVE_MATRIX
            | wgpu::Features::EXPERIMENTAL_RAY_QUERY,
    ) {
        // Safety: experimental capabilities are requested only after intersecting
        // with the selected adapter's advertised feature set. Callers must also
        // explicitly opt in through QUALIA_WGPU_EXPERIMENTAL_FEATURES.
        unsafe { wgpu::ExperimentalFeatures::enabled() }
    } else {
        wgpu::ExperimentalFeatures::disabled()
    };
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features,
            required_limits,
            experimental_features,
            ..Default::default()
        })
        .await
        .map_err(|e| e.to_string())?;

    let timestamp_period_ns = if ts_supported {
        queue.get_timestamp_period()
    } else {
        0.0
    };

    Ok(SharedGpuContext {
        device,
        queue,
        instance,
        adapter,
        adapter_caps,
        enabled_features,
        timestamps_supported: ts_supported,
        timestamp_period_ns,
    })
}

/// Process-wide wgpu device + queue (lazy init, reused by QTensorEngine + render).
#[cfg(not(target_arch = "wasm32"))]
pub fn shared_gpu() -> &'static SharedGpuContext {
    SHARED_GPU.get_or_init(|| {
        let handle = tokio::runtime::Handle::try_current().unwrap_or_else(|_| {
            let rt = Box::leak(Box::new(
                tokio::runtime::Runtime::new().expect("tokio runtime for shared gpu"),
            ));
            rt.handle().clone()
        });
        tokio::task::block_in_place(|| {
            handle
                .block_on(init_shared_gpu_async())
                .expect("shared wgpu init failed")
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reports which GPU backend the engine's shared device actually selected for inference on this
    /// machine (default, or pinned by `QUALIA_WGPU_BACKEND`). Run default vs `QUALIA_WGPU_BACKEND=vulkan`
    /// in separate processes to confirm the override drives the real device.
    #[test]
    #[serial_test::serial(gpu)]
    fn report_inference_backend() {
        if !crate::wgsl_forge::test_gpu_available() {
            return;
        }
        let g = shared_gpu();
        eprintln!(
            "[inference-backend] selected = {} | recommend: {}",
            g.adapter_caps.backend_label(),
            recommend_inference_backend(&g.adapter_caps),
        );
        eprintln!("[inference-backend] {}", g.adapter_caps.summary_line());
        // The device must really exist (a backend was selected and an adapter acquired).
        assert!(!g.adapter_caps.backend_label().is_empty());
    }

    #[test]
    fn pressure_triggers_eco_and_reserve() {
        let ledger = VramLedger::new(1000);
        ledger.record_tensor(500);
        assert_eq!(ledger.mode(), OperationalMode::Full);
        ledger.record_kv_cache(300);
        assert_eq!(ledger.mode(), OperationalMode::Eco);
        ledger.record_render(200);
        assert_eq!(ledger.mode(), OperationalMode::Reserve);
    }

    #[test]
    fn universe_partitions_sum_below_total() {
        let orch = UniverseOrchestrator::from_total_budget_full(10_000);
        let sum: u64 = orch.partitions.iter().map(|p| p.vram_budget_bytes).sum();
        assert!(sum <= 10_000);
        assert_eq!(
            orch.partition(ComputeUniverse::LlmInference)
                .vram_budget_bytes,
            5500
        );
    }

    #[test]
    fn reserve_mode_caps_u2_at_ten_percent() {
        let orch = UniverseOrchestrator::from_total_budget(10_000, OperationalMode::Reserve);
        assert_eq!(
            orch.partition(ComputeUniverse::Viewport).vram_budget_bytes,
            1_000
        );
        assert_eq!(
            orch.partition(ComputeUniverse::LlmInference)
                .vram_budget_bytes,
            4_500
        );
        assert_eq!(orch.active_mode, OperationalMode::Reserve);
    }

    #[test]
    fn ledger_ranges_are_consecutive_non_overlapping() {
        let orch = UniverseOrchestrator::from_total_budget(10_000, OperationalMode::Eco);
        let u0 = orch.partition(ComputeUniverse::LlmInference).ledger_range;
        let u1 = orch.partition(ComputeUniverse::Tensor10D).ledger_range;
        let u2 = orch.partition(ComputeUniverse::Viewport).ledger_range;
        assert_eq!(u0.offset, 0);
        assert_eq!(u1.offset, u0.end());
        assert_eq!(u2.offset, u1.end());
        assert!(u2.end() <= 10_000);
    }

    #[test]
    fn u2_cannot_evict_u0_kv_under_reserve() {
        let orch = UniverseOrchestrator::from_total_budget_full(10_000);
        assert_eq!(
            orch.effective_mode(ComputeUniverse::LlmInference, OperationalMode::Reserve),
            OperationalMode::Full
        );
        assert_eq!(
            orch.effective_mode(ComputeUniverse::Viewport, OperationalMode::Reserve),
            OperationalMode::Reserve
        );
    }

    #[test]
    fn universe_budget_isolated() {
        let ledger = VramLedger::new(10_000);
        let orch = UniverseOrchestrator::from_total_budget_full(10_000);
        ledger.record_render(2000);
        assert!(ledger.can_allocate_in_universe(&orch, ComputeUniverse::LlmInference, 5000));
        assert!(!ledger.can_allocate_in_universe(&orch, ComputeUniverse::Viewport, 2000));
    }

    #[test]
    fn ambient_draw_instant_step_by_mode() {
        let orch =
            UniverseOrchestrator::from_total_budget(6 * 1024 * 1024 * 1024, OperationalMode::Full);
        assert_eq!(
            orch.max_particles(ComputeUniverse::Viewport, OperationalMode::Full),
            50_000
        );
        assert_eq!(
            orch.max_particles(ComputeUniverse::Viewport, OperationalMode::Eco),
            8_000
        );
        assert_eq!(
            orch.max_particles(ComputeUniverse::Viewport, OperationalMode::Reserve),
            0
        );

        let resident = 50_000_u32;
        assert_eq!(
            ambient_draw_instances_for_mode(resident, OperationalMode::Full),
            50_000
        );
        assert_eq!(
            ambient_draw_instances_for_mode(resident, OperationalMode::Eco),
            8_000
        );
        assert_eq!(
            ambient_draw_instances_for_mode(resident, OperationalMode::Reserve),
            0
        );
        assert_eq!(
            ambient_draw_instances_for_mode(3_000, OperationalMode::Eco),
            3_000
        );
    }
}
