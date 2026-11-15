//! `WgpuRenderer` state, VRAM residency admission, and target management.

use super::pipelines::{ProjectorVertex, ScreenVertex};
#[cfg(not(feature = "qualia"))]
use super::vegetation_effects;
use super::{
    bounded_offscreen_extent, offscreen_rgba8_bytes, probe::capability_report_from_adapter, Camera,
    ScreenPoint, WebGpuCapabilityReport,
};

#[cfg(feature = "qualia")]
pub(super) type GraphicsReservation = qualia_core_db::gpu_context::VramReservation<'static>;
#[cfg(not(feature = "qualia"))]
pub(super) type GraphicsReservation = ();
#[cfg(feature = "qualia")]
pub(super) type GraphicsClass = qualia_core_db::gpu_context::VramResourceClass;
#[cfg(not(feature = "qualia"))]
#[derive(Clone, Copy)]
pub(super) enum GraphicsClass {
    Geometry,
    FrameTarget,
    UploadStaging,
}

#[derive(Default)]
pub(super) struct RendererReservations {
    pub(super) frame_target: Option<GraphicsReservation>,
    pub(super) geometry: Option<GraphicsReservation>,
}

#[cfg(feature = "qualia")]
pub(super) fn reserve_graphics_bytes(
    class: GraphicsClass,
    bytes: u64,
    label: &str,
) -> Result<GraphicsReservation, String> {
    qualia_core_db::gpu_context::global_vram_ledger()
        .try_reserve_graphics(class, bytes)
        .map_err(|e| format!("webizen-render {label} admission failed: {e}"))
}

#[cfg(not(feature = "qualia"))]
pub(super) fn reserve_graphics_bytes(
    _class: GraphicsClass,
    _bytes: u64,
    _label: &str,
) -> Result<GraphicsReservation, String> {
    Ok(())
}

pub(super) fn vertex_buffer_residency_bytes(max_vertices: usize) -> Option<u64> {
    (max_vertices as u64)
        .checked_mul(std::mem::size_of::<ScreenVertex>() as u64)?
        .checked_add(
            (max_vertices as u64).checked_mul(std::mem::size_of::<ProjectorVertex>() as u64)?,
        )
}

pub(super) fn reserve_vertex_buffer_residency() -> Result<GraphicsReservation, String> {
    let bytes = vertex_buffer_residency_bytes(10_000)
        .ok_or_else(|| "webizen-render vertex-buffer size overflow".to_string())?;
    reserve_graphics_bytes(GraphicsClass::Geometry, bytes, "vertex buffer")
}

pub(super) fn reserve_offscreen_extent(
    requested_width: u32,
    requested_height: u32,
) -> Result<(u32, u32, GraphicsReservation), String> {
    #[allow(unused_mut)]
    let (mut width, mut height) = bounded_offscreen_extent(requested_width, requested_height);
    #[cfg(feature = "qualia")]
    {
        for _ in 0..33 {
            let bytes = offscreen_rgba8_bytes(width, height)
                .ok_or_else(|| "webizen-render target size overflow".to_string())?;
            match reserve_graphics_bytes(GraphicsClass::FrameTarget, bytes, "offscreen target") {
                Ok(reservation) => return Ok((width, height, reservation)),
                Err(_) if width > 1 || height > 1 => {
                    width = width.div_ceil(2);
                    height = height.div_ceil(2);
                }
                Err(error) => return Err(error),
            }
        }
        return Err("webizen-render could not admit a minimum offscreen target".into());
    }
    #[cfg(not(feature = "qualia"))]
    {
        let _ = offscreen_rgba8_bytes(width, height)
            .ok_or_else(|| "webizen-render target size overflow".to_string())?;
        Ok((width, height, ()))
    }
}

/// Where the renderer draws each frame.
pub(super) enum RenderTarget<'a> {
    /// Windowed swapchain.
    Surface {
        surface: etalon::Surface<'a>,
        config: etalon::SurfaceConfiguration,
    },
    /// Headless render-target texture with CPU readback.
    Offscreen {
        texture: etalon::Texture,
        format: etalon::TextureFormat,
        width: u32,
        height: u32,
    },
}

/// A single acquired frame's drawable view, plus (for surfaces) the swapchain
/// texture that must be presented after submission.
pub(super) enum Frame {
    Surface {
        texture: etalon::SurfaceTexture,
        view: etalon::TextureView,
    },
    Offscreen {
        view: etalon::TextureView,
    },
}

impl Frame {
    pub(super) fn view(&self) -> &etalon::TextureView {
        match self {
            Frame::Surface { view, .. } | Frame::Offscreen { view } => view,
        }
    }

    /// Presents the swapchain texture. No-op for offscreen targets (their
    /// contents persist in the texture and are read back on demand).
    pub(super) fn present(self, queue: &etalon::Queue) {
        if let Frame::Surface { texture, .. } = self {
            queue.present(texture);
        }
    }
}

/// WGPU-based renderer implementation.
pub struct WgpuRenderer<'a> {
    pub(super) device: etalon::Device,
    pub(super) queue: etalon::Queue,
    pub(super) target: RenderTarget<'a>,
    pub(super) capabilities: WebGpuCapabilityReport,
    pub(super) requested_extent: (u32, u32),
    pub(super) camera: Camera,
    pub(super) viewport_size: (f64, f64),
    pub(super) render_pipeline: etalon::RenderPipeline,
    #[allow(dead_code)]
    pub(super) projector_pipeline: Option<etalon::RenderPipeline>,
    pub(super) vertex_buffer: etalon::Buffer,
    #[allow(dead_code)]
    pub(super) projector_vertex_buffer: etalon::Buffer,
    pub(super) max_vertices: usize,
    pub(super) reservations: RendererReservations,
    pub(super) node_positions: Vec<(usize, ScreenPoint, f64)>,
    #[cfg(not(feature = "qualia"))]
    pub(super) ambient_config: AmbientConfig,
    #[cfg(not(feature = "qualia"))]
    pub(super) ambient_pipeline: Option<etalon::RenderPipeline>,
    #[cfg(not(feature = "qualia"))]
    pub(super) particle_buffer: Option<etalon::Buffer>,
    #[cfg(not(feature = "qualia"))]
    pub(super) ambient_uniform_buffer: Option<etalon::Buffer>,
    #[cfg(not(feature = "qualia"))]
    pub(super) telemetry_buffer: Option<etalon::Buffer>,
    #[cfg(not(feature = "qualia"))]
    pub(super) particle_count: usize,
    #[cfg(not(feature = "qualia"))]
    pub(super) ambient_effects: vegetation_effects::EffectPool,
}

impl<'a> WgpuRenderer<'a> {
    /// Create a new WgpuRenderer that draws to a window surface.
    pub async fn new(
        surface: etalon::Surface<'a>,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        let instance = etalon::Instance::default();

        let (device, queue, adapter, capabilities) =
            Self::request_device(&instance, Some(&surface)).await?;
        let mut reservations = RendererReservations::default();
        reservations.geometry = Some(reserve_vertex_buffer_residency()?);

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.has_srgb_suffix())
            .or_else(|| surface_caps.formats.first().copied())
            .ok_or_else(|| "webizen-render surface has no supported formats".to_string())?;
        let alpha_mode = surface_caps
            .alpha_modes
            .first()
            .copied()
            .ok_or_else(|| "webizen-render surface has no supported alpha modes".to_string())?;

        let surface_config = etalon::SurfaceConfiguration {
            usage: etalon::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: etalon::PresentMode::Fifo,
            alpha_mode,
            color_space: etalon::SurfaceColorSpace::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        let render_pipeline = Self::create_render_pipeline(&device, surface_format);
        let projector_pipeline = Self::create_projector_pipeline(&device, surface_format);

        let (vertex_buffer, projector_vertex_buffer, max_vertices) =
            Self::create_vertex_buffer(&device);

        #[cfg(not(feature = "qualia"))]
        let (
            ambient_config,
            ambient_pipeline,
            particle_buffer,
            ambient_uniform_buffer,
            telemetry_buffer,
            particle_count,
            ambient_effects,
        ) = Self::ambient_fields(&device, width, height);

        Ok(Self {
            device,
            queue,
            target: RenderTarget::Surface {
                surface,
                config: surface_config,
            },
            capabilities,
            requested_extent: (width, height),
            camera: Camera::default(),
            viewport_size: (width as f64, height as f64),
            render_pipeline,
            projector_pipeline: Some(projector_pipeline),
            vertex_buffer,
            projector_vertex_buffer,
            max_vertices,
            reservations,
            node_positions: Vec::new(),
            #[cfg(not(feature = "qualia"))]
            ambient_config,
            #[cfg(not(feature = "qualia"))]
            ambient_pipeline,
            #[cfg(not(feature = "qualia"))]
            particle_buffer,
            #[cfg(not(feature = "qualia"))]
            ambient_uniform_buffer,
            #[cfg(not(feature = "qualia"))]
            telemetry_buffer,
            #[cfg(not(feature = "qualia"))]
            particle_count,
            #[cfg(not(feature = "qualia"))]
            ambient_effects,
        })
    }

    /// Create a headless renderer that draws to an offscreen texture.
    pub async fn new_offscreen(width: u32, height: u32) -> Result<WgpuRenderer<'static>, String> {
        let requested_extent = (width, height);
        #[cfg(feature = "qualia")]
        let (device, queue, capabilities) = {
            let shared = qualia_core_db::gpu_context::try_shared_gpu().ok_or_else(|| {
                "webizen-render adapter unavailable: shared GPU context is unavailable".to_string()
            })?;
            let capabilities =
                capability_report_from_adapter(&shared.adapter, Some(&shared.device));
            (shared.device.clone(), shared.queue.clone(), capabilities)
        };

        let (width, height, frame_target_reservation) = reserve_offscreen_extent(width, height)?;
        let mut reservations = RendererReservations::default();
        reservations.frame_target = Some(frame_target_reservation);
        reservations.geometry = Some(reserve_vertex_buffer_residency()?);
        #[cfg(not(feature = "qualia"))]
        let (device, queue, capabilities) = {
            let instance = etalon::Instance::default();
            let (device, queue, _adapter, capabilities) =
                Self::request_device(&instance, None).await?;
            (device, queue, capabilities)
        };

        let format = etalon::TextureFormat::Rgba8Unorm;
        let texture = Self::create_offscreen_texture(&device, format, width, height);

        let render_pipeline = Self::create_render_pipeline(&device, format);
        let projector_pipeline = Self::create_projector_pipeline(&device, format);

        let (vertex_buffer, projector_vertex_buffer, max_vertices) =
            Self::create_vertex_buffer(&device);

        #[cfg(not(feature = "qualia"))]
        let (
            ambient_config,
            ambient_pipeline,
            particle_buffer,
            ambient_uniform_buffer,
            telemetry_buffer,
            particle_count,
            ambient_effects,
        ) = Self::ambient_fields(&device, width, height);

        Ok(WgpuRenderer {
            device,
            queue,
            target: RenderTarget::Offscreen {
                texture,
                format,
                width,
                height,
            },
            capabilities,
            requested_extent,
            camera: Camera::default(),
            viewport_size: (width as f64, height as f64),
            render_pipeline,
            projector_pipeline: Some(projector_pipeline),
            vertex_buffer,
            projector_vertex_buffer,
            max_vertices,
            reservations,
            node_positions: Vec::new(),
            #[cfg(not(feature = "qualia"))]
            ambient_config,
            #[cfg(not(feature = "qualia"))]
            ambient_pipeline,
            #[cfg(not(feature = "qualia"))]
            particle_buffer,
            #[cfg(not(feature = "qualia"))]
            ambient_uniform_buffer,
            #[cfg(not(feature = "qualia"))]
            telemetry_buffer,
            #[cfg(not(feature = "qualia"))]
            particle_count,
            #[cfg(not(feature = "qualia"))]
            ambient_effects,
        })
    }

    async fn request_device(
        instance: &etalon::Instance,
        compatible_surface: Option<&etalon::Surface<'_>>,
    ) -> Result<
        (
            etalon::Device,
            etalon::Queue,
            etalon::Adapter,
            WebGpuCapabilityReport,
        ),
        String,
    > {
        #[cfg(not(target_arch = "wasm32"))]
        let power_preference = etalon::PowerPreference::HighPerformance;
        #[cfg(target_arch = "wasm32")]
        let power_preference = etalon::PowerPreference::LowPower;

        let adapter = instance
            .request_adapter(&etalon::RequestAdapterOptions {
                power_preference,
                compatible_surface,
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|e| format!("webizen-render adapter unavailable: {e}"))?;

        let (device, queue) = adapter
            .request_device(&etalon::DeviceDescriptor {
                label: Some("webizen-render-device"),
                required_features: etalon::Features::empty(),
                required_limits: etalon::Limits::downlevel_defaults(),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("webizen-render device unavailable: {e}"))?;

        let capabilities = capability_report_from_adapter(&adapter, Some(&device));
        Ok((device, queue, adapter, capabilities))
    }

    pub(super) fn create_offscreen_texture(
        device: &etalon::Device,
        format: etalon::TextureFormat,
        width: u32,
        height: u32,
    ) -> etalon::Texture {
        device.create_texture(&etalon::TextureDescriptor {
            label: Some("webizen-offscreen-target"),
            size: etalon::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: etalon::TextureDimension::D2,
            format,
            usage: etalon::TextureUsages::RENDER_ATTACHMENT | etalon::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    pub(super) fn begin_frame(&self) -> Result<Frame, String> {
        match &self.target {
            RenderTarget::Surface { surface, .. } => {
                let texture = match surface.get_current_texture() {
                    etalon::CurrentSurfaceTexture::Success(texture)
                    | etalon::CurrentSurfaceTexture::Suboptimal(texture) => texture,
                    other => return Err(format!("Failed to get next surface texture: {other:?}")),
                };
                let view = texture
                    .texture
                    .create_view(&etalon::TextureViewDescriptor::default());
                Ok(Frame::Surface { texture, view })
            }
            RenderTarget::Offscreen { texture, .. } => {
                let view = texture.create_view(&etalon::TextureViewDescriptor::default());
                Ok(Frame::Offscreen { view })
            }
        }
    }

    /// Return the bounded capability facts observed while constructing this renderer.
    pub fn capabilities(&self) -> &WebGpuCapabilityReport {
        &self.capabilities
    }

    /// Return the effective offscreen extent, or `None` for a surface target.
    pub fn offscreen_extent(&self) -> Option<(u32, u32)> {
        match &self.target {
            RenderTarget::Offscreen { width, height, .. } => Some((*width, *height)),
            RenderTarget::Surface { .. } => None,
        }
    }

    /// Resize the render target.
    pub fn resize(&mut self, width: u32, height: u32) {
        if let Err(error) = self.try_resize(width, height) {
            log::warn!("webizen-render resize retained previous target: {error}");
        }
    }

    /// Resize while retaining the old target if the shared graphics budget
    /// cannot admit the replacement. Offscreen targets may degrade by powers
    /// of two and return their effective dimensions.
    pub fn try_resize(&mut self, width: u32, height: u32) -> Result<(u32, u32), String> {
        let requested_extent = (width, height);
        let (width, height) = (width.max(1), height.max(1));
        match &mut self.target {
            RenderTarget::Surface { surface, config } => {
                config.width = width;
                config.height = height;
                surface.configure(&self.device, config);
                self.viewport_size = (width as f64, height as f64);
                self.requested_extent = requested_extent;
                Ok((width, height))
            }
            RenderTarget::Offscreen {
                texture,
                format,
                width: current_width,
                height: current_height,
            } => {
                let (width, height, reservation) = reserve_offscreen_extent(width, height)?;
                if (*current_width, *current_height) == (width, height) {
                    drop(reservation);
                    self.requested_extent = requested_extent;
                    return Ok((width, height));
                }
                let replacement =
                    Self::create_offscreen_texture(&self.device, *format, width, height);
                let retired_texture = std::mem::replace(texture, replacement);
                *current_width = width;
                *current_height = height;
                let retired_reservation = self.reservations.frame_target.replace(reservation);
                self.viewport_size = (width as f64, height as f64);
                self.requested_extent = requested_extent;
                let _ = self.device.poll(etalon::PollType::wait_indefinitely());
                drop(retired_texture);
                drop(retired_reservation);
                Ok((width, height))
            }
        }
    }
}
