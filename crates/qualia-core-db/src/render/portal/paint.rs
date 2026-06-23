//! Canvas2D fallback painters — the non-WebGPU 2.5D field (background, ambient, tensor, HUD).
use super::*;
pub(super) fn paint_background(ctx: &CanvasRenderingContext2d, w: f64, h: f64, _spectral_shift: f32) {
    // Black background. (Previously a spectral-shift `rgb(r,g,b)` top stop, which read as a pink
    // wash.) The σ spectral signal still drives the particle colours in `paint_ambient_field`, so
    // the spectral projection stays visible — on black, where it reads cleanly.
    let gradient = ctx.create_linear_gradient(0.0, 0.0, w, h);
    let _ = gradient.add_color_stop(0.0, "#05070b");
    let _ = gradient.add_color_stop(1.0, "#000000");
    ctx.set_fill_style(&JsValue::from(gradient));
    ctx.fill_rect(0.0, 0.0, w, h);
}

pub(super) fn paint_ambient_field(
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

pub(super) fn paint_tensor_projection(
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

pub(super) fn stroke_segment(ctx: &CanvasRenderingContext2d, a: ProjectedNode, b: ProjectedNode, alpha: f32) {
    ctx.set_stroke_style(&JsValue::from_str(&format!(
        "rgba({},{},{},{alpha:.2})",
        a.r, a.g, a.b
    )));
    ctx.begin_path();
    ctx.move_to(a.px, a.py);
    ctx.line_to(b.px, b.py);
    ctx.stroke();
}

pub(super) fn project_xyz(x: f32, y: f32, z: f32, w: f64, h: f64, yaw: f64) -> (f64, f64, f32) {
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

pub(super) fn paint_hud(ctx: &CanvasRenderingContext2d, portal: &QualiaPortal, mode: OperationalMode) {
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

pub(super) fn append_parsed_dom(document: &Document, panel: &Element, parsed: &JsValue) -> Result<(), JsValue> {
    if parsed.is_array() {
        let arr: Array = parsed.clone().dyn_into()?;
        for entry in arr.iter() {
            append_triple_dom(document, panel, &entry)?;
        }
        return Ok(());
    }
    append_triple_dom(document, panel, parsed)
}

pub(super) fn append_triple_dom(document: &Document, panel: &Element, triple: &JsValue) -> Result<(), JsValue> {
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

pub(super) fn field_as_string(value: &JsValue, key: &str) -> Option<String> {
    Reflect::get(value, &JsValue::from_str(key))
        .ok()
        .filter(|v| !v.is_null() && !v.is_undefined())
        .and_then(|v| v.as_string())
}

pub(super) fn ontology_tag(predicate: &str) -> &'static str {
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

pub(super) fn html_escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[inline]
pub(super) fn acoustic_uniform_to_floats(u: &AcousticUniform) -> [f32; ACOUSTIC_UNIFORM_FLOAT_COUNT] {
    let mut floats = [0.0_f32; ACOUSTIC_UNIFORM_FLOAT_COUNT];
    floats[0] = u.alpha;
    floats[1] = u.mu;
    floats[2] = u.position[0];
    floats[3] = u.position[1];
    floats[4] = u.position[2];
    floats[5] = u.track_v;
    floats[6] = u.manifold_w;
    floats[7] = u.epistemic_q;
    floats[8] = u.fm_index;
    floats[9] = u.frequency_hz;
    floats[10] = u.enabled as f32;
    floats[11] = u.gain_l;
    floats[12] = u.gain_r;
    floats[13] = u.itd_seconds;
    floats[14] = u.azimuth_rad;
    floats[15] = u.elevation_rad;
    floats[16] = u.room_damp;
    floats[17] = u.stft_frame;
    floats[18..].copy_from_slice(&u.preview_bins);
    floats
}
