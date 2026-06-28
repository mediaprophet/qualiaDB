use std::sync::mpsc;
use std::time::Instant;

use super::{
    compare_f32, emit_wgsl, validate_wgsl, AdapterConstraints, AdapterIdentity, BuiltinKernel,
    CandidateEvaluation, CertificationManifest, ForgeError, GeneratedShader, OracleCase,
    OracleTolerance, Schedule, TimingSource, TimingSummary, ValidationLevel,
};

#[derive(Debug, Clone)]
pub struct GpuEvaluation {
    pub adapter: AdapterIdentity,
    pub constraints: AdapterConstraints,
    pub oracle: super::ComparisonReport,
    pub timing: TimingSummary,
    pub samples_ns: Vec<u64>,
}

pub struct GpuForgeRunner {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter: AdapterIdentity,
    constraints: AdapterConstraints,
    timestamp_supported: bool,
    timestamp_period_ns: f32,
}

impl GpuForgeRunner {
    pub fn new() -> Result<Self, ForgeError> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))
        .map_err(|error| ForgeError::GpuUnavailable(error.to_string()))?;
        let info = adapter.get_info();
        let available_features = adapter.features();
        let required_features = available_features & wgpu::Features::TIMESTAMP_QUERY;
        let timestamp_supported = required_features.contains(wgpu::Features::TIMESTAMP_QUERY);
        let limits = adapter.limits();
        let constraints = AdapterConstraints::from_wgpu_limits(&limits);
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_features,
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }))
        .map_err(|error| ForgeError::GpuUnavailable(error.to_string()))?;
        let timestamp_period_ns = if timestamp_supported {
            queue.get_timestamp_period()
        } else {
            0.0
        };
        Ok(Self {
            device,
            queue,
            adapter: AdapterIdentity {
                name: info.name,
                vendor: info.vendor,
                device: info.device,
                device_type: format!("{:?}", info.device_type),
                backend: format!("{:?}", info.backend),
                driver: info.driver,
                driver_info: info.driver_info,
            },
            constraints,
            timestamp_supported,
            timestamp_period_ns,
        })
    }

    pub fn adapter(&self) -> &AdapterIdentity {
        &self.adapter
    }

    pub const fn constraints(&self) -> AdapterConstraints {
        self.constraints
    }

    pub fn evaluate(
        &self,
        generated: &GeneratedShader,
        case: &OracleCase,
        warmups: usize,
        sample_count: usize,
        tolerance: OracleTolerance,
    ) -> Result<GpuEvaluation, ForgeError> {
        if sample_count == 0 {
            return Err(ForgeError::GpuValidation(
                "sample count must be non-zero".to_string(),
            ));
        }
        let kernel = BuiltinKernel::AffineF32.spec();
        generated.schedule.validate(&kernel, &self.constraints)?;
        let expected = emit_wgsl(&kernel, generated.schedule)?;
        if generated != &expected {
            return Err(ForgeError::GpuValidation(
                "generated shader does not match deterministic re-emission".to_string(),
            ));
        }
        validate_wgsl(&generated.source)?;

        let error_scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("qualia-wgsl-forge"),
                source: wgpu::ShaderSource::Wgsl(generated.source.as_str().into()),
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("qualia-wgsl-forge-affine"),
                layout: None,
                module: &shader,
                entry_point: Some(kernel.entry_point.as_str()),
                compilation_options: Default::default(),
                cache: None,
            });
        if let Some(error) = pollster::block_on(error_scope.pop()) {
            return Err(ForgeError::GpuValidation(error.to_string()));
        }

        let input_bytes = bytemuck::cast_slice(case.input.as_slice());
        let input = create_buffer(
            &self.device,
            &self.queue,
            "forge-input",
            input_bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let output_bytes = (case.input.len() * size_of::<f32>()).max(4) as u64;
        let output = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("forge-output"),
            size: output_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let params = create_buffer(
            &self.device,
            &self.queue,
            "forge-params",
            bytemuck::bytes_of(&case.params),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("forge-affine-bind-group"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params.as_entire_binding(),
                },
            ],
        });
        let dispatch_x = generated.schedule.dispatch_workgroups(case.input.len());

        for _ in 0..warmups {
            self.dispatch(&pipeline, &bind_group, dispatch_x, None)?;
        }

        let timestamp_resources = self
            .timestamp_supported
            .then(|| TimestampResources::new(&self.device));
        let mut samples = Vec::with_capacity(sample_count);
        for _ in 0..sample_count {
            samples.push(self.dispatch(
                &pipeline,
                &bind_group,
                dispatch_x,
                timestamp_resources.as_ref(),
            )?);
        }
        if samples.iter().any(|sample| *sample == 0) {
            return Err(ForgeError::GpuValidation(
                "GPU produced a zero-duration timing sample".to_string(),
            ));
        }

        let actual = read_f32_buffer(&self.device, &self.queue, &output, case.input.len())?;
        let oracle = compare_f32(&case.expected, &actual, tolerance);
        if !oracle.passed() {
            return Err(ForgeError::OracleMismatch(format!(
                "{} mismatches; first={:?}, max_abs={}, max_rel={}",
                oracle.mismatch_count,
                oracle.first_mismatch,
                oracle.max_absolute_error,
                oracle.max_relative_error
            )));
        }
        let source = if self.timestamp_supported {
            TimingSource::GpuTimestamp
        } else {
            TimingSource::CompletionClock
        };
        let timing = TimingSummary::from_samples(source, &samples).ok_or_else(|| {
            ForgeError::GpuValidation("GPU produced no timing samples".to_string())
        })?;
        Ok(GpuEvaluation {
            adapter: self.adapter.clone(),
            constraints: self.constraints,
            oracle,
            timing,
            samples_ns: samples,
        })
    }

    fn dispatch(
        &self,
        pipeline: &wgpu::ComputePipeline,
        bind_group: &wgpu::BindGroup,
        dispatch_x: u32,
        timestamp: Option<&TimestampResources>,
    ) -> Result<u64, ForgeError> {
        let started = Instant::now();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("forge-dispatch"),
            });
        {
            let timestamp_writes = timestamp.map(|resources| wgpu::ComputePassTimestampWrites {
                query_set: &resources.query_set,
                beginning_of_pass_write_index: Some(0),
                end_of_pass_write_index: Some(1),
            });
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("forge-compute-pass"),
                timestamp_writes,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups(dispatch_x, 1, 1);
        }
        if let Some(resources) = timestamp {
            encoder.resolve_query_set(&resources.query_set, 0..2, &resources.resolve, 0);
            encoder.copy_buffer_to_buffer(&resources.resolve, 0, &resources.staging, 0, 16);
        }
        self.queue.submit(Some(encoder.finish()));

        if let Some(resources) = timestamp {
            let bytes = map_read(&self.device, &resources.staging)?;
            let ticks: &[u64] = bytemuck::cast_slice(&bytes);
            let elapsed = ticks
                .get(1)
                .copied()
                .unwrap_or(0)
                .saturating_sub(ticks.first().copied().unwrap_or(0));
            drop(bytes);
            resources.staging.unmap();
            Ok((elapsed as f64 * self.timestamp_period_ns as f64) as u64)
        } else {
            let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
            Ok(started.elapsed().as_nanos().min(u64::MAX as u128) as u64)
        }
    }
}

struct TimestampResources {
    query_set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    staging: wgpu::Buffer,
}

impl TimestampResources {
    fn new(device: &wgpu::Device) -> Self {
        Self {
            query_set: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("forge-timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count: 2,
            }),
            resolve: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("forge-timestamp-resolve"),
                size: 16,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            staging: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("forge-timestamp-staging"),
                size: 16,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
        }
    }
}

fn create_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &'static str,
    bytes: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: bytes.len().max(4) as u64,
        usage,
        mapped_at_creation: false,
    });
    if !bytes.is_empty() {
        queue.write_buffer(&buffer, 0, bytes);
    }
    buffer
}

fn map_read(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Result<wgpu::BufferView, ForgeError> {
    let slice = buffer.slice(..);
    let (sender, receiver) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    receiver
        .recv()
        .map_err(|error| ForgeError::GpuValidation(error.to_string()))?
        .map_err(|error| ForgeError::GpuValidation(error.to_string()))?;
    Ok(slice.get_mapped_range())
}

fn read_f32_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: &wgpu::Buffer,
    length: usize,
) -> Result<Vec<f32>, ForgeError> {
    let size = (length * size_of::<f32>()).max(4) as u64;
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("forge-output-staging"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("forge-output-copy"),
    });
    encoder.copy_buffer_to_buffer(source, 0, &staging, 0, size);
    queue.submit(Some(encoder.finish()));
    let bytes = map_read(device, &staging)?;
    let output = bytemuck::cast_slice::<u8, f32>(&bytes)[..length].to_vec();
    drop(bytes);
    staging.unmap();
    Ok(output)
}

pub fn evaluate_builtin(
    runner: &GpuForgeRunner,
    builtin: BuiltinKernel,
    schedule: Schedule,
    length: usize,
    warmups: usize,
    samples: usize,
) -> Result<(GeneratedShader, GpuEvaluation), ForgeError> {
    let kernel = builtin.spec();
    schedule.validate(&kernel, &runner.constraints)?;
    let generated = emit_wgsl(&kernel, schedule)?;
    let case = OracleCase::affine(length, 0x5141_4C49_4157_4753, 1.618_034, -0.125);
    let evaluation = runner.evaluate(
        &generated,
        &case,
        warmups,
        samples,
        OracleTolerance::default(),
    )?;
    Ok((generated, evaluation))
}

pub fn certify_builtin(
    runner: &GpuForgeRunner,
    builtin: BuiltinKernel,
    schedule: Schedule,
    length: usize,
    warmups: usize,
    samples: usize,
) -> Result<CertificationManifest, ForgeError> {
    let (generated, evaluation) =
        evaluate_builtin(runner, builtin, schedule, length, warmups, samples)?;
    let validation = validate_wgsl(&generated.source)?;
    let cache_key =
        evaluation
            .adapter
            .cache_key(&generated.semantic_hash, &generated.source_hash, schedule)?;
    Ok(CertificationManifest {
        forge_schema_version: super::FORGE_SCHEMA_VERSION,
        crate_version: env!("CARGO_PKG_VERSION").to_string(),
        wgpu_api_version: super::WGPU_API_VERSION.to_string(),
        naga_api_version: super::NAGA_API_VERSION.to_string(),
        kernel_id: generated.kernel_id,
        semantic_hash: generated.semantic_hash,
        source_hash: generated.source_hash,
        schedule,
        validation_level: ValidationLevel::Certified,
        validation,
        adapter: Some(evaluation.adapter),
        oracle: Some(evaluation.oracle),
        timing: Some(evaluation.timing),
        cache_key: Some(cache_key),
    })
}

pub fn candidate_evaluation(
    runner: &GpuForgeRunner,
    builtin: BuiltinKernel,
    schedule: Schedule,
    length: usize,
    warmups: usize,
    samples: usize,
) -> Result<CandidateEvaluation, ForgeError> {
    let (_, evaluation) = evaluate_builtin(runner, builtin, schedule, length, warmups, samples)?;
    let timing_source = evaluation.timing.source;
    Ok(CandidateEvaluation {
        oracle: evaluation.oracle,
        timing_source,
        samples_ns: evaluation.samples_ns,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a native wgpu adapter"]
    fn generated_affine_certifies_on_real_gpu() {
        let runner = GpuForgeRunner::new().expect("adapter");
        let manifest = certify_builtin(
            &runner,
            BuiltinKernel::AffineF32,
            Schedule {
                workgroup_size: 64,
                items_per_invocation: 2,
                vector_width: 4,
            },
            4_099,
            2,
            5,
        )
        .expect("certification");
        assert_eq!(manifest.validation_level, ValidationLevel::Certified);
        assert!(manifest.oracle.unwrap().passed());
    }
}
