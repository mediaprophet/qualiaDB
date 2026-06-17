//! Qualia WASM — Semantic Subjectivity Bifurcation Portal (browser surface).

use js_sys::{Array, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, Document, Element, HtmlCanvasElement};

use crate::gpu_context::{ambient_draw_instances, global_vram_ledger, OperationalMode};
use crate::portal_spectral::sigma_to_display_rgb;
use crate::portal_camera::CameraState;
use crate::portal_navigation::{camera_frame_node, cpu_pick_node_at, CameraFlyTo, Q_COLLAPSED_EPS};
use crate::portal_standpoint::{resolve_standpoint_hash, spectator_default};
use crate::portal_telemetry::{
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
use crate::portal_gpu::{particle_cap_for_mode, PortalGpu};

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
}

#[wasm_bindgen]
impl QualiaPortal {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<QualiaPortal, JsValue> {
        let tier = detect_tier();
        let session_nonce = crate::portal_standpoint::generate_session_nonce();
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
        };
        portal.paint_frame(&canvas)?;
        Ok(portal)
    }

    pub fn tier(&self) -> u8 {
        self.tier
    }

    pub fn operational_mode(&self) -> u8 {
        global_vram_ledger().mode() as u8
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
        self.time += dt_ms as f64 * 0.001;
        self.telemetry.refresh_from_ledger();
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

    pub(crate) fn paint_frame(&mut self, canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
        let mode = global_vram_ledger().mode();

        #[cfg(target_arch = "wasm32")]
        {
            if self.gpu.is_none() && !self.gpu_init_failed && has_webgpu() {
                let cap = particle_cap_for_mode(mode, 2);
                match PortalGpu::try_new(canvas, cap) {
                    Ok(mut gpu) => {
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
                    Err(_) => {
                        self.gpu_init_failed = true;
                    }
                }
            }

            if let Some(ref mut gpu) = self.gpu {
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

fn paint_background(ctx: &CanvasRenderingContext2d, w: f64, h: f64, spectral_shift: f32) {
    let (r, g, b) = sigma_to_display_rgb(spectral_shift);
    let gradient = ctx.create_linear_gradient(0.0, 0.0, w, h);
    let _ = gradient.add_color_stop(0.0, &format!("rgb({r},{g},{b})"));
    let _ = gradient.add_color_stop(1.0, "#080c12");
    ctx.set_fill_style(&JsValue::from(gradient));
    ctx.fill_rect(0.0, 0.0, w, h);
}

fn paint_ambient_field(
    ctx: &CanvasRenderingContext2d,
    w: f64,
    h: f64,
    time: f64,
    n: usize,
    telemetry: &SystemTelemetry,
) {
    let heat = telemetry.llm_heat as f64;
    let ripple = telemetry.network_ripple as f64;
    let logic = telemetry.logic_flashes as f64;
    let quantum = telemetry.quantum_activity as f64;

    for i in 0..n {
        let fi = i as f64;
        let px = w * 0.5
            + w * 0.38
                * (time * (0.35 + heat * 0.4) + fi * 0.01 + ripple * 2.0).sin()
                * (fi * 0.003 + quantum * 0.1).cos();
        let py = h * 0.5
            + h * 0.38
                * (time * (0.28 + logic * 0.5) + fi * 0.02).cos()
                * (fi * 0.005 + ripple).sin();
        let sigma = ((fi * 0.017 + telemetry.spectral_shift as f64) % 1.0) as f32;
        let (r, g, b) = sigma_to_display_rgb(sigma);
        let alpha = 0.08 + (fi * 0.001 + heat).sin().abs() * 0.35;
        ctx.set_fill_style(&JsValue::from_str(&format!(
            "rgba({r},{g},{b},{alpha:.2})"
        )));
        ctx.begin_path();
        let _ = ctx.arc(px, py, 0.8 + (fi % 3.0) + heat * 2.0, 0.0, std::f64::consts::TAU);
        ctx.fill();
    }
}

fn paint_tensor_projection(
    ctx: &CanvasRenderingContext2d,
    w: f64,
    h: f64,
    tensor: &[u8],
    mode: OperationalMode,
    display: DisplayMode,
    yaw: f32,
    standpoint: &ObserverStandpoint,
) {
    let count = match tensor_node_count(tensor) {
        Ok(n) => n,
        Err(_) => return,
    };
    if count == 0 {
        return;
    }

    let cap = mode.max_particles().max(800) as usize;
    let step = (count / cap).max(1);
    let mut nodes: Vec<ProjectedNode> = Vec::with_capacity(cap.min(count));

    for i in (0..count).step_by(step) {
        let Ok(t) = read_tensor_at(tensor, i) else {
            continue;
        };
        if !standpoint.temporal_visible(t.t) {
            continue;
        }
        let (px, py, depth) = project_xyz(t.x, t.y, t.z, w, h, yaw as f64);
        let (r, g, b) = sigma_to_display_rgb(t.sigma);
        let alpha = (0.35 + t.alpha * 0.55) * (0.55 + depth * 0.45);
        let radius = match display {
            DisplayMode::Solid => 2.8 + t.alpha as f64 * 3.5,
            DisplayMode::Points => 1.2 + t.alpha as f64 * 1.8,
            _ => 1.4 + t.alpha as f64 * 2.2,
        };
        nodes.push(ProjectedNode {
            px,
            py,
            r,
            g,
            b,
            alpha,
            radius,
            epistemic_ring: t.q > 0.0,
        });
    }

    let draw_wire = matches!(display, DisplayMode::Wireframe | DisplayMode::Hybrid);
    let draw_fill = matches!(
        display,
        DisplayMode::Points | DisplayMode::Solid | DisplayMode::Hybrid
    );

    if draw_wire && nodes.len() > 1 {
        for pair in nodes.windows(2) {
            stroke_segment(ctx, pair[0], pair[1], 0.35);
        }
        let last = nodes[nodes.len() - 1];
        stroke_segment(ctx, last, nodes[0], 0.25);
    }

    if draw_fill {
        for node in &nodes {
            let fill_alpha = match display {
                DisplayMode::Solid => node.alpha * 0.85,
                _ => node.alpha,
            };
            ctx.set_fill_style(&JsValue::from_str(&format!(
                "rgba({},{},{},{fill_alpha:.2})",
                node.r, node.g, node.b
            )));
            ctx.begin_path();
            let _ = ctx.arc(node.px, node.py, node.radius, 0.0, std::f64::consts::TAU);
            ctx.fill();

            if node.epistemic_ring {
                ctx.set_stroke_style(&JsValue::from_str(&format!(
                    "rgba({},{},{},0.55)",
                    node.r, node.g, node.b
                )));
                ctx.begin_path();
                let _ = ctx.arc(node.px, node.py, node.radius + 2.5, 0.0, std::f64::consts::TAU);
                ctx.stroke();
            }
        }
    }
}

fn stroke_segment(ctx: &CanvasRenderingContext2d, a: ProjectedNode, b: ProjectedNode, alpha: f32) {
    ctx.set_stroke_style(&JsValue::from_str(&format!(
        "rgba({},{},{},{alpha:.2})",
        a.r, a.g, a.b
    )));
    ctx.begin_path();
    ctx.move_to(a.px, a.py);
    ctx.line_to(b.px, b.py);
    ctx.stroke();
}

fn project_xyz(x: f32, y: f32, z: f32, w: f64, h: f64, yaw: f64) -> (f64, f64, f32) {
    let cx = yaw.cos() as f32;
    let sx = yaw.sin() as f32;
    let xr = x * cx + z * sx;
    let zr = -x * sx + z * cx;
    let depth = (1.0 / (1.0 + zr * 0.35)).clamp(0.2, 1.0);
    let scale = 0.42 * w.min(h) * depth as f64;
    let px = w * 0.5 + xr as f64 * scale;
    let py = h * 0.5 - y as f64 * scale;
    (px, py, depth)
}

fn paint_hud(ctx: &CanvasRenderingContext2d, portal: &QualiaPortal, mode: OperationalMode) {
    ctx.set_fill_style(&JsValue::from_str("#67e8f9"));
    ctx.set_font("14px Inter, system-ui, sans-serif");
    let tier_label = match portal.tier {
        2 => "T2 · Phenomenal",
        1 => "T1 · Tensor",
        _ => "T0 · CPU fallback",
    };
    let mode_label = match mode {
        OperationalMode::Full => "Full",
        OperationalMode::Eco => "Eco",
        OperationalMode::Reserve => "Reserve",
    };
    let _ = ctx.fill_text(
        &format!("Qualia WASM · {tier_label} · {mode_label} · {}", portal.description),
        16.0,
        28.0,
    );

    if let Some(ref tensor) = portal.last_tensor {
        let count = tensor_node_count(tensor).unwrap_or(0);
        let _ = ctx.fill_text(
            &format!("10D tensor buffer: {count} nodes · σ spectral projection"),
            16.0,
            48.0,
        );
    }
}

fn append_parsed_dom(document: &Document, panel: &Element, parsed: &JsValue) -> Result<(), JsValue> {
    if parsed.is_array() {
        let arr: Array = parsed.clone().dyn_into()?;
        for entry in arr.iter() {
            append_triple_dom(document, panel, &entry)?;
        }
        return Ok(());
    }
    append_triple_dom(document, panel, parsed)
}

fn append_triple_dom(document: &Document, panel: &Element, triple: &JsValue) -> Result<(), JsValue> {
    let subject = field_as_string(triple, "subject").or_else(|| field_as_string(triple, "s"));
    let predicate = field_as_string(triple, "predicate").or_else(|| field_as_string(triple, "p"));
    let object = field_as_string(triple, "object").or_else(|| field_as_string(triple, "o"));

    let (Some(predicate), Some(object)) = (predicate, object) else {
        return Ok(());
    };

    let tag = ontology_tag(&predicate);
    let el = document.create_element(tag)?;
    el.set_class_name("qapp-semantic");
    if let Some(subject) = subject {
        let _ = el.set_attribute("data-subject", &subject);
    }
    let _ = el.set_attribute("data-predicate", &predicate);
    el.set_text_content(Some(&object));
    panel.append_child(&el)?;
    Ok(())
}

fn field_as_string(value: &JsValue, key: &str) -> Option<String> {
    Reflect::get(value, &JsValue::from_str(key))
        .ok()
        .filter(|v| !v.is_null() && !v.is_undefined())
        .and_then(|v| v.as_string())
}

fn ontology_tag(predicate: &str) -> &'static str {
    let p = predicate.to_ascii_lowercase();
    if p.contains("title") || p.contains("label") || p.contains("name") {
        "h2"
    } else if p.contains("header") {
        "h1"
    } else if p.contains("description") || p.contains("summary") {
        "p"
    } else if p.contains("button") || p.contains("action") {
        "button"
    } else if p.contains("list") || p.contains("assertion") {
        "li"
    } else {
        "div"
    }
}

fn html_escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}