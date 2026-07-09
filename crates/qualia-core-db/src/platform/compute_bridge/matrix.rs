//! The per-class capability matrix and the built-in backends (plan §3).
//!
//! `benchmark_devices` (the existing AH-track probe) measures ONE GEMV and ranks
//! circuits globally. This extends that to a **per-kernel-class** matrix: each class
//! is measured on every available backend, and a winner is recorded *per class*
//! (the fastest backend for GEMV is not the fastest for an FFT). The result is what
//! `ComputePolicy::select` consults.
//!
//! Built-in backends registered by default:
//! - [`CpuBackend`] — the native `rayon` reference kernels (`super::reference`),
//!   always available, a row for every class.
//! - [`WgpuBackend`] — the portable wgpu path. Today it measures the `DenseLinear`
//!   class via the existing `device_benchmark` GEMV (a real GPU number); the other
//!   classes have no portable GPU microkernel *yet*, so it returns no rows for them
//!   (recorded honestly as "not probed on GPU", never faked). Those per-class GPU
//!   kernels land per module (plan P2/P5) and appear here with zero further wiring.

use std::time::Instant;

use super::backend::{BackendId, KernelPanel, ProbeableBackend};
use super::kernel_class::KernelClass;
use super::reference;
use crate::device_benchmark::{benchmark_devices, CircuitBench, CircuitKind};

/// Measured per-class capability: for each class, the backend rows ranked
/// fastest-first. `best_for` is the O(1)-ish lookup the STEM call sites use.
#[derive(Debug, Clone)]
pub struct ClassMatrix {
    per_class: Vec<(KernelClass, Vec<CircuitBench>)>,
}

impl ClassMatrix {
    /// Construct directly from per-class ranked rows — used by the passport loader
    /// (a cached matrix) and by tests that synthesise a matrix without a GPU.
    pub fn from_per_class(per_class: Vec<(KernelClass, Vec<CircuitBench>)>) -> Self {
        Self { per_class }
    }

    /// Ranked rows for a class (fastest first); empty if the class was not probed.
    pub fn rows(&self, class: KernelClass) -> &[CircuitBench] {
        self.per_class
            .iter()
            .find(|(c, _)| *c == class)
            .map(|(_, r)| r.as_slice())
            .unwrap_or(&[])
    }

    /// The measured fastest (circuit, backend) for a class, if any was probed.
    pub fn best_for(&self, class: KernelClass) -> Option<&CircuitBench> {
        self.rows(class).first()
    }

    pub fn summary(&self) -> String {
        let mut s = String::from("ClassMatrix (per kernel-class, ranked fastest-first):\n");
        for (class, rows) in &self.per_class {
            s.push_str(&format!("  {}:\n", class.label()));
            for (i, r) in rows.iter().enumerate() {
                s.push_str(&format!(
                    "    {}. {:<26} [{:?}/{}] {:>9.4} ms  score {:.3}\n",
                    i + 1,
                    r.label,
                    r.kind,
                    r.backend,
                    r.ms_per_gemv,
                    r.rel_score
                ));
            }
        }
        s
    }
}

/// Time `f` over `iters` runs (with one warmup) and return ms/run.
fn time_ms(iters: u32, mut f: impl FnMut()) -> f64 {
    f(); // warmup
    let t0 = Instant::now();
    for _ in 0..iters {
        f();
    }
    t0.elapsed().as_secs_f64() * 1e3 / iters as f64
}

/// Native-CPU backend: the `rayon` reference kernels. Always available.
pub struct CpuBackend;

impl ProbeableBackend for CpuBackend {
    fn id(&self) -> BackendId {
        BackendId::CPU
    }
    fn available(&self) -> bool {
        true // CPU is always present (plan §7)
    }
    fn probe_class(&self, class: KernelClass, panel: &KernelPanel) -> Vec<CircuitBench> {
        let label = format!("CPU native (rayon, {} cores)", num_cpus::get());
        let ms = match class {
            KernelClass::DenseLinear => {
                let n = panel.dense_n;
                let w = vec![0.05f32; n * n];
                let x = vec![0.1f32; n];
                let mut y = vec![0.0f32; n];
                time_ms(5, || reference::gemv(&w, &x, &mut y))
            }
            KernelClass::ElementwiseMap => {
                let x = vec![0.1f32; panel.vector_len];
                let mut y = vec![0.0f32; panel.vector_len];
                time_ms(10, || reference::axpb(2.0, &x, 1.0, &mut y))
            }
            KernelClass::Reduction => {
                let x = vec![0.1f32; panel.vector_len];
                time_ms(10, || {
                    let _ = reference::reduce_sum(&x);
                })
            }
            KernelClass::Stencil => {
                let x = vec![0.1f32; panel.grid_n];
                let mut y = vec![0.0f32; panel.grid_n];
                time_ms(10, || reference::stencil3(&x, &mut y))
            }
            KernelClass::AllPairs => {
                let pts: Vec<f32> = (0..panel.nbody_n * 3)
                    .map(|i| (i % 97) as f32 * 0.1)
                    .collect();
                time_ms(3, || {
                    let _ = reference::allpairs_potential(&pts);
                })
            }
            KernelClass::Fft => {
                let n = panel.fft_n.next_power_of_two();
                let base: Vec<f32> = (0..n).map(|i| (i as f32 * 0.01).sin()).collect();
                time_ms(5, || {
                    let mut re = base.clone();
                    let mut im = vec![0.0f32; n];
                    reference::fft_radix2(&mut re, &mut im, false);
                })
            }
            KernelClass::Scan => {
                let x = vec![0.1f32; panel.vector_len];
                let mut y = vec![0.0f32; panel.vector_len];
                time_ms(10, || reference::prefix_sum(&x, &mut y))
            }
            KernelClass::Divergent => time_ms(3, || {
                let _ = reference::monte_carlo_pi(panel.mc_steps);
            }),
        };
        vec![CircuitBench {
            label,
            kind: CircuitKind::Cpu,
            backend: "native".to_string(),
            ms_per_gemv: ms,
            gflops: 0.0, // throughput proxy not derived here; ranking uses ms
            upload_gbps: f64::INFINITY, // data already in the CPU pool — no transfer
            rel_score: 1.0,
            decode_proxy_tok_s: None,
        }]
    }
}

/// Portable wgpu backend. Measures `DenseLinear` via the existing GEMV probe; other
/// classes have no portable GPU microkernel yet (returns no rows — honest).
pub struct WgpuBackend;

impl ProbeableBackend for WgpuBackend {
    fn id(&self) -> BackendId {
        BackendId::WGPU
    }
    fn available(&self) -> bool {
        // Available if any non-CPU wgpu adapter enumerates. Cheap-ish; only at boot.
        let instance = wgpu::Instance::default();
        pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()))
            .iter()
            .any(|a| {
                let info = a.get_info();
                info.device_type != wgpu::DeviceType::Cpu && info.device != 0
            })
    }
    fn probe_class(&self, class: KernelClass, panel: &KernelPanel) -> Vec<CircuitBench> {
        match class {
            KernelClass::DenseLinear => {
                // Reuse the real GEMV probe; keep only the GPU rows (the CPU row is
                // contributed by CpuBackend).
                benchmark_devices(panel.dense_n)
                    .circuits
                    .into_iter()
                    .filter(|c| c.kind != CircuitKind::Cpu)
                    .collect()
            }
            // No portable GPU microkernel for these classes yet → no rows (plan P2/P5).
            _ => Vec::new(),
        }
    }
}

/// Probe every available backend across every kernel class and assemble the ranked
/// per-class matrix. Boot-time only (cache it in the passport).
pub fn probe_class_matrix(
    registry: &super::backend::BackendRegistry,
    panel: &KernelPanel,
) -> ClassMatrix {
    let mut per_class = Vec::with_capacity(KernelClass::ALL.len());
    for class in KernelClass::ALL {
        let mut rows: Vec<CircuitBench> = Vec::new();
        for backend in registry.available() {
            rows.extend(backend.probe_class(class, panel));
        }
        // Rank fastest-first by measured ms, fill relative scores.
        rows.sort_by(|a, b| {
            a.ms_per_gemv
                .partial_cmp(&b.ms_per_gemv)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if let Some(best) = rows.first().map(|c| c.ms_per_gemv) {
            for r in &mut rows {
                r.rel_score = if r.ms_per_gemv > 0.0 {
                    best / r.ms_per_gemv
                } else {
                    0.0
                };
            }
        }
        per_class.push((class, rows));
    }
    ClassMatrix { per_class }
}

#[cfg(test)]
mod tests {
    use super::super::backend::BackendRegistry;
    use super::*;

    #[test]
    fn cpu_backend_probes_every_class_with_real_rows() {
        let cpu = CpuBackend;
        let panel = KernelPanel::quick();
        for class in KernelClass::ALL {
            let rows = cpu.probe_class(class, &panel);
            assert_eq!(rows.len(), 1, "CPU must yield a row for {}", class.label());
            assert!(
                rows[0].ms_per_gemv > 0.0,
                "{} CPU time must be positive",
                class.label()
            );
            assert!(rows[0].upload_gbps.is_infinite(), "CPU is in-pool");
        }
    }

    #[test]
    fn class_matrix_has_cpu_winner_for_every_class_headless() {
        // CPU-only registry (deterministic, no GPU): every class gets a ranked row,
        // and the best is score 1.0.
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(CpuBackend));
        let m = probe_class_matrix(&reg, &KernelPanel::quick());
        for class in KernelClass::ALL {
            let best = m.best_for(class).expect("a class winner must exist");
            assert_eq!(best.kind, CircuitKind::Cpu);
            assert!((best.rel_score - 1.0).abs() < 1e-9);
        }
    }
}
