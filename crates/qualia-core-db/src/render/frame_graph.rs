//! Deterministic, allocation-free frame graph resource and pass dependency model.
//!
//! Provides compile-time and runtime pass dependency DAG resolution, capability-driven
//! pass pruning (e.g. no-AO, no-HDR), resource lifetime tracking, and transient VRAM
//! estimation. Adheres strictly to the Rule 0 zero-heap execution contract.

use super::quality_profiles::RenderQualityProfile;
use super::temporal_resolve::TemporalResolveConfig;

/// Maximum number of render passes in a single frame graph.
pub const MAX_PASSES: usize = 16;
/// Maximum number of distinct resources tracked in the frame graph.
pub const MAX_RESOURCES: usize = 24;
/// Maximum number of read or write dependencies per pass.
pub const MAX_PASS_DEPENDENCIES: usize = 8;

/// Stable identity of standard render targets and textures in the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrameResourceId {
    SceneColor,
    LinearDepth,
    Normals,
    MotionVectors,
    ReactiveMask,
    TemporalColor,
    HistoryColor,
    ShadowCascade(u8),
    AoVisibility,
    BloomIntermediate(u8),
    VelocityHistory,
    OutputSurface,
    Custom(u64),
}

/// Format of a managed frame target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceFormat {
    Rgba16Float,
    Rgba8Unorm,
    Rgba8Srgb,
    Depth32Float,
    R8Unorm,
    Rg16Float,
}

impl ResourceFormat {
    /// Bytes per pixel for format.
    pub const fn bytes_per_pixel(self) -> u32 {
        match self {
            Self::Rgba16Float => 8,
            Self::Rgba8Unorm | Self::Rgba8Srgb | Self::Depth32Float => 4,
            Self::R8Unorm => 1,
            Self::Rg16Float => 4,
        }
    }
}

/// Description of a frame target resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceDesc {
    pub id: FrameResourceId,
    pub format: ResourceFormat,
    pub width: u32,
    pub height: u32,
    pub is_transient: bool,
}

impl ResourceDesc {
    /// Compute exact byte requirement for this resource with checked arithmetic.
    pub fn byte_size(&self) -> Option<u64> {
        let pixels = u64::from(self.width.max(1)).checked_mul(u64::from(self.height.max(1)))?;
        pixels.checked_mul(u64::from(self.format.bytes_per_pixel()))
    }
}

/// Unique identifier for each modular rendering pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PassId {
    ShadowCascade(u8),
    DepthPrepass,
    AoCompute,
    AoBilateralFilter,
    ForwardLighting,
    TemporalResolve,
    HistoryPublication,
    Skybox,
    BloomExtract,
    BloomBlur(u8),
    BloomComposite,
    SdrOutputComposite,
    SemanticPicking,
    Custom(u64),
}

/// Single pass registration node.
#[derive(Clone, Copy, Debug)]
pub struct PassNode {
    pub id: PassId,
    pub enabled: bool,
    pub reads: [Option<FrameResourceId>; MAX_PASS_DEPENDENCIES],
    pub read_count: usize,
    pub writes: [Option<FrameResourceId>; MAX_PASS_DEPENDENCIES],
    pub write_count: usize,
}

impl PassNode {
    pub const fn new(id: PassId) -> Self {
        Self {
            id,
            enabled: true,
            reads: [None; MAX_PASS_DEPENDENCIES],
            read_count: 0,
            writes: [None; MAX_PASS_DEPENDENCIES],
            write_count: 0,
        }
    }

    pub fn with_read(mut self, res: FrameResourceId) -> Self {
        if self.read_count < MAX_PASS_DEPENDENCIES {
            self.reads[self.read_count] = Some(res);
            self.read_count += 1;
        }
        self
    }

    pub fn with_write(mut self, res: FrameResourceId) -> Self {
        if self.write_count < MAX_PASS_DEPENDENCIES {
            self.writes[self.write_count] = Some(res);
            self.write_count += 1;
        }
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Fail-closed errors from frame graph compilation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameGraphError {
    PassLimitExceeded,
    ResourceLimitExceeded,
    CyclicDependency,
    UnsatisfiedDependency,
    BudgetExceeded { required: u64, budget: u64 },
}

/// Output execution schedule for a compiled frame.
#[derive(Clone, Copy, Debug)]
pub struct CompiledSchedule {
    pub passes: [Option<PassId>; MAX_PASSES],
    pub pass_count: usize,
    pub peak_transient_bytes: u64,
    pub total_resource_bytes: u64,
}

/// Deterministic, stack-allocated frame graph builder and scheduler.
pub struct FrameGraphBuilder {
    passes: [Option<PassNode>; MAX_PASSES],
    pass_count: usize,
    resources: [Option<ResourceDesc>; MAX_RESOURCES],
    resource_count: usize,
}

impl Default for FrameGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameGraphBuilder {
    pub const fn new() -> Self {
        Self {
            passes: [None; MAX_PASSES],
            pass_count: 0,
            resources: [None; MAX_RESOURCES],
            resource_count: 0,
        }
    }

    /// Register a resource target.
    pub fn add_resource(&mut self, desc: ResourceDesc) -> Result<(), FrameGraphError> {
        for slot in self.resources[..self.resource_count].iter().flatten() {
            if slot.id == desc.id {
                return Ok(()); // Idempotent registration
            }
        }
        if self.resource_count >= MAX_RESOURCES {
            return Err(FrameGraphError::ResourceLimitExceeded);
        }
        self.resources[self.resource_count] = Some(desc);
        self.resource_count += 1;
        Ok(())
    }

    /// Register a pass node.
    pub fn add_pass(&mut self, pass: PassNode) -> Result<(), FrameGraphError> {
        if self.pass_count >= MAX_PASSES {
            return Err(FrameGraphError::PassLimitExceeded);
        }
        self.passes[self.pass_count] = Some(pass);
        self.pass_count += 1;
        Ok(())
    }

    /// Configure active passes and resource dimensions from a RenderQualityProfile.
    pub fn configure_from_profile(
        &mut self,
        profile: &RenderQualityProfile,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Result<(), FrameGraphError> {
        self.configure_from_profile_with_temporal(
            profile,
            viewport_width,
            viewport_height,
            TemporalResolveConfig::disabled(),
        )
    }

    /// Configure the frame graph with an explicitly admitted temporal output path.
    ///
    /// The temporal pass is opt-in and fail-closed: the existing renderer must declare both
    /// motion-vector and reactive-mask inputs before this graph adds accumulation. Linear depth
    /// is already owned by this graph. The final bloom/SDR output pass consumes the resolved
    /// scene colour, while history publication remains a distinct scheduled pass.
    pub fn configure_from_profile_with_temporal(
        &mut self,
        profile: &RenderQualityProfile,
        viewport_width: u32,
        viewport_height: u32,
        temporal: TemporalResolveConfig,
    ) -> Result<(), FrameGraphError> {
        let render_w =
            ((viewport_width as u64 * profile.render_scale_bps as u64) / 10_000).max(1) as u32;
        let render_h =
            ((viewport_height as u64 * profile.render_scale_bps as u64) / 10_000).max(1) as u32;

        let color_format = if profile.hdr_enabled {
            ResourceFormat::Rgba16Float
        } else {
            ResourceFormat::Rgba8Unorm
        };

        self.add_resource(ResourceDesc {
            id: FrameResourceId::SceneColor,
            format: color_format,
            width: render_w,
            height: render_h,
            is_transient: false,
        })?;

        self.add_resource(ResourceDesc {
            id: FrameResourceId::LinearDepth,
            format: ResourceFormat::Depth32Float,
            width: render_w,
            height: render_h,
            is_transient: true,
        })?;

        self.add_resource(ResourceDesc {
            id: FrameResourceId::OutputSurface,
            format: ResourceFormat::Rgba8Srgb,
            width: viewport_width.max(1),
            height: viewport_height.max(1),
            is_transient: false,
        })?;

        if profile.shadows_enabled && profile.shadow_map_dimension > 0 {
            for cascade in 0..profile.shadow_cascade_count {
                self.add_resource(ResourceDesc {
                    id: FrameResourceId::ShadowCascade(cascade),
                    format: ResourceFormat::Depth32Float,
                    width: profile.shadow_map_dimension as u32,
                    height: profile.shadow_map_dimension as u32,
                    is_transient: true,
                })?;
                self.add_pass(
                    PassNode::new(PassId::ShadowCascade(cascade))
                        .with_write(FrameResourceId::ShadowCascade(cascade)),
                )?;
            }
        }

        if profile.ambient_occlusion_enabled {
            let ao_w = ((render_w as u64 * profile.ao_scale_bps as u64) / 10_000).max(1) as u32;
            let ao_h = ((render_h as u64 * profile.ao_scale_bps as u64) / 10_000).max(1) as u32;
            self.add_resource(ResourceDesc {
                id: FrameResourceId::Normals,
                format: ResourceFormat::Rgba8Unorm,
                width: render_w,
                height: render_h,
                is_transient: true,
            })?;
            self.add_resource(ResourceDesc {
                id: FrameResourceId::AoVisibility,
                format: ResourceFormat::R8Unorm,
                width: ao_w,
                height: ao_h,
                is_transient: true,
            })?;
            self.add_pass(
                PassNode::new(PassId::DepthPrepass)
                    .with_write(FrameResourceId::LinearDepth)
                    .with_write(FrameResourceId::Normals),
            )?;
            self.add_pass(
                PassNode::new(PassId::AoCompute)
                    .with_read(FrameResourceId::LinearDepth)
                    .with_read(FrameResourceId::Normals)
                    .with_write(FrameResourceId::AoVisibility),
            )?;
        }

        let mut forward =
            PassNode::new(PassId::ForwardLighting).with_write(FrameResourceId::SceneColor);

        if profile.ambient_occlusion_enabled {
            forward = forward
                .with_read(FrameResourceId::AoVisibility)
                .with_read(FrameResourceId::LinearDepth);
        } else {
            forward = forward.with_write(FrameResourceId::LinearDepth);
        }
        if profile.shadows_enabled {
            for cascade in 0..profile.shadow_cascade_count {
                forward = forward.with_read(FrameResourceId::ShadowCascade(cascade));
            }
        }
        self.add_pass(forward)?;

        let temporal_enabled = temporal.can_schedule();
        if temporal_enabled {
            self.add_resource(ResourceDesc {
                id: FrameResourceId::MotionVectors,
                format: ResourceFormat::Rg16Float,
                width: render_w,
                height: render_h,
                is_transient: true,
            })?;
            self.add_resource(ResourceDesc {
                id: FrameResourceId::ReactiveMask,
                format: ResourceFormat::R8Unorm,
                width: render_w,
                height: render_h,
                is_transient: true,
            })?;
            self.add_resource(ResourceDesc {
                id: FrameResourceId::TemporalColor,
                format: color_format,
                width: render_w,
                height: render_h,
                is_transient: true,
            })?;
            self.add_resource(ResourceDesc {
                id: FrameResourceId::HistoryColor,
                format: color_format,
                width: render_w,
                height: render_h,
                is_transient: false,
            })?;

            let temporal_pass = PassNode::new(PassId::TemporalResolve)
                .with_read(FrameResourceId::SceneColor)
                .with_read(FrameResourceId::LinearDepth)
                .with_read(FrameResourceId::MotionVectors)
                .with_read(FrameResourceId::ReactiveMask)
                .with_write(FrameResourceId::TemporalColor);
            // `HistoryColor` is a persistent previous-frame input. The current graph compiler
            // models same-frame dependencies, so registering it as a read here would create a
            // false cycle with `HistoryPublication`, which writes the next history image. The
            // temporal policy still exposes `reads_history()` to the backend binding layer.
            self.add_pass(temporal_pass)?;
            self.add_pass(
                PassNode::new(PassId::HistoryPublication)
                    .with_read(FrameResourceId::TemporalColor)
                    .with_write(FrameResourceId::HistoryColor),
            )?;
        }

        let output_source = if temporal_enabled {
            FrameResourceId::TemporalColor
        } else {
            FrameResourceId::SceneColor
        };

        if profile.bloom_enabled && profile.hdr_enabled {
            self.add_resource(ResourceDesc {
                id: FrameResourceId::BloomIntermediate(0),
                format: ResourceFormat::Rgba16Float,
                width: render_w / 2,
                height: render_h / 2,
                is_transient: true,
            })?;
            self.add_pass(
                PassNode::new(PassId::BloomExtract)
                    .with_read(output_source)
                    .with_write(FrameResourceId::BloomIntermediate(0)),
            )?;
            self.add_pass(
                PassNode::new(PassId::BloomComposite)
                    .with_read(output_source)
                    .with_read(FrameResourceId::BloomIntermediate(0))
                    .with_write(FrameResourceId::OutputSurface),
            )?;
        } else {
            self.add_pass(
                PassNode::new(PassId::SdrOutputComposite)
                    .with_read(output_source)
                    .with_write(FrameResourceId::OutputSurface),
            )?;
        }

        Ok(())
    }

    /// Compile passes into a deterministic execution schedule using topological sorting.
    /// Prunes disabled passes and unreferenced transient resources.
    pub fn compile(
        &self,
        vram_target_budget: Option<u64>,
    ) -> Result<CompiledSchedule, FrameGraphError> {
        let mut active_indices = [0usize; MAX_PASSES];
        let mut active_count = 0;
        for (i, slot) in self.passes[..self.pass_count].iter().enumerate() {
            if let Some(pass) = slot {
                if pass.enabled {
                    active_indices[active_count] = i;
                    active_count += 1;
                }
            }
        }

        // In-degree array for active passes
        let mut in_degrees = [0u16; MAX_PASSES];
        for (ai, &p_idx) in active_indices[..active_count].iter().enumerate() {
            let pass = self.passes[p_idx].as_ref().unwrap();
            for read_slot in pass.reads[..pass.read_count].iter().flatten() {
                // Check if another active pass writes to this resource
                for (other_ai, &other_p_idx) in active_indices[..active_count].iter().enumerate() {
                    if ai == other_ai {
                        continue;
                    }
                    let other_pass = self.passes[other_p_idx].as_ref().unwrap();
                    if other_pass.writes[..other_pass.write_count]
                        .iter()
                        .flatten()
                        .any(|w| w == read_slot)
                    {
                        in_degrees[ai] += 1;
                    }
                }
            }
        }

        // Kahn's algorithm
        let mut schedule_passes = [None; MAX_PASSES];
        let mut scheduled_count = 0;
        let mut visited = [false; MAX_PASSES];

        while scheduled_count < active_count {
            let mut found = false;
            for (ai, &p_idx) in active_indices[..active_count].iter().enumerate() {
                if !visited[ai] && in_degrees[ai] == 0 {
                    visited[ai] = true;
                    let pass = self.passes[p_idx].as_ref().unwrap();
                    schedule_passes[scheduled_count] = Some(pass.id);
                    scheduled_count += 1;
                    found = true;

                    // Reduce in-degrees of dependent passes
                    for write_slot in pass.writes[..pass.write_count].iter().flatten() {
                        for (other_ai, &other_p_idx) in
                            active_indices[..active_count].iter().enumerate()
                        {
                            if !visited[other_ai] {
                                let other_pass = self.passes[other_p_idx].as_ref().unwrap();
                                if other_pass.reads[..other_pass.read_count]
                                    .iter()
                                    .flatten()
                                    .any(|r| r == write_slot)
                                {
                                    in_degrees[other_ai] = in_degrees[other_ai].saturating_sub(1);
                                }
                            }
                        }
                    }
                    break;
                }
            }
            if !found {
                return Err(FrameGraphError::CyclicDependency);
            }
        }

        // Calculate total and peak transient resource byte sizes
        let mut total_bytes = 0u64;
        let mut peak_transient_bytes = 0u64;
        for res_slot in self.resources[..self.resource_count].iter().flatten() {
            let bytes = res_slot.byte_size().unwrap_or(0);
            total_bytes = total_bytes.saturating_add(bytes);
            if res_slot.is_transient {
                peak_transient_bytes = peak_transient_bytes.saturating_add(bytes);
            }
        }

        if let Some(budget) = vram_target_budget {
            if total_bytes > budget {
                return Err(FrameGraphError::BudgetExceeded {
                    required: total_bytes,
                    budget,
                });
            }
        }

        Ok(CompiledSchedule {
            passes: schedule_passes,
            pass_count: scheduled_count,
            peak_transient_bytes,
            total_resource_bytes: total_bytes,
        })
    }
}

#[cfg(test)]
#[path = "frame_graph_tests.rs"]
mod tests;
