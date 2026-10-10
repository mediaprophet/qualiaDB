#[cfg(not(target_arch = "wasm32"))]
use super::super::native_device;
#[cfg(target_arch = "wasm32")]
use super::super::{mark_portal_gpu_canvas_claimed, portal_gpu_init_aborted};
use super::super::PortalGpu;
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
                    wgpu::TextureFormat::Rgba8Unorm,
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
            wgpu::TextureFormat::Rgba8Unorm,
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
            wgpu::TextureFormat::Rgba8Unorm,
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
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = native_device::backend_override()?.unwrap_or_else(wgpu::Backends::all);
        let instance = wgpu::Instance::new(desc);

        let win32_handle =
            Win32WindowHandle::new(std::num::NonZeroIsize::new(hwnd).ok_or("invalid HWND (zero)")?);
        let raw_window = RawWindowHandle::Win32(win32_handle);
        let raw_display = RawDisplayHandle::Windows(WindowsDisplayHandle::new());

        let surface = unsafe {
            instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: Some(raw_display),
                    raw_window_handle: raw_window,
                })
                .map_err(|e| format!("create_surface from HWND: {e:?}"))?
        };

        // Request an adapter that supports the surface
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
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
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Webizen GPU Surface"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            memory_hints: wgpu::MemoryHints::default(),
            ..Default::default()
        }))
        .map_err(|e| format!("Failed to request device for surface: {e}"))?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            // wgpu 30: surfaces declare their colour space; Auto preserves the
            // pre-30 (implicit sRGB/linear-by-format) behaviour.
            color_space: wgpu::SurfaceColorSpace::Auto,
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

    /// Async WebGPU init — awaits `request_adapter` / `request_device` (the browser main thread
    /// cannot block). Native callers use the `try_new` wrapper above.
    #[cfg(all(target_arch = "wasm32", feature = "portal"))]
    /// True when a browser adapter answers. Does not bind a canvas, so a hang
    /// or a miss leaves the 2d tick free to draw.
    pub async fn adapter_responds() -> bool {
        let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = wgpu::Backends::BROWSER_WEBGPU;
        let instance = wgpu::Instance::new(instance_desc);
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                ..Default::default()
            })
            .await
            .is_ok()
    }

    #[cfg(all(target_arch = "wasm32", feature = "portal"))]
    pub async fn try_new_async(
        canvas: &web_sys::HtmlCanvasElement,
        particle_cap: usize,
    ) -> Result<Self, String> {
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);

        let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = wgpu::Backends::BROWSER_WEBGPU;
        let instance = wgpu::Instance::new(instance_desc);

        // Probe already ran without a canvas. Claim only when we are about
        // to present, and only with an adapter that can target this surface.
        if portal_gpu_init_aborted() {
            return Err("aborted".into());
        }

        mark_portal_gpu_canvas_claimed();
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| format!("surface: {e}"))?;
        if portal_gpu_init_aborted() {
            return Err("aborted".into());
        }
        // Phones often refuse low-power or the first preference and then
        // answer the next. One miss must not drop the lit frame.
        let mut adapter = None;
        let mut last_err = String::from("no WebGPU adapter");
        for pref in [
            wgpu::PowerPreference::None,
            wgpu::PowerPreference::LowPower,
            wgpu::PowerPreference::HighPerformance,
        ] {
            if portal_gpu_init_aborted() {
                return Err("aborted".into());
            }
            match instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: pref,
                    compatible_surface: Some(&surface),
                    ..Default::default()
                })
                .await
            {
                Ok(found) => {
                    adapter = Some(found);
                    break;
                }
                Err(e) => last_err = format!("no WebGPU adapter: {e}"),
            }
        }
        let adapter = adapter.ok_or(last_err)?;

        if portal_gpu_init_aborted() {
            return Err("aborted".into());
        }
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("qualia-portal-gpu"),
                required_features: wgpu::Features::empty(),
                // Request exactly the adapter's advertised limits. `Limits::default()`
                // under wgpu 30 asks for desktop-tier limits that a browser WebGPU
                // adapter does not grant, so `request_device` would fail device
                // validation. `adapter.limits()` never over-requests and still preserves
                // the non-zero storage-buffer limits the portal pipelines need (unlike
                // `downlevel_webgl2_defaults`, which zeroes them and blacks out the view).
                required_limits: crate::gpu_context::webgpu_minimum_limits(),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("device: {e}"))?;

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
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            // wgpu 30: Auto preserves pre-30 colour-space behaviour.
            color_space: wgpu::SurfaceColorSpace::Auto,
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
        Ok(portal_gpu)
    }


}
