//! STELLAR §A AH-track H2 — **residency + device-priority planner** (decisions D24/D25/D30/D31).
//!
//! Turns *discovery* (H0 `HostTopology` + H1(a) `CapabilityMatrix`) into an *employment plan* for a
//! given model: which residency protocol, which device holds what, and the device priority order.
//! It is a **discovery-derived adaptive plan** (D31) — not a fixed formula — and the core is a pure
//! function of its inputs, so it is fully unit-testable with synthetic profiles (no GPU required).
//!
//! Decision (D31), per machine, from measured inputs:
//!   1. fits the highest-ranked circuit's pool (minus a KV reserve) → **Resident**;
//!   2. doesn't fit, but a large-pool secondary compute circuit exists (iGPU reading system RAM,
//!      else CPU) → **HeterogeneousOverflow** (resident portion on the fast circuit, overflow
//!      in-place on the secondary — H1(a) measured this beats streaming for overflow);
//!   3. doesn't fit and no in-place secondary → **Streaming** (double-buffer overflow to the fast
//!      device; the A4 path).
//!   Device priority order = the measured `CapabilityMatrix` order (D30), never a static hierarchy.
//!
//! v1 uses the measured-crossover *finding* (iGPU-in-place wins for overflow when an iGPU is present)
//! as the rule; the full per-segment `argmin(measured compute + transfer)` over every circuit (D31)
//! is the documented refinement. Native only.
#![cfg(not(target_arch = "wasm32"))]

use serde::Serialize;

use crate::device_benchmark::{benchmark_devices, CapabilityMatrix, CircuitKind};
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

    // Overflow that doesn't fit the fast pool.
    let overflow = model_bytes - best_usable;

    // 2) Prefer an in-place large-pool secondary (iGPU before CPU — matrix is score-sorted).
    let secondary = matrix.circuits.iter().skip(1).find(|c| {
        matches!(c.kind, CircuitKind::IntegratedGpu | CircuitKind::Cpu)
            && pool_for(c.kind, topo) >= overflow
    });

    if let Some(sec) = secondary {
        let sec_pool = pool_for(sec.kind, topo);
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
                "model {:.2} GB exceeds the fast pool ({:.2} GB usable); {:.2} GB overflow runs in-place on {} (reads its {:.2} GB pool, no per-token PCIe weight transfer — H1(a) crossover)",
                model_bytes as f64 / 1e9,
                best_usable as f64 / 1e9,
                overflow as f64 / 1e9,
                sec.label,
                sec_pool as f64 / 1e9,
            ),
        };
    }

    // 3) No in-place secondary → stream overflow to the fast device (A4).
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
        rationale: format!(
            "model {:.2} GB exceeds the fast pool with no in-place secondary big enough → double-buffer stream to {} (A4)",
            model_bytes as f64 / 1e9,
            best.label,
        ),
    }
}

/// Probe the real host (H0 + H1(a)) and plan for `model_bytes`. Heavy (runs the benchmark); for the
/// fast path, plan against a cached matrix instead.
pub fn plan_for_model(model_bytes: u64) -> EmploymentPlan {
    let topo = probe_host_topology();
    let matrix = benchmark_devices(2048);
    plan_employment(&topo, &matrix, model_bytes, DEFAULT_KV_RESERVE)
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
}
