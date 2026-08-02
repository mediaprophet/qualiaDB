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
use crate::render::camera::CameraState;
use crate::render::navigation::{
    camera_frame_node, cpu_pick_node_at, CameraFlyTo, Q_COLLAPSED_EPS,
};
use crate::render::spectral::sigma_to_display_rgb;
use crate::render::standpoint::{resolve_standpoint_hash, spectator_default};
use crate::render::telemetry::{
    ObserverStandpoint, SystemTelemetry, DEONTIC_LANE_COMMONS, FABRIC_SHARED,
    FABRIC_VIEWPORT_LOCAL, STANDPOINT_DID, STANDPOINT_EPHEMERAL, STANDPOINT_SPECTATOR,
    STANDPOINT_VAULT,
};
use crate::sonic_token::SonicToken;
use crate::tensor::buffer_export::{read_tensor_at, tensor_node_count, write_tensor_q_at};
use crate::{
    export_tensor_buffer_wasm, parse_cbor_ld_wasm, parse_json_wasm, sample_browser_telemetry_wasm,
    spatial_encode_wasm,
};

#[cfg(target_arch = "wasm32")]
use crate::render::anatomy::webgl2::AnatomyWebGl2;
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

/// Accumulates decoded organ meshes for the whole-body anatomy path.
/// Keeps decoding entirely in Rust so phone browsers never hold N organ copies in JS.
struct BodyMeshAccum {
    positions: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
    gmin: [f32; 3],
    gmax: [f32; 3],
    organs_loaded: u32,
    organs_refused: u32,
    total_triangles: u32,
}

impl BodyMeshAccum {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            colors: Vec::new(),
            indices: Vec::new(),
            gmin: [f32::INFINITY; 3],
            gmax: [f32::NEG_INFINITY; 3],
            organs_loaded: 0,
            organs_refused: 0,
            total_triangles: 0,
        }
    }

    /// Decode one sealed `.10d` organ. One owned buffer for CRC verify only —
    /// no JS heap intermediate, no second clone after `to_vec`.
    fn append_organ_10d(&mut self, organ_bytes: &[u8], rgba: [f32; 4]) {
        use crate::container_10d::{
            self,
            header::{Container10dHeader, FLAG_DEFAULT_DISPOSITION_REFUSE},
        };

        let mut bytes = organ_bytes.to_vec();
        let header = match Container10dHeader::parse(&bytes) {
            Ok(h) => h,
            Err(_) => return,
        };
        if container_10d::verify_whole_file_crc32c(&mut bytes).is_err() {
            return;
        }
        let descs = match container_10d::parse_section_table(&bytes, &header) {
            Ok(d) => d,
            Err(_) => return,
        };

        let mut mesh = None;
        let mut has_attestation = false;
        for desc in descs.iter() {
            let st = match container_10d::SectionType::from_u8(desc.section_type) {
                Some(st) => st,
                None => continue,
            };
            let off = desc.byte_offset as usize;
            let len = desc.byte_length as usize;
            if off.saturating_add(len) > bytes.len() {
                continue;
            }
            let payload = &bytes[off..off + len];
            match st {
                container_10d::SectionType::QuantizedMesh => {
                    if let Ok(m) = container_10d::decode_mesh_section(payload) {
                        mesh = Some(m);
                    }
                }
                container_10d::SectionType::ProvenanceSidecar => {
                    if let Ok(view) =
                        container_10d::provenance_section::decode_provenance_section(payload)
                    {
                        if container_10d::provenance_section::validate_provenance(&view).is_ok() {
                            has_attestation = true;
                        }
                    }
                }
                _ => {}
            }
        }

        let Some(mesh) = mesh else {
            return;
        };
        let governance_refused =
            (header.flags & FLAG_DEFAULT_DISPOSITION_REFUSE) != 0 && !has_attestation;
        if governance_refused {
            self.organs_refused += 1;
            return;
        }

        for k in 0..3 {
            if mesh.min[k] < self.gmin[k] {
                self.gmin[k] = mesh.min[k];
            }
            if mesh.max[k] > self.gmax[k] {
                self.gmax[k] = mesh.max[k];
            }
        }
        let base = self.positions.len() as u32;
        let [r, g, b, a] = rgba;
        for p in mesh.positions.iter() {
            self.positions.push([p[0], p[1], p[2]]);
            self.colors.push([r, g, b, a]);
        }
        for t in mesh.triangles.iter() {
            self.indices.push(base + t[0]);
            self.indices.push(base + t[1]);
            self.indices.push(base + t[2]);
        }
        self.total_triangles += mesh.triangles.len() as u32;
        self.organs_loaded += 1;
    }

    /// One global centre + scale so the body fits ~1.7 of the orbit frame.
    fn normalise_to_orbit_frame(&mut self) {
        if self.organs_loaded == 0 {
            return;
        }
        let gc = [
            (self.gmin[0] + self.gmax[0]) * 0.5,
            (self.gmin[1] + self.gmax[1]) * 0.5,
            (self.gmin[2] + self.gmax[2]) * 0.5,
        ];
        let gspan = (self.gmax[0] - self.gmin[0])
            .max(self.gmax[1] - self.gmin[1])
            .max(self.gmax[2] - self.gmin[2])
            .max(1e-6);
        let s = 1.7 / gspan;
        for p in self.positions.iter_mut() {
            p[0] = (p[0] - gc[0]) * s;
            p[1] = (p[1] - gc[1]) * s;
            p[2] = (p[2] - gc[2]) * s;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BodyRendererBackend {
    None,
    WebGpu,
    WebGl2,
}

impl BodyRendererBackend {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
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
    anatomy_webgl2: Option<AnatomyWebGl2>,
    #[cfg(target_arch = "wasm32")]
    gpu_init_failed: bool,
    body_renderer: BodyRendererBackend,
    body_vertex_count: u32,
    body_index_count: u32,
    body_frames_presented: u32,
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
            telemetry: SystemTelemetry::from_samples(
                &crate::gpu_context::sample_ambient_telemetry(),
            ),
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
            anatomy_webgl2: None,
            #[cfg(target_arch = "wasm32")]
            gpu_init_failed: false,
            body_renderer: BodyRendererBackend::None,
            body_vertex_count: 0,
            body_index_count: 0,
            body_frames_presented: 0,
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
        let t = read_tensor_at(tensor, index as usize).map_err(|e| JsValue::from_str(e))?;
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

    pub fn set_telemetry(&mut self, floats: &[f32]) -> Result<(), JsValue> {
        self.telemetry.apply_floats(floats);
        Ok(())
    }

    pub fn set_display_mode(&mut self, mode: &str) -> Result<(), JsValue> {
        self.display_mode = DisplayMode::from_str(mode);
        Ok(())
    }

    /// Enable/disable the **ambient particle field** — the mixer's "ambient" channel. Off by default
    /// (a plain mesh/anatomy view has no use for the decorative random cloud); a Tensor10D upload
    /// turns it on automatically because the particles then encode epistemic nodes.
    pub fn set_ambient_enabled(&mut self, on: bool) {
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            gpu.set_ambient_enabled(on);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = on;
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

    pub fn set_temporal_slice(&mut self, t_slice: f32, t_window: f32) {
        self.standpoint = self.standpoint.with_temporal(t_slice, t_window);
        #[cfg(target_arch = "wasm32")]
        if let Some(ref mut gpu) = self.gpu {
            gpu.set_standpoint(self.standpoint);
        }
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
            gpu.upload_mesh(&positions, &indices);
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

        if let Some(ref mut gpu) = self.gpu {
            let colors = vec![[r, g, b, a]; positions.len()];
            gpu.upload_mesh_colored(&positions, &colors, &indices);
            self.tier = 2;
        }

        if governance_refused {
            self.description = format!(
                "{tri_count} triangles · governance REFUSED (no attestation) · μ={provenance_mu:.3}"
            );
        } else {
            self.description = format!(
                "{tri_count} triangles · {vert_count} vertices · μ={provenance_mu:.3} · coloured · T2"
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
        for i in 0..organs.length() {
            let organ = organs.get(i);
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
        }
        self.finish_body_mesh_upload(accum)
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

    fn finish_body_mesh_upload(&mut self, mut accum: BodyMeshAccum) -> Result<JsValue, JsValue> {
        accum.normalise_to_orbit_frame();
        if accum.positions.is_empty() || accum.indices.is_empty() {
            return Err(JsValue::from_str("anatomy_body_mesh_empty"));
        }

        let mut renderer = BodyRendererBackend::None;
        if let Some(ref mut gpu) = self.gpu {
            gpu.upload_mesh_colored(&accum.positions, &accum.colors, &accum.indices);
            self.tier = 2;
            renderer = BodyRendererBackend::WebGpu;
        } else if let Some(ref mut webgl2) = self.anatomy_webgl2 {
            webgl2.upload_mesh(&accum.positions, &accum.colors, &accum.indices)?;
            self.tier = 1;
            renderer = BodyRendererBackend::WebGl2;
        }
        if renderer == BodyRendererBackend::None {
            return Err(JsValue::from_str("anatomy_renderer_unsupported"));
        }

        self.body_renderer = renderer;
        self.body_vertex_count = accum.positions.len().min(u32::MAX as usize) as u32;
        self.body_index_count = accum.indices.len().min(u32::MAX as usize) as u32;
        self.body_frames_presented = 0;
        self.description = format!(
            "{} organs · {} triangles · {} refused · coloured · {}",
            accum.organs_loaded,
            accum.total_triangles,
            accum.organs_refused,
            renderer.as_str()
        );
        let result = js_sys::Object::new();
        Reflect::set(
            &result,
            &JsValue::from_str("organs_loaded"),
            &JsValue::from_f64(accum.organs_loaded as f64),
        )?;
        Reflect::set(
            &result,
            &JsValue::from_str("organs_refused"),
            &JsValue::from_f64(accum.organs_refused as f64),
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
        Reflect::set(&result, &JsValue::from_str("uploaded"), &JsValue::TRUE)?;
        Ok(result.into())
    }

    /// Cold-path Anatomy lifecycle receipt. Success requires a retained upload
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
                match gpu.render(self.time as f32, &self.telemetry) {
                    Ok(()) => {
                        if self.body_renderer == BodyRendererBackend::WebGpu
                            && self.body_index_count > 0
                        {
                            self.body_frames_presented =
                                self.body_frames_presented.saturating_add(1);
                        }
                        if self.pending_gpu_pick {
                            if let Some(idx) = gpu.poll_pick_readback() {
                                self.selected_node = Some(idx);
                                self.pending_gpu_pick = false;
                            }
                        }
                        return Ok(());
                    }
                    Err(error) if self.body_renderer == BodyRendererBackend::WebGpu => {
                        return Err(JsValue::from_str(&format!(
                            "anatomy_webgpu_render_failed: {error}"
                        )));
                    }
                    Err(_) => {}
                }
            }

            if self.anatomy_webgl2.is_none() {
                if let Some(webgl2) = PENDING_WEBGL2.with(|p| p.borrow_mut().take()) {
                    self.tier = 1;
                    self.anatomy_webgl2 = Some(webgl2);
                }
            }
            if let Some(ref mut webgl2) = self.anatomy_webgl2 {
                webgl2.render(
                    self.camera.yaw,
                    self.camera.pitch,
                    self.camera.zoom,
                    canvas.width(),
                    canvas.height(),
                )?;
                if self.body_renderer == BodyRendererBackend::WebGl2 && self.body_index_count > 0 {
                    self.body_frames_presented = webgl2.frame_count();
                }
                return Ok(());
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
    let cap = particle_cap_for_mode(global_vram_ledger().mode(), 2);
    match PortalGpu::try_new_async(&canvas, cap).await {
        Ok(gpu) => {
            PENDING_GPU.with(|p| *p.borrow_mut() = Some(gpu));
            Ok(true)
        }
        Err(e) => Err(JsValue::from_str(&format!("portal_init_webgpu: {e}"))),
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

fn detect_tier() -> u8 {
    if has_webgpu() {
        1
    } else {
        0
    }
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
