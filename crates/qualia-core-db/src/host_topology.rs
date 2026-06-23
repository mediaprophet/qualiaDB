//! STELLAR §A AH-track H0 — **host topology + capability sensor** (decision D23/D24).
//!
//! The engine must *sense then route* instead of grabbing one adapter. Today `gpu_context::
//! shared_gpu()` requests a single `PowerPreference::HighPerformance` device — on a discrete +
//! integrated box that silently takes only the discrete GPU and ignores the integrated GPU and all
//! of system RAM. This module enumerates **every** adapter, classifies the memory topology
//! (discrete vs unified), reads host RAM, and computes the bounded OS floor (D24). The residency
//! planner (H2) and heterogeneous/cluster dispatch (H3/H5) consume this; it makes no routing
//! decision itself and changes no existing behaviour.
//!
//! Native only — `enumerate_adapters` is not available on the wasm/WebGPU path.
#![cfg(not(target_arch = "wasm32"))]

use serde::Serialize;

/// Coarse adapter class (maps `wgpu::DeviceType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AdapterClass {
    Discrete,
    Integrated,
    Cpu,
    Virtual,
    Other,
}

impl AdapterClass {
    fn from_wgpu(t: wgpu::DeviceType) -> Self {
        match t {
            wgpu::DeviceType::DiscreteGpu => Self::Discrete,
            wgpu::DeviceType::IntegratedGpu => Self::Integrated,
            wgpu::DeviceType::Cpu => Self::Cpu,
            wgpu::DeviceType::VirtualGpu => Self::Virtual,
            wgpu::DeviceType::Other => Self::Other,
        }
    }
}

/// Whole-host memory topology: is there a dedicated VRAM pool, or one shared pool?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum HostMemoryTopology {
    /// At least one discrete GPU with its own VRAM (PCIe boundary matters).
    Discrete,
    /// Only integrated/CPU adapters — VRAM and system RAM are one pool (Apple Silicon, iGPU-only, phone).
    Unified,
}

/// One enumerated adapter.
#[derive(Debug, Clone, Serialize)]
pub struct AdapterDesc {
    pub name: String,
    pub backend: String,
    pub class: AdapterClass,
    pub vendor: u32,
    pub device: u32,
    /// Best-effort dedicated VRAM (Windows/DXGI only; 0 = unknown on this platform).
    pub dedicated_vram_bytes: u64,
}

/// The sensed host: adapters, topology, host memory, and the bounded OS floor (D24).
#[derive(Debug, Clone, Serialize)]
pub struct HostTopology {
    pub adapters: Vec<AdapterDesc>,
    pub topology: HostMemoryTopology,
    pub host_ram_bytes: u64,
    pub host_ram_available_bytes: u64,
    pub cpu_cores: usize,
    /// Hard floor reserved for the OS/display (D24): discrete → VRAM display floor; unified → host floor.
    pub os_floor_bytes: u64,
    /// Best-effort memory the LLM may inhabit after the floor (discrete: VRAM−floor; unified: RAM−floor).
    pub usable_model_budget_bytes: u64,
}

const DISCRETE_VRAM_FLOOR: u64 = 1_500 * 1024 * 1024; // ~1.5 GB for the display server
const UNIFIED_HOST_FLOOR_MAX: u64 = 6 * 1024 * 1024 * 1024; // cap the unified host reservation

/// Rank backends so the same physical GPU (enumerated once per backend) collapses to the preferred one.
fn backend_rank(b: wgpu::Backend) -> u8 {
    match b {
        // Prefer the backend each OS actually runs the engine on.
        wgpu::Backend::Metal => 0,
        wgpu::Backend::Dx12 => 1,
        wgpu::Backend::Vulkan => 2,
        wgpu::Backend::Gl => 3,
        _ => 4,
    }
}

/// Probe the host: enumerate all adapters (deduped across backends), classify, size the floor.
pub fn probe_host_topology() -> HostTopology {
    use sysinfo::System;
    let sys = System::new_all();
    let host_ram_bytes = sys.total_memory(); // bytes (sysinfo ≥ 0.30)
    let host_ram_available_bytes = sys.available_memory();
    let cpu_cores = num_cpus::get();

    // Enumerate every adapter across every backend, then dedup by physical (vendor, device).
    let instance = wgpu::Instance::default();
    let raw = instance.enumerate_adapters(wgpu::Backends::all());
    let mut best: std::collections::HashMap<(u32, u32), (u8, AdapterDesc)> = std::collections::HashMap::new();
    for adapter in raw {
        let info = adapter.get_info();
        let rank = backend_rank(info.backend);
        let desc = AdapterDesc {
            name: info.name.clone(),
            backend: format!("{:?}", info.backend),
            class: AdapterClass::from_wgpu(info.device_type),
            vendor: info.vendor,
            device: info.device,
            dedicated_vram_bytes: 0,
        };
        best.entry((info.vendor, info.device))
            .and_modify(|(r, d)| {
                if rank < *r {
                    *r = rank;
                    *d = desc.clone();
                }
            })
            .or_insert((rank, desc));
    }
    let mut adapters: Vec<AdapterDesc> = best.into_values().map(|(_, d)| d).collect();
    // Drop phantom duplicates: some backends (notably GL) report device id 0 for a card already
    // enumerated with a real id on another backend. (Limitation: two *identical-model* cards share
    // (vendor, device) and collapse to one — precise multi-GPU counting of identical cards needs a
    // per-OS PCI-bus / LUID probe that wgpu doesn't expose; deferred to H3/H5.)
    let vendors_with_real_dev: std::collections::HashSet<u32> =
        adapters.iter().filter(|a| a.device != 0).map(|a| a.vendor).collect();
    adapters.retain(|a| a.device != 0 || !vendors_with_real_dev.contains(&a.vendor));
    // Discrete first, then integrated, then the rest — deterministic order for the planner.
    adapters.sort_by_key(|a| (a.class as u8, a.vendor, a.device));

    let has_discrete = adapters.iter().any(|a| a.class == AdapterClass::Discrete);
    let topology = if has_discrete {
        HostMemoryTopology::Discrete
    } else {
        HostMemoryTopology::Unified
    };

    // Best-effort dedicated VRAM for the discrete card (Windows/DXGI).
    #[cfg(target_os = "windows")]
    if has_discrete {
        if let Ok(mem) = crate::directml_bridge::probe_best_adapter_memory() {
            let vram = mem.dedicated_vram_bytes.max(mem.local_budget_bytes);
            if let Some(disc) = adapters.iter_mut().find(|a| a.class == AdapterClass::Discrete) {
                disc.dedicated_vram_bytes = vram;
            }
        }
    }

    let (os_floor_bytes, usable_model_budget_bytes) = match topology {
        HostMemoryTopology::Discrete => {
            let vram = adapters
                .iter()
                .filter(|a| a.class == AdapterClass::Discrete)
                .map(|a| a.dedicated_vram_bytes)
                .max()
                .unwrap_or(0);
            let floor = DISCRETE_VRAM_FLOOR.min(vram / 4); // never reserve more than ¼ of a tiny card
            (floor, vram.saturating_sub(floor))
        }
        HostMemoryTopology::Unified => {
            // Reserve up to 6 GB or a quarter of RAM, whichever is smaller, for the host OS.
            let floor = UNIFIED_HOST_FLOOR_MAX.min(host_ram_bytes / 4);
            (floor, host_ram_bytes.saturating_sub(floor))
        }
    };

    HostTopology {
        adapters,
        topology,
        host_ram_bytes,
        host_ram_available_bytes,
        cpu_cores,
        os_floor_bytes,
        usable_model_budget_bytes,
    }
}

impl HostTopology {
    /// True when there is both a discrete GPU and a (system-RAM-backed) integrated GPU —
    /// the heterogeneous-overflow opportunity (H3): the iGPU can host overflow layers in system RAM.
    pub fn has_heterogeneous_overflow(&self) -> bool {
        self.topology == HostMemoryTopology::Discrete
            && self.adapters.iter().any(|a| a.class == AdapterClass::Integrated)
    }

    /// One-line-per-field human summary for logs / the progress record.
    pub fn summary(&self) -> String {
        let mut s = format!(
            "HostTopology: {:?} | host RAM {:.1} GB ({:.1} GB free) | {} cores | OS floor {:.1} GB | model budget {:.1} GB | heterogeneous={}\n",
            self.topology,
            self.host_ram_bytes as f64 / 1e9,
            self.host_ram_available_bytes as f64 / 1e9,
            self.cpu_cores,
            self.os_floor_bytes as f64 / 1e9,
            self.usable_model_budget_bytes as f64 / 1e9,
            self.has_heterogeneous_overflow(),
        );
        for a in &self.adapters {
            s.push_str(&format!(
                "  - {:?} [{}] {} (vendor 0x{:04x} dev 0x{:04x}){}\n",
                a.class,
                a.backend,
                a.name,
                a.vendor,
                a.device,
                if a.dedicated_vram_bytes > 0 {
                    format!(" VRAM {:.1} GB", a.dedicated_vram_bytes as f64 / 1e9)
                } else {
                    String::new()
                },
            ));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn h0_probes_host_topology() {
        let topo = probe_host_topology();
        eprintln!("{}", topo.summary());

        // Host memory + CPU are always available.
        assert!(topo.host_ram_bytes > 0, "host RAM must be sensed");
        assert!(topo.cpu_cores > 0, "cpu cores must be sensed");

        if topo.adapters.is_empty() {
            eprintln!("[h0] no wgpu adapters (headless CI) — RAM/CPU still sensed");
            return;
        }

        // Topology classification is consistent with the enumerated adapters.
        let any_discrete = topo.adapters.iter().any(|a| a.class == AdapterClass::Discrete);
        assert_eq!(
            any_discrete,
            topo.topology == HostMemoryTopology::Discrete,
            "topology must match presence of a discrete adapter"
        );
        // The model budget never exceeds the relevant pool.
        assert!(topo.usable_model_budget_bytes <= topo.host_ram_bytes.max(
            topo.adapters.iter().map(|a| a.dedicated_vram_bytes).max().unwrap_or(0)
        ).max(1));
    }
}
