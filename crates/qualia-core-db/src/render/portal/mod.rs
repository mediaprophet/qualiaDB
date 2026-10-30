//! Qualia WASM — Semantic Subjectivity Bifurcation Portal (browser surface).

use js_sys::{Array, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, Document, Element, HtmlCanvasElement};

#[allow(unused_imports)]
use etalon_wasm::{
    MessageHeader, SharedArrayBufferRing, TransferableMessageTransport, TransportMode, WorkerState,
    WorkerSteward, SAB_RING_MAGIC, SAB_RING_VERSION,
};

pub trait WorkerStewardExt {
    fn mark_ready(&mut self);
    fn notify_device_lost(&mut self);
    fn recover(&mut self, new_gen: u32);
    fn state(&self) -> WorkerState;
    fn generation(&self) -> u32;
}

impl WorkerStewardExt for WorkerSteward {
    fn mark_ready(&mut self) {
        self.set_ready();
    }

    fn notify_device_lost(&mut self) {
        self.state = WorkerState::Suspended;
    }

    fn recover(&mut self, new_gen: u32) {
        self.state = WorkerState::Ready;
        let req = self.next_request_id.load(core::sync::atomic::Ordering::Relaxed) & 0x0000_0000_FFFF_FFFF;
        self.next_request_id.store(((new_gen as u64) << 32) | req, core::sync::atomic::Ordering::Relaxed);
    }

    fn state(&self) -> WorkerState {
        self.state
    }

    fn generation(&self) -> u32 {
        (self.next_request_id.load(core::sync::atomic::Ordering::Relaxed) >> 32) as u32
    }
}

use crate::audio::acoustic_plane::{
    acoustic_enabled_for_mode, acoustic_params_from_tensor, apply_binaural_to_uniform,
    drain_sonic_tokens, push_sonic_token, sonify_tensor_node, AcousticUniform,
};
use crate::audio::acoustic_sab::{
    init_acoustic_sab, push_token_to_sab, write_uniform_to_sab_with_mirror, ACOUSTIC_SAB_BYTES,
};
use crate::audio::audio_sidecar_link::enrich_preview_from_sidecar;
use crate::audio::audio_spectral_sheet::parse_sidecar_header;
use crate::audio::audio_spectral_sheet::preview_bins_from_tensor;
use crate::audio::audio_spectral_sheet::SPECTRAL_PREVIEW_BINS;
use crate::audio::cqt_bake::bake_cqt_sidecar_from_preview;
use crate::audio::stft_bake::bake_tensor_stft_sidecar;
use crate::render::acoustic::ACOUSTIC_UNIFORM_FLOAT_COUNT;
use crate::render::control::{
    control_pending, pop_control_command, push_control_raw, PortalControlCommand, MENU_ACTION_HOME,
    MENU_ACTION_SONIFY_TOGGLE, OP_BUTTON_ACTION, OP_COLLAPSE_Q, OP_MENU_ACTION, OP_NAVIGATE_INDEX,
    OP_SET_CAMERA_DELTA, OP_SET_STANDPOINT_SCALAR, OP_SONIC_TOKEN_FORWARD, OP_SWIPE_GESTURE,
    OP_TILT_FRAME, STANDPOINT_SCALAR_EPISTEMIC_Q, STANDPOINT_SCALAR_T_SLICE,
    STANDPOINT_SCALAR_T_WINDOW,
};

use crate::gpu_context::{ambient_draw_instances, global_vram_ledger, OperationalMode};
use crate::render::atmosphere::AtmospherePreset;
use crate::render::camera::CameraState;
mod camera_lens;
use crate::render::hmc_playback::{resolve_hmc_mesh, HmcMeshResidency};
use crate::render::navigation::{
    camera_frame_node, cpu_pick_node_at_camera, CameraFlyTo, Q_COLLAPSED_EPS,
};
use crate::render::spectral::sigma_to_display_rgb;
use crate::render::standpoint::{resolve_standpoint_hash, spectator_default};
use crate::render::telemetry::{
    ObserverStandpoint, SystemTelemetry, DEONTIC_LANE_COMMONS, FABRIC_SHARED,
    FABRIC_VIEWPORT_LOCAL, STANDPOINT_DID, STANDPOINT_EPHEMERAL, STANDPOINT_SPECTATOR,
    STANDPOINT_VAULT,
};
use crate::sonic_token::SonicToken;
use crate::tensor::buffer_export::{
    read_tensor_at, tensor_node_count, write_tensor_buffer, write_tensor_q_at, TENSOR_HEADER_BYTES,
};
use crate::{
    export_tensor_buffer_wasm, parse_cbor_ld_wasm, parse_json_wasm, sample_browser_telemetry_wasm,
    spatial_encode_wasm,
};

#[cfg(target_arch = "wasm32")]
use crate::render::anatomy::webgl2::AnatomyWebGl2;
#[cfg(target_arch = "wasm32")]
use crate::render::gpu::{
    abort_portal_gpu_init, particle_cap_for_mode, portal_gpu_canvas_claimed,
    portal_gpu_init_aborted, reset_portal_gpu_init_flags, MeshInstanceUploadError, PortalGpu,
};
#[cfg(target_arch = "wasm32")]
use crate::render::instance_culling::GpuInstanceRecord;

/// Viewport display mode (geometry projection style).
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum DisplayMode {
    Wireframe = 0,
    Points = 1,
    Solid = 2,
    Hybrid = 3,
}

impl DisplayMode {
    /// Parse a display-mode name; unknown strings fall back to
    /// `Wireframe`.
    fn from_str(s: &str) -> Self {
        match s {
            "points" => Self::Points,
            "solid" => Self::Solid,
            "hybrid" => Self::Hybrid,
            _ => Self::Wireframe,
        }
    }
}

#[derive(Clone, Copy)]
struct ProjectedNode {
    px: f64,
    py: f64,
    r: u8,
    g: u8,
    b: u8,
    alpha: f32,
    radius: f64,
    epistemic_ring: bool,
}

mod body_scene;
mod scene_animation;
mod scene_mesh;
pub(crate) use scene_mesh::BodyMeshAccum;
#[allow(unused_imports)]
pub(crate) use scene_mesh::SceneMeshAccum;
mod capabilities;
pub use capabilities::probe_portal_graphics;
mod exposure;
mod part_vertices;
mod graphics;
use graphics::PortalGraphicsState;
mod recovery;
mod texture_upload;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BodyRendererBackend {
    None,
    /// Canvas2d projection of the loaded mesh. Used when no WebGPU/WebGL2
    /// adapter is bound, so a tick still draws the part.
    CpuCanvas,
    WebGpu,
    WebGl2,
}

impl BodyRendererBackend {
    /// Backend name for receipts/diagnostics.
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::CpuCanvas => "canvas2d",
            Self::WebGpu => "webgpu",
            Self::WebGl2 => "webgl2",
        }
    }
}

/// Portal tier: 0 = CPU canvas2d fallback, 1 = tensor projection, 2 = WebGPU ambient.
#[wasm_bindgen]
pub struct QualiaPortal {
    description: String,
    last_parsed: Option<JsValue>,
    tier: u8,
    time: f64,
    last_tensor: Option<Vec<u8>>,
    /// Tensor node data remains pickable when its visual projection is hidden.
    tensor_projection_enabled: bool,
    telemetry: SystemTelemetry,
    display_mode: DisplayMode,
    camera: CameraState,
    camera_fly: CameraFlyTo,
    selected_node: Option<u32>,
    selected_semantic_id: Option<u64>,
    #[cfg(target_arch = "wasm32")]
    pending_gpu_pick: bool,
    session_nonce: u64,
    standpoint: ObserverStandpoint,
    #[cfg(target_arch = "wasm32")]
    gpu: Option<PortalGpu>,
    #[cfg(target_arch = "wasm32")]
    anatomy_webgl2: Option<AnatomyWebGl2>,
    #[cfg(target_arch = "wasm32")]
    gpu_init_failed: bool,
    #[cfg(target_arch = "wasm32")]
    graphics_recovery_requested: bool,
    #[cfg(target_arch = "wasm32")]
    graphics_recovery_pending: bool,
    #[cfg(target_arch = "wasm32")]
    graphics_recovery_canvas_only: bool,
    body_renderer: BodyRendererBackend,
    body_vertex_count: u32,
    body_index_count: u32,
    body_frames_presented: u32,
    /// Verified HMC mesh identity and upload generation for browser playback.
    hmc_mesh_residency: HmcMeshResidency,
    acoustic_enabled: bool,
    acoustic_pulse_accum: f32,
    /// Pinned mmap-ready STFT/CQT sidecar for selected node (cold bake → hot frame read).
    acoustic_sidecar: Option<Vec<u8>>,
    acoustic_sidecar_frame: u32,
    /// Person-authored body fit (JSON-compatible with wellfare `BodyFit`).
    body_fit: crate::render::body_fit::AnatomyBodyFit,
    /// When true, uploaded meshes keep authored coordinates (town / world)
    /// instead of being recentred into the orbit frame.
    preserve_authored_frame: bool,
    /// Decimated mesh for the proof canvas tick. Kept even when WebGPU owns
    /// the draw, so a failed present can still paint. Not the lit look.
    cpu_body: Option<CpuBodyMesh>,
    scene_animation: scene_animation::SceneAnimation,
    /// Last `set_sky_preset` value, applied when a GPU is adopted later.
    sky_preset: Option<u32>,
    /// Last atmosphere profile, preserved when callers override the sky clear colour.
    atmosphere_preset: u32,
    /// Manual HDR exposure compensation, preserved across device recovery.
    hdr_exposure_ev: f32,
    /// Linear Rec.709 temperature/tint balance controls in stops.
    white_balance_ev: [f32; 2],
    /// Clear colour shared by WebGPU and WebGL2. Daylight, not a black sky.
    sky_clear: [f32; 4],
    /// Failed presents since the mesh was uploaded. A few misses are not
    /// adapter death; proof is only the failover after they stick.
    present_misses: u8,
    graphics: PortalGraphicsState,
    etalon_steward: WorkerSteward,
}

#[wasm_bindgen]
impl QualiaPortal {
    /// Construct the portal on `canvas`: detect the device tier, mint a
    /// session nonce + spectator standpoint, sample ambient telemetry,
    /// and paint the first frame.
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<QualiaPortal, JsValue> {
        let tier = detect_tier();
        let session_nonce = crate::render::standpoint::generate_session_nonce();
        let standpoint = spectator_default(session_nonce);
        let cross_origin_isolated_detected = web_sys::window()
            .and_then(|w| Reflect::get(&w, &JsValue::from_str("crossOriginIsolated")).ok())
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let etalon_steward = WorkerSteward::new(cross_origin_isolated_detected);
        let mut portal = QualiaPortal {
            description: "Qualia portal initialized".to_string(),
            last_parsed: None,
            tier,
            time: 0.0,
            last_tensor: None,
            tensor_projection_enabled: true,
            telemetry: SystemTelemetry::from_samples(
                &crate::gpu_context::sample_ambient_telemetry(),
            ),
            display_mode: DisplayMode::Hybrid,
            camera: CameraState::default(),
            camera_fly: CameraFlyTo::default(),
            selected_node: None,
            selected_semantic_id: None,
            #[cfg(target_arch = "wasm32")]
            pending_gpu_pick: false,
            session_nonce,
            standpoint,
            #[cfg(target_arch = "wasm32")]
            gpu: None,
            #[cfg(target_arch = "wasm32")]
            anatomy_webgl2: None,
            #[cfg(target_arch = "wasm32")]
            gpu_init_failed: false,
            #[cfg(target_arch = "wasm32")]
            graphics_recovery_requested: false,
            #[cfg(target_arch = "wasm32")]
            graphics_recovery_pending: false,
            #[cfg(target_arch = "wasm32")]
            graphics_recovery_canvas_only: false,
            body_renderer: BodyRendererBackend::None,
            body_vertex_count: 0,
            body_index_count: 0,
            body_frames_presented: 0,
            hmc_mesh_residency: HmcMeshResidency::default(),
            acoustic_enabled: true,
            acoustic_pulse_accum: 0.0,
            acoustic_sidecar: None,
            acoustic_sidecar_frame: 0,
            body_fit: crate::render::body_fit::AnatomyBodyFit::default(),
            preserve_authored_frame: false,
            cpu_body: None,
            scene_animation: scene_animation::SceneAnimation::default(),
            sky_preset: None,
            atmosphere_preset: 0,
            hdr_exposure_ev: crate::render::output::DEFAULT_HDR_EXPOSURE_EV,
            white_balance_ev: [0.0; 2],
            sky_clear: [0.55, 0.74, 0.92, 1.0],
            present_misses: 0,
            graphics: PortalGraphicsState::default(),
            etalon_steward,
        };
        portal.paint_frame(&canvas)?;
        Ok(portal)
    }

    /// Device tier — 0 = Canvas2D, 1 = WebGPU-capable, 2 = phenomenal
    /// viewport (tensor upload confirmed).
    pub fn tier(&self) -> u8 {
        self.tier
    }

    /// True when the browser host must replace the context-bound canvas and
    /// recreate a renderer before ticking this portal again.
    #[cfg(target_arch = "wasm32")]
    pub fn graphics_recovery_requested(&self) -> bool {
        self.graphics_recovery_requested
    }

    /// The last hardware backend refused even reduced geometry. The host must
    /// replace the context-bound canvas and settle on Canvas2D for this scene.
    #[cfg(target_arch = "wasm32")]
    pub fn graphics_recovery_canvas_only(&self) -> bool {
        self.graphics_recovery_canvas_only
    }

    /// Complete a host recovery attempt. The host reports `webgpu`, `webgl2`,
    /// or `canvas2d`; recovered GPU backends re-upload the retained reduced
    /// mesh and full semantic tensor on the next frame.
    #[cfg(target_arch = "wasm32")]
    pub fn complete_graphics_recovery(&mut self, backend: &str) {
        self.graphics_recovery_requested = false;
        self.graphics_recovery_pending = matches!(backend, "webgpu" | "webgl2");
        if backend == "canvas2d" || backend == "unavailable" {
            self.graphics_recovery_canvas_only = false;
        }
        self.gpu_init_failed = backend != "webgpu";
        self.present_misses = 0;
        if backend == "canvas2d" || backend == "unavailable" {
            self.tier = 0;
            self.body_renderer = BodyRendererBackend::CpuCanvas;
            self.etalon_steward.notify_device_lost();
        } else {
            let new_gen = self.etalon_steward.generation().wrapping_add(1);
            self.etalon_steward.recover(new_gen);
        }
    }

    /// Push a packed Interface Control Plane command (`PortalControlCommand` raw `u64`).
    pub fn push_control_command(&self, raw: u64) -> bool {
        push_control_raw(raw)
    }

    /// Pending ICP commands in the SPSC ring.
    pub fn control_pending(&self) -> u32 {
        control_pending()
    }

    /// Drain up to `max` control commands and apply to this portal. Returns count applied.
    pub fn drain_control_commands(&mut self, max: u32) -> u32 {
        self.drain_and_apply_control(max)
    }

    /// VRAM-ledger operational mode as its `u8` code
    /// (0 = Full, 1 = Eco, 2 = Reserve).
    pub fn operational_mode(&self) -> u8 {
        global_vram_ledger().mode() as u8
    }

    /// Operational transport mode for dedicated WebWorker IPC.
    pub fn etalon_transport_mode(&self) -> String {
        match self.etalon_steward.transport_mode {
            TransportMode::SharedArrayBufferRing => "SharedArrayBuffer".to_string(),
            TransportMode::TransferableArrayBuffer => "TransferableMessage".to_string(),
        }
    }

    /// Dedicated GPU worker steward lifecycle state (0=Initializing, 1=Ready, 2=Running, 3=Suspended, 4=Terminated).
    pub fn etalon_worker_state(&self) -> u8 {
        self.etalon_steward.state() as u8
    }

    /// Dedicated GPU worker recovery session generation.
    pub fn etalon_session_generation(&self) -> u32 {
        self.etalon_steward.generation()
    }

    /// Phase 5 (affordability rail) — whether a device tier (`0`=Full, `1`=Eco, `2`=Reserve)
    /// collapses a qapp's 3D scene to its 2D pane under the budget rule. Pure (no state change);
    /// the qapp planner (`render::authoring`) uses the same `OperationalMode::supports_3d` source.
    pub fn budget_collapses_3d(&self, mode_code: u8) -> bool {
        let mode = match mode_code {
            0 => OperationalMode::Full,
            1 => OperationalMode::Eco,
            _ => OperationalMode::Reserve,
        };
        !mode.supports_3d()
    }

    /// Enable or mute U3 AcousticPlane (automatically off in Reserve mode).
    pub fn set_acoustic_enabled(&mut self, enabled: bool) {
        self.acoustic_enabled = enabled;
    }

    /// Whether the U3 AcousticPlane is effectively on — user flag AND
    /// the operational-mode gate (off in Reserve).
    pub fn acoustic_enabled(&self) -> bool {
        self.acoustic_enabled && acoustic_enabled_for_mode(global_vram_ledger().mode())
    }

    /// Drain pending sonic tokens into a JS `BigUint64Array` or `Array` of token raw values.
    pub fn drain_sonic_tokens(&self, max: u32) -> Result<JsValue, JsValue> {
        let cap = max.clamp(1, 64) as usize;
        let mut buf = vec![0u64; cap];
        let n = drain_sonic_tokens(&mut buf);
        buf.truncate(n);
        let arr = Array::new();
        for raw in buf {
            arr.push(&JsValue::from_f64(raw as f64));
        }
        Ok(arr.into())
    }

    /// Number of sonic tokens currently queued in the SPSC ring.
    pub fn sonic_token_pending(&self) -> u32 {
        crate::audio::acoustic_plane::sonic_token_ring().len() as u32
    }

    /// Serialized `AcousticUniform` bytes for AudioWorklet `SharedArrayBuffer` handoff.
    pub fn acoustic_uniform_bytes(&mut self) -> Result<js_sys::Uint8Array, JsValue> {
        let uniform = self.build_acoustic_uniform();
        let bytes = bytemuck::bytes_of(&uniform);
        Ok(js_sys::Uint8Array::from(bytes))
    }

    /// Flat `f32` uniform for AudioWorklet message port (18 scalars + 64 preview bins).
    pub fn acoustic_uniform_floats(&mut self) -> Result<js_sys::Float32Array, JsValue> {
        let u = self.build_acoustic_uniform();
        Ok(js_sys::Float32Array::from(
            &acoustic_uniform_to_floats(&u)[..],
        ))
    }

    /// Push a raw `u64` sonic token onto the acoustic ring; `false`
    /// when the ring is full.
    pub fn push_sonic_token_raw(&self, raw: u64) -> bool {
        push_sonic_token(SonicToken { raw })
    }

    /// SharedArrayBuffer byte length for zero-copy U3 handoff (requires COOP/COEP).
    pub fn acoustic_sab_byte_length(&self) -> u32 {
        ACOUSTIC_SAB_BYTES as u32
    }

    /// Allocate zeroed acoustic SAB with Q3AS header.
    pub fn create_acoustic_sab(&self) -> Result<js_sys::SharedArrayBuffer, JsValue> {
        let sab = js_sys::SharedArrayBuffer::new(ACOUSTIC_SAB_BYTES as u32);
        let view = js_sys::Uint8Array::new(&sab);
        let mut buf = [0u8; ACOUSTIC_SAB_BYTES];
        if !init_acoustic_sab(&mut buf) {
            return Err(JsValue::from_str("acoustic sab init failed"));
        }
        view.copy_from(&buf);
        Ok(sab)
    }

    /// Publish phenomenal uniform + pending sonic tokens into SAB.
    pub fn publish_acoustic_sab(&mut self, sab: &js_sys::SharedArrayBuffer) -> Result<(), JsValue> {
        if sab.byte_length() as usize != ACOUSTIC_SAB_BYTES {
            return Err(JsValue::from_str("acoustic sab size mismatch"));
        }
        let view = js_sys::Uint8Array::new(sab);
        let mut buf = [0u8; ACOUSTIC_SAB_BYTES];
        view.copy_to(&mut buf);
        let uniform = self.build_acoustic_uniform();
        let floats = acoustic_uniform_to_floats(&uniform);
        if !write_uniform_to_sab_with_mirror(&mut buf, &uniform, Some(&floats)) {
            return Err(JsValue::from_str("sab uniform write failed"));
        }
        let mut token_buf = [0u64; 16];
        let n = drain_sonic_tokens(&mut token_buf);
        for i in 0..n {
            let _ = push_token_to_sab(&mut buf, SonicToken { raw: token_buf[i] });
        }
        view.copy_from(&buf);
        Ok(())
    }

    /// Cold-bake STFT sidecar for selected tensor node; pins bytes for hot frame reads.
    pub fn bake_stft_sidecar_demo(&mut self, frames: u32) -> Result<js_sys::Uint8Array, JsValue> {
        self.bake_acoustic_sidecar_demo(frames, false)
    }

    /// Cold-bake CQT sidecar (log-spaced bins) for selected tensor node.
    pub fn bake_cqt_sidecar_demo(&mut self, frames: u32) -> Result<js_sys::Uint8Array, JsValue> {
        self.bake_acoustic_sidecar_demo(frames, true)
    }

    /// Cold-bake a spectral sidecar (STFT or CQT, `frames` clamped to
    /// 1–128) from the selected tensor node's preview bins; pins it on
    /// the portal and returns it as a `Uint8Array`. Fails when no
    /// tensor buffer is resident.
    fn bake_acoustic_sidecar_demo(
        &mut self,
        frames: u32,
        use_cqt: bool,
    ) -> Result<js_sys::Uint8Array, JsValue> {
        let node = self.selected_node.unwrap_or(0);
        let tensor = self
            .last_tensor
            .as_ref()
            .ok_or_else(|| JsValue::from_str("no tensor buffer"))?;
        let t = read_tensor_at(tensor, node as usize).map_err(|e| JsValue::from_str(e))?;
        let preview = preview_bins_from_tensor(&t);
        let frame_count = frames.clamp(1, 128);
        let need = std::mem::size_of::<
            crate::audio::audio_spectral_sheet::AudioSpectralSidecarHeader,
        >() + SPECTRAL_PREVIEW_BINS * frame_count as usize * 4;
        let mut buf = vec![0u8; need];
        if use_cqt {
            bake_cqt_sidecar_from_preview(&preview, frame_count, 48_000, &mut buf)
                .map_err(|_| JsValue::from_str("cqt bake failed"))?;
        } else {
            bake_tensor_stft_sidecar(&preview, frame_count, &mut buf)
                .map_err(|_| JsValue::from_str("stft bake failed"))?;
        }
        self.acoustic_sidecar = Some(buf.clone());
        self.acoustic_sidecar_frame = 0;
        Ok(js_sys::Uint8Array::from(&buf[..]))
    }

    /// Whether a baked STFT/CQT sidecar is pinned on the portal.
    pub fn acoustic_sidecar_pinned(&self) -> bool {
        self.acoustic_sidecar.is_some()
    }

    /// Resize the canvas backing store (and the GPU surface when
    /// active), then repaint.
    pub fn resize(
        &mut self,
        canvas: HtmlCanvasElement,
        width: u32,
        height: u32,
    ) -> Result<(), JsValue> {
        canvas.set_width(width);
        canvas.set_height(height);
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            gpu.resize(width, height)
                .map_err(|error| JsValue::from_str(&error))?;
        }
        self.paint_frame(&canvas)
    }

    /// Per-frame advance: device-loss check, scene animation, ICP
    /// control drain (≤16), time/telemetry/acoustic updates, camera-fly
    /// interpolation, then paint.
    pub fn tick(&mut self, canvas: HtmlCanvasElement, dt_ms: f32) -> Result<(), JsValue> {
        if self.note_device_loss() {
            self.etalon_steward.notify_device_lost();
        }
        self.tick_scene_animation(dt_ms)?;
        self.drain_and_apply_control(16);
        self.time += dt_ms as f64 * 0.001;
        self.telemetry.refresh_from_ledger();
        self.tick_acoustic_plane(dt_ms);
        if self.camera_fly.is_active() {
            self.camera = self.camera_fly.advance(self.camera);
            #[cfg(target_arch = "wasm32")]
            if let Some(ref mut gpu) = self.gpu {
                gpu.set_camera(self.camera.yaw, self.camera.pitch, self.camera.zoom);
            }
        }
        self.paint_frame(&canvas)
    }

    /// Queue GPU picking at canvas pixel `(x, y)`. Result available after the next `tick`.
    pub fn select_node_at(
        &mut self,
        x: f32,
        y: f32,
        canvas_w: u32,
        canvas_h: u32,
    ) -> Result<(), JsValue> {
        // A new pick must not expose the preceding result while GPU readback
        // is pending or after a miss.
        self.selected_node = None;
        self.selected_semantic_id = None;
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            gpu.queue_pick(x, y);
            self.pending_gpu_pick = true;
            return Ok(());
        }
        if let Some(ref tensor) = self.last_tensor {
            if let Some(idx) = cpu_pick_node_at_camera(
                tensor,
                canvas_w.max(1),
                canvas_h.max(1),
                x as f64,
                y as f64,
                self.time as f32,
                self.camera,
                &self.standpoint,
            ) {
                self.selected_node = Some(idx);
            }
        }
        Ok(())
    }

    /// Returns selected tensor index, or `-1` if none / pick still pending.
    pub fn poll_selected_node(&self) -> i32 {
        self.selected_node_index()
    }

    /// Selected tensor-node index, or `-1` while none/pick pending.
    pub fn selected_node_index(&self) -> i32 {
        self.selected_node.map(|n| n as i32).unwrap_or(-1)
    }

    /// Selected mesh semantic identity as hexadecimal, or an empty string while absent/pending.
    pub fn selected_semantic_id(&self) -> String {
        self.selected_semantic_id
            .map(|identity| format!("{identity:016x}"))
            .unwrap_or_default()
    }

    /// Frame the camera on a tensor node (`Maps_to_node`).
    pub fn navigate_to_node(&mut self, index: u32) -> Result<(), JsValue> {
        let tensor = self
            .last_tensor
            .as_ref()
            .ok_or_else(|| JsValue::from_str("no tensor buffer"))?;
        let t = read_tensor_at(tensor, index as usize).map_err(|e| JsValue::from_str(e))?;
        self.selected_node = Some(index);
        self.selected_semantic_id = None;
        let target = camera_frame_node([t.x, t.y, t.z]);
        self.camera_fly = CameraFlyTo::start_toward(target);
        Ok(())
    }

    /// Wavefunction collapse — set node `q` to 0 in the resident session manifold.
    pub fn collapse_node_q(&mut self, index: u32) -> Result<(), JsValue> {
        let Some(ref mut tensor) = self.last_tensor else {
            return Err(JsValue::from_str("no tensor buffer"));
        };
        let prev =
            write_tensor_q_at(tensor, index as usize, 0.0).map_err(|e| JsValue::from_str(e))?;
        if prev <= Q_COLLAPSED_EPS {
            return Ok(());
        }
        let bytes = tensor.clone();
        self.upload_tensor_buffer(&bytes)?;
        Ok(())
    }

    /// Select at pixel; returns index immediately on CPU fallback, else `-1` until next `tick`.
    pub fn observe_node_at(
        &mut self,
        x: f32,
        y: f32,
        canvas_w: u32,
        canvas_h: u32,
    ) -> Result<i32, JsValue> {
        self.select_node_at(x, y, canvas_w, canvas_h)?;
        Ok(self.poll_selected_node())
    }

    /// Apply a flat `f32` telemetry sample block to the HUD model.
    pub fn set_telemetry(&mut self, floats: &[f32]) -> Result<(), JsValue> {
        self.telemetry.apply_floats(floats);
        Ok(())
    }

    /// Set the display mode by name (`DisplayMode::from_str`;
    /// unrecognised names fall back to its default).
    pub fn set_display_mode(&mut self, mode: &str) -> Result<(), JsValue> {
        self.display_mode = DisplayMode::from_str(mode);
        Ok(())
    }

    /// Enable/disable the **ambient particle field** — the mixer's "ambient" channel. Off by
    /// default and opt-in: a plain mesh/anatomy view (or a game scene) keeps it off, and a
    /// Tensor10D upload never forces it on. Hosts that want the tensor-node particle view
    /// enable it explicitly.
    pub fn set_ambient_enabled(&mut self, on: bool) {
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            gpu.set_ambient_enabled(on);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = on;
    }

    /// Show or hide semantic tensor nodes while retaining uploaded data for picking.
    pub fn set_tensor_projection_enabled(&mut self, on: bool) {
        self.tensor_projection_enabled = on;
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            gpu.set_tensor_projection_enabled(on);
        }
    }

    /// Orbit camera IPC from the UI shell (yaw/pitch in radians, zoom = eye distance).
    pub fn set_camera(&mut self, yaw: f32, pitch: f32, zoom: f32) -> Result<(), JsValue> {
        let current_target = self.camera.target;
        self.camera = CameraState {
            near_plane: self.camera.near_plane,
            yaw,
            pitch,
            zoom,
            target: current_target,
            sun_dir: self.camera.sun_dir,
            sun_intensity: self.camera.sun_intensity,
            ambient_intensity: self.camera.ambient_intensity,
        }
        .clamped();
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            gpu.set_camera(self.camera.yaw, self.camera.pitch, self.camera.zoom);
        }
        Ok(())
    }

    /// Camera orbit yaw (radians).
    pub fn camera_yaw(&self) -> f32 {
        self.camera.yaw
    }

    /// Camera orbit pitch (radians).
    pub fn camera_pitch(&self) -> f32 {
        self.camera.pitch
    }

    /// Camera zoom (eye distance in world units).
    pub fn camera_zoom(&self) -> f32 {
        self.camera.zoom
    }

    /// RTS camera pan IPC (moves look-at center point in world space).
    pub fn set_camera_pan(
        &mut self,
        target_x: f32,
        target_y: f32,
        target_z: f32,
    ) -> Result<(), JsValue> {
        self.camera.target = [target_x, target_y, target_z];
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            gpu.set_camera_pan(target_x, target_y, target_z);
        }
        Ok(())
    }

    /// Full camera placement (yaw, pitch, zoom, and world-space target center).
    pub fn set_camera_target(
        &mut self,
        yaw: f32,
        pitch: f32,
        zoom: f32,
        target_x: f32,
        target_y: f32,
        target_z: f32,
    ) -> Result<(), JsValue> {
        self.camera = CameraState {
            yaw,
            pitch,
            zoom,
            target: [target_x, target_y, target_z],
            near_plane: self.camera.near_plane,
            sun_dir: self.camera.sun_dir,
            sun_intensity: self.camera.sun_intensity,
            ambient_intensity: self.camera.ambient_intensity,
        }
        .clamped();
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            gpu.set_camera_target(yaw, pitch, zoom, target_x, target_y, target_z);
        }
        Ok(())
    }

    /// X of the camera look-at target.
    pub fn camera_target_x(&self) -> f32 {
        self.camera.target[0]
    }

    /// Y of the camera look-at target.
    pub fn camera_target_y(&self) -> f32 {
        self.camera.target[1]
    }

    /// Z of the camera look-at target.
    pub fn camera_target_z(&self) -> f32 {
        self.camera.target[2]
    }

    /// Configure viewport background / sky clear color (r, g, b, a in 0.0..1.0).
    pub fn set_clear_color(&mut self, r: f64, g: f64, b: f64, a: f64) {
        self.sky_preset = None;
        self.sky_clear = [r as f32, g as f32, b as f32, a as f32];
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            gpu.set_clear_color(r, g, b, a);
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut webgl2) = self.anatomy_webgl2 {
            webgl2.set_clear(self.sky_clear);
        }
    }

    /// Configure directional sun lighting (direction vector, sun intensity, and ambient intensity).
    pub fn set_lighting(
        &mut self,
        sun_x: f32,
        sun_y: f32,
        sun_z: f32,
        sun_intensity: f32,
        ambient_intensity: f32,
    ) {
        self.camera.sun_dir = [sun_x, sun_y, sun_z];
        self.camera.sun_intensity = sun_intensity;
        self.camera.ambient_intensity = ambient_intensity;
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            gpu.set_lighting(sun_x, sun_y, sun_z, sun_intensity, ambient_intensity);
        }
    }

    /// Enable the budgeted half-resolution screen-space AO pass when available.
    pub fn set_screen_space_ao_enabled(&mut self, enabled: bool) {
        self.graphics
            .set_screen_space_ao_enabled(enabled, self.gpu.as_mut());
    }

    /// Tune SSAO kernel parameters (world-space radius, strength,
    /// depth bias); forwarded to the GPU when active.
    pub fn set_screen_space_ao(&mut self, radius: f32, strength: f32, bias: f32) {
        self.graphics
            .set_screen_space_ao(radius, strength, bias, self.gpu.as_mut());
    }

    /// Set the SSAO per-pixel sample count.
    pub fn set_screen_space_ao_sample_count(&mut self, samples: u32) {
        self.graphics
            .set_screen_space_ao_sample_count(samples, self.gpu.as_mut());
    }

    /// Toggle auto temporal smoothing; a change invalidates the
    /// temporal history so the first frame isn't blended from stale data.
    pub fn set_auto_temporal_enabled(&mut self, enabled: bool) {
        if self.graphics.set_auto_temporal_enabled(enabled) {
            self.graphics.invalidate_temporal_history(self.gpu.as_mut());
        }
    }

    /// Drop accumulated temporal history (e.g. after a camera cut or
    /// scene swap) so the next frame isn't ghosted.
    pub fn invalidate_temporal_history(&mut self) {
        self.graphics.invalidate_temporal_history(self.gpu.as_mut());
    }

    /// Toggle the shadow pass on the GPU renderer when present.
    pub fn set_shadows_enabled(&mut self, enabled: bool) {
        self.graphics
            .set_shadows_enabled(enabled, self.gpu.as_mut());
    }

    /// Whether the active backend supports the shadow pass.
    pub fn shadows_available(&self) -> bool {
        PortalGraphicsState::shadows_available(self.gpu.as_ref())
    }

    /// Whether SSAO is currently enabled on the GPU backend
    /// (`false` without an active GPU).
    pub fn screen_space_ao_enabled(&self) -> bool {
        self.gpu
            .as_ref()
            .map(PortalGpu::screen_space_ao_enabled)
            .unwrap_or(false)
    }

    /// Upload packed IBL probe coefficients plus a fallback radiance
    /// colour; `false`/error when the GPU backend can't take them.
    pub fn set_environment_probe_data(
        &mut self,
        packed_probes: &[f32],
        fallback_radiance: &[f32],
    ) -> Result<bool, JsValue> {
        self.graphics.set_environment_probe_data(
            packed_probes,
            fallback_radiance,
            self.gpu.as_mut(),
        )
    }

    /// Replace the water surface mesh using world-space XYZ triples and triangle indices.
    pub fn set_water_geometry(
        &mut self,
        positions_xyz: &[f32],
        indices: &[u32],
    ) -> Result<bool, JsValue> {
        self.graphics
            .set_water_geometry(positions_xyz, indices, self.gpu.as_mut())
    }

    /// Set water-surface appearance — RGBA colour, wave amplitude and
    /// frequency, and mesh `quality` tier.
    pub fn set_water_surface(
        &mut self,
        r: f32,
        g: f32,
        b: f32,
        alpha: f32,
        wave_amplitude: f32,
        wave_frequency: f32,
        quality: u32,
    ) {
        self.graphics.set_water_surface(
            [r, g, b, alpha, wave_amplitude, wave_frequency],
            quality,
            self.gpu.as_mut(),
        );
    }

    /// Temporal-graphics execution receipt for honesty reporting.
    pub fn temporal_graphics_receipt(&self) -> JsValue {
        self.graphics.temporal_receipt(self.gpu.as_ref())
    }

    /// Whether the backend supports the SSAO pass at all.
    pub fn screen_space_ao_available(&self) -> bool {
        PortalGraphicsState::screen_space_ao_available(self.gpu.as_ref())
    }

    /// SSAO target width (0 without an active GPU pass).
    pub fn screen_space_ao_width(&self) -> u32 {
        self.gpu
            .as_ref()
            .and_then(PortalGpu::screen_space_ao_resolution)
            .map(|extent| extent.0)
            .unwrap_or(0)
    }

    /// SSAO target height (0 without an active GPU pass).
    pub fn screen_space_ao_height(&self) -> u32 {
        self.gpu
            .as_ref()
            .and_then(PortalGpu::screen_space_ao_resolution)
            .map(|extent| extent.1)
            .unwrap_or(0)
    }
    /// Apply an authored atmospheric sky preset:
    /// - 0: Cyber-Dark / Deep Void
    /// - 1: Daylight (clear sky, high warm sun, balanced ambient)
    /// - 2: Sunset / Golden Hour (warm orange sky, low golden sun)
    /// - 3: Night / Moonlight (dark indigo sky, cold moonlight)
    /// Keep the next `load_body_*` upload in authored coordinates.
    /// Anatomy stays false (orbit frame). A town scene sets this so the
    /// camera and the mesh share one frame.
    pub fn set_preserve_authored_frame(&mut self, on: bool) {
        if self.preserve_authored_frame != on {
            self.hmc_mesh_residency.clear();
        }
        self.preserve_authored_frame = on;
    }

    /// Select an authored sky preset index
    /// (0 = deep void, 1 = daylight, 2 = sunset, 3 = moonlight).
    pub fn set_sky_preset(&mut self, preset: u32) {
        self.sky_preset = Some(preset);
        self.atmosphere_preset = preset;
        let profile = AtmospherePreset::from_id(preset);
        self.camera.sun_dir = profile.sun_direction;
        self.camera.sun_intensity = profile.sun_radiance;
        self.camera.ambient_intensity = profile.ambient_irradiance;
        self.sky_clear = profile.clear_rgba;
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            gpu.set_sky_preset(preset);
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut webgl2) = self.anatomy_webgl2 {
            webgl2.set_sky_preset(preset);
        }
    }

    /// Directional (sun) light intensity.
    pub fn sun_intensity(&self) -> f32 {
        self.camera.sun_intensity
    }

    /// Ambient light intensity.
    pub fn ambient_intensity(&self) -> f32 {
        self.camera.ambient_intensity
    }

    /// Human-Centric observer standpoint IPC (independent of camera lens).
    ///
    /// `standpoint_class`: 0=spectator, 1=ephemeral, 2=identifier (DID), 3=vault.
    /// `identifier_did`: empty for spectator/ephemeral; supply DID IRI to bind a verified
    /// identifier. Vault standpoints require a sealed local data plane (not exposed here).
    pub fn set_standpoint(
        &mut self,
        standpoint_class: u32,
        epistemic_q: f32,
        t_slice: f32,
        t_window: f32,
        identifier_did: &str,
    ) -> Result<(), JsValue> {
        let class = standpoint_class.min(STANDPOINT_VAULT);
        let hash = resolve_standpoint_hash(class, self.session_nonce, identifier_did);
        let fabric_gate = if class == STANDPOINT_DID && !identifier_did.is_empty() {
            FABRIC_SHARED
        } else {
            FABRIC_VIEWPORT_LOCAL
        };
        let deontic_lane = match class {
            STANDPOINT_VAULT => 2,
            _ => DEONTIC_LANE_COMMONS,
        };
        let epistemic = match class {
            STANDPOINT_SPECTATOR | STANDPOINT_EPHEMERAL => 1.0,
            STANDPOINT_VAULT => 0.0,
            _ => epistemic_q.clamp(0.0, 1.0),
        };
        self.standpoint = ObserverStandpoint::new(
            hash,
            self.session_nonce,
            class,
            epistemic,
            t_slice,
            t_window.max(0.0),
            deontic_lane,
            fabric_gate,
        );
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            gpu.set_standpoint(self.standpoint);
        }
        Ok(())
    }

    /// Current standpoint class (observer role code).
    pub fn standpoint_class(&self) -> u32 {
        self.standpoint.standpoint_class
    }

    /// Standpoint epistemic-quality scalar ∈ [0, 1].
    pub fn epistemic_q(&self) -> f32 {
        self.standpoint.epistemic_q
    }

    /// Standpoint temporal-slice position ∈ [0, 1].
    pub fn t_slice(&self) -> f32 {
        self.standpoint.t_slice
    }

    /// Standpoint temporal-window width ∈ (0, 1].
    pub fn t_window(&self) -> f32 {
        self.standpoint.t_window
    }

    /// Replace the standpoint temporal slice/window and push it to the
    /// GPU when active.
    pub fn set_temporal_slice(&mut self, t_slice: f32, t_window: f32) {
        self.standpoint = self.standpoint.with_temporal(t_slice, t_window);
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            gpu.set_standpoint(self.standpoint);
        }
    }

    /// Spatially encode a JSON scene, upload the resulting tensor
    /// buffer, and return the parsed scene record.
    pub fn encode_geometry(&mut self, json: &str) -> Result<JsValue, JsValue> {
        let parsed = spatial_encode_wasm(json)?;
        let buf_js = export_tensor_buffer_wasm(json)?;
        let u8arr: js_sys::Uint8Array = buf_js.dyn_into()?;
        let mut bytes = vec![0u8; u8arr.length() as usize];
        u8arr.copy_to(&mut bytes);
        self.upload_tensor_buffer(&bytes)?;
        Ok(parsed)
    }

    /// Import a 3D mesh asset (OBJ / STL / GLB bytes) and render it as a solid surface (Phase 1.2).
    /// The mesh is centred on its bounding-box centroid and scaled so its largest extent is ~1.6
    /// units — fitting the orbit camera's default frame (eye at distance 3.5, looking at the origin)
    /// — then uploaded to the GPU. `hint` is an optional lowercase extension ("obj"/"stl"/"glb");
    /// empty = sniff from the bytes. Returns the triangle count (0 if the GPU path isn't active).
    pub fn upload_mesh_asset(&mut self, bytes: &[u8], hint: &str) -> Result<u32, JsValue> {
        self.hmc_mesh_residency.clear();
        let hint_opt = if hint.is_empty() { None } else { Some(hint) };
        let imported = crate::render::assets::import_asset_with_normals(bytes, hint_opt)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let mesh = imported.mesh;
        let c = mesh.centroid();
        let ext = [
            mesh.max[0] - mesh.min[0],
            mesh.max[1] - mesh.min[1],
            mesh.max[2] - mesh.min[2],
        ];
        let span = ext[0].max(ext[1]).max(ext[2]).max(1e-6);
        let s = 1.6 / span;
        let positions: Vec<[f32; 3]> = mesh
            .positions
            .iter()
            .map(|p| [(p[0] - c[0]) * s, (p[1] - c[1]) * s, (p[2] - c[2]) * s])
            .collect();
        let indices: Vec<u32> = mesh
            .triangles
            .iter()
            .flat_map(|t| [t[0], t[1], t[2]])
            .collect();
        let tris = match self.gpu {
            Some(ref mut gpu) => gpu.upload_mesh_colored_with_frames(
                &positions,
                &[],
                imported.normals.as_deref(),
                imported.tangents.as_deref(),
                &indices,
            ),
            None => 0,
        };
        self.description = format!("{tris} mesh triangles · T2 surface");
        Ok(tris)
    }

    /// Replace the current WebGPU mesh instance stream from caller-owned packed records.
    /// Each 128-byte record is the native `GpuInstanceRecord` ABI: column-major world transform,
    /// inverse-transpose normal frame, low/high `u32` semantic identity words, orientation sign,
    /// and zero padding. Callers may compact by CPU visibility indices before passing this slice.
    pub fn set_mesh_instances(
        &mut self,
        abi_version: u32,
        packed_records: &[u8],
    ) -> Result<u32, JsValue> {
        if abi_version != crate::render::instance_culling::GPU_INSTANCE_RECORD_ABI_VERSION {
            return Err(JsValue::from_str(
                "mesh_instance_record_abi_version_unsupported",
            ));
        }
        let record_size = std::mem::size_of::<GpuInstanceRecord>();
        if packed_records.len() % record_size != 0 {
            return Err(JsValue::from_str(
                "mesh_instance_stream_length_not_multiple_of_record_size",
            ));
        }
        let Some(gpu) = self.gpu.as_mut() else {
            return Err(JsValue::from_str("webgpu_mesh_instance_stream_unavailable"));
        };
        gpu.set_mesh_instances_packed(packed_records).map_err(
            |error: MeshInstanceUploadError| {
                JsValue::from_str(&format!("mesh_instance_upload_failed:{error:?}"))
            },
        )?;
        u32::try_from(packed_records.len() / record_size)
            .map_err(|_| JsValue::from_str("mesh_instance_count_exceeds_u32"))
    }

    /// Select visible source instances without allocating. Bounds use three floats per record,
    /// transforms use 16 column-major floats, and the view-projection matrix uses 16 floats.
    /// Invalid bounds fail open; returned indices preserve source order.
    pub fn cull_mesh_instance_indices(
        &self,
        bounds_min: &[f32],
        bounds_max: &[f32],
        transforms_column_major: &[f32],
        view_projection_column_major: &[f32],
        visible_indices: &mut [u32],
    ) -> Result<u32, JsValue> {
        crate::render::instance_culling::select_visible_instances_packed(
            bounds_min,
            bounds_max,
            transforms_column_major,
            view_projection_column_major,
            visible_indices,
        )
        .and_then(|count| {
            u32::try_from(count).map_err(|_| {
                crate::render::instance_culling::VisibilityListError::TooManyInstances { count }
            })
        })
        .map_err(|error| JsValue::from_str(&format!("mesh_instance_cull_failed:{error:?}")))
    }

    /// Compact ABI-v2 records by stable visible indices into caller-owned bytes, preserving IDs.
    pub fn compact_mesh_instance_records(
        &self,
        source_records: &[u8],
        visible_indices: &[u32],
        compacted_output: &mut [u8],
    ) -> Result<u32, JsValue> {
        crate::render::instance_culling::compact_visible_packed_records(
            source_records,
            visible_indices,
            compacted_output,
        )
        .and_then(|count| {
            u32::try_from(count).map_err(|_| {
                crate::render::instance_culling::VisibilityListError::TooManyInstances { count }
            })
        })
        .map_err(|error| JsValue::from_str(&format!("mesh_instance_compact_failed:{error:?}")))
    }

    /// Build the packed ABI from caller-owned column-major transforms and low/high ID words.
    /// Output must hold `instance_count * mesh_instance_record_stride()` bytes.
    pub fn pack_mesh_instance_records(
        &self,
        transforms_column_major: &[f32],
        semantic_id_words: &[u32],
        packed_output: &mut [u8],
    ) -> Result<u32, JsValue> {
        if transforms_column_major.len() % 16 != 0 || semantic_id_words.len() % 2 != 0 {
            return Err(JsValue::from_str("mesh_instance_pack_input_shape_invalid"));
        }
        let count = transforms_column_major.len() / 16;
        if count > crate::render::instance_culling::MAX_GPU_MESH_INSTANCES {
            return Err(JsValue::from_str("mesh_instance_count_exceeds_10000"));
        }
        if semantic_id_words.len() / 2 != count {
            return Err(JsValue::from_str(
                "mesh_instance_pack_identity_count_mismatch",
            ));
        }
        let record_size = std::mem::size_of::<GpuInstanceRecord>();
        let required = count
            .checked_mul(record_size)
            .ok_or_else(|| JsValue::from_str("mesh_instance_pack_size_overflow"))?;
        if packed_output.len() < required {
            return Err(JsValue::from_str("mesh_instance_pack_output_too_small"));
        }
        for index in 0..count {
            let matrix_values = &transforms_column_major[index * 16..(index + 1) * 16];
            let matrix = std::array::from_fn(|column| {
                std::array::from_fn(|row| matrix_values[column * 4 + row])
            });
            let id_words = &semantic_id_words[index * 2..index * 2 + 2];
            let semantic_id = u64::from(id_words[0]) | (u64::from(id_words[1]) << 32);
            let record = GpuInstanceRecord::new(matrix, semantic_id);
            let output = &mut packed_output[index * record_size..(index + 1) * record_size];
            output.copy_from_slice(bytemuck::bytes_of(&record));
        }
        u32::try_from(count).map_err(|_| JsValue::from_str("mesh_instance_count_exceeds_u32"))
    }

    /// Byte stride of the packed mesh-instance record ABI exposed by `set_mesh_instances`.
    pub fn mesh_instance_record_stride(&self) -> u32 {
        std::mem::size_of::<GpuInstanceRecord>() as u32
    }

    /// Version of the packed ABI accepted by `set_mesh_instances`.
    pub fn mesh_instance_record_version(&self) -> u32 {
        crate::render::instance_culling::GPU_INSTANCE_RECORD_ABI_VERSION
    }

    /// P9.2 — Load a `.10d` container asset: parse the section table, extract
    /// the QuantizedMesh and Tensor10DNodes (provenance) sections, upload the
    /// mesh to the GPU, and report node/triangle counts.
    ///
    /// **Governance fail-closed:** if the header carries
    /// `FLAG_DEFAULT_DISPOSITION_REFUSE` (bit 0) and no attestation section is
    /// present, the mesh is loaded for display but `description` marks it as
    /// governance-refused — not citable as provenance until attested.
    ///
    /// Returns a JS object `{ vertex_count, triangle_count, provenance_mu, tier }`.
    pub fn load_10d(&mut self, bytes: &[u8]) -> Result<JsValue, JsValue> {
        self.hmc_mesh_residency.clear();
        use crate::container_10d::{
            self,
            header::{Container10dHeader, FLAG_DEFAULT_DISPOSITION_REFUSE},
        };

        let mut bytes_mut = bytes.to_vec();
        let header = Container10dHeader::parse(&bytes_mut)
            .map_err(|e| JsValue::from_str(&format!("10d header: {e}")))?;

        // Verify whole-file CRC-32C.
        container_10d::verify_whole_file_crc32c(&mut bytes_mut)
            .map_err(|e| JsValue::from_str(&format!("10d CRC: {e}")))?;

        let descs = container_10d::parse_section_table(&bytes_mut, &header)
            .map_err(|e| JsValue::from_str(&format!("10d section table: {e}")))?;

        let mut mesh = None;

        let mut provenance_mu: f32 = 0.0;
        let mut has_attestation = false;

        for desc in descs.iter() {
            let st = container_10d::SectionType::from_u8(desc.section_type)
                .ok_or_else(|| JsValue::from_str("10d: unknown section type"))?;
            let off = desc.byte_offset as usize;
            let len = desc.byte_length as usize;
            let payload = &bytes_mut[off..off + len];

            match st {
                container_10d::SectionType::QuantizedMesh => {
                    mesh = Some(
                        container_10d::decode_mesh_section(payload)
                            .map_err(|e| JsValue::from_str(&format!("10d mesh decode: {e}")))?,
                    );
                }
                container_10d::SectionType::Tensor10DNodes => {
                    if let Ok(t) = container_10d::read_node(payload, 0) {
                        provenance_mu = t.mu;
                    }
                }
                container_10d::SectionType::ProvenanceSidecar => {
                    if let Ok(view) =
                        crate::container_10d::provenance_section::decode_provenance_section(payload)
                    {
                        if crate::container_10d::provenance_section::validate_provenance(&view)
                            .is_ok()
                        {
                            has_attestation = true;
                        }
                    }
                }
                _ => {}
            }
        }

        let mesh = mesh.ok_or_else(|| JsValue::from_str("10d: no mesh section"))?;
        // Shared SDK owns legacy/MID2 selection; keep this oversized portal as glue.
        let frames = container_10d::read_container_mesh_frames_with_limits(
            &bytes_mut, mesh.vertex_count(),
            container_10d::MeshFrameDecodeLimits {
                max_vertices: container_10d::MAX_NORMAL_COUNT,
                max_payload_bytes: 64 * 1024 * 1024,
                max_decoded_bytes: 256 * 1024 * 1024,
            },
        )
            .map_err(|e| JsValue::from_str(&format!("10d frames decode: {e}")))?;
        let (authored_normals, authored_tangents) = frames
            .map(|frames| (frames.normals, frames.tangents)).unwrap_or((None, None));
        if authored_normals
            .as_ref()
            .is_some_and(|normals: &Vec<[f32; 3]>| normals.len() != mesh.vertex_count())
        {
            return Err(JsValue::from_str(
                "10d normal field vertex count does not match mesh",
            ));
        }
        if authored_tangents
            .as_ref()
            .is_some_and(|tangents: &Vec<[f32; 4]>| tangents.len() != mesh.vertex_count())
        {
            return Err(JsValue::from_str(
                "10d tangent field vertex count does not match mesh",
            ));
        }

        // Governance fail-closed: default-Refuse flag set and no attestation →
        // mesh is displayable but not citable.
        let governance_refused =
            (header.flags & FLAG_DEFAULT_DISPOSITION_REFUSE) != 0 && !has_attestation;

        // Centre + scale the mesh to the orbit frame (same as upload_mesh_asset).
        let c = mesh.centroid();
        let ext = [
            mesh.max[0] - mesh.min[0],
            mesh.max[1] - mesh.min[1],
            mesh.max[2] - mesh.min[2],
        ];
        let span = ext[0].max(ext[1]).max(ext[2]).max(1e-6);
        let s = 1.6 / span;
        let positions: Vec<[f32; 3]> = mesh
            .positions
            .iter()
            .map(|p| [(p[0] - c[0]) * s, (p[1] - c[1]) * s, (p[2] - c[2]) * s])
            .collect();
        let indices: Vec<u32> = mesh
            .triangles
            .iter()
            .flat_map(|t| [t[0], t[1], t[2]])
            .collect();

        let tri_count = mesh.triangles.len() as u32;
        let vert_count = mesh.positions.len() as u32;

        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            gpu.upload_mesh_colored_with_frames(
                &positions,
                &[],
                authored_normals.as_deref(),
                authored_tangents.as_deref(),
                &indices,
            );
            self.tier = 2;
        }

        if governance_refused {
            self.description = format!(
                "{tri_count} triangles · governance REFUSED (no attestation) · μ={provenance_mu:.3}"
            );
        } else {
            self.description = format!(
                "{tri_count} triangles · {vert_count} vertices · μ={provenance_mu:.3} · T2"
            );
        }

        let result = js_sys::Object::new();
        Reflect::set(
            &result,
            &JsValue::from_str("vertex_count"),
            &JsValue::from_f64(vert_count as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("triangle_count"),
            &JsValue::from_f64(tri_count as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("provenance_mu"),
            &JsValue::from_f64(provenance_mu as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("tier"),
            &JsValue::from_f64(self.tier as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("governance_refused"),
            &JsValue::from_bool(governance_refused),
        )?;

        Ok(result.into())
    }

    /// Load one verified `.10d` mesh from an HMC bundle and retain its digest-backed
    /// residency identity. Replaying the same asset returns the existing receipt without
    /// rebuilding the GPU mesh; a different digest replaces the resident mesh.
    pub fn load_hmc_mesh(&mut self, hmc_bytes: &[u8], asset_key: &str) -> Result<JsValue, JsValue> {
        let source =
            resolve_hmc_mesh(hmc_bytes, asset_key).map_err(|error| JsValue::from_str(&error))?;
        if let Some(receipt) = self.hmc_mesh_residency.resident(source.digest) {
            let result = js_sys::Object::new();
            Reflect::set(
                &result,
                &JsValue::from_str("vertex_count"),
                &JsValue::from_f64(receipt.vertex_count as f64),
            )?;
            Reflect::set(
                &result,
                &JsValue::from_str("triangle_count"),
                &JsValue::from_f64(receipt.triangle_count as f64),
            )?;
            Reflect::set(
                &result,
                &JsValue::from_str("generation"),
                &JsValue::from_f64(receipt.generation as f64),
            )?;
            Reflect::set(
                &result,
                &JsValue::from_str("upload_count"),
                &JsValue::from_f64(receipt.upload_count as f64),
            )?;
            return Ok(result.into());
        }

        let loaded = self.load_10d(source.bytes)?;
        let vertex_count = Reflect::get(&loaded, &JsValue::from_str("vertex_count"))?
            .as_f64()
            .ok_or_else(|| JsValue::from_str("hmc_mesh_result_missing_vertex_count"))?;
        let triangle_count = Reflect::get(&loaded, &JsValue::from_str("triangle_count"))?
            .as_f64()
            .ok_or_else(|| JsValue::from_str("hmc_mesh_result_missing_triangle_count"))?;
        let receipt = self.hmc_mesh_residency.record_upload(
            source.digest,
            u32::try_from(vertex_count as u64)
                .map_err(|_| JsValue::from_str("hmc_mesh_vertex_count_overflow"))?,
            u32::try_from((triangle_count as u64).saturating_mul(3))
                .map_err(|_| JsValue::from_str("hmc_mesh_index_count_overflow"))?,
            u32::try_from(triangle_count as u64)
                .map_err(|_| JsValue::from_str("hmc_mesh_triangle_count_overflow"))?,
        );
        Reflect::set(
            &loaded,
            &JsValue::from_str("generation"),
            &JsValue::from_f64(receipt.generation as f64),
        )?;
        Reflect::set(
            &loaded,
            &JsValue::from_str("upload_count"),
            &JsValue::from_f64(receipt.upload_count as f64),
        )?;
        Ok(loaded)
    }

    /// S5.1 colour-by-load — like [`load_10d`] but paints the whole organ mesh a single uniform linear
    /// RGBA. The host resolves each organ's body-system percept
    /// (`qualia-client-core … AnatomyViewReport::paint_organs`) and passes that system's σ-derived colour
    /// (`OrganPercept.percept.rgba`) here, so the 3D body is coloured by accumulated burden. Same
    /// governance fail-closed as `load_10d`. (Deliberately parallels `load_10d` rather than sharing a
    /// refactored helper: the portal path is wasm+GPU-only and not runtime-testable here, so the proven
    /// `load_10d` is left untouched — unify them in the browser-test pass when the anatomy GLBs land.)
    pub fn load_10d_colored(
        &mut self,
        bytes: &[u8],
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    ) -> Result<JsValue, JsValue> {
        self.hmc_mesh_residency.clear();
        use crate::container_10d::{
            self,
            header::{Container10dHeader, FLAG_DEFAULT_DISPOSITION_REFUSE},
        };

        let mut bytes_mut = bytes.to_vec();
        let header = Container10dHeader::parse(&bytes_mut)
            .map_err(|e| JsValue::from_str(&format!("10d header: {e}")))?;
        container_10d::verify_whole_file_crc32c(&mut bytes_mut)
            .map_err(|e| JsValue::from_str(&format!("10d CRC: {e}")))?;
        let descs = container_10d::parse_section_table(&bytes_mut, &header)
            .map_err(|e| JsValue::from_str(&format!("10d section table: {e}")))?;

        let mut mesh = None;
        let mut provenance_mu: f32 = 0.0;
        let mut has_attestation = false;
        for desc in descs.iter() {
            let st = container_10d::SectionType::from_u8(desc.section_type)
                .ok_or_else(|| JsValue::from_str("10d: unknown section type"))?;
            let off = desc.byte_offset as usize;
            let len = desc.byte_length as usize;
            let payload = &bytes_mut[off..off + len];
            match st {
                container_10d::SectionType::QuantizedMesh => {
                    mesh = Some(
                        container_10d::decode_mesh_section(payload)
                            .map_err(|e| JsValue::from_str(&format!("10d mesh decode: {e}")))?,
                    );
                }
                container_10d::SectionType::Tensor10DNodes => {
                    if let Ok(t) = container_10d::read_node(payload, 0) {
                        provenance_mu = t.mu;
                    }
                }
                container_10d::SectionType::ProvenanceSidecar => {
                    if let Ok(view) =
                        crate::container_10d::provenance_section::decode_provenance_section(payload)
                    {
                        if crate::container_10d::provenance_section::validate_provenance(&view)
                            .is_ok()
                        {
                            has_attestation = true;
                        }
                    }
                }
                _ => {}
            }
        }

        let mesh = mesh.ok_or_else(|| JsValue::from_str("10d: no mesh section"))?;
        let governance_refused =
            (header.flags & FLAG_DEFAULT_DISPOSITION_REFUSE) != 0 && !has_attestation;

        let c = mesh.centroid();
        let ext = [
            mesh.max[0] - mesh.min[0],
            mesh.max[1] - mesh.min[1],
            mesh.max[2] - mesh.min[2],
        ];
        let span = ext[0].max(ext[1]).max(ext[2]).max(1e-6);
        let s = 1.6 / span;
        let positions: Vec<[f32; 3]> = mesh
            .positions
            .iter()
            .map(|p| [(p[0] - c[0]) * s, (p[1] - c[1]) * s, (p[2] - c[2]) * s])
            .collect();
        let indices: Vec<u32> = mesh
            .triangles
            .iter()
            .flat_map(|t| [t[0], t[1], t[2]])
            .collect();

        let tri_count = mesh.triangles.len() as u32;
        let vert_count = mesh.positions.len() as u32;

        let mut uploaded = false;
        if let Some(ref mut gpu) = self.gpu {
            let colors = vec![[r, g, b, a]; positions.len()];
            uploaded = gpu.upload_mesh_colored(&positions, &colors, &indices) > 0;
            if uploaded {
                self.tier = 2;
            }
        }

        if governance_refused {
            self.description = format!(
                "{tri_count} triangles · governance REFUSED (no attestation) · μ={provenance_mu:.3}"
            );
        } else {
            self.description = format!(
                "{tri_count} triangles · {vert_count} vertices · μ={provenance_mu:.3} · coloured · {}",
                if uploaded {
                    "T2"
                } else {
                    "not uploaded: geometry budget/backend unavailable"
                }
            );
        }

        let result = js_sys::Object::new();
        Reflect::set(
            &result,
            &JsValue::from_str("vertex_count"),
            &JsValue::from_f64(vert_count as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("triangle_count"),
            &JsValue::from_f64(tri_count as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("provenance_mu"),
            &JsValue::from_f64(provenance_mu as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("governance_refused"),
            &JsValue::from_bool(governance_refused),
        )?;

        Ok(result.into())
    }

    /// S5.8 — load the **whole body** as a set of per-organ `.10d` meshes, each painted its body-system's
    /// σ-derived RGBA, accumulated into one combined GPU mesh. This is the real-mesh render path.
    ///
    /// The CCF/HRA reference organs are authored in ONE shared body coordinate space (a brain's vertices
    /// sit at the head, a bladder's at the pelvis, skin envelops the whole body), so this **preserves
    /// each organ's TRUE position and relative size**: it accumulates the whole-body bounds across all
    /// organs and applies **one global centre + scale**, rather than normalising each organ separately
    /// (which would flatten proportions and shrink the full-body skin mesh to a dot). Governance
    /// fail-closed per organ, as in `load_10d_colored`.
    ///
    /// `organs` is a JS `Array` of objects: `{ bytes: Uint8Array, r: f32, g: f32, b: f32, a: f32 }`
    /// (per-organ colour). Any `x/y/z` fields are ignored — the mesh already carries its position.
    /// Returns `{ organs_loaded, organs_refused, total_triangles }`.
    ///
    /// Prefer [`Self::load_body_from_qualia_bundle_mixed`] for packs — that path never materialises a
    /// per-organ JS `Uint8Array` copy (critical on phones).
    pub fn load_body_organs_colored(&mut self, organs: &Array) -> Result<JsValue, JsValue> {
        let mut accum = BodyMeshAccum::new();
        let spans = Array::new();
        for i in 0..organs.length() {
            let organ = organs.get(i);
            let before = accum.positions.len() as u32;
            let bytes_val = Reflect::get(&organ, &JsValue::from_str("bytes"))
                .map_err(|_| JsValue::from_str("organ.bytes missing"))?;
            // One JS→Rust copy only (no secondary clone for CRC).
            let bytes: Vec<u8> = js_sys::Uint8Array::new(&bytes_val).to_vec();
            let r = Reflect::get(&organ, &JsValue::from_str("r"))
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5) as f32;
            let g = Reflect::get(&organ, &JsValue::from_str("g"))
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.6) as f32;
            let b = Reflect::get(&organ, &JsValue::from_str("b"))
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.8) as f32;
            let a = Reflect::get(&organ, &JsValue::from_str("a"))
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32;
            accum.append_organ_10d(&bytes, [r, g, b, a]);
            let count = accum.positions.len() as u32 - before;
            if count > 0 {
                let id = Reflect::get(&organ, &JsValue::from_str("id"))
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();
                let span = js_sys::Object::new();
                Reflect::set(&span, &JsValue::from_str("id"), &JsValue::from_str(&id))?;
                Reflect::set(
                    &span,
                    &JsValue::from_str("vertex_start"),
                    &JsValue::from_f64(before as f64),
                )?;
                Reflect::set(
                    &span,
                    &JsValue::from_str("vertex_count"),
                    &JsValue::from_f64(count as f64),
                )?;
                spans.push(&span);
            }
        }
        let result = self.finish_body_mesh_upload(accum)?;
        Reflect::set(&result, &JsValue::from_str("parts"), &spans)?;
        self.present_misses = 0;
        Ok(result)
    }

    /// Load a multi-asset scene: `Array<{bytes: .10d, r,g,b,a, id?}>` (QG-20).
    /// General scene asset loader for environment/game scenes, town layouts,
    /// vehicles, and user props.
    pub fn load_assets_colored(&mut self, assets: &Array) -> Result<JsValue, JsValue> {
        self.load_body_organs_colored(assets)
    }

    /// Generic scene asset alias for [`Self::load_assets_colored`] (QG-20).
    pub fn load_scene_assets(&mut self, assets: &Array) -> Result<JsValue, JsValue> {
        self.load_body_organs_colored(assets)
    }

    /// S5.8 (web) — load the whole body directly from a `.hmc` **anatomy pack**
    /// bundle (see [`crate::bundle`]). Parses the bundle with the *shared* Rust
    /// reader (the same code the native host uses — "one reader, both channels"),
    /// reads each organ's sealed `.10d` plus its
    /// [`AnatomyOrganMeta`](crate::render::anatomy_pack::AnatomyOrganMeta) (system
    /// colour + anatomical position), and hands them to
    /// [`Self::load_body_organs_colored`]. This is the pure-web render path — no
    /// Tauri host / `webizen://` needed: the browser fetches one `.hmc` file and
    /// renders the real body. Returns the same `{organs_loaded, organs_refused,
    /// total_triangles}` summary.
    pub fn load_body_from_qualia_bundle(&mut self, bytes: &[u8]) -> Result<JsValue, JsValue> {
        self.load_body_from_qualia_bundle_mixed(bytes, JsValue::UNDEFINED, JsValue::UNDEFINED)
    }

    /// Read a `.hmc` pack's **manifest** without rendering — the list of parts the UI builds its
    /// dynamic system + part selectors from. Returns a JS array of `{ key, label, system, systems }`
    /// (one per `.10d` entry), so the demo can offer per-system *and* per-part select/deselect driven by
    /// what is actually in the loaded pack, not a hardcoded list. Read-only.
    pub fn pack_manifest(&self, bytes: &[u8]) -> Result<JsValue, JsValue> {
        use crate::bundle::BundleReader;
        use crate::render::anatomy_pack::AnatomyOrganMeta;

        let reader = BundleReader::parse(bytes)
            .map_err(|e| JsValue::from_str(&format!("qualia bundle: {e}")))?;
        let parts = js_sys::Array::new();
        for entry in reader.entries() {
            if entry.kind != "10d" {
                continue;
            }
            let Some(meta) = entry.meta.as_deref().and_then(AnatomyOrganMeta::from_cbor) else {
                continue;
            };
            let obj = js_sys::Object::new();
            Reflect::set(
                &obj,
                &JsValue::from_str("key"),
                &JsValue::from_str(&entry.key),
            )?;
            let label = if meta.label.is_empty() {
                entry.key.as_str()
            } else {
                meta.label.as_str()
            };
            Reflect::set(&obj, &JsValue::from_str("label"), &JsValue::from_str(label))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("system"),
                &JsValue::from_str(&meta.system),
            )?;
            let systems = js_sys::Array::new();
            if meta.systems.is_empty() {
                systems.push(&JsValue::from_str(&meta.system));
            } else {
                for s in &meta.systems {
                    systems.push(&JsValue::from_str(s));
                }
            }
            Reflect::set(&obj, &JsValue::from_str("systems"), &systems)?;
            parts.push(&obj);
        }
        Ok(parts.into())
    }

    /// Like [`Self::load_body_from_qualia_bundle`] but honours the **mixer's per-body-system
    /// channels**: `system_levels` is a JS object `{ <system_id>: <level 0..1> }`. An organ whose
    /// system level is ≤ 0 is omitted (muted); otherwise its colour alpha is scaled by the level.
    /// (The mesh pipeline is currently opaque, so a nonzero level acts as show; smooth opacity lands
    /// when the mesh pipeline gains alpha blending — mixer plan P2.) An absent/empty map shows every
    /// system at full — so `load_body_from_qualia_bundle` is exactly this with no mixer applied.
    ///
    /// Decodes organs **in Rust** from the pack buffer — no per-organ JS `Uint8Array` materialisation.
    /// That cut peak heap by ~1–2× pack size and is the phone-safe path.
    pub fn load_body_from_qualia_bundle_mixed(
        &mut self,
        bytes: &[u8],
        system_levels: JsValue,
        disabled_parts: JsValue,
    ) -> Result<JsValue, JsValue> {
        use crate::bundle::BundleReader;
        use crate::render::anatomy_pack::AnatomyOrganMeta;

        let levels: std::collections::HashMap<String, f32> =
            serde_wasm_bindgen::from_value(system_levels).unwrap_or_default();
        // Individually deselected parts (by entry key) — the parts-list checkboxes. Absent/empty = none.
        let disabled: std::collections::HashSet<String> =
            serde_wasm_bindgen::from_value(disabled_parts).unwrap_or_default();

        let reader = BundleReader::parse(bytes)
            .map_err(|e| JsValue::from_str(&format!("qualia bundle: {e}")))?;

        let mut accum = BodyMeshAccum::new();
        for entry in reader.entries() {
            if entry.kind != "10d" {
                continue;
            }
            if disabled.contains(&entry.key) {
                continue;
            }
            let Some(meta) = entry.meta.as_deref().and_then(AnatomyOrganMeta::from_cbor) else {
                continue;
            };
            let level = levels.get(&meta.system).copied().unwrap_or(1.0);
            if level <= 0.0 {
                continue;
            }
            let Some(organ_bytes) = reader.get(&entry.key) else {
                continue;
            };
            // Single Rust-side copy for CRC verify (mutates header CRC field in-place).
            // Never crosses the JS heap — critical on Android Chrome (~100 MB packs).
            accum.append_organ_10d(
                organ_bytes,
                [
                    meta.rgba[0],
                    meta.rgba[1],
                    meta.rgba[2],
                    meta.rgba[3] * level,
                ],
            );
        }
        self.finish_body_mesh_upload(accum)
    }

    /// Replace the person-authored body fit. Pass JSON matching wellfare `BodyFit`.
    /// Applied on the next `load_body_*` upload. Empty / invalid JSON resets to identity.
    pub fn set_body_fit_json(&mut self, json: &str) {
        if json.trim().is_empty() {
            self.body_fit = crate::render::body_fit::AnatomyBodyFit::default();
            return;
        }
        match serde_json::from_str::<crate::render::body_fit::AnatomyBodyFit>(json) {
            Ok(fit) => self.body_fit = fit,
            Err(_) => self.body_fit = crate::render::body_fit::AnatomyBodyFit::default(),
        }
    }

    /// Finalise a body-mesh upload accumulated across append calls:
    /// uploads to the active backend (WebGPU/WebGL2/CPU canvas), updates
    /// residency accounting, and returns the manifest receipt.
    fn finish_body_mesh_upload(&mut self, mut accum: BodyMeshAccum) -> Result<JsValue, JsValue> {
        self.adopt_pending_gpu();
        accum
            .apply_body_fit(&self.body_fit)
            .map_err(JsValue::from_str)?;
        if !self.preserve_authored_frame {
            accum
                .normalise_to_orbit_frame()
                .map_err(JsValue::from_str)?;
        }
        if accum.positions.is_empty() || accum.indices.is_empty() {
            return Err(JsValue::from_str("anatomy_body_mesh_empty"));
        }

        // Keep a compact, bounded replay mesh for adapter loss and a CPU proof.
        // It also gives admission refusal a lower-memory retry before degrading
        // to canvas. The full authoring mesh remains cold-path input only.
        let reduced = CpuBodyMesh::decimate(
            &accum.positions,
            &accum.colors,
            &accum.indices,
            &accum.index_spans,
        );
        let mut renderer = BodyRendererBackend::None;
        let mut material_pipeline = false;
        let mut admitted_vertex_count = 0usize;
        let mut admitted_index_count = 0usize;
        let mut compact_gpu = true;
        if let Some(ref mut gpu) = self.gpu {
            let pbr_count = if accum.materials.is_empty() || accum.material_ranges.is_empty() {
                0
            } else {
                gpu.upload_mesh_colored_with_frames_and_materials_and_uv0(
                    &accum.positions,
                    &accum.colors,
                    Some(&accum.normals),
                    None,
                    Some(&accum.uv0),
                    &accum.indices,
                    &accum.materials,
                    &accum.material_ranges,
                )
                .unwrap_or(0)
            };
            let count = if pbr_count > 0 {
                material_pipeline = true;
                pbr_count
            } else {
                gpu.upload_mesh_colored(&accum.positions, &accum.colors, &accum.indices)
            };
            if count > 0 {
                compact_gpu = false;
                renderer = BodyRendererBackend::WebGpu;
                admitted_vertex_count = accum.positions.len();
                admitted_index_count = accum.indices.len();
                self.graphics.apply_environment_to_gpu(gpu);
            } else if !reduced.indices.is_empty()
                && gpu.upload_mesh_colored(&reduced.positions, &reduced.colors, &reduced.indices)
                    > 0
            {
                renderer = BodyRendererBackend::WebGpu;
                admitted_vertex_count = reduced.positions.len();
                admitted_index_count = reduced.indices.len();
            }
        }
        if renderer == BodyRendererBackend::None {
            if let Some(ref mut webgl2) = self.anatomy_webgl2 {
                webgl2.set_near_plane(self.camera.near_plane);
                if webgl2
                    .upload_mesh(&accum.positions, &accum.colors, &accum.indices)
                    .is_ok()
                {
                    compact_gpu = false;
                    renderer = BodyRendererBackend::WebGl2;
                    admitted_vertex_count = accum.positions.len();
                    admitted_index_count = accum.indices.len();
                } else if webgl2
                    .upload_mesh(&reduced.positions, &reduced.colors, &reduced.indices)
                    .is_ok()
                {
                    renderer = BodyRendererBackend::WebGl2;
                    admitted_vertex_count = reduced.positions.len();
                    admitted_index_count = reduced.indices.len();
                }
            }
        }
        self.cpu_body = Some(reduced);
        self.scene_animation
            .install(std::mem::take(&mut accum.animation), compact_gpu);
        if renderer == BodyRendererBackend::None {
            renderer = BodyRendererBackend::CpuCanvas;
            self.tier = 0;
            admitted_vertex_count = self
                .cpu_body
                .as_ref()
                .map_or(0, |body| body.positions.len());
            admitted_index_count = self.cpu_body.as_ref().map_or(0, |body| body.indices.len());
            #[cfg(target_arch = "wasm32")]
            if self.gpu.is_some() || self.anatomy_webgl2.is_some() {
                // A context-bound GPU canvas cannot become Canvas2D in place.
                // Ask the host to replace it and walk the VibeScript fallback
                // policy with the retained reduced mesh.
                self.graphics_recovery_requested = true;
            }
        } else if renderer == BodyRendererBackend::WebGpu {
            self.tier = 2;
        } else {
            self.tier = 1;
        }

        self.body_renderer = renderer;
        self.body_vertex_count = admitted_vertex_count.min(u32::MAX as usize) as u32;
        self.body_index_count = admitted_index_count.min(u32::MAX as usize) as u32;
        self.body_frames_presented = 0;

        // Semantic nodes embedded in the organs' `Tensor10DNodes` sections become
        // the pick/projection substrate — already normalised into the same orbit
        // frame as the uploaded mesh, so hits land on drawn geometry.
        let semantic_node_count = accum.nodes.len();
        if !accum.nodes.is_empty() {
            let need = TENSOR_HEADER_BYTES
                + accum.nodes.len() * crate::container_10d::node_section::TENSOR10D_SIZE;
            let mut buf = vec![0u8; need];
            if write_tensor_buffer(&accum.nodes, &mut buf).is_ok() {
                let _ = self.upload_tensor_buffer(&buf);
            }
        }

        self.description = format!(
            "{} assets · {} triangles · {} materials · {} refused · {}{}",
            accum.assets_loaded,
            accum.total_triangles,
            accum.materials.len(),
            accum.assets_refused,
            renderer.as_str(),
            if material_pipeline {
                " · PBR scene"
            } else {
                " · colour fallback"
            }
        );
        let result = js_sys::Object::new();
        Reflect::set(
            &result,
            &JsValue::from_str("assets_loaded"),
            &JsValue::from_f64(accum.assets_loaded as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("organs_loaded"),
            &JsValue::from_f64(accum.organs_loaded() as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("assets_refused"),
            &JsValue::from_f64(accum.assets_refused as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("organs_refused"),
            &JsValue::from_f64(accum.organs_refused() as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("total_triangles"),
            &JsValue::from_f64(accum.total_triangles as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("vertex_count"),
            &JsValue::from_f64(self.body_vertex_count as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("renderer"),
            &JsValue::from_str(renderer.as_str()),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("material_pipeline"),
            &JsValue::from_bool(material_pipeline),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("materials_loaded"),
            &JsValue::from_f64(accum.materials.len() as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("material_ranges"),
            &JsValue::from_f64(accum.material_ranges.len() as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("semantic_nodes"),
            &JsValue::from_f64(semantic_node_count as f64),
        )?;
        Reflect::set(&result, &JsValue::from_str("uploaded"), &JsValue::TRUE)?;
        Ok(result.into())
    }

    /// Cold-path Scene / Anatomy lifecycle receipt. Success requires a retained upload
    /// and at least one presented renderer frame.
    pub fn body_render_receipt(&self) -> Result<JsValue, JsValue> {
        let result = js_sys::Object::new();
        Reflect::set(
            &result,
            &JsValue::from_str("renderer"),
            &JsValue::from_str(self.body_renderer.as_str()),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("vertex_count"),
            &JsValue::from_f64(self.body_vertex_count as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("index_count"),
            &JsValue::from_f64(self.body_index_count as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("frames_presented"),
            &JsValue::from_f64(self.body_frames_presented as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("uploaded"),
            &JsValue::from_bool(
                self.body_renderer != BodyRendererBackend::None && self.body_vertex_count > 0,
            ),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("success"),
            &JsValue::from_bool(
                self.body_renderer != BodyRendererBackend::None
                    && self.body_vertex_count > 0
                    && self.body_index_count > 0
                    && self.body_frames_presented > 0,
            ),
        )?;
        Ok(result.into())
    }

    /// Generic scene alias for [`Self::body_render_receipt`] (QG-20).
    pub fn scene_render_receipt(&self) -> Result<JsValue, JsValue> {
        self.body_render_receipt()
    }

    /// Phase 2 — drive the loaded mesh artefact with a kinematic joint (visible physics). `kind` is
    /// `"prismatic"` (slide) or anything else = `"revolute"` (spin); `(ax,ay,az)` is the axis
    /// (normalised here; defaults to +Y if zero); `rate` is rad/s (revolute) or units/s (prismatic).
    pub fn animate_artefact(&mut self, kind: &str, ax: f32, ay: f32, az: f32, rate: f32) {
        use crate::render::physics::{Joint, JointKind};
        let len = (ax * ax + ay * ay + az * az).sqrt();
        let axis = if len > 1e-6 {
            [ax / len, ay / len, az / len]
        } else {
            [0.0, 1.0, 0.0]
        };
        let joint = if kind == "prismatic" {
            Joint {
                kind: JointKind::Prismatic { axis },
                rate,
            }
        } else {
            Joint {
                kind: JointKind::Revolute { axis },
                rate,
            }
        };
        if let Some(ref mut gpu) = self.gpu {
            gpu.set_artefact_joint(Some(joint));
            gpu.set_artefact_world(None); // a free spin/slide — no admission clamp
        }
    }

    /// Phase 2 — visible **deterministic refusal**: slide the artefact along +X (prismatic joint)
    /// into a world bound; the admission gate refuses poses that would leave the bound, so the
    /// artefact deterministically halts at the wall instead of passing through.
    pub fn demo_artefact_refusal(&mut self) {
        use crate::render::physics::{Aabb, Joint, JointKind};
        let joint = Joint {
            kind: JointKind::Prismatic {
                axis: [1.0, 0.0, 0.0],
            },
            rate: 0.4,
        };
        let world = Aabb::new([-1.5, -3.0, -3.0], [1.5, 3.0, 3.0]);
        if let Some(ref mut gpu) = self.gpu {
            gpu.set_artefact_joint(Some(joint));
            gpu.set_artefact_world(Some(world));
        }
    }

    /// Phase 2 — whether the artefact's proposed motion is currently being refused (clamped).
    pub fn artefact_refused(&self) -> bool {
        self.gpu
            .as_ref()
            .map(|g| g.artefact_refused())
            .unwrap_or(false)
    }

    /// Phase 2 — freeze the artefact (joint → identity, no world clamp).
    pub fn stop_artefact_animation(&mut self) {
        if let Some(ref mut gpu) = self.gpu {
            gpu.set_artefact_joint(None);
            gpu.set_artefact_world(None);
        }
    }

    /// Phase 1.4 — the **2D view** of the resident manifold: each tensor node's `project(.., Plane2D)`
    /// shadow as a flat `[x0,y0,x1,y1,...]` array (world units, ~[-1,1]). The 3D scene draws the same
    /// nodes through the GPU projector (the `Volume3D` view); both are the *one* manifold projection
    /// seen two ways (see `manifold_project`). JS paints this on the 2D companion canvas.
    pub fn project_resident_plane2d(&self, time: f32) -> Vec<f32> {
        let bytes = match self.last_tensor.as_ref() {
            Some(b) => b,
            None => return Vec::new(),
        };
        let count = crate::tensor::buffer_export::tensor_node_count(bytes).unwrap_or(0);
        let mut out = Vec::with_capacity(count * 2);
        for i in 0..count {
            if let Ok(t) = crate::tensor::buffer_export::read_tensor_at(bytes, i) {
                let p = crate::render::projection::project(
                    &t,
                    time,
                    crate::render::projection::ProjectionTarget::Plane2D,
                );
                out.push(p[0]);
                out.push(p[1]);
            }
        }
        out
    }

    /// Upload a packed tensor buffer: stores it, feeds the resident
    /// substrate and VRAM ledger, forwards to the GPU (promoting the
    /// tier to 2 on success), and updates the description line.
    pub fn upload_tensor_buffer(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        let count = tensor_node_count(bytes).unwrap_or(0);
        self.last_tensor = Some(bytes.to_vec());
        self.description = format!("{count} tensor nodes resident");
        global_vram_ledger().record_tensor(bytes.len() as u64);

        if count > 0 {
            let _ = crate::tensor::resident_substrate::global_resident_substrate()
                .load_from_tensor_buffer(bytes, 0);
        }

        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            match gpu.upload_tensor_buffer(bytes) {
                Ok(gpu_count) if gpu_count > 0 => {
                    self.tier = 2;
                    self.description = format!("{gpu_count} tensor nodes · T2 phenomenal viewport");
                }
                _ => {}
            }
        }

        if self.tier >= 1 && count > 0 {
            self.tier = self.tier.max(1);
        }
        Ok(())
    }

    /// Spatially encode a JSON scene without uploading —
    /// [`spatial_encode_wasm`] passthrough.
    pub fn spatial_encode(&self, json: &str) -> Result<JsValue, JsValue> {
        spatial_encode_wasm(json)
    }

    /// Parse a `.q42` (CBOR-LD) payload, stash it as `last_parsed`,
    /// and return the parsed record.
    pub fn load_q42(&mut self, bytes: &[u8]) -> Result<JsValue, JsValue> {
        let parsed = parse_cbor_ld_wasm(bytes);
        self.description = format!("loaded .q42 ({} bytes)", bytes.len());
        self.last_parsed = Some(parsed.clone());
        Ok(parsed)
    }

    /// Parse a JSON scene payload, stash it as `last_parsed`, and
    /// return the parsed record.
    pub fn load_json_scene(&mut self, json: &str) -> Result<JsValue, JsValue> {
        let parsed = parse_json_wasm(json);
        self.description = "loaded JSON scene".to_string();
        self.last_parsed = Some(parsed.clone());
        Ok(parsed)
    }

    /// Sample browser ambient telemetry (perf/memory probes) as JS.
    pub fn sample_telemetry(&self) -> Result<JsValue, JsValue> {
        sample_browser_telemetry_wasm()
    }

    /// The most recently parsed scene payload, if any.
    pub fn last_parsed(&self) -> Option<JsValue> {
        self.last_parsed.clone()
    }

    /// Mount a minimal qapp document into the DOM element `root_id`:
    /// clears it, emits a header with the description line, and appends
    /// the parsed scene (or a "load a scene first" hint).
    pub fn mount_qapp(&self, root_id: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("no document"))?;
        let root = document
            .get_element_by_id(root_id)
            .ok_or_else(|| JsValue::from_str(&format!("element #{root_id} not found")))?;

        root.set_inner_html("");
        root.set_class_name("qapp-root");

        let chrome = document.create_element("header")?;
        chrome.set_class_name("qapp-chrome");
        chrome.set_inner_html(&format!(
            "<h1>Qualia QApp</h1><p class=\"qapp-meta\">{}</p>",
            html_escape(&self.description)
        ));
        root.append_child(&chrome)?;

        let panel = document.create_element("section")?;
        panel.set_class_name("qapp-panel");

        match &self.last_parsed {
            None => {
                panel.set_inner_html(
                    "<p class=\"qapp-hint\">Load a <code>.q42</code> or JSON scene payload first.</p>",
                );
            }
            Some(parsed) => {
                append_parsed_dom(&document, &panel, parsed)?;
            }
        }

        root.append_child(&panel)?;
        Ok(())
    }

    /// Frame count from a sidecar header, `1` when unparseable.
    #[inline]
    fn sidecar_frame_count(bytes: &[u8]) -> u32 {
        parse_sidecar_header(bytes)
            .map(|h| h.frame_count)
            .unwrap_or(1)
    }

    /// Pop and apply up to `max` pending ICP control commands;
    /// returns the count applied (failures are dropped, not fatal).
    fn drain_and_apply_control(&mut self, max: u32) -> u32 {
        let mut applied = 0u32;
        for _ in 0..max {
            let Some(cmd) = pop_control_command() else {
                break;
            };
            if self.apply_control_command(cmd).is_ok() {
                applied += 1;
            }
        }
        applied
    }

    /// Dispatch one ICP command by opcode: camera deltas, navigate/
    /// collapse by node index, standpoint scalar nudges, menu actions,
    /// sonic-token forwards; `invalid_icp_magic` / `unknown_icp_opcode`
    /// on malformed input.
    fn apply_control_command(&mut self, cmd: PortalControlCommand) -> Result<(), JsValue> {
        if !cmd.has_icp_magic() {
            return Err(JsValue::from_str("invalid_icp_magic"));
        }
        match cmd.opcode() {
            OP_SET_CAMERA_DELTA | OP_TILT_FRAME | OP_SWIPE_GESTURE => {
                let (dy, dp, dz) = cmd.decode_camera_delta();
                self.camera.yaw += dy;
                self.camera.pitch += dp;
                self.camera.zoom = (self.camera.zoom + dz).clamp(0.35, 48.0);
                self.camera = self.camera.clamped();
                #[cfg(target_arch = "wasm32")]
                if let Some(ref mut gpu) = self.gpu {
                    gpu.set_camera(self.camera.yaw, self.camera.pitch, self.camera.zoom);
                }
            }
            OP_NAVIGATE_INDEX => {
                let idx = cmd.tensor_or_menu_index() as u32;
                self.navigate_to_node(idx)?;
            }
            OP_COLLAPSE_Q => {
                let idx = cmd.tensor_or_menu_index() as u32;
                self.collapse_node_q(idx)?;
            }
            OP_SET_STANDPOINT_SCALAR => {
                let delta = cmd.param_a_i16() as f32 / 1000.0;
                let mut t_slice = self.standpoint.t_slice;
                let mut t_window = self.standpoint.t_window;
                let mut epistemic_q = self.standpoint.epistemic_q;
                match cmd.channel() {
                    STANDPOINT_SCALAR_T_SLICE => t_slice = (t_slice + delta).clamp(0.0, 1.0),
                    STANDPOINT_SCALAR_T_WINDOW => t_window = (t_window + delta).clamp(0.01, 1.0),
                    STANDPOINT_SCALAR_EPISTEMIC_Q => {
                        epistemic_q = (epistemic_q + delta).clamp(0.0, 1.0)
                    }
                    _ => {}
                }
                self.set_standpoint(
                    self.standpoint.standpoint_class,
                    epistemic_q,
                    t_slice,
                    t_window,
                    "",
                )?;
            }
            OP_MENU_ACTION => match cmd.tensor_or_menu_index() {
                MENU_ACTION_HOME => {
                    self.camera = CameraState::default();
                    #[cfg(target_arch = "wasm32")]
                    if let Some(ref mut gpu) = self.gpu {
                        gpu.set_camera(self.camera.yaw, self.camera.pitch, self.camera.zoom);
                    }
                    let _ = self.set_standpoint(
                        STANDPOINT_SPECTATOR,
                        1.0,
                        self.standpoint.t_slice,
                        self.standpoint.t_window,
                        "",
                    );
                }
                MENU_ACTION_SONIFY_TOGGLE => {
                    self.acoustic_enabled = !self.acoustic_enabled;
                }
                _ => {}
            },
            OP_SONIC_TOKEN_FORWARD => {
                let raw = cmd.embedded_sonic_raw();
                if raw != 0 {
                    push_sonic_token(SonicToken { raw });
                }
            }
            OP_BUTTON_ACTION => {
                let _ = self.apply_control_command(PortalControlCommand::menu_action(
                    cmd.tensor_or_menu_index(),
                ));
            }
            _ => return Err(JsValue::from_str("unknown_icp_opcode")),
        }
        Ok(())
    }

    /// Assemble the `AcousticUniform` for the AudioWorklet: tensor-node
    /// params + sidecar-enriched preview when a tensor is resident
    /// (advancing the sidecar frame), else a telemetry-derived default.
    fn build_acoustic_uniform(&mut self) -> AcousticUniform {
        let enabled = self.acoustic_enabled();
        let node = self.selected_node.unwrap_or(0);
        if let Some(ref tensor) = self.last_tensor {
            if let Ok(t) = read_tensor_at(tensor, node as usize) {
                let mut u = acoustic_params_from_tensor(&t).to_phenomenal_uniform(
                    enabled,
                    &t,
                    self.camera.yaw,
                );
                if let Some(ref sidecar) = self.acoustic_sidecar {
                    let frame = self.acoustic_sidecar_frame;
                    if enrich_preview_from_sidecar(sidecar, frame, &mut u.preview_bins) {
                        u.stft_frame = frame as f32;
                    }
                    self.acoustic_sidecar_frame =
                        (frame + 1) % Self::sidecar_frame_count(sidecar).max(1);
                }
                return u;
            }
        }
        let mut uniform = AcousticUniform::default();
        uniform.enabled = u32::from(enabled);
        uniform.alpha = self.telemetry.spectral_shift.max(0.05);
        uniform.epistemic_q = self.standpoint.epistemic_q;
        uniform.frequency_hz =
            crate::render::acoustic::sigma_to_center_frequency_hz(self.telemetry.spectral_shift);
        apply_binaural_to_uniform(&mut uniform, self.camera.yaw);
        uniform
    }

    /// Phenomenal U3 float uniform count (18 scalars + 64 preview bins).
    pub fn acoustic_uniform_float_count(&self) -> u32 {
        ACOUSTIC_UNIFORM_FLOAT_COUNT as u32
    }

    /// 250 ms sonification pulse: emits a sonic token for the selected
    /// tensor node when acoustics are enabled.
    fn tick_acoustic_plane(&mut self, dt_ms: f32) {
        if !self.acoustic_enabled() {
            return;
        }
        self.acoustic_pulse_accum += dt_ms;
        if self.acoustic_pulse_accum < 250.0 {
            return;
        }
        self.acoustic_pulse_accum = 0.0;
        let node = self.selected_node.unwrap_or(0);
        if let Some(ref tensor) = self.last_tensor {
            if let Ok(t) = read_tensor_at(tensor, node as usize) {
                sonify_tensor_node(node, &t, false);
            }
        }
    }

    #[allow(dead_code)]
    /// Adopt a `PortalGpu`/`AnatomyWebGl2` placed in the pending slots
    /// by the async init path (called once per frame until consumed).
    fn adopt_pending_gpu(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            if self.gpu.is_none() && !self.gpu_init_failed {
                if let Some(mut gpu) = PENDING_GPU.with(|p| p.borrow_mut().take()) {
                    gpu.set_camera_target(
                        self.camera.yaw,
                        self.camera.pitch,
                        self.camera.zoom,
                        self.camera.target[0],
                        self.camera.target[1],
                        self.camera.target[2],
                    );
                    gpu.set_lighting(
                        self.camera.sun_dir[0],
                        self.camera.sun_dir[1],
                        self.camera.sun_dir[2],
                        self.camera.sun_intensity,
                        self.camera.ambient_intensity,
                    );
                    gpu.set_atmosphere_preset(self.atmosphere_preset);
                    let _ = gpu.set_exposure_compensation(self.hdr_exposure_ev);
                    let _ =
                        gpu.set_white_balance(self.white_balance_ev[0], self.white_balance_ev[1]);
                    if let Some(preset) = self.sky_preset {
                        gpu.set_sky_preset(preset);
                    }
                    gpu.set_standpoint(self.standpoint);
                    if let Some(ref tensor) = self.last_tensor {
                        if gpu.upload_tensor_buffer(tensor).ok().unwrap_or(0) > 0 {
                            self.description = format!(
                                "{} tensor nodes · T2 phenomenal viewport",
                                gpu.tensor_node_count()
                            );
                        }
                    }
                    if let Some(ref body) = self.cpu_body {
                        if self.body_renderer != BodyRendererBackend::WebGpu {
                            let count = gpu.upload_mesh_colored(
                                &body.positions,
                                &body.colors,
                                &body.indices,
                            );
                            if count > 0 {
                                self.scene_animation.compact_gpu = true;
                                self.body_renderer = BodyRendererBackend::WebGpu;
                                self.body_vertex_count =
                                    body.positions.len().min(u32::MAX as usize) as u32;
                                self.body_index_count =
                                    body.indices.len().min(u32::MAX as usize) as u32;
                                self.description = format!(
                                    "graphics active · mesh detail · {} indices",
                                    body.indices.len()
                                );
                            }
                        }
                    }
                    self.graphics_recovery_pending = false;
                    self.tier = 2;
                    gpu.set_tensor_projection_enabled(self.tensor_projection_enabled);
                    self.graphics.apply_to_gpu(&mut gpu);
                    self.gpu = Some(gpu);
                    self.etalon_steward.mark_ready();
                }
            }
            if self.anatomy_webgl2.is_none() {
                if let Some(gl) = PENDING_WEBGL2.with(|p| p.borrow_mut().take()) {
                    self.anatomy_webgl2 = Some(gl);
                }
            }
        }
    }

    /// Paint one frame onto `canvas`: on wasm32 handles device-loss
    /// early-out, pending-GPU adoption, surface/depth size
    /// reconciliation, and backend present; falls back to Canvas2D
    /// painters when no GPU is bound.
    pub(crate) fn paint_frame(&mut self, canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
        let mode = global_vram_ledger().mode();
        self.graphics.record_temporal_execution(false);

        #[cfg(target_arch = "wasm32")]
        {
            if self.note_device_loss() || self.graphics_recovery_requested {
                self.etalon_steward.notify_device_lost();
                // JavaScript replaces the old canvas at the next frame boundary;
                // its WebGPU context cannot be converted into Canvas2D/WebGL2.
                return Ok(());
            }
            self.adopt_pending_gpu();

            enum Present {
                Lit,
                Absent,
                Failed,
            }
            let present = if let Some(ref mut gpu) = self.gpu {
                // The WebGPU swapchain texture tracks the canvas backing store, but the depth
                // texture is only re-created on `resize()`. If the canvas was resized after init
                // (layout settle, DPR, window resize) without a `resize()` call, color and depth
                // attachments diverge and every render pass fails validation → black viewport.
                // Reconcile here so the GPU path self-heals to whatever the canvas actually is.
                let cw = canvas.width();
                let ch = canvas.height();
                let resize_refused = cw > 0
                    && ch > 0
                    && gpu.surface_size() != (cw, ch)
                    && gpu.resize(cw, ch).is_err();
                if resize_refused {
                    self.gpu = None;
                    self.gpu_init_failed = true;
                    self.graphics_recovery_requested = true;
                    self.graphics_recovery_canvas_only = true;
                    self.tier = 0;
                    if self.body_renderer == BodyRendererBackend::WebGpu {
                        self.body_renderer = BodyRendererBackend::CpuCanvas;
                    }
                    self.etalon_steward.notify_device_lost();
                    return Ok(());
                }
                let temporal_enabled = self.graphics.auto_temporal_enabled;
                let frame_result = if temporal_enabled {
                    gpu.render_with_auto_temporal(self.time as f32, &self.telemetry)
                } else {
                    gpu.render(self.time as f32, &self.telemetry)
                        .map(|()| false)
                };
                match frame_result {
                    Ok(temporal_executed) => {
                        self.graphics.record_temporal_execution(temporal_executed);
                        if self.body_renderer == BodyRendererBackend::WebGpu
                            && self.body_index_count > 0
                        {
                            self.body_frames_presented =
                                self.body_frames_presented.saturating_add(1);
                        }
                        if self.pending_gpu_pick {
                            if let Some(idx) = gpu.poll_pick_readback() {
                                self.selected_node = Some(idx);
                                self.selected_semantic_id = None;
                            } else if let Some(identity) = gpu.poll_semantic_pick_readback() {
                                self.selected_semantic_id = Some(identity);
                                self.selected_node = None;
                            }
                            if !gpu.pick_readback_pending() {
                                self.pending_gpu_pick = false;
                            }
                        }
                        Present::Lit
                    }
                    Err(_) => Present::Failed,
                }
            } else {
                Present::Absent
            };
            match present {
                Present::Lit => {
                    self.present_misses = 0;
                    self.tier = 2;
                    if self.body_renderer == BodyRendererBackend::WebGpu
                        || self.body_index_count == 0
                    {
                        return Ok(());
                    }
                }
                Present::Failed => {
                    // No mesh yet, or a transient surface miss: keep the device.
                    // Proof is the failover after presents keep failing, and it
                    // is not stamped as the lit look.
                    if self.body_index_count == 0 || self.present_misses < 30 {
                        self.present_misses = self.present_misses.saturating_add(1);
                        return Ok(());
                    }
                    self.gpu = None;
                    self.gpu_init_failed = true;
                    self.tier = 0;
                    if self.body_renderer == BodyRendererBackend::WebGpu {
                        self.body_renderer = BodyRendererBackend::CpuCanvas;
                    }
                    self.etalon_steward.notify_device_lost();
                    if let Ok(fresh) = release_claimed_canvas(canvas) {
                        return self.paint_cpu_proof(&fresh, mode);
                    }
                }
                Present::Absent => {}
            }

            if self.anatomy_webgl2.is_none() {
                if let Some(webgl2) = PENDING_WEBGL2.with(|p| p.borrow_mut().take()) {
                    let mut webgl2 = webgl2;
                    if self.graphics_recovery_pending {
                        let mut replayed = false;
                        if let Some(ref body) = self.cpu_body {
                            if webgl2
                                .upload_mesh(&body.positions, &body.colors, &body.indices)
                                .is_ok()
                            {
                                replayed = true;
                                self.body_renderer = BodyRendererBackend::WebGl2;
                                self.body_vertex_count =
                                    body.positions.len().min(u32::MAX as usize) as u32;
                                self.body_index_count =
                                    body.indices.len().min(u32::MAX as usize) as u32;
                                self.description = format!(
                                    "graphics recovered · reduced mesh detail · {} indices",
                                    self.body_index_count
                                );
                            }
                        }
                        self.graphics_recovery_pending = false;
                        if !replayed {
                            self.graphics_recovery_canvas_only = true;
                            self.graphics_recovery_requested = true;
                        }
                    }
                    if let Some(preset) = self.sky_preset {
                        webgl2.set_sky_preset(preset);
                    } else {
                        webgl2.set_clear(self.sky_clear);
                        webgl2.set_atmosphere_preset(self.atmosphere_preset);
                    }
                    let _ = webgl2.set_exposure_compensation(self.hdr_exposure_ev);
                    let _ = webgl2
                        .set_white_balance(self.white_balance_ev[0], self.white_balance_ev[1]);
                    self.tier = 1;
                    self.anatomy_webgl2 = Some(webgl2);
                }
            }
            if let Some(ref mut webgl2) = self.anatomy_webgl2 {
                webgl2.render(
                    self.camera.yaw,
                    self.camera.pitch,
                    self.camera.zoom,
                    self.camera.target,
                    self.camera.sun_dir,
                    self.camera.sun_intensity,
                    self.camera.ambient_intensity,
                    canvas.width(),
                    canvas.height(),
                )?;
                if self.body_renderer == BodyRendererBackend::WebGl2 && self.body_index_count > 0 {
                    self.body_frames_presented = webgl2.frame_count();
                }
                return Ok(());
            }
        }

        // Authored world: the lit mesh or nothing. Flat proof triangles are a
        // different picture, and a phone must not wear them as the build.
        if self.preserve_authored_frame {
            return self.paint_quiet(canvas);
        }
        self.paint_cpu_proof(canvas, mode)
    }

    /// Dark frame. Used when the lit present has not arrived yet. Not a model
    /// and not a badge.
    fn paint_quiet(&self, canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
        let ctx = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("no 2d context"))?
            .dyn_into::<CanvasRenderingContext2d>()?;
        let w = canvas.width() as f64;
        let h = canvas.height() as f64;
        ctx.set_fill_style_str("#05070b");
        ctx.fill_rect(0.0, 0.0, w, h);
        Ok(())
    }

    /// Canvas proof tick. Same orbit camera as the lit path, no depth buffer.
    /// Solid fills are not occlusion and must not be read as the lit look.
    fn paint_cpu_proof(
        &mut self,
        canvas: &HtmlCanvasElement,
        mode: crate::gpu_context::OperationalMode,
    ) -> Result<(), JsValue> {
        let ctx = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("no 2d context"))?
            .dyn_into::<CanvasRenderingContext2d>()?;

        let w = canvas.width() as f64;
        let h = canvas.height() as f64;
        let resident = if self.tier >= 2 {
            50_000
        } else if self.tier >= 1 {
            12_000
        } else {
            400
        };
        let particle_cap = ambient_draw_instances(resident) as usize;

        paint_background(&ctx, w, h, self.telemetry.spectral_shift);
        // An authored world is the picture. The particle field, the tensor
        // wire, and the engine HUD are the lab reel — they stay for a portal
        // that has not loaded a mesh, and they stay off the game once the
        // page has asked to keep the authored frame.
        let world = self.cpu_body.is_some() && self.preserve_authored_frame;
        if !world {
            paint_ambient_field(&ctx, w, h, self.time, particle_cap, &self.telemetry);
        }
        if let Some(mesh) = &self.cpu_body {
            let aspect = if h > 1.0 { (w / h) as f32 } else { 1.0 };
            paint_cpu_body(&ctx, w, h, mesh, &self.camera, aspect);
            if self.body_renderer == BodyRendererBackend::CpuCanvas && self.body_index_count > 0 {
                self.body_frames_presented = self.body_frames_presented.saturating_add(1);
            }
        }

        if !world {
            if self.tensor_projection_enabled {
                if let Some(ref tensor) = self.last_tensor {
                    paint_tensor_projection(
                        &ctx,
                        w,
                        h,
                        tensor,
                        mode,
                        self.display_mode,
                        self.camera.yaw,
                        &self.standpoint,
                    );
                }
            }
            paint_hud(&ctx, self, mode);
        }
        Ok(())
    }
}

// A PortalGpu created asynchronously by `portal_init_webgpu`, handed to the next `paint_frame`.
#[cfg(target_arch = "wasm32")]
thread_local! {
    static PENDING_GPU: std::cell::RefCell<Option<PortalGpu>> = std::cell::RefCell::new(None);
    static PENDING_WEBGL2: std::cell::RefCell<Option<AnatomyWebGl2>> = std::cell::RefCell::new(None);
}

/// Create the WebGPU device + surface asynchronously and stash it for the render loop to adopt.
/// JS calls this **once, awaited**, right after constructing the portal and **before** the render
/// loop starts — the canvas must still be context-free (no 2d context yet) so the WebGPU surface
/// can bind to it. Returns `true` if the GPU path is now armed; on `false`/throw the portal keeps
/// the canvas2d fallback.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn portal_init_webgpu(canvas: HtmlCanvasElement) -> Result<bool, JsValue> {
    if !has_webgpu() {
        return Ok(false);
    }
    reset_portal_gpu_init_flags();
    let cap = particle_cap_for_mode(global_vram_ledger().mode(), 2);
    // One adapter request inside try_new_async. A second probe doubles the
    // wait on a phone and is what used to trip the short timeout.
    let outcome = race_init(async move {
        if portal_gpu_init_aborted() {
            return "aborted".to_string();
        }
        match PortalGpu::try_new_async(&canvas, cap).await {
            Ok(gpu) => {
                if portal_gpu_init_aborted() {
                    return "aborted".to_string();
                }
                PENDING_GPU.with(|p| *p.borrow_mut() = Some(gpu));
                "ok".to_string()
            }
            Err(e) if e == "aborted" => "aborted".to_string(),
            Err(e) => format!("err:{e}"),
        }
    })
    .await;
    match outcome.as_str() {
        "ok" => Ok(true),
        "timeout" | "no-adapter" | "aborted" => {
            // A late device must not be adopted after the page has given up.
            PENDING_GPU.with(|p| *p.borrow_mut() = None);
            Ok(false)
        }
        other => Err(JsValue::from_str(&format!("portal_init_webgpu: {other}"))),
    }
}

/// Page-side timeout. Stops a late surface from being adopted. If the canvas
/// was already claimed, [`portal_webgpu_canvas_claimed`] is true and the page
/// must replace that element before the proof tick.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn portal_abort_webgpu() {
    abort_portal_gpu_init();
    PENDING_GPU.with(|p| *p.borrow_mut() = None);
}

/// True once WebGPU has called `getContext("webgpu")` on the init canvas.
/// A 2d tick on that same element cannot paint; replace it.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn portal_webgpu_canvas_claimed() -> bool {
    portal_gpu_canvas_claimed()
}

/// Race GPU-init `work` against a 20 s timeout; resolves to the
/// backend name or `"timeout"` (which also aborts the pending init).
#[cfg(target_arch = "wasm32")]
async fn race_init<F>(work: F) -> String
where
    F: std::future::Future<Output = String> + 'static,
{
    let work_promise =
        wasm_bindgen_futures::future_to_promise(async move { Ok(JsValue::from_str(&work.await)) });
    let timeout = js_sys::Promise::new(&mut |resolve, _reject| {
        let resolve = resolve.clone();
        let closure = wasm_bindgen::closure::Closure::once(move || {
            abort_portal_gpu_init();
            let _ = resolve.call1(&JsValue::NULL, &JsValue::from_str("timeout"));
        });
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                20000,
            );
        }
        closure.forget();
    });
    let racers = js_sys::Array::of2(&work_promise, &timeout);
    match wasm_bindgen_futures::JsFuture::from(js_sys::Promise::race(&racers)).await {
        Ok(value) => value.as_string().unwrap_or_else(|| "timeout".into()),
        Err(_) => "timeout".into(),
    }
}

/// Bind a hardware WebGL2 Anatomy renderer before `QualiaPortal` construction.
/// This is selected only after capability probing proves that WebGPU has no
/// usable adapter and WebGL2 context creation succeeds.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn portal_init_webgl2(canvas: HtmlCanvasElement) -> Result<bool, JsValue> {
    let renderer = AnatomyWebGl2::try_new(&canvas)?;
    PENDING_WEBGL2.with(|p| *p.borrow_mut() = Some(renderer));
    Ok(true)
}

/// Replace `canvas` with a fresh element (same id/size/aria-label) —
/// the only way to release a claimed WebGPU context binding so a
/// Canvas2D or WebGL2 context can be created.
fn release_claimed_canvas(canvas: &HtmlCanvasElement) -> Result<HtmlCanvasElement, JsValue> {
    let doc = canvas
        .owner_document()
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let fresh: HtmlCanvasElement = doc
        .create_element("canvas")?
        .dyn_into()
        .map_err(|_| JsValue::from_str("canvas element"))?;
    fresh.set_id(&canvas.id());
    fresh.set_width(canvas.width());
    fresh.set_height(canvas.height());
    if let Some(label) = canvas.get_attribute("aria-label") {
        let _ = fresh.set_attribute("aria-label", &label);
    }
    canvas.replace_with_with_node_1(&fresh)?;
    Ok(fresh)
}

/// Initial device tier: 1 when `navigator.gpu` exists, else 0.
fn detect_tier() -> u8 {
    if has_webgpu() {
        1
    } else {
        0
    }
}

/// Whether `navigator.gpu` is present (WebGPU capability probe —
/// presence only, not adapter usability).
fn has_webgpu() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let navigator = window.navigator();
    js_sys::Reflect::get(&navigator, &JsValue::from_str("gpu"))
        .ok()
        .map(|v| !v.is_undefined() && !v.is_null())
        .unwrap_or(false)
}

// Phase 0.2a: canvas2d fallback painters.
mod paint;
use paint::*;

mod cpu_mesh;
use cpu_mesh::CpuBodyMesh;
