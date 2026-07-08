//! Native GPU surface rendering — renders directly to a child HWND via wgpu,
//! bypassing the PNG/webview round-trip entirely.
//!
//! The desktop is the "future native windowed host" that the renderer's Surface
//! mode was built for. This module creates a child HWND inside the Tauri window,
//! creates a `wgpu::Surface` from it, and drives a render loop that presents
//! frames directly to the GPU swapchain.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use webizen_render::scene_contract::RenderScene;
use webizen_render::telemetry::SystemTelemetry;

/// State held by the app for the native GPU surface.
pub struct NativeSurfaceState {
    /// The volumetric renderer with surface mode (renders directly to swapchain).
    pub renderer: Mutex<Option<webizen_render::VolumetricRenderer>>,
    /// The render scene to display, set by Tauri commands.
    pub scene: Mutex<Option<RenderScene>>,
    /// Whether the render loop is running.
    pub running: AtomicBool,
    /// The child window handle — stored as isize for wgpu surface creation.
    #[cfg(windows)]
    pub child_hwnd: Mutex<Option<isize>>,
    /// Current surface dimensions.
    pub width: Mutex<u32>,
    pub height: Mutex<u32>,
    /// Time accumulator for animations.
    pub time: Mutex<f32>,
    /// Camera control.
    pub camera_yaw: Mutex<f32>,
    pub camera_pitch: Mutex<f32>,
    pub camera_zoom: Mutex<f32>,
    /// Whether a mesh is loaded.
    pub mesh_loaded: AtomicBool,
}

impl Default for NativeSurfaceState {
    fn default() -> Self {
        Self {
            renderer: Mutex::new(None),
            scene: Mutex::new(None),
            running: AtomicBool::new(false),
            #[cfg(windows)]
            child_hwnd: Mutex::new(None),
            width: Mutex::new(800),
            height: Mutex::new(600),
            time: Mutex::new(0.0),
            camera_yaw: Mutex::new(0.0),
            camera_pitch: Mutex::new(-0.3),
            camera_zoom: Mutex::new(1.0),
            mesh_loaded: AtomicBool::new(false),
        }
    }
}

#[cfg(windows)]
mod win {
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::Graphics::Gdi::ValidateRect;
    use windows::Win32::UI::WindowsAndMessaging::{
        DefWindowProcA, RegisterClassA, WNDCLASSA, CS_HREDRAW, CS_VREDRAW,
        WM_DESTROY, WM_PAINT, WS_CHILD, WS_VISIBLE, WS_CLIPSIBLINGS,
        SW_SHOW, ShowWindow, DestroyWindow, MoveWindow,
        CreateWindowExA, WS_EX_NOPARENTNOTIFY,
    };
    use windows::core::PCSTR;
    use windows::Win32::System::LibraryLoader::GetModuleHandleA;

    static mut CHILD_CLASS_REGISTERED: bool = false;

    /// Create a child HWND inside the parent window at the given position.
    /// Returns the HWND as an isize for wgpu surface creation.
    pub fn create_child_hwnd(parent: HWND, x: i32, y: i32, width: i32, height: i32) -> Result<isize, String> {
        unsafe {
            if !CHILD_CLASS_REGISTERED {
                let class_name = b"WebizenGpuChild\0";
                let wc = WNDCLASSA {
                    style: CS_HREDRAW | CS_VREDRAW,
                    lpfnWndProc: Some(child_wnd_proc),
                    hInstance: GetModuleHandleA(None).map_err(|e| format!("GetModuleHandle: {e}"))?.into(),
                    lpszClassName: PCSTR(class_name.as_ptr()),
                    ..Default::default()
                };
                let atom = RegisterClassA(&wc);
                if atom == 0 {
                    return Err(format!("RegisterClass failed: {}", windows::core::Error::from_win32()));
                }
                CHILD_CLASS_REGISTERED = true;
            }

            let class_name = b"WebizenGpuChild\0";
            let hwnd = CreateWindowExA(
                WS_EX_NOPARENTNOTIFY,
                PCSTR(class_name.as_ptr()),
                PCSTR(b"Webizen GPU\0".as_ptr()),
                WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
                x, y, width, height,
                Some(parent),
                None,
                Some(GetModuleHandleA(None).map_err(|e| format!("GetModuleHandle: {e}"))?.into()),
                None,
            ).map_err(|e| format!("CreateWindowEx: {e}"))?;

            let _ = ShowWindow(hwnd, SW_SHOW);
            Ok(hwnd.0 as isize)
        }
    }

    pub fn destroy_child_hwnd(hwnd_raw: isize) {
        unsafe {
            let hwnd = HWND(hwnd_raw as *mut core::ffi::c_void);
            let _ = DestroyWindow(hwnd);
        }
    }

    pub fn move_child_hwnd(hwnd_raw: isize, x: i32, y: i32, width: i32, height: i32) {
        unsafe {
            let hwnd = HWND(hwnd_raw as *mut core::ffi::c_void);
            let _ = MoveWindow(hwnd, x, y, width, height, true);
        }
    }

    unsafe extern "system" fn child_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_PAINT => {
                let _ = ValidateRect(Some(hwnd), None);
                LRESULT(0)
            }
            WM_DESTROY => LRESULT(0),
            _ => DefWindowProcA(hwnd, msg, wparam, lparam),
        }
    }
}

/// Initialize the native GPU surface: create child HWND, wgpu surface, and renderer.
#[cfg(windows)]
pub fn init_native_surface(
    parent_window: &WebviewWindow,
    x: i32, y: i32, width: u32, height: u32,
) -> Result<(isize, webizen_render::VolumetricRenderer), String> {
    let parent_hwnd = parent_window.hwnd()
        .map_err(|e| format!("failed to get parent HWND: {e}"))?;

    let hwnd_raw = win::create_child_hwnd(parent_hwnd, x, y, width as i32, height as i32)?;

    // Create the volumetric renderer with surface mode — renders directly to the swapchain
    let renderer = webizen_render::VolumetricRenderer::new_surface(hwnd_raw, width, height, 4096)?;

    Ok((hwnd_raw, renderer))
}

/// Start the render loop on a background thread.
pub fn spawn_render_loop(
    app: AppHandle,
    state: Arc<NativeSurfaceState>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut last_time = std::time::Instant::now();

        while state.running.load(Ordering::SeqCst) {
            let now = std::time::Instant::now();
            let dt = now.duration_since(last_time).as_secs_f32();
            last_time = now;

            let time_seconds = {
                let mut t = state.time.lock().unwrap();
                *t += dt;
                *t
            };

            let mut renderer_guard = state.renderer.lock().unwrap();
            if let Some(ref mut renderer) = *renderer_guard {
                let yaw = *state.camera_yaw.lock().unwrap();
                let pitch = *state.camera_pitch.lock().unwrap();
                let zoom = *state.camera_zoom.lock().unwrap();
                renderer.set_camera(yaw, pitch, zoom);

                let telemetry = SystemTelemetry::default();
                if let Err(e) = renderer.render(time_seconds, &telemetry) {
                    eprintln!("[native_surface] render error: {e}");
                }
            }
            drop(renderer_guard);

            std::thread::sleep(std::time::Duration::from_millis(16));
        }

        let _ = app.emit("gpu-surface-stopped", ());
    })
}

// ── Tauri commands ───────────────────────────────────────────────────────────

/// Mount the native GPU surface at a position in the window.
#[tauri::command]
pub fn mount_gpu_surface(
    app: AppHandle,
    x: i32, y: i32, width: u32, height: u32,
) -> Result<(), String> {
    let state = app.state::<std::sync::Arc<NativeSurfaceState>>();

    #[cfg(windows)]
    {
        let window = app.get_webview_window("main")
            .ok_or("main window not found")?;

        let mut renderer_guard = state.renderer.lock().unwrap();

        // If already initialized, just move/resize
        if renderer_guard.is_some() {
            let hwnd_opt = state.child_hwnd.lock().unwrap();
            if let Some(hwnd_raw) = *hwnd_opt {
                win::move_child_hwnd(hwnd_raw, x, y, width as i32, height as i32);
            }
            if let Some(ref mut renderer) = *renderer_guard {
                renderer.resize(width, height);
            }
            *state.width.lock().unwrap() = width;
            *state.height.lock().unwrap() = height;
            return Ok(());
        }

        drop(renderer_guard);
        let (hwnd_raw, renderer) = init_native_surface(&window, x, y, width, height)?;
        *state.renderer.lock().unwrap() = Some(renderer);
        *state.child_hwnd.lock().unwrap() = Some(hwnd_raw);
        *state.width.lock().unwrap() = width;
        *state.height.lock().unwrap() = height;
        state.running.store(true, Ordering::SeqCst);

        let state_arc = state.inner().clone();
        spawn_render_loop(app.clone(), state_arc);

        let _ = app.emit("gpu-surface-mounted", ());
    }

    #[cfg(not(windows))]
    {
        let _ = (app, x, y, width, height);
        return Err("Native GPU surface is only supported on Windows".to_string());
    }

    Ok(())
}

/// Set the render scene for the native surface.
#[tauri::command]
pub fn set_gpu_scene(
    app: AppHandle,
    scene: RenderScene,
) -> Result<(), String> {
    let state = app.state::<std::sync::Arc<NativeSurfaceState>>();
    *state.scene.lock().unwrap() = Some(scene);
    Ok(())
}

/// Update camera position.
#[tauri::command]
pub fn set_gpu_camera(
    app: AppHandle,
    yaw: f32,
    pitch: f32,
    zoom: f32,
) -> Result<(), String> {
    let state = app.state::<std::sync::Arc<NativeSurfaceState>>();
    *state.camera_yaw.lock().unwrap() = yaw;
    *state.camera_pitch.lock().unwrap() = pitch;
    *state.camera_zoom.lock().unwrap() = zoom;
    Ok(())
}

/// Upload a mesh to the GPU surface renderer.
#[tauri::command]
pub fn upload_gpu_mesh(
    app: AppHandle,
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
) -> Result<u32, String> {
    let state = app.state::<std::sync::Arc<NativeSurfaceState>>();
    let mut renderer_guard = state.renderer.lock().unwrap();
    if let Some(ref mut renderer) = *renderer_guard {
        let id = renderer.upload_mesh(&positions, &indices);
        state.mesh_loaded.store(true, Ordering::SeqCst);
        Ok(id)
    } else {
        Err("GPU surface not mounted".to_string())
    }
}

/// Upload a colored mesh to the GPU surface renderer.
#[tauri::command]
pub fn upload_gpu_mesh_colored(
    app: AppHandle,
    positions: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
) -> Result<u32, String> {
    let state = app.state::<std::sync::Arc<NativeSurfaceState>>();
    let mut renderer_guard = state.renderer.lock().unwrap();
    if let Some(ref mut renderer) = *renderer_guard {
        let id = renderer.upload_mesh_colored(&positions, &colors, &indices);
        state.mesh_loaded.store(true, Ordering::SeqCst);
        Ok(id)
    } else {
        Err("GPU surface not mounted".to_string())
    }
}

/// Upload a .10d mesh section to the GPU surface renderer.
#[tauri::command]
pub fn upload_gpu_10d_mesh(
    app: AppHandle,
    bytes: Vec<u8>,
) -> Result<u32, String> {
    let state = app.state::<std::sync::Arc<NativeSurfaceState>>();
    let mut renderer_guard = state.renderer.lock().unwrap();
    if let Some(ref mut renderer) = *renderer_guard {
        let id = renderer.upload_10d_mesh(&bytes)?;
        state.mesh_loaded.store(true, Ordering::SeqCst);
        Ok(id)
    } else {
        Err("GPU surface not mounted".to_string())
    }
}

/// Load a full .10d container asset (mesh + tensor nodes + provenance).
#[tauri::command]
pub fn load_gpu_10d_asset(
    app: AppHandle,
    bytes: Vec<u8>,
) -> Result<(u32, u32, f32), String> {
    let state = app.state::<std::sync::Arc<NativeSurfaceState>>();
    let mut renderer_guard = state.renderer.lock().unwrap();
    if let Some(ref mut renderer) = *renderer_guard {
        let result = renderer.load_10d_asset(&bytes)?;
        state.mesh_loaded.store(true, Ordering::SeqCst);
        Ok(result)
    } else {
        Err("GPU surface not mounted".to_string())
    }
}

/// Unmount the native GPU surface.
#[tauri::command]
pub fn unmount_gpu_surface(app: AppHandle) -> Result<(), String> {
    let state = app.state::<std::sync::Arc<NativeSurfaceState>>();
    state.running.store(false, Ordering::SeqCst);

    #[cfg(windows)]
    {
        let mut hwnd_guard = state.child_hwnd.lock().unwrap();
        if let Some(hwnd_raw) = *hwnd_guard {
            win::destroy_child_hwnd(hwnd_raw);
            *hwnd_guard = None;
        }
    }

    *state.renderer.lock().unwrap() = None;
    *state.scene.lock().unwrap() = None;
    state.mesh_loaded.store(false, Ordering::SeqCst);

    let _ = app.emit("gpu-surface-unmounted", ());
    Ok(())
}
