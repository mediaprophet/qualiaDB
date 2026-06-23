//! Qualia WASM — Semantic Subjectivity Bifurcation Portal (browser surface).

use js_sys::{Array, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, Document, Element, HtmlCanvasElement};

use crate::audio::acoustic_plane::{
    acoustic_enabled_for_mode, acoustic_params_from_tensor, apply_binaural_to_uniform,
    drain_sonic_tokens, push_sonic_token, sonify_tensor_node, AcousticUniform,
};
use crate::audio::acoustic_sab::{
    init_acoustic_sab, push_token_to_sab, write_uniform_to_sab_with_mirror, ACOUSTIC_SAB_BYTES,
};
use crate::audio::audio_spectral_sheet::preview_bins_from_tensor;
use crate::audio::audio_sidecar_link::enrich_preview_from_sidecar;
use crate::audio::audio_spectral_sheet::parse_sidecar_header;
use crate::audio::cqt_bake::bake_cqt_sidecar_from_preview;
use crate::audio::stft_bake::bake_tensor_stft_sidecar;
use crate::audio::audio_spectral_sheet::SPECTRAL_PREVIEW_BINS;
use crate::render::acoustic::ACOUSTIC_UNIFORM_FLOAT_COUNT;
use crate::render::control::{
    control_pending, pop_control_command, push_control_raw, PortalControlCommand,
    MENU_ACTION_HOME, MENU_ACTION_SONIFY_TOGGLE, OP_BUTTON_ACTION, OP_COLLAPSE_Q,
    OP_MENU_ACTION, OP_NAVIGATE_INDEX, OP_SET_CAMERA_DELTA, OP_SET_STANDPOINT_SCALAR,
    OP_SONIC_TOKEN_FORWARD, OP_SWIPE_GESTURE, OP_TILT_FRAME, STANDPOINT_SCALAR_EPISTEMIC_Q,
    STANDPOINT_SCALAR_T_SLICE, STANDPOINT_SCALAR_T_WINDOW,
};

use crate::gpu_context::{ambient_draw_instances, global_vram_ledger, OperationalMode};
use crate::sonic_token::SonicToken;
use crate::render::spectral::sigma_to_display_rgb;
use crate::render::camera::CameraState;
use crate::render::navigation::{camera_frame_node, cpu_pick_node_at, CameraFlyTo, Q_COLLAPSED_EPS};
use crate::render::standpoint::{resolve_standpoint_hash, spectator_default};
use crate::render::telemetry::{
    ObserverStandpoint, DEONTIC_LANE_COMMONS, FABRIC_SHARED, FABRIC_VIEWPORT_LOCAL,
    STANDPOINT_DID, STANDPOINT_EPHEMERAL, STANDPOINT_SPECTATOR, STANDPOINT_VAULT,
    SystemTelemetry,
};
use crate::tensor::buffer_export::{read_tensor_at, tensor_node_count, write_tensor_q_at};
use crate::{
    export_tensor_buffer_wasm, geosparql_operation_wasm, parse_cbor_ld_wasm, parse_json_wasm,
    sample_browser_telemetry_wasm, spatial_encode_wasm,
};

#[cfg(target_arch = "wasm32")]
use crate::render::gpu::{particle_cap_for_mode, PortalGpu};

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

/// Portal tier: 0 = CPU canvas2d fallback, 1 = tensor projection, 2 = WebGPU ambient.
#[wasm_bindgen]
pub struct QualiaPortal {
    description: String,
    last_parsed: Option<JsValue>,
    tier: u8,
    time: f64,
    last_tensor: Option<Vec<u8>>,
    telemetry: SystemTelemetry,
    display_mode: DisplayMode,
    camera: CameraState,
    camera_fly: CameraFlyTo,
    selected_node: Option<u32>,
    #[cfg(target_arch = "wasm32")]
    pending_gpu_pick: bool,
    session_nonce: u64,
    standpoint: ObserverStandpoint,
    #[cfg(target_arch = "wasm32")]
    gpu: Option<PortalGpu>,
    #[cfg(target_arch = "wasm32")]
    gpu_init_failed: bool,
    acoustic_enabled: bool,
    acoustic_pulse_accum: f32,
    /// Pinned mmap-ready STFT/CQT sidecar for selected node (cold bake → hot frame read).
    acoustic_sidecar: Option<Vec<u8>>,
    acoustic_sidecar_frame: u32,
}

#[wasm_bindgen]
impl QualiaPortal {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<QualiaPortal, JsValue> {
        let tier = detect_tier();
        let session_nonce = crate::render::standpoint::generate_session_nonce();
        let standpoint = spectator_default(session_nonce);
        let mut portal = QualiaPortal {
            description: "Qualia portal initialized".to_string(),
            last_parsed: None,
            tier,
            time: 0.0,
            last_tensor: None,
            telemetry: SystemTelemetry::from_samples(&crate::gpu_context::sample_ambient_telemetry()),
            display_mode: DisplayMode::Hybrid,
            camera: CameraState::default(),
            camera_fly: CameraFlyTo::default(),
            selected_node: None,
            #[cfg(target_arch = "wasm32")]
            pending_gpu_pick: false,
            session_nonce,
            standpoint,
            #[cfg(target_arch = "wasm32")]
            gpu: None,
            #[cfg(target_arch = "wasm32")]
            gpu_init_failed: false,
            acoustic_enabled: true,
            acoustic_pulse_accum: 0.0,
            acoustic_sidecar: None,
            acoustic_sidecar_frame: 0,
        };
        portal.paint_frame(&canvas)?;
        Ok(portal)
    }

    pub fn tier(&self) -> u8 {
        self.tier
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

    pub fn operational_mode(&self) -> u8 {
        global_vram_ledger().mode() as u8
    }

    /// Enable or mute U3 AcousticPlane (automatically off in Reserve mode).
    pub fn set_acoustic_enabled(&mut self, enabled: bool) {
        self.acoustic_enabled = enabled;
    }

    pub fn acoustic_enabled(&self) -> bool {
        self.acoustic_enabled
            && acoustic_enabled_for_mode(global_vram_ledger().mode())
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
        Ok(js_sys::Float32Array::from(&acoustic_uniform_to_floats(&u)[..]))
    }

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
        let need = std::mem::size_of::<crate::audio::audio_spectral_sheet::AudioSpectralSidecarHeader>()
            + SPECTRAL_PREVIEW_BINS * frame_count as usize * 4;
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

    pub fn resize(&mut self, canvas: HtmlCanvasElement, width: u32, height: u32) -> Result<(), JsValue> {
        canvas.set_width(width);
        canvas.set_height(height);
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            gpu.resize(width, height);
        }
        self.paint_frame(&canvas)
    }

    pub fn tick(&mut self, canvas: HtmlCanvasElement, dt_ms: f32) -> Result<(), JsValue> {
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
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            if gpu.has_tensor_buffer() {
                gpu.queue_pick(x, y);
                self.pending_gpu_pick = true;
                return Ok(());
            }
        }
        if let Some(ref tensor) = self.last_tensor {
            if let Some(idx) = cpu_pick_node_at(
                tensor,
                canvas_w.max(1) as f64,
                canvas_h.max(1) as f64,
                x as f64,
                y as f64,
                self.camera.yaw,
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

    pub fn selected_node_index(&self) -> i32 {
        self.selected_node.map(|n| n as i32).unwrap_or(-1)
    }

    /// Frame the camera on a tensor node (`Maps_to_node`).
    pub fn navigate_to_node(&mut self, index: u32) -> Result<(), JsValue> {
        let tensor = self
            .last_tensor
            .as_ref()
            .ok_or_else(|| JsValue::from_str("no tensor buffer"))?;
        let t = read_tensor_at(tensor, index as usize)
            .map_err(|e| JsValue::from_str(e))?;
        self.selected_node = Some(index);
        let target = camera_frame_node([t.x, t.y, t.z]);
        self.camera_fly = CameraFlyTo::start_toward(target);
        Ok(())
    }

    /// Wavefunction collapse — set node `q` to 0 in the resident session manifold.
    pub fn collapse_node_q(&mut self, index: u32) -> Result<(), JsValue> {
        let Some(ref mut tensor) = self.last_tensor else {
            return Err(JsValue::from_str("no tensor buffer"));
        };
        let prev = write_tensor_q_at(tensor, index as usize, 0.0)
            .map_err(|e| JsValue::from_str(e))?;
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

    pub fn set_telemetry(&mut self, floats: &[f32]) -> Result<(), JsValue> {
        self.telemetry.apply_floats(floats);
        Ok(())
    }

    pub fn set_display_mode(&mut self, mode: &str) -> Result<(), JsValue> {
        self.display_mode = DisplayMode::from_str(mode);
        Ok(())
    }

    /// Orbit camera IPC from the UI shell (yaw/pitch in radians, zoom = eye distance).
    pub fn set_camera(&mut self, yaw: f32, pitch: f32, zoom: f32) -> Result<(), JsValue> {
        self.camera = CameraState { yaw, pitch, zoom }.clamped();
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            gpu.set_camera(self.camera.yaw, self.camera.pitch, self.camera.zoom);
        }
        Ok(())
    }

    pub fn camera_yaw(&self) -> f32 {
        self.camera.yaw
    }

    pub fn camera_pitch(&self) -> f32 {
        self.camera.pitch
    }

    pub fn camera_zoom(&self) -> f32 {
        self.camera.zoom
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

    pub fn standpoint_class(&self) -> u32 {
        self.standpoint.standpoint_class
    }

    pub fn epistemic_q(&self) -> f32 {
        self.standpoint.epistemic_q
    }

    pub fn t_slice(&self) -> f32 {
        self.standpoint.t_slice
    }

    pub fn t_window(&self) -> f32 {
        self.standpoint.t_window
    }

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
        let hint_opt = if hint.is_empty() { None } else { Some(hint) };
        let mesh = crate::render::assets::import_asset(bytes, hint_opt)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
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
            Some(ref mut gpu) => gpu.upload_mesh(&positions, &indices),
            None => 0,
        };
        self.description = format!("{tris} mesh triangles · T2 surface");
        Ok(tris)
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
            Joint { kind: JointKind::Prismatic { axis }, rate }
        } else {
            Joint { kind: JointKind::Revolute { axis }, rate }
        };
        if let Some(ref mut gpu) = self.gpu {
            gpu.set_artefact_joint(Some(joint));
        }
    }

    /// Phase 2 — freeze the artefact (joint → identity).
    pub fn stop_artefact_animation(&mut self) {
        if let Some(ref mut gpu) = self.gpu {
            gpu.set_artefact_joint(None);
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
                    self.description =
                        format!("{gpu_count} tensor nodes · T2 phenomenal viewport");
                }
                _ => {}
            }
        }

        if self.tier >= 1 && count > 0 {
            self.tier = self.tier.max(1);
        }
        Ok(())
    }

    pub fn spatial_encode(&self, json: &str) -> Result<JsValue, JsValue> {
        spatial_encode_wasm(json)
    }

    pub fn load_q42(&mut self, bytes: &[u8]) -> Result<JsValue, JsValue> {
        let parsed = parse_cbor_ld_wasm(bytes);
        self.description = format!("loaded .q42 ({} bytes)", bytes.len());
        self.last_parsed = Some(parsed.clone());
        Ok(parsed)
    }

    pub fn load_json_scene(&mut self, json: &str) -> Result<JsValue, JsValue> {
        let parsed = parse_json_wasm(json);
        self.description = "loaded JSON scene".to_string();
        self.last_parsed = Some(parsed.clone());
        Ok(parsed)
    }

    pub fn sample_telemetry(&self) -> Result<JsValue, JsValue> {
        sample_browser_telemetry_wasm()
    }

    pub fn last_parsed(&self) -> Option<JsValue> {
        self.last_parsed.clone()
    }

    pub fn mount_qapp(&self, root_id: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        let document = window.document().ok_or_else(|| JsValue::from_str("no document"))?;
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

    #[inline]
    fn sidecar_frame_count(bytes: &[u8]) -> u32 {
        parse_sidecar_header(bytes)
            .map(|h| h.frame_count)
            .unwrap_or(1)
    }

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
                    STANDPOINT_SCALAR_T_WINDOW => {
                        t_window = (t_window + delta).clamp(0.01, 1.0)
                    }
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

    fn build_acoustic_uniform(&mut self) -> AcousticUniform {
        let enabled = self.acoustic_enabled();
        let node = self.selected_node.unwrap_or(0);
        if let Some(ref tensor) = self.last_tensor {
            if let Ok(t) = read_tensor_at(tensor, node as usize) {
                let mut u = acoustic_params_from_tensor(&t)
                    .to_phenomenal_uniform(enabled, &t, self.camera.yaw);
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

    pub(crate) fn paint_frame(&mut self, canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
        let mode = global_vram_ledger().mode();

        #[cfg(target_arch = "wasm32")]
        {
            // Async WebGPU init (`portal_init_webgpu`, awaited by JS before the render loop starts)
            // stashes a ready PortalGpu in PENDING_GPU; adopt it on the first frame. The device is
            // created asynchronously off the loop because the browser main thread cannot `block_on`.
            if self.gpu.is_none() && !self.gpu_init_failed {
                if let Some(mut gpu) = PENDING_GPU.with(|p| p.borrow_mut().take()) {
                    gpu.set_camera(self.camera.yaw, self.camera.pitch, self.camera.zoom);
                    gpu.set_standpoint(self.standpoint);
                    if let Some(ref tensor) = self.last_tensor {
                        if gpu.upload_tensor_buffer(tensor).ok().unwrap_or(0) > 0 {
                            self.description = format!(
                                "{} tensor nodes · T2 phenomenal viewport",
                                gpu.tensor_node_count()
                            );
                        }
                    }
                    self.tier = 2;
                    self.gpu = Some(gpu);
                }
            }

            if let Some(ref mut gpu) = self.gpu {
                // The WebGPU swapchain texture tracks the canvas backing store, but the depth
                // texture is only re-created on `resize()`. If the canvas was resized after init
                // (layout settle, DPR, window resize) without a `resize()` call, color and depth
                // attachments diverge and every render pass fails validation → black viewport.
                // Reconcile here so the GPU path self-heals to whatever the canvas actually is.
                let cw = canvas.width();
                let ch = canvas.height();
                if cw > 0 && ch > 0 && gpu.surface_size() != (cw, ch) {
                    gpu.resize(cw, ch);
                }
                gpu.sync_bloom_targets();
                if gpu
                    .render(self.time as f32, &self.telemetry)
                    .is_ok()
                {
                    if self.pending_gpu_pick {
                        if let Some(idx) = gpu.poll_pick_readback() {
                            self.selected_node = Some(idx);
                            self.pending_gpu_pick = false;
                        }
                    }
                    return Ok(());
                }
            }
        }

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
        paint_ambient_field(&ctx, w, h, self.time, particle_cap, &self.telemetry);

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

        paint_hud(&ctx, self, mode);
        Ok(())
    }
}

/// A PortalGpu created asynchronously by `portal_init_webgpu`, handed to the next `paint_frame`.
#[cfg(target_arch = "wasm32")]
thread_local! {
    static PENDING_GPU: std::cell::RefCell<Option<PortalGpu>> = std::cell::RefCell::new(None);
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
    let cap = particle_cap_for_mode(global_vram_ledger().mode(), 2);
    match PortalGpu::try_new_async(&canvas, cap).await {
        Ok(gpu) => {
            PENDING_GPU.with(|p| *p.borrow_mut() = Some(gpu));
            Ok(true)
        }
        Err(e) => Err(JsValue::from_str(&format!("portal_init_webgpu: {e}"))),
    }
}

fn detect_tier() -> u8 {
    if has_webgpu() { 1 } else { 0 }
}

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
