#[cfg(not(target_arch = "wasm32"))]
use super::super::native_device;
use super::super::PortalGpu;
#[cfg(all(target_arch = "wasm32", feature = "portal"))]
use super::super::{mark_portal_gpu_canvas_claimed, portal_gpu_init_aborted};
#[cfg(all(target_arch = "wasm32", feature = "portal"))]
use wasm_bindgen::JsCast;
use std::sync::Arc;

impl PortalGpu {
    /// Build a native offscreen renderer on QualiaDB's process-wide shared GPU device.
    ///
    /// The output target is linear `Rgba8Unorm`. Call [`Self::render`] and then
    /// [`Self::read_rgba8_into`] to retrieve tightly packed pixels into a caller-owned buffer.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_offscreen(width: u32, height: u32, particle_cap: usize) -> Result<Self, String> {
        let width = width.max(1);
        let height = height.max(1);
        let explicit_renderer_backend = native_device::backend_override()?.is_some();
        let mut shared_failure = None;

        // Keep the zero-copy shared device as the normal path. A renderer-only
        // backend pin bypasses it, so graphics can select GL without changing
        // inference's process-wide backend.
        if !explicit_renderer_backend {
            if let Some(shared) = crate::gpu_context::try_shared_gpu() {
                match pollster::block_on(Self::from_device(
                    Arc::new(shared.device.clone()),
                    Arc::new(shared.queue.clone()),
                    width,
                    height,
                    etalon::TextureFormat::Rgba8Unorm,
                    None,
                    None,
                    particle_cap,
                )) {
                    Ok(renderer) => return Ok(renderer),
                    Err(error) => shared_failure = Some(error),
                }
            }
        }

        let (device, queue) = native_device::request_offscreen_device()?;
        pollster::block_on(Self::from_device(
            device,
            queue,
            width,
            height,
            etalon::TextureFormat::Rgba8Unorm,
            None,
            None,
            particle_cap,
        ))
        .map_err(|error| match shared_failure {
            Some(shared) => format!(
                "shared renderer initialization failed ({shared}); native renderer fallback failed ({error})"
            ),
            None => error,
        })
    }

    /// Async offscreen WebGPU renderer on the process-wide shared device.
    /// Logic/scientific/LLM WASM packages construct this without a canvas.
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub async fn new_offscreen_async(
        width: u32,
        height: u32,
        particle_cap: usize,
    ) -> Result<Self, String> {
        crate::gpu_context::ensure_shared_gpu().await?;
        let shared = crate::gpu_context::try_shared_gpu()
            .ok_or_else(|| "shared WebGPU device missing after ensure_shared_gpu".to_string())?;
        Self::from_device(
            Arc::new(shared.device.clone()),
            Arc::new(shared.queue.clone()),
            width.max(1),
            height.max(1),
            etalon::TextureFormat::Rgba8Unorm,
            None,
            None,
            particle_cap,
        )
        .await
    }

    /// Build a native **surface** renderer that draws directly to a window's GPU swapchain.
    ///
    /// This is the native desktop path — no PNG round-trip, no webview `<img>`. The surface
    /// is created from a raw window handle (HWND on Windows) and frames are presented directly
    /// to the OS swapchain.
    ///
    /// The surface format is chosen from the adapter's capabilities (sRGB preferred).
    /// Call [`Self::render`] to draw a frame; the swapchain present is automatic.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub fn new_surface(
        hwnd: isize,
        width: u32,
        height: u32,
        particle_cap: usize,
    ) -> Result<Self, String> {
        use raw_window_handle::{
            RawDisplayHandle, RawWindowHandle, Win32WindowHandle, WindowsDisplayHandle,
        };

        // Create a DEDICATED instance/adapter/device for the surface renderer.
        // The shared GPU context is optimised for compute (LLM inference) and its
        // adapter may not support presentation to an HWND (e.g. Vulkan without a
        // VkSurfaceKHR, or a compute-only adapter). A dedicated instance ensures
        // the surface, adapter, and device are all from the same wgpu instance and
        // the adapter is picked with surface compatibility.
        let mut desc = etalon::InstanceDescriptor::new_without_display_handle();
        desc.backends = native_device::backend_override()?.unwrap_or_else(etalon::Backends::all);
        let instance = etalon::Instance::new(desc);

        let win32_handle =
            Win32WindowHandle::new(std::num::NonZeroIsize::new(hwnd).ok_or("invalid HWND (zero)")?);
        let raw_window = RawWindowHandle::Win32(win32_handle);
        let raw_display = RawDisplayHandle::Windows(WindowsDisplayHandle::new());

        let surface = unsafe {
            instance
                .create_surface_unsafe(etalon::SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: Some(raw_display),
                    raw_window_handle: raw_window,
                })
                .map_err(|e| format!("create_surface from HWND: {e:?}"))?
        };

        // Request an adapter that supports the surface
        let adapter =
            pollster::block_on(instance.request_adapter(&etalon::RequestAdapterOptions {
                power_preference: etalon::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            }))
            .map_err(|e| format!("Failed to find wgpu adapter for surface: {e}"))?;

        let caps = surface.get_capabilities(&adapter);
        if caps.formats.is_empty() {
            return Err(format!(
                "Surface supports no formats — adapter backend {:?} may not support presentation to HWND",
                adapter.get_info().backend
            ));
        }
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.has_srgb_suffix())
            .unwrap_or(caps.formats[0]);

        let (device, queue) =
            pollster::block_on(adapter.request_device(&etalon::DeviceDescriptor {
                label: Some("Webizen GPU Surface"),
                required_features: adapter.features() & etalon::Features::TIMESTAMP_QUERY,
                required_limits: etalon::Limits::downlevel_defaults(),
                memory_hints: etalon::MemoryHints::default(),
                ..Default::default()
            }))
            .map_err(|e| format!("Failed to request device for surface: {e}"))?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let config = etalon::SurfaceConfiguration {
            usage: etalon::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: etalon::PresentMode::Fifo,
            alpha_mode: caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(etalon::CompositeAlphaMode::Auto),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            // wgpu 30: surfaces declare their colour space; Auto preserves the
            // pre-30 (implicit sRGB/linear-by-format) behaviour.
            color_space: etalon::SurfaceColorSpace::Auto,
        };

        surface.configure(&device, &config);

        pollster::block_on(Self::from_device(
            device,
            queue,
            width.max(1),
            height.max(1),
            format,
            Some(surface),
            Some(config),
            particle_cap,
        ))
    }

    /// True when a browser adapter answers. Probes WebGPU first, then GL.
    /// Does not bind the presentation canvas or request a device, so a hang
    /// or a miss leaves the 2d tick free to draw.
    #[cfg(all(target_arch = "wasm32", feature = "portal"))]
    pub async fn adapter_responds() -> bool {
        if Self::webgpu_adapter_responds().await {
            return true;
        }
        Self::gl_adapter_responds().await
    }

    /// True when a WebGPU adapter answers. No canvas claim, no `request_device`.
    #[cfg(all(target_arch = "wasm32", feature = "portal"))]
    pub async fn webgpu_adapter_responds() -> bool {
        Self::adapter_responds_on(etalon::Backends::BROWSER_WEBGPU, None).await
    }

    /// True when a GL adapter answers on a detached canvas. No presentation
    /// canvas claim, no `request_device`.
    #[cfg(all(target_arch = "wasm32", feature = "portal"))]
    async fn gl_adapter_responds() -> bool {
        let Some(canvas) = detached_adapter_probe_canvas() else {
            return false;
        };
        let responded =
            Self::adapter_responds_on(etalon::Backends::GL, Some(canvas.clone())).await;
        release_detached_webgl2(&canvas);
        responded
    }

    #[cfg(all(target_arch = "wasm32", feature = "portal"))]
    async fn adapter_responds_on(
        backends: etalon::Backends,
        canvas: Option<web_sys::HtmlCanvasElement>,
    ) -> bool {
        let mut instance_desc = etalon::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = backends;
        let instance = etalon::Instance::new(instance_desc);
        let surface = match canvas {
            Some(canvas) => match instance.create_surface(etalon::SurfaceTarget::Canvas(canvas)) {
                Ok(surface) => Some(surface),
                Err(_) => return false,
            },
            None => None,
        };
        instance
            .request_adapter(&etalon::RequestAdapterOptions {
                power_preference: etalon::PowerPreference::HighPerformance,
                compatible_surface: surface.as_ref(),
                ..Default::default()
            })
            .await
            .is_ok()
    }

    /// Async WebGPU init — awaits `request_adapter` / `request_device` (the browser main thread
    /// cannot block). Native callers use the `try_new` wrapper above.
    ///
    /// Default path prefers WebGPU. If adapter/device/surface fails on a WebGPU
    /// adapter, retries once on a GL-only instance. `?webgl2=1` is GL-only and
    /// does not retry WebGPU. Abort flags still win.
    #[cfg(all(target_arch = "wasm32", feature = "portal"))]
    pub async fn try_new_async(
        canvas: &web_sys::HtmlCanvasElement,
        particle_cap: usize,
    ) -> Result<Self, String> {
        if location_forces_webgl2() {
            return Self::try_new_gl_instance(canvas, particle_cap).await;
        }

        match Self::try_new_webgpu_instance(canvas, particle_cap).await {
            Ok(gpu) => Ok(gpu),
            Err(error) if error == "aborted" => Err(error),
            Err(webgpu_error) => {
                if portal_gpu_init_aborted() {
                    return Err("aborted".into());
                }
                match Self::try_new_gl_instance(canvas, particle_cap).await {
                    Ok(gpu) => Ok(gpu),
                    Err(error) if error == "aborted" => Err(error),
                    Err(gl_error) => Err(format!("{webgpu_error}; gl retry: {gl_error}")),
                }
            }
        }
    }

    /// WebGPU-only instance: adapter and device before claiming the canvas so a
    /// later GL retry can still bind WebGL2 on the same element.
    #[cfg(all(target_arch = "wasm32", feature = "portal"))]
    async fn try_new_webgpu_instance(
        canvas: &web_sys::HtmlCanvasElement,
        particle_cap: usize,
    ) -> Result<Self, String> {
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);
        let mut instance_desc = etalon::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = etalon::Backends::BROWSER_WEBGPU;
        let instance = etalon::Instance::new(instance_desc);

        if portal_gpu_init_aborted() {
            return Err("aborted".into());
        }
        let adapter = Self::request_portal_adapter(&instance, None).await?;
        if adapter.get_info().backend == etalon::Backend::Gl {
            return Err("webgpu instance selected a GL adapter".into());
        }

        let (device, queue) = Self::request_portal_device(&adapter).await?;

        if portal_gpu_init_aborted() {
            return Err("aborted".into());
        }
        mark_portal_gpu_canvas_claimed();
        let surface = instance
            .create_surface(etalon::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| format!("surface: {e}"))?;

        Self::assemble_portal_gpu(
            adapter,
            device,
            queue,
            surface,
            width,
            height,
            particle_cap,
        )
        .await
    }

    /// GL-only instance. WebGL2 adapters require a canvas surface before
    /// `request_adapter`. Uses `adapter.limits()`, which pairs with
    /// `DownlevelFlags::READ_ONLY_STORAGE_EMULATION`.
    #[cfg(all(target_arch = "wasm32", feature = "portal"))]
    async fn try_new_gl_instance(
        canvas: &web_sys::HtmlCanvasElement,
        particle_cap: usize,
    ) -> Result<Self, String> {
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);
        let mut instance_desc = etalon::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = etalon::Backends::GL;
        let instance = etalon::Instance::new(instance_desc);

        if portal_gpu_init_aborted() {
            return Err("aborted".into());
        }
        mark_portal_gpu_canvas_claimed();
        let surface = instance
            .create_surface(etalon::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| format!("surface: {e}"))?;
        if portal_gpu_init_aborted() {
            return Err("aborted".into());
        }

        let adapter = Self::request_portal_adapter(&instance, Some(&surface)).await?;
        let (device, queue) = Self::request_portal_device(&adapter).await?;
        Self::assemble_portal_gpu(
            adapter,
            device,
            queue,
            surface,
            width,
            height,
            particle_cap,
        )
        .await
    }

    #[cfg(all(target_arch = "wasm32", feature = "portal"))]
    async fn request_portal_adapter(
        instance: &etalon::Instance,
        surface: Option<&etalon::Surface<'static>>,
    ) -> Result<etalon::Adapter, String> {
        // Hybrid laptops answer LowPower on the iGPU. Ask for the discrete
        // GPU first; phones that refuse it still get None then LowPower.
        let mut last_err = String::from("no adapter");
        for pref in [
            etalon::PowerPreference::HighPerformance,
            etalon::PowerPreference::None,
            etalon::PowerPreference::LowPower,
        ] {
            if portal_gpu_init_aborted() {
                return Err("aborted".into());
            }
            match instance
                .request_adapter(&etalon::RequestAdapterOptions {
                    power_preference: pref,
                    compatible_surface: surface,
                    ..Default::default()
                })
                .await
            {
                Ok(found) => return Ok(found),
                Err(e) => last_err = format!("no adapter: {e}"),
            }
        }
        Err(last_err)
    }

    #[cfg(all(target_arch = "wasm32", feature = "portal"))]
    async fn request_portal_device(
        adapter: &etalon::Adapter,
    ) -> Result<(etalon::Device, etalon::Queue), String> {
        if portal_gpu_init_aborted() {
            return Err("aborted".into());
        }
        let required_limits = if adapter.get_info().backend == etalon::Backend::Gl {
            adapter.limits()
        } else {
            crate::gpu_context::webgpu_minimum_limits()
        };
        adapter
            .request_device(&etalon::DeviceDescriptor {
                label: Some("qualia-portal-gpu"),
                required_features: adapter.features() & etalon::Features::TIMESTAMP_QUERY,
                // Request minimum WebGPU limits on WebGPU, or the adapter's advertised
                // limits on WebGL2 (which pairs with DownlevelFlags::READ_ONLY_STORAGE_EMULATION).
                required_limits,
                ..Default::default()
            })
            .await
            .map_err(|e| format!("device: {e}"))
    }

    #[cfg(all(target_arch = "wasm32", feature = "portal"))]
    async fn assemble_portal_gpu(
        adapter: etalon::Adapter,
        device: etalon::Device,
        queue: etalon::Queue,
        surface: etalon::Surface<'static>,
        width: u32,
        height: u32,
        particle_cap: usize,
    ) -> Result<Self, String> {
        if portal_gpu_init_aborted() {
            return Err("aborted".into());
        }

        // This device is uniquely owned by the browser portal. Never install this
        // callback on the shared native/inference device: that device has multiple
        // owners and a device-lost callback is a single-owner notification slot.
        let device_lost = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let lost_signal = Arc::clone(&device_lost);
        device.set_device_lost_callback(move |_reason, _message| {
            lost_signal.store(true, std::sync::atomic::Ordering::Release);
        });

        let caps = surface.get_capabilities(&adapter);
        if caps.formats.is_empty() {
            return Err("surface has no presentable format".into());
        }
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.has_srgb_suffix())
            .unwrap_or(caps.formats[0]);

        let config = etalon::SurfaceConfiguration {
            usage: etalon::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: etalon::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            // wgpu 30: Auto preserves pre-30 colour-space behaviour.
            color_space: etalon::SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &config);

        let mut portal_gpu = Self::from_device(
            Arc::new(device),
            Arc::new(queue),
            width,
            height,
            format,
            Some(surface),
            Some(config),
            particle_cap,
        )
        .await?;
        portal_gpu.device_lost = device_lost;
        portal_gpu.pass_timings.set_adapter_info(adapter.get_info());
        Ok(portal_gpu)
    }
}

#[cfg(all(target_arch = "wasm32", feature = "portal"))]
fn location_forces_webgl2() -> bool {
    web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &"location".into()).ok())
        .and_then(|loc| js_sys::Reflect::get(&loc, &"search".into()).ok())
        .and_then(|s| s.as_string())
        .is_some_and(|s| s.contains("webgl2=1"))
}

#[cfg(all(target_arch = "wasm32", feature = "portal"))]
fn detached_adapter_probe_canvas() -> Option<web_sys::HtmlCanvasElement> {
    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.create_element("canvas").ok())
        .and_then(|element| element.dyn_into::<web_sys::HtmlCanvasElement>().ok())
}

#[cfg(all(target_arch = "wasm32", feature = "portal"))]
fn release_detached_webgl2(canvas: &web_sys::HtmlCanvasElement) {
    let Ok(Some(context)) = canvas.get_context("webgl2") else {
        return;
    };
    let Ok(context) = context.dyn_into::<web_sys::WebGl2RenderingContext>() else {
        return;
    };
    let Ok(Some(extension)) = context.get_extension("WEBGL_lose_context") else {
        return;
    };
    if let Ok(lose_context) = js_sys::Reflect::get(&extension, &"loseContext".into())
        .and_then(JsCast::dyn_into::<js_sys::Function>)
    {
        let _ = lose_context.call0(&extension);
    }
}
