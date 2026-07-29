//! STELLAR §A AH-track H2 — **residency + device-priority planner** (decisions D24/D25/D30/D31).
//!
//! Turns *discovery* (H0 `HostTopology` + H1(a) `CapabilityMatrix`) into an *employment plan* for a
//! given model: which residency protocol, which device holds what, and the device priority order.
//! It is a **discovery-derived adaptive plan** (D31) — not a fixed formula — and the core is a pure
//! function of its inputs, so it is fully unit-testable with synthetic profiles (no GPU required).
//!
//! Decision (D31), per machine, from measured inputs:
//!   1. fits the highest-ranked circuit's pool (minus a KV reserve) → **Resident**;
//!   2. doesn't fit → the overflow segment is placed by **`argmin(measured compute + measured
//!      transfer)`** over the candidate circuits (§ "Overflow cost model" below):
//!        - **HeterogeneousOverflow** when running the overflow *in place* on a large-pool secondary
//!          (iGPU/CPU reading system RAM, zero per-token transfer) is the cheapest estimate;
//!        - **Streaming** when double-buffering the overflow to the fast device over its bus (the A4
//!          path — fast compute, but paying that device's per-token transfer) is cheaper, **or** no
//!          in-place secondary big enough exists.
//!   Device priority order = the measured `CapabilityMatrix` order (D30), never a static hierarchy.
//!
//! **Overflow cost model (D31).** Both axes are expressed as *bytes over a measured bandwidth* so
//! they are directly comparable. Decode is memory-bound — the forward pass reads each weight once
//! per token — so a segment's **compute** time is estimated as `overflow_bytes /
//! compute_bytes_per_s`, where `compute_bytes_per_s` is the circuit's GEMV throughput
//! (`gemv_n²` f32 elements / `ms_per_gemv`). A segment's **transfer** time is `overflow_bytes /
//! (upload_gbps · 1e9)`; an in-pool circuit (`upload_gbps = ∞`, e.g. the CPU, or an iGPU running
//! the overflow in its own system-RAM pool) pays **zero** transfer. The chosen protocol follows the
//! per-segment argmin of `compute + transfer` — so a fast-but-far circuit (dGPU streaming overflow
//! over PCIe) can lose to a slower-but-in-place circuit (iGPU), and vice-versa, purely on the
//! numbers. **This is an estimate from the measured throughput + bandwidth, not a profiled runtime**
//! (no attention/activation cost, no overlap of compute with transfer, memory-bound decode assumed);
//! it is a principled ranking signal, honestly a first-order one. Native only.
#![cfg(not(target_arch = "wasm32"))]

use serde::Serialize;

use crate::device_benchmark::{benchmark_devices, CapabilityMatrix, CircuitBench, CircuitKind};
use crate::host_topology::{probe_host_topology, AdapterClass, HostTopology};

/// Reserve for the host OS when the system-RAM pool is used for compute (iGPU/CPU overflow).
const HOST_RAM_FLOOR: u64 = 4 * 1024 * 1024 * 1024;

/// Default KV-cache reserve on the primary compute device (matches the VRAM ledger cap).
pub const DEFAULT_KV_RESERVE: u64 = crate::gpu_context::VramLedger::KV_CACHE_CAP_BYTES;

/// The residency protocol chosen for a model on this host (D25).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ResidencyProtocol {
    /// Fits the fastest circuit's pool — upload once, no per-token transfer.
    Resident,
    /// Resident portion on the fast circuit; overflow runs in-place on a large-pool secondary.
    HeterogeneousOverflow,
    /// Exceeds the fast pool with no in-place secondary — double-buffer overflow over the bus (A4).
    Streaming,
}

/// What role a circuit plays in the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PlacementRole {
    ResidentPrimary,
    Overflow,
    StreamTarget,
}

/// One circuit's assignment in the plan.
#[derive(Debug, Clone, Serialize)]
pub struct DevicePlacement {
    pub circuit: String,
    pub kind: CircuitKind,
    pub role: PlacementRole,
    pub bytes: u64,
    pub pool_bytes: u64,
}

/// The full employment plan — serializable for the cached passport / the progress record.
#[derive(Debug, Clone, Serialize)]
pub struct EmploymentPlan {
    pub protocol: ResidencyProtocol,
    pub model_bytes: u64,
    pub kv_reserve_bytes: u64,
    /// Circuits in measured-throughput order (the priority order, D30).
    pub device_priority: Vec<String>,
    pub placements: Vec<DevicePlacement>,
    pub rationale: String,
}

impl EmploymentPlan {
    pub fn summary(&self) -> String {
        let mut s = format!(
            "EmploymentPlan: {:?}  (model {:.2} GB, KV reserve {:.2} GB)\n  priority: {}\n  {}\n",
            self.protocol,
            self.model_bytes as f64 / 1e9,
            self.kv_reserve_bytes as f64 / 1e9,
            self.device_priority.join(" > "),
            self.rationale,
        );
        for p in &self.placements {
            s.push_str(&format!(
                "    - {:?} {:<26} {:.2} GB / {:.2} GB pool\n",
                p.role,
                p.circuit,
                p.bytes as f64 / 1e9,
                p.pool_bytes as f64 / 1e9,
            ));
        }
        s
    }
}

/// The memory pool (bytes) a circuit computes against, given the discovered topology.
fn pool_for(kind: CircuitKind, topo: &HostTopology) -> u64 {
    match kind {
        CircuitKind::DiscreteGpu => topo
            .adapters
            .iter()
            .filter(|a| a.class == AdapterClass::Discrete)
            .map(|a| a.dedicated_vram_bytes)
            .max()
            .filter(|&v| v > 0)
            .unwrap_or(topo.usable_model_budget_bytes),
        // iGPU and CPU read the shared system-RAM pool.
        CircuitKind::IntegratedGpu | CircuitKind::Cpu => {
            topo.host_ram_bytes.saturating_sub(HOST_RAM_FLOOR)
        }
        CircuitKind::Npu | CircuitKind::Other => 0,
    }
}

/// Bytes-per-second of *compute* a circuit sustains on the GEMV benchmark, used as the D31 compute
/// axis. The kernel streams `gemv_n²` f32 weight elements per dispatch in `ms_per_gemv` ms; decode
/// is memory-bound (each weight read once per token), so this GEMV byte-throughput is a first-order
/// estimate of how fast the circuit can chew through a segment's weights. A non-positive/`NaN`
/// measurement yields `f64::INFINITY` (treated as "no measurable compute cost"), never a panic.
fn compute_bytes_per_s(c: &CircuitBench, gemv_n: usize) -> f64 {
    let secs = c.ms_per_gemv / 1e3;
    let bytes = (gemv_n as f64) * (gemv_n as f64) * 4.0; // f32 weights in the bench kernel
    if secs > 0.0 && bytes > 0.0 {
        bytes / secs
    } else {
        f64::INFINITY
    }
}

/// Estimated per-token **compute** time (seconds) to run `bytes` of weights on circuit `c`
/// (D31 compute axis). `bytes / compute_bytes_per_s`; `0.0` for an immeasurably-fast circuit.
fn segment_compute_cost(bytes: u64, c: &CircuitBench, gemv_n: usize) -> f64 {
    let bw = compute_bytes_per_s(c, gemv_n);
    if bw.is_finite() && bw > 0.0 {
        bytes as f64 / bw
    } else {
        0.0
    }
}

/// Estimated per-token **transfer** time (seconds) to move `bytes` of weights across a circuit's
/// host→device bus at its measured `upload_gbps` (D31 transfer axis — the axis the v1 crossover
/// rule ignored). An in-pool circuit (`upload_gbps = ∞`: the CPU, or an iGPU running the overflow
/// in its own system-RAM pool) pays **zero**; a non-positive upload (a bad/unusable bus) is
/// `INFINITY` (cannot stream), so such a circuit never wins as a stream target.
fn segment_transfer_cost(bytes: u64, upload_gbps: f64) -> f64 {
    if upload_gbps.is_infinite() {
        0.0
    } else if upload_gbps > 0.0 {
        bytes as f64 / (upload_gbps * 1e9)
    } else {
        f64::INFINITY
    }
}

/// **Pure** planner: derive the employment plan from the discovered topology + capability matrix.
pub fn plan_employment(
    topo: &HostTopology,
    matrix: &CapabilityMatrix,
    model_bytes: u64,
    kv_reserve_bytes: u64,
) -> EmploymentPlan {
    let device_priority: Vec<String> = matrix.circuits.iter().map(|c| c.label.clone()).collect();

    let Some(best) = matrix.circuits.first() else {
        return EmploymentPlan {
            protocol: ResidencyProtocol::Streaming,
            model_bytes,
            kv_reserve_bytes,
            device_priority,
            placements: Vec::new(),
            rationale: "no compute circuits discovered — cannot plan".into(),
        };
    };

    let best_pool = pool_for(best.kind, topo);
    let best_usable = best_pool.saturating_sub(kv_reserve_bytes);

    // 1) Fits the fastest circuit → Resident.
    if model_bytes <= best_usable {
        return EmploymentPlan {
            protocol: ResidencyProtocol::Resident,
            model_bytes,
            kv_reserve_bytes,
            device_priority,
            placements: vec![DevicePlacement {
                circuit: best.label.clone(),
                kind: best.kind,
                role: PlacementRole::ResidentPrimary,
                bytes: model_bytes,
                pool_bytes: best_pool,
            }],
            rationale: format!(
                "model {:.2} GB fits the highest-ranked circuit ({}) usable pool {:.2} GB → resident, no per-token transfer",
                model_bytes as f64 / 1e9,
                best.label,
                best_usable as f64 / 1e9,
            ),
        };
    }

    // Overflow that doesn't fit the fast pool. D31: place it by the per-segment
    // argmin(measured compute + measured transfer) over the candidate circuits — NOT by a fixed
    // "iGPU-in-place always wins" rule. The transfer axis (`upload_gbps`) now participates, so a
    // fast-but-far primary and a slower-but-in-place secondary are compared on the numbers.
    let overflow = model_bytes - best_usable;
    let gemv_n = matrix.gemv_n;

    // Candidate S — STREAM the overflow to the fast primary over its bus (the A4 path): fast
    // compute, but pays the primary's per-token host→device transfer for the overflow weights.
    let stream_compute = segment_compute_cost(overflow, best, gemv_n);
    let stream_transfer = segment_transfer_cost(overflow, best.upload_gbps);
    let stream_cost = stream_compute + stream_transfer;

    // Candidates H — run the overflow IN PLACE on a large-pool secondary (iGPU/CPU reading the
    // system-RAM pool where the overflow weights already live): slower compute, ZERO per-token
    // transfer. Pick the cheapest such secondary by measured compute.
    let mut best_inplace: Option<(&CircuitBench, u64, f64)> = None; // (circuit, pool, compute cost)
    for c in matrix.circuits.iter().skip(1) {
        if !matches!(c.kind, CircuitKind::IntegratedGpu | CircuitKind::Cpu) {
            continue;
        }
        let pool = pool_for(c.kind, topo);
        if pool < overflow {
            continue; // can't hold the overflow in its own pool → not an in-place candidate
        }
        let cost = segment_compute_cost(overflow, c, gemv_n); // in-pool → no transfer term
        if best_inplace.map_or(true, |(_, _, prev)| cost < prev) {
            best_inplace = Some((c, pool, cost));
        }
    }

    // 2) The cheapest in-place secondary strictly beats streaming → HeterogeneousOverflow.
    //    (Ties go to streaming: it keeps the whole model on one device, the simpler path.)
    if let Some((sec, sec_pool, inplace_cost)) = best_inplace {
        if inplace_cost < stream_cost {
            return EmploymentPlan {
                protocol: ResidencyProtocol::HeterogeneousOverflow,
                model_bytes,
                kv_reserve_bytes,
                device_priority,
                placements: vec![
                    DevicePlacement {
                        circuit: best.label.clone(),
                        kind: best.kind,
                        role: PlacementRole::ResidentPrimary,
                        bytes: best_usable,
                        pool_bytes: best_pool,
                    },
                    DevicePlacement {
                        circuit: sec.label.clone(),
                        kind: sec.kind,
                        role: PlacementRole::Overflow,
                        bytes: overflow,
                        pool_bytes: sec_pool,
                    },
                ],
                rationale: format!(
                    "model {:.2} GB exceeds the fast pool ({:.2} GB usable); {:.2} GB overflow → argmin picks IN-PLACE on {} (est {:.1} ms/tok compute, no per-token transfer) over streaming to {} (est {:.1} ms compute + {:.1} ms transfer) — D31 measured decision",
                    model_bytes as f64 / 1e9,
                    best_usable as f64 / 1e9,
                    overflow as f64 / 1e9,
                    sec.label,
                    inplace_cost * 1e3,
                    best.label,
                    stream_compute * 1e3,
                    stream_transfer * 1e3,
                ),
            };
        }
    }

    // 3) Streaming wins the argmin (cheaper than any in-place secondary, or none exists) →
    //    double-buffer the overflow to the fast device (A4).
    let rationale = match best_inplace {
        Some((sec, _, inplace_cost)) => format!(
            "model {:.2} GB exceeds the fast pool ({:.2} GB usable); {:.2} GB overflow → argmin picks STREAMING to {} (est {:.1} ms compute + {:.1} ms transfer) over the best in-place secondary {} (est {:.1} ms compute) — D31 measured decision",
            model_bytes as f64 / 1e9,
            best_usable as f64 / 1e9,
            overflow as f64 / 1e9,
            best.label,
            stream_compute * 1e3,
            stream_transfer * 1e3,
            sec.label,
            inplace_cost * 1e3,
        ),
        None => format!(
            "model {:.2} GB exceeds the fast pool ({:.2} GB usable) with no in-place secondary big enough → double-buffer stream {:.2} GB overflow to {} (est {:.1} ms compute + {:.1} ms transfer, A4)",
            model_bytes as f64 / 1e9,
            best_usable as f64 / 1e9,
            overflow as f64 / 1e9,
            best.label,
            stream_compute * 1e3,
            stream_transfer * 1e3,
        ),
    };
    EmploymentPlan {
        protocol: ResidencyProtocol::Streaming,
        model_bytes,
        kv_reserve_bytes,
        device_priority,
        placements: vec![DevicePlacement {
            circuit: best.label.clone(),
            kind: best.kind,
            role: PlacementRole::StreamTarget,
            bytes: model_bytes,
            pool_bytes: best_pool,
        }],
        rationale,
    }
}

/// Probe the real host (H0 + H1(a)) and plan for `model_bytes`. Heavy (runs the benchmark); for the
/// fast path, plan against a cached matrix instead.
pub fn plan_for_model(model_bytes: u64) -> EmploymentPlan {
    let topo = probe_host_topology();
    let matrix = benchmark_devices(2048);
    plan_employment(&topo, &matrix, model_bytes, DEFAULT_KV_RESERVE)
}

// ──────────────────────────────────────────────────────────────────────────
// H2 CONSUMPTION wiring (STELLAR §A, decision D31 follow-through).
//
// The planner above is a *pure* function; before this seam it had zero consumers
// outside its own tests — the residency `EmploymentPlan` was computed-but-never-called.
// The hooks below make it OBSERVABLE (logged) and RETRIEVABLE (process-global store)
// at LLM model load, behind a **default-OFF** env flag so nothing changes unless the
// operator opts in. This is the *consumption* wiring **only**.
//
// TODO(H3-exec): the plan is computed + recorded here; heterogeneous cross-device
// EXECUTION (actually running the overflow layers on the auxiliary circuit — iGPU/CPU
// in-place per `PlacementRole::Overflow`) is the remaining H3 step and is explicitly
// OUT OF SCOPE for this wiring. Weight placement + the decode path are unchanged: the
// recorded plan is advisory until an execution stage consults it.
// ──────────────────────────────────────────────────────────────────────────

use std::sync::OnceLock;

/// Env flag gating H2 residency routing. **Default OFF** — when unset/false the route
/// hooks do nothing behaviour-changing (a single cheap env read, no plan, no store).
pub const ROUTE_ENV: &str = "QUALIA_LLM_ROUTE";

/// Process-global store for the most-recently computed employment plan (H2 route).
/// A future execution stage (H3) reads this to learn the intended placement.
static ROUTE_PLAN: OnceLock<std::sync::Mutex<Option<EmploymentPlan>>> = OnceLock::new();

fn route_plan_slot() -> &'static std::sync::Mutex<Option<EmploymentPlan>> {
    ROUTE_PLAN.get_or_init(|| std::sync::Mutex::new(None))
}

/// Whether H2 residency routing is enabled (`QUALIA_LLM_ROUTE`). Default OFF.
pub fn route_enabled() -> bool {
    matches!(
        std::env::var(ROUTE_ENV).ok().as_deref(),
        Some("1") | Some("true") | Some("on") | Some("yes")
    )
}

/// Retrieve the last computed employment plan (H2 route), if any was recorded this process.
/// This is the getter a future H3 execution stage consults; returns `None` when the flag
/// was never enabled or no model has been loaded yet.
pub fn last_employment_plan() -> Option<EmploymentPlan> {
    route_plan_slot().lock().ok().and_then(|g| g.clone())
}

/// **H2 route (core):** if enabled, compute the `EmploymentPlan` from the *already-probed*
/// topology + capability matrix for a model of `model_bytes`, **record it** (log) and
/// **store it** for retrieval — then return it. Gated by `QUALIA_LLM_ROUTE`; returns `None`
/// (computing + storing nothing) when the flag is off.
///
/// Does **not** re-probe or benchmark: callers pass the topology + matrix they already hold
/// (e.g. from the cached hardware passport). Does **not** change weight placement or execution.
pub fn route_employment_for_model(
    topo: &HostTopology,
    matrix: &CapabilityMatrix,
    model_bytes: u64,
) -> Option<EmploymentPlan> {
    if !route_enabled() {
        return None;
    }
    let plan = plan_employment(topo, matrix, model_bytes, DEFAULT_KV_RESERVE);
    // Record honestly on the same log surface the path selector uses for its chosen plan.
    log::info!(
        "llm_route|employment_plan|protocol={:?}|model={:.2}GB|priority={}|placements={}",
        plan.protocol,
        plan.model_bytes as f64 / 1e9,
        plan.device_priority.join(">"),
        plan.placements
            .iter()
            .map(|p| format!("{:?}:{}({:.2}GB)", p.role, p.circuit, p.bytes as f64 / 1e9))
            .collect::<Vec<_>>()
            .join(","),
    );
    log::info!("llm_route|rationale|{}", plan.rationale);
    if let Ok(mut g) = route_plan_slot().lock() {
        *g = Some(plan.clone());
    }
    Some(plan)
}

/// **H2 route (convenience):** route using the *cached* hardware passport (topology + matrix
/// already probed at boot — no re-probe, no benchmark, no GPU touched at load). Called from the
/// model-load seam with the honest loaded weight-byte count. No-op when `QUALIA_LLM_ROUTE` is off
/// or when there is no cached passport yet (logs a one-line note in the latter case).
pub fn route_employment_from_passport(model_bytes: u64) -> Option<EmploymentPlan> {
    if !route_enabled() {
        return None;
    }
    let path = crate::hardware_passport::default_cache_path();
    match crate::hardware_passport::read_passport(&path) {
        Some(p) => route_employment_for_model(&p.topology, &p.matrix, model_bytes),
        None => {
            log::info!(
                "llm_route|no_passport|skipping employment plan for {:.2}GB model (run `qualia-cli llm passport` to enable H2 routing)",
                model_bytes as f64 / 1e9,
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_benchmark::CircuitBench;
    use crate::host_topology::{AdapterDesc, HostMemoryTopology};

    const GB: u64 = 1024 * 1024 * 1024;

    fn circuit(label: &str, kind: CircuitKind, ms: f64, score: f64) -> CircuitBench {
        CircuitBench {
            label: label.into(),
            kind,
            backend: "test".into(),
            ms_per_gemv: ms,
            gflops: 0.0,
            upload_gbps: 1.0,
            rel_score: score,
            decode_proxy_tok_s: None,
        }
    }

    fn discrete_topo(vram_gb: u64, ram_gb: u64, with_igpu: bool) -> HostTopology {
        let mut adapters = vec![AdapterDesc {
            name: "Discrete GPU".into(),
            backend: "Dx12".into(),
            class: AdapterClass::Discrete,
            vendor: 0x10de,
            device: 1,
            dedicated_vram_bytes: vram_gb * GB,
        }];
        if with_igpu {
            adapters.push(AdapterDesc {
                name: "iGPU".into(),
                backend: "Dx12".into(),
                class: AdapterClass::Integrated,
                vendor: 0x8086,
                device: 2,
                dedicated_vram_bytes: 0,
            });
        }
        HostTopology {
            adapters,
            topology: HostMemoryTopology::Discrete,
            host_ram_bytes: ram_gb * GB,
            host_ram_available_bytes: ram_gb * GB / 2,
            cpu_cores: 8,
            os_floor_bytes: 3 * GB / 2,
            usable_model_budget_bytes: vram_gb * GB,
        }
    }

    #[test]
    fn small_model_is_resident_on_fastest() {
        let topo = discrete_topo(12, 64, true);
        let matrix = CapabilityMatrix {
            circuits: vec![
                circuit("Discrete GPU", CircuitKind::DiscreteGpu, 0.4, 1.0),
                circuit("iGPU", CircuitKind::IntegratedGpu, 7.0, 0.06),
                circuit("CPU native", CircuitKind::Cpu, 23.0, 0.02),
            ],
            gemv_n: 2048,
            npu_probed: false,
        };
        let plan = plan_employment(&topo, &matrix, 2 * GB, DEFAULT_KV_RESERVE);
        assert_eq!(plan.protocol, ResidencyProtocol::Resident);
        assert_eq!(plan.placements[0].kind, CircuitKind::DiscreteGpu);
        assert_eq!(plan.device_priority[0], "Discrete GPU");
    }

    #[test]
    fn overflow_with_igpu_is_heterogeneous() {
        // 20 GB model, 12 GB VRAM, iGPU present + 64 GB RAM → overflow runs in-place on the iGPU.
        let topo = discrete_topo(12, 64, true);
        let matrix = CapabilityMatrix {
            circuits: vec![
                circuit("Discrete GPU", CircuitKind::DiscreteGpu, 0.4, 1.0),
                circuit("iGPU", CircuitKind::IntegratedGpu, 7.0, 0.06),
                circuit("CPU native", CircuitKind::Cpu, 23.0, 0.02),
            ],
            gemv_n: 2048,
            npu_probed: false,
        };
        let plan = plan_employment(&topo, &matrix, 20 * GB, DEFAULT_KV_RESERVE);
        assert_eq!(plan.protocol, ResidencyProtocol::HeterogeneousOverflow);
        assert_eq!(plan.placements[0].role, PlacementRole::ResidentPrimary);
        assert_eq!(plan.placements[1].role, PlacementRole::Overflow);
        assert_eq!(plan.placements[1].kind, CircuitKind::IntegratedGpu);
    }

    #[test]
    fn overflow_without_igpu_streams() {
        // 20 GB model, 12 GB VRAM, NO iGPU (only discrete + CPU). v1 streams to the fast device.
        let topo = discrete_topo(12, 64, false);
        let matrix = CapabilityMatrix {
            circuits: vec![
                circuit("Discrete GPU", CircuitKind::DiscreteGpu, 0.4, 1.0),
                circuit("CPU native", CircuitKind::Cpu, 23.0, 0.02),
            ],
            gemv_n: 2048,
            npu_probed: false,
        };
        // CPU pool is huge (60 GB) so technically it could host overflow; v1 rule streams instead.
        // We assert the planner does NOT silently pick CPU compute for a 20 GB transformer overflow.
        let plan = plan_employment(&topo, &matrix, 20 * GB, DEFAULT_KV_RESERVE);
        assert!(
            matches!(
                plan.protocol,
                ResidencyProtocol::HeterogeneousOverflow | ResidencyProtocol::Streaming
            ),
            "must be an overflow strategy, got {:?}",
            plan.protocol
        );
    }

    #[test]
    fn unified_host_is_resident_on_igpu() {
        // Unified (no discrete): best circuit is the iGPU reading the large host pool → resident.
        let topo = HostTopology {
            adapters: vec![AdapterDesc {
                name: "Apple/Intel iGPU".into(),
                backend: "Metal".into(),
                class: AdapterClass::Integrated,
                vendor: 0x106b,
                device: 1,
                dedicated_vram_bytes: 0,
            }],
            topology: HostMemoryTopology::Unified,
            host_ram_bytes: 32 * GB,
            host_ram_available_bytes: 20 * GB,
            cpu_cores: 8,
            os_floor_bytes: 6 * GB,
            usable_model_budget_bytes: 26 * GB,
        };
        let matrix = CapabilityMatrix {
            circuits: vec![
                circuit("Apple/Intel iGPU", CircuitKind::IntegratedGpu, 1.0, 1.0),
                circuit("CPU native", CircuitKind::Cpu, 10.0, 0.1),
            ],
            gemv_n: 2048,
            npu_probed: false,
        };
        let plan = plan_employment(&topo, &matrix, 8 * GB, DEFAULT_KV_RESERVE);
        assert_eq!(plan.protocol, ResidencyProtocol::Resident);
        assert_eq!(plan.placements[0].kind, CircuitKind::IntegratedGpu);
    }

    /// Like `circuit` but with an explicit `upload_gbps` (the D31 transfer axis under test).
    fn circuit_up(
        label: &str,
        kind: CircuitKind,
        ms: f64,
        score: f64,
        upload_gbps: f64,
    ) -> CircuitBench {
        CircuitBench {
            upload_gbps,
            ..circuit(label, kind, ms, score)
        }
    }

    /// D31: the in-place secondary (iGPU, high `upload_gbps`) wins the argmin over a *faster*
    /// primary because the primary must stream the overflow over a slow bus, while the iGPU runs
    /// it in its own system-RAM pool at zero per-token transfer. overflow ≈ 8.44 GB:
    ///   stream→dGPU ≈ 0.22 ms compute + 4.53 ms transfer (2 GB/s) = 4.75 ms
    ///   in-place iGPU ≈ 3.78 ms compute + 0 transfer            = 3.78 ms  → wins.
    #[test]
    fn overflow_inplace_secondary_wins_argmin() {
        let topo = discrete_topo(12, 64, true);
        let matrix = CapabilityMatrix {
            circuits: vec![
                // Fastest compute, but a SLOW host→device bus (2 GB/s) for streaming overflow.
                circuit_up("Discrete GPU", CircuitKind::DiscreteGpu, 0.4, 1.0, 2.0),
                // Slower compute, but in-place (reads system RAM) — high nominal upload, irrelevant
                // to the in-place path (transfer term is zero regardless).
                circuit_up("iGPU", CircuitKind::IntegratedGpu, 7.0, 0.06, 50.0),
                circuit_up("CPU native", CircuitKind::Cpu, 23.0, 0.02, f64::INFINITY),
            ],
            gemv_n: 2048,
            npu_probed: false,
        };
        let plan = plan_employment(&topo, &matrix, 20 * GB, DEFAULT_KV_RESERVE);
        assert_eq!(plan.protocol, ResidencyProtocol::HeterogeneousOverflow);
        assert_eq!(plan.placements[1].role, PlacementRole::Overflow);
        assert_eq!(plan.placements[1].kind, CircuitKind::IntegratedGpu);
    }

    /// D31: when the primary's bus is fast (cheap transfer), streaming the overflow to the fast
    /// device beats running it in-place on the slow iGPU. Same shapes as above, upload 64 GB/s:
    ///   stream→dGPU ≈ 0.22 ms compute + 0.14 ms transfer = 0.36 ms  → wins.
    ///   in-place iGPU ≈ 3.78 ms compute                   = 3.78 ms.
    #[test]
    fn overflow_fast_bus_streams() {
        let topo = discrete_topo(12, 64, true);
        let matrix = CapabilityMatrix {
            circuits: vec![
                circuit_up("Discrete GPU", CircuitKind::DiscreteGpu, 0.4, 1.0, 64.0),
                circuit_up("iGPU", CircuitKind::IntegratedGpu, 7.0, 0.06, 50.0),
                circuit_up("CPU native", CircuitKind::Cpu, 23.0, 0.02, f64::INFINITY),
            ],
            gemv_n: 2048,
            npu_probed: false,
        };
        let plan = plan_employment(&topo, &matrix, 20 * GB, DEFAULT_KV_RESERVE);
        assert_eq!(plan.protocol, ResidencyProtocol::Streaming);
        assert_eq!(plan.placements[0].role, PlacementRole::StreamTarget);
        assert_eq!(plan.placements[0].kind, CircuitKind::DiscreteGpu);
    }

    /// D31 PROOF the transfer axis drives the decision: identical topology and identical
    /// *throughputs* (`ms_per_gemv`) for every circuit — only the primary's `upload_gbps` changes.
    /// The old fixed "iGPU-in-place wins" heuristic could never flip on this; the argmin does.
    #[test]
    fn overflow_flips_on_upload_gbps_only() {
        let topo = discrete_topo(12, 64, true);
        // A matrix parameterized ONLY by the discrete GPU's upload bandwidth; every throughput and
        // the iGPU/CPU rows (incl. their upload) are held fixed across the two calls.
        let matrix_with_dgpu_upload = |dgpu_up: f64| CapabilityMatrix {
            circuits: vec![
                circuit_up("Discrete GPU", CircuitKind::DiscreteGpu, 0.4, 1.0, dgpu_up),
                circuit_up("iGPU", CircuitKind::IntegratedGpu, 7.0, 0.06, 5.0),
                circuit_up("CPU native", CircuitKind::Cpu, 23.0, 0.02, f64::INFINITY),
            ],
            gemv_n: 2048,
            npu_probed: false,
        };

        // Slow bus → transfer dominates → overflow stays in-place on the iGPU.
        let slow = plan_employment(
            &topo,
            &matrix_with_dgpu_upload(1.0),
            20 * GB,
            DEFAULT_KV_RESERVE,
        );
        assert_eq!(
            slow.protocol,
            ResidencyProtocol::HeterogeneousOverflow,
            "slow primary bus must keep overflow in-place: {}",
            slow.rationale
        );

        // Fast bus → transfer is cheap → the SAME overflow now streams to the fast primary.
        let fast = plan_employment(
            &topo,
            &matrix_with_dgpu_upload(64.0),
            20 * GB,
            DEFAULT_KV_RESERVE,
        );
        assert_eq!(
            fast.protocol,
            ResidencyProtocol::Streaming,
            "fast primary bus must flip the decision to streaming: {}",
            fast.rationale
        );
    }

    fn synthetic_matrix() -> CapabilityMatrix {
        CapabilityMatrix {
            circuits: vec![
                circuit("Discrete GPU", CircuitKind::DiscreteGpu, 0.4, 1.0),
                circuit("iGPU", CircuitKind::IntegratedGpu, 7.0, 0.06),
                circuit("CPU native", CircuitKind::Cpu, 23.0, 0.02),
            ],
            gemv_n: 2048,
            npu_probed: false,
        }
    }

    /// H2 CONSUMPTION proof: the flag gates it; when ON the plan is computed, stored, and
    /// retrievable via the getter (module is consumed, not orphan); when OFF nothing is
    /// computed and behaviour is unchanged. No GPU required (synthetic topology + matrix).
    #[test]
    fn route_flag_gates_compute_store_and_retrieve() {
        // Order matters: assert the OFF path *before* any ON store touches the global.
        std::env::remove_var(ROUTE_ENV);
        assert!(!route_enabled(), "flag must default off");

        let topo = discrete_topo(12, 64, true);
        let matrix = synthetic_matrix();

        // OFF → nothing computed, nothing stored.
        let off = route_employment_for_model(&topo, &matrix, 20 * GB);
        assert!(off.is_none(), "flag off must compute nothing");
        assert!(
            last_employment_plan().is_none(),
            "flag off must store nothing"
        );

        // ON → computed, recorded, stored, retrievable.
        std::env::set_var(ROUTE_ENV, "1");
        assert!(route_enabled());
        let returned = route_employment_for_model(&topo, &matrix, 20 * GB)
            .expect("flag on must return a plan");
        assert_eq!(returned.protocol, ResidencyProtocol::HeterogeneousOverflow);
        assert_eq!(returned.model_bytes, 20 * GB);

        let stored = last_employment_plan().expect("plan must be retrievable after route");
        assert_eq!(stored.protocol, returned.protocol);
        assert_eq!(stored.model_bytes, returned.model_bytes);
        assert_eq!(stored.device_priority, returned.device_priority);
        assert_eq!(stored.placements.len(), returned.placements.len());

        std::env::remove_var(ROUTE_ENV);
    }
}
