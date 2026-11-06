//! Effect bitflags (`EffectSet`), determinism and reversibility classifications,
//! bounded resource envelopes (`ResourceEnvelope`), and explicit execution modes (`ExecutionMode`).

/// Composable effect bitflags for extension functions.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectSet(pub u32);

impl EffectSet {
    /// Pure deterministic calculation with zero side effects or state reads.
    pub const PURE_COMPUTE: Self = Self(0);
    /// Reads from the semantic graph or snapshot without mutation.
    pub const GRAPH_READ: Self = Self(1 << 0);
    /// Stages or mutates state in the semantic graph.
    pub const GRAPH_MUTATE: Self = Self(1 << 1);
    /// Interacts with local hardware devices (GPU compute queue, HID, audio, sensors).
    pub const HARDWARE_IO: Self = Self(1 << 2);
    /// Performs external network disclosure or transport publish.
    pub const EXTERNAL_NET: Self = Self(1 << 3);
    /// Accesses cryptographic key vault operations (signing, key generation).
    pub const CRYPTO_VAULT: Self = Self(1 << 4);
    /// Spawns bounded child tasks or DAG nodes.
    pub const TASK_SPAWN: Self = Self(1 << 5);

    /// Combines two `EffectSet` masks.
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Returns `true` if `self` has no side effects (`PURE_COMPUTE`).
    #[inline]
    pub const fn is_pure(self) -> bool {
        self.0 == 0
    }

    /// Returns `true` if `self` contains all effects in `required`.
    #[inline]
    pub const fn contains_all(self, required: Self) -> bool {
        (self.0 & required.0) == required.0
    }

    /// Returns `true` if `self` is a subset of `allowed_mask`.
    #[inline]
    pub const fn is_subset_of(self, allowed_mask: Self) -> bool {
        (self.0 & !allowed_mask.0) == 0
    }
}

/// Determinism guarantee declared by an extension function.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeterminismGuarantee {
    /// Bit-exact across all target architectures (integer/fixed-point/exact hash).
    DeterministicExact = 0,
    /// Deterministic within a bounded IEEE-754 floating-point ULPs / tolerance envelope.
    DeterministicTolerance = 1,
    /// Depends on external clocks, hardware sensors, or network state; requires replay receipt.
    NonDeterministicExternal = 2,
}

/// Reversibility / undo contract for an extension function.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReversibilityKind {
    /// Pure computation; mutates no external or graph state.
    PureNoState = 0,
    /// Mutates staged snapshot state that can be cleanly aborted before commit.
    StagedReversible = 1,
    /// Requires an explicit compensating action to undo after commit.
    CompensatingAction = 2,
    /// Irreversible external disclosure or hardware actuation.
    IrreversibleExternal = 3,
}

/// Synchronous vs bounded-async execution classification.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AsyncExecutionClass {
    /// Completes synchronously within the caller's frame and tick budget.
    SynchronousBounded = 0,
    /// Schedules a bounded asynchronous job monitored by a lease ticket.
    AsyncBoundedTask = 1,
}

/// Worst-case resource budget declared by an extension function and enforced by the Sentinel.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceEnvelope {
    /// Maximum logical instruction / evaluator ticks consumed per invocation.
    pub max_ticks: u64,
    /// Maximum cold/scratch arena bytes required (`0` for Tier-1 `HotZeroHeap`).
    pub max_scratch_bytes: u32,
    /// Maximum GPU VRAM bytes required (`0` for CPU-only calls).
    pub max_vram_bytes: u32,
    /// Maximum output buffer bytes written (`<= CallerOutputSlice` capacity).
    pub max_output_bytes: u32,
    /// Maximum child tasks spawned (`0` for leaf calls).
    pub max_spawned_tasks: u16,
    /// Reserved flags (`0`).
    pub flags: u16,
}

impl ResourceEnvelope {
    /// Default Tier-1 zero-heap compute envelope (`max_scratch_bytes = 0`, `max_vram_bytes = 0`).
    #[inline]
    pub const fn hot_zero_heap(max_ticks: u64, max_output_bytes: u32) -> Self {
        Self {
            max_ticks,
            max_scratch_bytes: 0,
            max_vram_bytes: 0,
            max_output_bytes,
            max_spawned_tasks: 0,
            flags: 0,
        }
    }

    /// Checks whether this envelope fits within `budget`.
    #[inline]
    pub const fn fits_within(&self, budget: &ResourceEnvelope) -> bool {
        self.max_ticks <= budget.max_ticks
            && self.max_scratch_bytes <= budget.max_scratch_bytes
            && self.max_vram_bytes <= budget.max_vram_bytes
            && self.max_output_bytes <= budget.max_output_bytes
            && self.max_spawned_tasks <= budget.max_spawned_tasks
    }
}

/// Target hardware/runtime profiles supported by a provider.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TargetProfileMask(pub u16);

impl TargetProfileMask {
    pub const NATIVE_DESKTOP: Self = Self(1 << 0);
    pub const WASM_SANDBOX: Self = Self(1 << 1);
    pub const EDGE_EMBEDDED: Self = Self(1 << 2);
    pub const GPU_COMPUTE: Self = Self(1 << 3);
    pub const ALL_CPU: Self =
        Self(Self::NATIVE_DESKTOP.0 | Self::WASM_SANDBOX.0 | Self::EDGE_EMBEDDED.0);

    #[inline]
    pub const fn supports(self, target: Self) -> bool {
        (self.0 & target.0) == target.0
    }
}

/// Explicit execution mode governing Sentinel policy enforcement and receipt semantics.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionMode {
    /// Production governed application mode. Requires a live `SentinelPolicyGate`;
    /// missing or disconnected controller fails closed (`VibeTrap::SentinelUnavailable`).
    GovernedApplication = 0,
    /// Standalone developer / CLI / REPL mode. Runs `PureCompute` (and explicitly
    /// mocked local scratch) without claiming governed authority.
    StandaloneDeveloper = 1,
    /// Deterministic in-memory policy table for unit and conformance tests.
    /// Receipts are explicitly marked `simulated = true`.
    SimulatedGovernanceTest = 2,
    /// End-to-end integration test mode wired to the real QualiaDB Webizen Sentinel
    /// and staged `PoetSnapshot` receipt pipeline.
    IntegratedGovernanceTest = 3,
}

impl ExecutionMode {
    /// Returns `true` when a live `SentinelPolicyGate` is mandatory.
    #[inline]
    pub const fn requires_sentinel_gate(self) -> bool {
        matches!(
            self,
            Self::GovernedApplication
                | Self::SimulatedGovernanceTest
                | Self::IntegratedGovernanceTest
        )
    }
}
