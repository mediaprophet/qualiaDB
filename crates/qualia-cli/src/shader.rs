use std::path::{Path, PathBuf};
use std::str::FromStr;

use clap::{Args, Subcommand};
use qualia_core_db::wgsl_forge::execute::WgpuComputeContext;
use qualia_core_db::wgsl_forge::{
    candidate_evaluation, generate_builtin, tune_with, validate_wgsl, validate_native,
    AdapterConstraints, BuiltinKernel, CertificationManifest, ForgeError, ManifestCache, Schedule,
    ScheduleSpace, TargetBackend, TuningConfig, TuningManifest,
};

#[derive(Debug, Clone, Args)]
pub struct ScheduleArgs {
    /// Compute invocations in one workgroup.
    #[arg(long, default_value_t = 64)]
    pub workgroup: u32,
    /// Scalar/vector items processed by one invocation.
    #[arg(long, default_value_t = 1)]
    pub items: u32,
    /// Local vector width (1, 2, or 4).
    #[arg(long, default_value_t = 1)]
    pub vector_width: u32,
}

impl ScheduleArgs {
    fn schedule(&self) -> Schedule {
        Schedule {
            workgroup_size: self.workgroup,
            items_per_invocation: self.items,
            vector_width: self.vector_width,
            ..Default::default()
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum ShaderAction {
    /// List deterministic kernels currently known to WGSL Forge.
    ListKernels,
    /// Check that the native backend toolchains (wgpu adapter, DXC, CUDA) are
    /// present and report how the Forge will degrade if any are missing.
    Doctor,
    /// Print the roofline estimate (FLOP/byte, memory- vs compute-bound) for a kernel.
    Roofline {
        #[arg(default_value = "affine-f32")]
        kernel: String,
        /// Representative problem size (output elements / records).
        #[arg(long, default_value_t = 65_536)]
        n: u64,
        #[arg(long)]
        json: bool,
    },
    /// Probe the local adapter and print a rich hardware/topology profile.
    ProfileHardware {
        /// Write the profile JSON to this path (also prints the topology hash).
        #[arg(long)]
        export: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Generate one deterministic WGSL module.
    Generate {
        #[arg(default_value = "affine-f32")]
        kernel: String,
        /// Target backend (wgsl, msl, hlsl, ptx)
        #[arg(long, default_value = "wgsl")]
        target: String,
        #[command(flatten)]
        schedule: ScheduleArgs,
        /// Write WGSL to this path instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Emit the complete generated record as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Run Naga parsing and semantic validation.
    Validate {
        /// Validate an existing WGSL file; otherwise generate the selected kernel.
        #[arg(long)]
        input: Option<PathBuf>,
        #[arg(default_value = "affine-f32")]
        kernel: String,
        /// Target backend (wgsl, msl, hlsl, ptx)
        #[arg(long, default_value = "wgsl")]
        target: String,
        #[command(flatten)]
        schedule: ScheduleArgs,
        /// Write a Naga-level certification manifest.
        #[arg(long)]
        manifest: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Create a real pipeline, compare GPU output with the CPU oracle, and profile it.
    Certify {
        #[arg(default_value = "affine-f32")]
        kernel: String,
        #[command(flatten)]
        schedule: ScheduleArgs,
        #[arg(long, default_value_t = 4_099)]
        length: usize,
        #[arg(long, default_value_t = 3)]
        warmups: usize,
        #[arg(long, default_value_t = 9)]
        samples: usize,
        #[arg(long)]
        manifest: Option<PathBuf>,
        /// Also store the adapter-keyed certification in this cache directory.
        #[arg(long)]
        cache_dir: Option<PathBuf>,
        /// Prune/emit/validate only; do not dispatch on the GPU.
        #[arg(long)]
        dry_run: bool,
    },
    /// Search the bounded schedule space and certify the fastest correct variant.
    Tune {
        #[arg(default_value = "affine-f32")]
        kernel: String,
        #[arg(long, default_value_t = 65_537)]
        length: usize,
        #[arg(long, default_value_t = 2)]
        warmups: usize,
        #[arg(long, default_value_t = 3)]
        initial_samples: usize,
        #[arg(long, default_value_t = 11)]
        finalist_samples: usize,
        #[arg(long, default_value_t = 6)]
        finalists: usize,
        #[arg(long, default_value_t = 48)]
        max_candidates: usize,
        #[arg(long)]
        manifest: Option<PathBuf>,
        /// Also store the adapter-keyed tuning record in this cache directory.
        #[arg(long)]
        cache_dir: Option<PathBuf>,
        /// Report adapter-pruned candidate counts only; do not dispatch on the GPU.
        #[arg(long)]
        dry_run: bool,
    },
    /// Tune every GPU-certifiable kernel, reusing the topology-keyed cache.
    AutoTuneAll {
        #[arg(long, default_value_t = 65_537)]
        length: usize,
        #[arg(long, default_value_t = 2)]
        warmups: usize,
        #[arg(long, default_value_t = 24)]
        max_candidates: usize,
        /// Cache directory to read existing manifests from and (with
        /// --update-local-manifest) write new ones to.
        #[arg(long)]
        cache_dir: Option<PathBuf>,
        /// Persist freshly tuned manifests to the cache, keyed by topology.
        #[arg(long)]
        update_local_manifest: bool,
        /// List what would be tuned vs. served from cache; do not dispatch.
        #[arg(long)]
        dry_run: bool,
    },
}

pub fn run(action: &ShaderAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        ShaderAction::ListKernels => {
            println!("Qualia WGSL Forge kernels:");
            for builtin in BuiltinKernel::ALL {
                let spec = builtin.spec();
                println!(
                    "  {:<16} v{}  {}",
                    builtin.name(),
                    spec.semantic_version,
                    spec.description
                );
            }
        }
        ShaderAction::Roofline { kernel, n, json } => {
            let builtin = parse_kernel(kernel)?;
            let estimate = qualia_core_db::wgsl_forge::roofline_for(builtin, *n);
            if *json {
                println!("{}", serde_json::to_string_pretty(&estimate)?);
            } else {
                println!(
                    "{} @ n={}: {} FLOP / {} bytes -> {:.3} FLOP/byte ({:?}-bound)",
                    builtin.name(),
                    n,
                    estimate.flops,
                    estimate.bytes,
                    estimate.arithmetic_intensity,
                    estimate.bound
                );
            }
        }
        ShaderAction::Doctor => {
            use std::process::Command;
            println!("Qualia WGSL Forge — environment doctor\n");

            match WgpuComputeContext::new(1024 * 1024) {
                Ok(runner) => println!(
                    "[ok]   wgpu adapter: {} ({})",
                    runner.adapter.name, runner.adapter.backend
                ),
                Err(error) => println!(
                    "[warn] wgpu adapter: none ({error}); generation/validation still work headless"
                ),
            }

            let dxc = std::env::var("QUALIA_DXC_PATH").unwrap_or_else(|_| "dxc".to_string());
            match Command::new(&dxc).arg("--version").output() {
                Ok(out) if out.status.success() => {
                    println!("[ok]   DXC (HLSL->SPIR-V/DXIL): {dxc}");
                }
                _ => println!(
                    "[warn] DXC not found — set QUALIA_DXC_PATH or add dxc to PATH.\n         HLSL native path disabled; get DXC: https://github.com/microsoft/DirectXShaderCompiler/releases"
                ),
            }

            let nvcc = std::env::var("CUDA_PATH")
                .map(|p| format!("{p}/bin/nvcc"))
                .unwrap_or_else(|_| "nvcc".to_string());
            match Command::new(&nvcc).arg("--version").output() {
                Ok(out) if out.status.success() => {
                    let text = String::from_utf8_lossy(&out.stdout);
                    let release = text.lines().find(|l| l.contains("release")).unwrap_or("present");
                    println!("[ok]   CUDA toolkit (nvcc): {}", release.trim());
                }
                _ => println!(
                    "[warn] CUDA toolkit not found — set CUDA_PATH.\n         PTX/CUDA backend disabled; get CUDA: https://developer.nvidia.com/cuda-downloads"
                ),
            }

            println!(
                "\nMissing native toolchains degrade gracefully to the wgpu/WGSL path (plan §12)."
            );
        }
        ShaderAction::ProfileHardware { export, json } => {
            let runner = WgpuComputeContext::new(1024 * 1024)?;
            let profile = &runner.profile;
            let topology_hash = profile.topology_hash()?;
            if let Some(path) = export {
                std::fs::write(path, profile.to_pretty_json()?.as_bytes())?;
                eprintln!("wrote {} (topology {})", path.display(), topology_hash);
            }
            if *json {
                println!("{}", profile.to_pretty_json()?);
            } else {
                println!("Adapter:        {} ({})", profile.adapter.name, profile.adapter.backend);
                println!("Device type:    {}", profile.adapter.device_type);
                println!("Driver:         {} {}", profile.adapter.driver, profile.adapter.driver_info);
                println!("Memory class:   {}", profile.memory_class);
                println!("Subgroups:      {}", profile.constraints.supports_subgroups);
                println!("Tensor (coopmat): {}", profile.constraints.supports_coopmat);
                println!("RT cores:       {}", profile.constraints.supports_rt_cores);
                println!("Timestamp query: {}", profile.supports_timestamp_query);
                println!(
                    "Max workgroup:  {} invocations, {} bytes shared",
                    profile.constraints.max_invocations_per_workgroup, profile.max_compute_workgroup_storage_size
                );
                println!(
                    "Bind alignment: storage {} / uniform {}",
                    profile.min_storage_buffer_offset_alignment, profile.min_uniform_buffer_offset_alignment
                );
                println!("Topology hash:  {topology_hash}");
            }
        }
        ShaderAction::Generate {
            kernel,
            target,
            schedule,
            out,
            json,
        } => {
            let builtin = parse_kernel(kernel)?;
            let target_backend = target.parse().map_err(|e: String| ForgeError::Emission(e))?;
            let generated = generate_builtin(builtin, schedule.schedule(), target_backend)?;
            if let Some(path) = out {
                std::fs::write(path, generated.source.as_bytes())?;
                eprintln!(
                    "generated {} -> {} ({})",
                    generated.kernel_id,
                    path.display(),
                    generated.source_hash
                );
            } else if *json {
                println!("{}", serde_json::to_string_pretty(&generated)?);
            } else {
                print!("{}", generated.source);
            }
        }
        ShaderAction::Validate {
            input,
            kernel,
            target,
            schedule,
            manifest,
            json,
        } => {
            let target_backend = target.parse().map_err(|e: String| ForgeError::Emission(e))?;
            let (source, generated) = if let Some(path) = input {
                (std::fs::read_to_string(path)?, None)
            } else {
                let builtin = parse_kernel(kernel)?;
                let generated = generate_builtin(builtin, schedule.schedule(), target_backend)?;
                (generated.source.clone(), Some(generated))
            };
            let report = if target_backend == TargetBackend::Wgsl {
                Some(validate_wgsl(&source)?)
            } else {
                match validate_native(&source, target_backend) {
                    Ok(r) => Some(r),
                    Err(ForgeError::WgslValidation(msg)) => {
                        eprintln!("Native validation skipped or failed: {}", msg);
                        None
                    }
                    Err(e) => return Err(Box::new(e)),
                }
            };
            if let Some(path) = manifest {
                let generated = generated.ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--manifest requires a generated kernel (omit --input)",
                    )
                })?;
                if let Some(report) = report.clone() {
                    let record = CertificationManifest::naga_only(&generated, report);
                    write_json(path, &record)?;
                } else {
                    eprintln!("Warning: Validation manifest generation is currently only supported for WGSL targets.");
                }
            }
            if let Some(report) = report {
                if *json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    println!(
                        "Validated {} binding(s), entry point(s): {}",
                        report.binding_count,
                        report.entry_points.join(", ")
                    );
                    if let Some(tool) = report.native_tool_validated {
                        println!("Validation success (via {}): entry points = {:?}", tool, report.entry_points);
                    } else {
                        println!("Validation success (via Naga): entry points = {:?}", report.entry_points);
                    }
                }
            } else {
                println!("Validation skipped for non-WGSL target.");
            }
        }
        ShaderAction::Certify {
            kernel,
            schedule,
            length,
            warmups,
            samples,
            manifest,
            cache_dir,
            dry_run,
        } => {
            let builtin = parse_kernel(kernel)?;
            if *dry_run {
                let sched = schedule.schedule();
                let spec = builtin.spec();
                let generated = generate_builtin(builtin, sched, TargetBackend::Wgsl)?;
                let report = validate_wgsl(&generated.source)?;
                let constraints = AdapterConstraints::portable();
                let schedule_ok = sched.validate(&spec, &constraints).is_ok();
                let adapter_ok = constraints.supports_kernel(&spec).is_ok();
                println!(
                    "DRY-RUN certify {}: naga={} ({} bindings), schedule_valid={}, adapter_supported(portable)={}, gpu_oracle={}",
                    builtin.name(), report.naga_validated, report.binding_count, schedule_ok, adapter_ok, builtin.has_gpu_oracle()
                );
                return Ok(());
            }
            let mut runner = WgpuComputeContext::new(4 * 1024 * 1024)?;
            runner.constraints.supports_kernel(&builtin.spec())?;
            let record = qualia_core_db::wgsl_forge::certify_builtin(
                &mut runner,
                builtin,
                schedule.schedule(),
                *length,
                *warmups,
                *samples,
            )?;
            if let Some(path) = manifest {
                write_json(path, &record)?;
            }
            if let Some(root) = cache_dir {
                let path = ManifestCache::new(root).store_certification(&record)?;
                eprintln!("cached {}", path.display());
            }
            println!(
                "CERTIFIED {} on {}: median {} ns, p95 {} ns",
                record.kernel_id,
                record
                    .adapter
                    .as_ref()
                    .map(|value| value.name.as_str())
                    .unwrap_or("unknown"),
                record
                    .timing
                    .as_ref()
                    .map(|value| value.median_ns)
                    .unwrap_or(0),
                record
                    .timing
                    .as_ref()
                    .map(|value| value.p95_ns)
                    .unwrap_or(0)
            );
            println!(
                "cache key: {}",
                record.cache_key.as_deref().unwrap_or("none")
            );
        }
        ShaderAction::Tune {
            kernel,
            length,
            warmups,
            initial_samples,
            finalist_samples,
            finalists,
            max_candidates,
            manifest,
            cache_dir,
            dry_run,
        } => {
            let builtin = parse_kernel(kernel)?;
            let spec = builtin.spec();
            if *dry_run {
                let constraints = match WgpuComputeContext::new(4 * 1024 * 1024) {
                    Ok(runner) => runner.constraints,
                    Err(_) => AdapterConstraints::portable(),
                };
                let space = ScheduleSpace::default();
                let total = space.workgroup_sizes.len() * space.items_per_invocation.len() * space.vector_widths.len();
                let candidates = space.candidates(&spec, &constraints);
                let adapter_ok = constraints.supports_kernel(&spec);
                let roofline = qualia_core_db::wgsl_forge::roofline_for(builtin, *length as u64);
                println!(
                    "DRY-RUN tune {}: {}/{} schedule(s) survive pruning; adapter_supported={}; gpu_oracle={}",
                    builtin.name(),
                    candidates.len(),
                    total,
                    adapter_ok.is_ok(),
                    builtin.has_gpu_oracle()
                );
                println!(
                    "  roofline @ n={}: {:.3} FLOP/byte ({:?}-bound)",
                    length, roofline.arithmetic_intensity, roofline.bound
                );
                println!("  warp size: {} (non-multiples pruned)", constraints.warp_size);
                for &wg in &space.workgroup_sizes {
                    let kept = candidates.iter().filter(|c| c.workgroup_size == wg).count();
                    let note = if wg % constraints.warp_size == 0 { "" } else { " [warp-pruned]" };
                    println!("    workgroup {wg:>4}: {kept} kept{note}");
                }
                if let Err(error) = adapter_ok {
                    println!("  pruned: {error}");
                }
                return Ok(());
            }
            let mut runner = WgpuComputeContext::new(4 * 1024 * 1024)?;
            let constraints = runner.constraints;
            let result = tune_with(
                &spec,
                &constraints,
                &ScheduleSpace::default(),
                TuningConfig {
                    initial_samples: *initial_samples,
                    finalist_samples: *finalist_samples,
                    finalist_count: *finalists,
                    max_candidates: *max_candidates,
                },
                |schedule, sample_count| {
                    candidate_evaluation(
                        &mut runner,
                        builtin,
                        schedule,
                        *length,
                        *warmups,
                        sample_count,
                    )
                },
            )?;
            println!("\nBest configuration:");
            let generated = generate_builtin(builtin, result.winner.schedule, TargetBackend::Wgsl)?;
            println!("{}", generated.source);
            let record = TuningManifest::new(&generated, runner.adapter.clone(), result)?;
            if let Some(path) = manifest {
                write_json(path, &record)?;
            }
            if let Some(root) = cache_dir {
                let path = ManifestCache::new(root).store_tuning(&record)?;
                eprintln!("cached {}", path.display());
            }
            let winner = &record.result.winner;
            println!(
                "TUNED {} on {}: wg={}, items={}, vector={} -> median {} ns, p95 {} ns",
                record.kernel_id,
                record.adapter.name,
                winner.schedule.workgroup_size,
                winner.schedule.items_per_invocation,
                winner.schedule.vector_width,
                winner.timing.median_ns,
                winner.timing.p95_ns
            );
            println!(
                "evaluated {}, rejected {}, cache key {}",
                record.result.evaluated_candidates,
                record.result.rejected_candidates,
                record.cache_key
            );
        }
        ShaderAction::AutoTuneAll {
            length,
            warmups,
            max_candidates,
            cache_dir,
            update_local_manifest,
            dry_run,
        } => {
            let mut runner = WgpuComputeContext::new(4 * 1024 * 1024)?;
            let topology_hash = runner.profile.topology_hash()?;
            let constraints = runner.constraints;
            let adapter_name = runner.adapter.name.clone();
            let cache = cache_dir.as_ref().map(|p| ManifestCache::new(p.clone()));
            println!("auto-tune-all on {adapter_name} (topology {topology_hash})");
            for builtin in BuiltinKernel::ALL {
                let spec = builtin.spec();
                let name = builtin.name();
                if constraints.supports_kernel(&spec).is_err() {
                    println!("  {name:<12} SKIP (adapter lacks required intrinsics)");
                    continue;
                }
                if !builtin.has_gpu_oracle() {
                    println!("  {name:<12} SKIP (no GPU oracle wired yet)");
                    continue;
                }
                if let Some(cache) = &cache {
                    if let Some(existing) = cache.load_tuning_for_topology(&topology_hash, name)? {
                        let winner = &existing.result.winner;
                        println!(
                            "  {name:<12} CACHED wg={} items={} -> median {} ns",
                            winner.schedule.workgroup_size,
                            winner.schedule.items_per_invocation,
                            winner.timing.median_ns
                        );
                        continue;
                    }
                }
                if *dry_run {
                    println!("  {name:<12} WOULD TUNE");
                    continue;
                }
                let result = tune_with(
                    &spec,
                    &constraints,
                    &ScheduleSpace::default(),
                    TuningConfig {
                        initial_samples: 3,
                        finalist_samples: 11,
                        finalist_count: 6,
                        max_candidates: *max_candidates,
                    },
                    |schedule, sample_count| {
                        candidate_evaluation(&mut runner, builtin, schedule, *length, *warmups, sample_count)
                    },
                );
                match result {
                    Ok(result) => {
                        let generated =
                            generate_builtin(builtin, result.winner.schedule, TargetBackend::Wgsl)?;
                        let record = TuningManifest::new(&generated, runner.adapter.clone(), result)?;
                        let winner = &record.result.winner;
                        println!(
                            "  {name:<12} TUNED wg={} items={} vec={} -> median {} ns, p95 {} ns",
                            winner.schedule.workgroup_size,
                            winner.schedule.items_per_invocation,
                            winner.schedule.vector_width,
                            winner.timing.median_ns,
                            winner.timing.p95_ns
                        );
                        if *update_local_manifest {
                            if let Some(cache) = &cache {
                                let path =
                                    cache.store_tuning_for_topology(&topology_hash, name, &record)?;
                                eprintln!("    cached {}", path.display());
                            }
                        }
                    }
                    Err(error) => println!("  {name:<12} FAILED: {error}"),
                }
            }
        }
    }
    Ok(())
}

fn parse_kernel(value: &str) -> Result<BuiltinKernel, Box<dyn std::error::Error>> {
    Ok(BuiltinKernel::from_str(value)?)
}

fn write_json<T: serde::Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(value)?;
    std::fs::write(path, json.as_bytes())?;
    eprintln!("wrote {}", path.display());
    Ok(())
}
