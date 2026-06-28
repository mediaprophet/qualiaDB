use std::path::{Path, PathBuf};
use std::str::FromStr;

use clap::{Args, Subcommand};
use qualia_core_db::wgsl_forge::{
    candidate_evaluation, generate_builtin, tune_with, validate_wgsl, BuiltinKernel,
    CertificationManifest, GpuForgeRunner, ManifestCache, Schedule, ScheduleSpace, TuningConfig,
    TuningManifest,
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
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum ShaderAction {
    /// List deterministic kernels currently known to WGSL Forge.
    ListKernels,
    /// Generate one deterministic WGSL module.
    Generate {
        #[arg(default_value = "affine-f32")]
        kernel: String,
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
        #[arg(long, default_value = "affine-f32")]
        kernel: String,
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
        ShaderAction::Generate {
            kernel,
            schedule,
            out,
            json,
        } => {
            let builtin = parse_kernel(kernel)?;
            let generated = generate_builtin(builtin, schedule.schedule())?;
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
            schedule,
            manifest,
            json,
        } => {
            let (source, generated) = if let Some(path) = input {
                (std::fs::read_to_string(path)?, None)
            } else {
                let generated = generate_builtin(parse_kernel(kernel)?, schedule.schedule())?;
                (generated.source.clone(), Some(generated))
            };
            let report = validate_wgsl(&source)?;
            if let Some(path) = manifest {
                let generated = generated.ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--manifest requires a generated kernel (omit --input)",
                    )
                })?;
                let record = CertificationManifest::naga_only(&generated, report.clone());
                write_json(path, &record)?;
            }
            if *json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "Naga validated {} binding(s), entry point(s): {}",
                    report.binding_count,
                    report.entry_points.join(", ")
                );
                println!("source hash: {}", report.source_hash);
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
        } => {
            let builtin = parse_kernel(kernel)?;
            let runner = GpuForgeRunner::new()?;
            let record = qualia_core_db::wgsl_forge::certify_builtin(
                &runner,
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
        } => {
            let builtin = parse_kernel(kernel)?;
            let spec = builtin.spec();
            let runner = GpuForgeRunner::new()?;
            let constraints = runner.constraints();
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
                        &runner,
                        builtin,
                        schedule,
                        *length,
                        *warmups,
                        sample_count,
                    )
                },
            )?;
            let generated = generate_builtin(builtin, result.winner.schedule)?;
            let record = TuningManifest::new(&generated, runner.adapter().clone(), result)?;
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
