//! Qualia WASM — Semantic Subjectivity Bifurcation Portal (browser surface).

use js_sys::{Array, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, Document, Element, HtmlCanvasElement};

use qualia_core_db::gpu_context::{global_vram_ledger, OperationalMode};
use qualia_core_db::tensor::buffer_export::{read_tensor_at, tensor_node_count};
use qualia_core_db::{
    export_tensor_buffer_wasm, geosparql_operation_wasm, parse_cbor_ld_wasm, parse_json_wasm,
    parse_n3logic_wasm, parse_turtle_wasm, sample_browser_telemetry_wasm, spatial_encode_wasm,
};

const TELEMETRY_DIM: usize = 11;

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

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Portal tier: 0 = display fallback, 1 = WebGPU-capable canvas2d+tensor, 2 = phenomenal wgpu.
#[wasm_bindgen]
pub struct QualiaPortal {
    description: String,
    last_parsed: Option<JsValue>,
    tier: u8,
    time: f64,
    last_tensor: Option<Vec<u8>>,
    telemetry: [f32; TELEMETRY_DIM],
    display_mode: DisplayMode,
}

#[wasm_bindgen]
impl QualiaPortal {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<QualiaPortal, JsValue> {
        let tier = detect_tier();
        let portal = QualiaPortal {
            description: "Qualia portal initialized".to_string(),
            last_parsed: None,
            tier,
            time: 0.0,
            last_tensor: None,
            telemetry: default_telemetry(),
            display_mode: DisplayMode::Hybrid,
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

    pub fn resize(&self, canvas: HtmlCanvasElement, width: u32, height: u32) -> Result<(), JsValue> {
        canvas.set_width(width);
        canvas.set_height(height);
        self.paint_frame(&canvas)
    }

    pub fn tick(&mut self, canvas: HtmlCanvasElement, dt_ms: f32) -> Result<(), JsValue> {
        self.time += dt_ms as f64 * 0.001;
        self.refresh_telemetry_from_ledger();
        self.paint_frame(&canvas)
    }

    pub fn set_telemetry(&mut self, floats: &[f32]) -> Result<(), JsValue> {
        for (i, slot) in self.telemetry.iter_mut().enumerate() {
            *slot = floats.get(i).copied().unwrap_or(0.0);
        }
        Ok(())
    }

    pub fn set_display_mode(&mut self, mode: &str) -> Result<(), JsValue> {
        self.display_mode = DisplayMode::from_str(mode);
        Ok(())
    }

    /// Encode geometry JSON → Quins + tensor SOA buffer in one call.
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
        if self.tier >= 1 && count > 0 {
            self.tier = 1;
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

    /// Lightweight generative DOM: map parsed triples → HTML elements (B2 person-controlled loader).
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

    fn refresh_telemetry_from_ledger(&mut self) {
        let ledger = global_vram_ledger();
        self.telemetry[0] = ledger.pressure();
        self.telemetry[9] = ledger.pressure() * 0.5;
        self.telemetry[10] = ledger.mode() as u32 as f32;
    }

    pub(crate) fn paint_frame(&self, canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
        let ctx = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("no 2d context"))?
            .dyn_into::<CanvasRenderingContext2d>()?;

        let w = canvas.width() as f64;
        let h = canvas.height() as f64;
        let mode = global_vram_ledger().mode();
        let particle_cap = mode.max_particles().min(if self.tier >= 1 { 12_000 } else { 400 }) as usize;

        paint_background(&ctx, w, h, self.telemetry[6]);
        paint_ambient_field(&ctx, w, h, self.time, particle_cap, &self.telemetry);

        if let Some(ref tensor) = self.last_tensor {
            paint_tensor_projection(&ctx, w, h, self.time, tensor, mode, self.display_mode);
        }

        paint_hud(&ctx, self, mode);
        Ok(())
    }
}

fn default_telemetry() -> [f32; TELEMETRY_DIM] {
    [
        0.05, 0.05, 0.1, 0.0, 0.0, 0.12, 0.2, 0.08, 0.25, 0.03, 0.0,
    ]
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

fn paint_background(ctx: &CanvasRenderingContext2d, w: f64, h: f64, spectral_shift: f32) {
    let (r, g, b) = sigma_to_rgb(spectral_shift);
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
    telemetry: &[f32; TELEMETRY_DIM],
) {
    let heat = telemetry[4] as f64;
    let ripple = telemetry[1] as f64;
    let logic = telemetry[3] as f64;
    let quantum = telemetry[5] as f64;

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
        let sigma = ((fi * 0.017 + telemetry[6] as f64) % 1.0) as f32;
        let (r, g, b) = sigma_to_rgb(sigma);
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
    time: f64,
    tensor: &[u8],
    mode: OperationalMode,
    display: DisplayMode,
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
    let yaw = time * 0.22;
    let mut nodes: Vec<ProjectedNode> = Vec::with_capacity(cap.min(count));

    for i in (0..count).step_by(step) {
        let Ok(t) = read_tensor_at(tensor, i) else {
            continue;
        };
        let (px, py, depth) = project_xyz(t.x, t.y, t.z, w, h, yaw);
        let (r, g, b) = sigma_to_rgb(t.sigma);
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

/// σ → approximate visible-spectrum sRGB (device projection at render time).
fn sigma_to_rgb(sigma: f32) -> (u8, u8, u8) {
    let h = (sigma.clamp(0.0, 1.0) * 300.0 + 30.0) % 360.0;
    hsv_to_rgb(h, 0.82, 0.92)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let hp = (h / 60.0) % 6.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    (
        ((r1 + m) * 255.0) as u8,
        ((g1 + m) * 255.0) as u8,
        ((b1 + m) * 255.0) as u8,
    )
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