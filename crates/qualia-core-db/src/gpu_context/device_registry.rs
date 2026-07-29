//! Device-per-circuit registry — obtain a `wgpu::Device` for a SPECIFIC adapter/circuit
//! (STELLAR §A H3 foundation for heterogeneous GPU routing).
//!
//! [`super::shared_gpu`] / [`super::try_shared_gpu`] give you the single process-wide *primary*
//! device (a `PowerPreference::HighPerformance` pick — the discrete GPU on a discrete+integrated
//! box). This module lets code obtain a device for a **specific** enumerated circuit — e.g. the
//! integrated GPU — so audio/vision work can run off the LLM's silicon.
//!
//! **Role policy:** keep the primary circuit free for the LLM; audio/vision use the *auxiliary*
//! circuit. [`try_auxiliary_gpu`] gives callers the fallback chain **auxiliary → primary → None**
//! (the caller then degrades to CPU) so they always get *a* device or `None`, never a panic.
//!
//! Mirrors [`super::try_shared_gpu`]'s discipline: no `unwrap()` outside tests, and it NEVER panics
//! on a missing or failed device — every failure path returns `None`. Native only.
#![cfg(not(target_arch = "wasm32"))]

use std::collections::hash_map::Entry;
use std::sync::OnceLock;

use super::{init_shared_gpu_for_adapter, try_shared_gpu, GpuAdapterCaps, SharedGpuContext};

/// One enumerated compute circuit: the live `wgpu::Adapter`, its capability snapshot, and a stable
/// identity (vendor / device / backend) matching `device_benchmark` / `host_topology`.
pub struct CircuitAdapter {
    /// The live adapter (cheap `Arc`-backed handle) this circuit routes to.
    pub adapter: wgpu::Adapter,
    /// Immutable capability snapshot (carries vendor / device / backend / device_type).
    pub caps: GpuAdapterCaps,
}

impl CircuitAdapter {
    /// wgpu device class for role heuristics.
    #[inline]
    pub fn device_type(&self) -> wgpu::DeviceType {
        self.caps.device_type
    }

    /// Stable physical identity — matches `device_benchmark` / `host_topology` conventions.
    #[inline]
    pub fn identity(&self) -> (u32, u32, wgpu::Backend) {
        (self.caps.vendor, self.caps.device, self.caps.backend)
    }
}

/// The wgpu instance the registry enumerated adapters from. Cloned into each per-circuit device so
/// the device and any surface it creates share one instance.
static REGISTRY_INSTANCE: OnceLock<wgpu::Instance> = OnceLock::new();
/// Enumerate-once cache of routable circuits (deduped to one entry per physical GPU).
static ADAPTERS: OnceLock<Vec<CircuitAdapter>> = OnceLock::new();
/// Per-circuit device cache — one `OnceLock<Option<..>>` slot per enumerated adapter (by index).
static DEVICES: OnceLock<Vec<OnceLock<Option<SharedGpuContext>>>> = OnceLock::new();

/// Prefer discrete GPUs, then integrated, then virtual/other, with software CPU adapters last.
fn device_type_rank(t: wgpu::DeviceType) -> u8 {
    match t {
        wgpu::DeviceType::DiscreteGpu => 0,
        wgpu::DeviceType::IntegratedGpu => 1,
        wgpu::DeviceType::VirtualGpu => 2,
        wgpu::DeviceType::Other => 3,
        wgpu::DeviceType::Cpu => 4,
    }
}

/// Backend preference when the same physical GPU is enumerated on several backends
/// (mirrors `host_topology` / `device_benchmark`).
fn backend_rank(b: wgpu::Backend) -> u8 {
    match b {
        wgpu::Backend::Metal => 0,
        wgpu::Backend::Dx12 => 1,
        wgpu::Backend::Vulkan => 2,
        wgpu::Backend::Gl => 3,
        _ => 4,
    }
}

#[inline]
fn registry_instance() -> &'static wgpu::Instance {
    REGISTRY_INSTANCE.get_or_init(wgpu::Instance::default)
}

/// Enumerate every adapter once (cached), deduped to **one entry per physical circuit**
/// (vendor, device) keeping the preferred backend, and sorted discrete → integrated → other → cpu
/// for a deterministic, index-stable list. Returns an empty slice on a headless / GPU-less box.
///
/// The returned indices are the stable handles used by [`try_device_for_adapter`],
/// [`primary_circuit_index`], and [`auxiliary_circuit_index`].
pub fn enumerate_circuits() -> &'static [CircuitAdapter] {
    ADAPTERS.get_or_init(|| {
        let instance = registry_instance();
        let raw = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
        // Dedup by physical (vendor, device); keep the top-ranked backend for each.
        let mut best: std::collections::HashMap<(u32, u32), (u8, wgpu::Adapter, GpuAdapterCaps)> =
            std::collections::HashMap::new();
        for adapter in raw {
            let caps = GpuAdapterCaps::from_adapter(&adapter);
            let rank = backend_rank(caps.backend);
            match best.entry((caps.vendor, caps.device)) {
                Entry::Occupied(mut e) => {
                    if rank < e.get().0 {
                        e.insert((rank, adapter, caps));
                    }
                }
                Entry::Vacant(e) => {
                    e.insert((rank, adapter, caps));
                }
            }
        }
        let mut circuits: Vec<CircuitAdapter> = best
            .into_values()
            .map(|(_, adapter, caps)| CircuitAdapter { adapter, caps })
            .collect();
        // Drop device==0 phantom rows for a vendor that also enumerated a real device id
        // (some backends, notably GL, report device 0 for a card already seen with a real id).
        let real_vendors: std::collections::HashSet<u32> = circuits
            .iter()
            .filter(|c| c.caps.device != 0)
            .map(|c| c.caps.vendor)
            .collect();
        circuits.retain(|c| c.caps.device != 0 || !real_vendors.contains(&c.caps.vendor));
        // Deterministic, stable ordering: discrete first, then integrated, then the rest.
        circuits.sort_by_key(|c| {
            (
                device_type_rank(c.caps.device_type),
                c.caps.vendor,
                c.caps.device,
            )
        });
        circuits
    })
}

/// Per-circuit device slots, lazily sized to the number of enumerated circuits.
fn device_slots() -> &'static Vec<OnceLock<Option<SharedGpuContext>>> {
    DEVICES.get_or_init(|| {
        let n = enumerate_circuits().len();
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(OnceLock::new());
        }
        v
    })
}

/// Lazily build (and cache) a [`SharedGpuContext`] for the enumerated circuit at `index`.
///
/// Returns `None` for an out-of-range index, when no tokio runtime can be started, or on any
/// device-creation failure — it **NEVER panics** (mirrors [`super::try_shared_gpu`]). Idempotent:
/// repeated calls return the same cached device, or the same cached `None` (each circuit probes at
/// most once).
pub fn try_device_for_adapter(index: usize) -> Option<&'static SharedGpuContext> {
    let circuits = enumerate_circuits();
    if index >= circuits.len() {
        return None;
    }
    device_slots()[index]
        .get_or_init(|| {
            let handle = match tokio::runtime::Handle::try_current() {
                Ok(h) => h,
                Err(_) => match tokio::runtime::Runtime::new() {
                    Ok(rt) => Box::leak(Box::new(rt)).handle().clone(),
                    Err(_) => return None,
                },
            };
            let instance = registry_instance().clone();
            let adapter = circuits[index].adapter.clone();
            tokio::task::block_in_place(|| {
                handle
                    .block_on(init_shared_gpu_for_adapter(instance, adapter))
                    .ok()
            })
        })
        .as_ref()
}

/// Index of the **primary** circuit — the discrete / HighPerformance adapter that
/// [`super::shared_gpu`] uses. If the process-wide shared device is already initialized, matches its
/// exact identity (that is definitionally the primary). Otherwise picks the first discrete GPU
/// (what `HighPerformance` selects), else the top-ranked adapter. `None` only when no circuit
/// exists. Never forces the shared device to initialize.
///
/// Policy: keep the primary circuit free for the LLM; audio/vision use the auxiliary circuit.
pub fn primary_circuit_index() -> Option<usize> {
    let circuits = enumerate_circuits();
    if circuits.is_empty() {
        return None;
    }
    // If the process-wide shared (primary) device already exists, match it exactly — but do NOT
    // force initialization here (peek only).
    if let Some(Some(shared)) = super::SHARED_GPU.get() {
        let want = (shared.adapter_caps.vendor, shared.adapter_caps.device);
        if let Some(i) = circuits
            .iter()
            .position(|c| (c.caps.vendor, c.caps.device) == want)
        {
            return Some(i);
        }
    }
    circuits
        .iter()
        .position(|c| c.caps.device_type == wgpu::DeviceType::DiscreteGpu)
        .or(Some(0))
}

/// Index of the best **auxiliary** circuit — the best NON-primary circuit, so the primary stays
/// free for the LLM. Prefers a (non-primary) `DeviceType::IntegratedGpu`; otherwise the next-best
/// non-primary circuit (the list is already ranked discrete → integrated → …). Returns `None` when
/// only one circuit exists.
///
/// A measured `device_benchmark::CapabilityMatrix` ranking would be more precise, but benchmarking
/// spawns worker processes and is not "cheaply available", so this uses the DeviceType heuristic.
///
/// Policy: keep the primary circuit free for the LLM; audio/vision use the auxiliary circuit.
pub fn auxiliary_circuit_index() -> Option<usize> {
    let circuits = enumerate_circuits();
    if circuits.len() < 2 {
        return None;
    }
    let primary = primary_circuit_index();
    let is_primary = |i: usize| Some(i) == primary;
    // Prefer a non-primary integrated GPU.
    if let Some(i) = (0..circuits.len()).find(|&i| {
        !is_primary(i) && circuits[i].caps.device_type == wgpu::DeviceType::IntegratedGpu
    }) {
        return Some(i);
    }
    // Else the first non-primary circuit.
    (0..circuits.len()).find(|&i| !is_primary(i))
}

/// A device for the **auxiliary** circuit if one exists, else the **primary** shared device, else
/// `None`. Fallback chain: **auxiliary → primary → None** (the caller then degrades to CPU). Never
/// panics — every failure resolves to `None`.
///
/// Policy: keep the primary circuit free for the LLM; audio/vision call this to use the auxiliary
/// GPU, transparently falling back to the primary (or CPU) when there is only one circuit.
pub fn try_auxiliary_gpu() -> Option<&'static SharedGpuContext> {
    auxiliary_circuit_index()
        .and_then(try_device_for_adapter)
        .or_else(try_shared_gpu)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn enumerate_is_nonempty_or_headless() {
        // Enumeration returns a cached list; its length + identities are stable across calls
        // (may be 0 on a headless box — we assert idempotence, not a nonzero count).
        let a = enumerate_circuits();
        let b = enumerate_circuits();
        assert_eq!(
            a.len(),
            b.len(),
            "enumeration must be cached (stable length)"
        );
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(
                x.identity(),
                y.identity(),
                "cached identities must be stable"
            );
        }
    }

    #[test]
    #[serial_test::serial(gpu)]
    fn try_device_for_adapter_never_panics_and_caches() {
        let n = enumerate_circuits().len();
        if n == 0 {
            // GPU-less / headless: index 0 must return None without panic.
            assert!(try_device_for_adapter(0).is_none());
            return;
        }
        let first = try_device_for_adapter(0).is_some();
        let second = try_device_for_adapter(0).is_some();
        assert_eq!(
            first, second,
            "try_device_for_adapter must be cached + consistent"
        );
        // Out-of-range never panics.
        assert!(try_device_for_adapter(n + 1000).is_none());
    }

    #[test]
    #[serial_test::serial(gpu)]
    fn try_auxiliary_gpu_never_panics() {
        // Consistent across calls, never panics.
        let a = try_auxiliary_gpu().is_some();
        let b = try_auxiliary_gpu().is_some();
        assert_eq!(a, b, "try_auxiliary_gpu must be consistent across calls");
        // When a working primary device exists, the fallback chain (aux → primary) must yield one.
        if try_shared_gpu().is_some() {
            assert!(a, "aux must fall back to the working primary device");
        }
    }

    #[test]
    #[serial_test::serial(gpu)]
    fn primary_and_auxiliary_indices_differ_when_multi_adapter() {
        let n = enumerate_circuits().len();
        let p = primary_circuit_index();
        let aux = auxiliary_circuit_index();
        if n >= 2 {
            assert!(p.is_some(), "multi-adapter → a primary exists");
            assert!(aux.is_some(), "multi-adapter → an auxiliary exists");
            assert_ne!(p, aux, "primary and auxiliary must differ");
        } else if n == 1 {
            assert_eq!(p, Some(0), "single adapter → primary is index 0");
            assert_eq!(
                aux, None,
                "single adapter → no auxiliary (fallback covers callers)"
            );
        } else {
            assert_eq!(p, None);
            assert_eq!(aux, None);
        }
    }
}
