//! Dual-path Tool Chest actions for curated `GeometricAlgebra.*` ALL_BOUND ids (wave 17).
//!
//! No Host widen — scopes must already exist in `poet_host/invoke/ids.rs`.
//! Local sketches mirror Host scalar / Cl(3,0) algorithms (CPU; Host path authoritative).

use serde_json::json;
use web_sys::{Document, Element};

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn selected_source(document: &Document) -> Option<String> {
    let container = selected_container(document)?;
    let text = container
        .query_selector(".vibe-editor, .vibe-editor-textarea, .doc-editor, .sheet-grid")
        .ok()
        .flatten()
        .and_then(|editor| editor.text_content())
        .or_else(|| container.text_content())?;
    let bounded: String = text.chars().take(16_384).collect();
    (!bounded.trim().is_empty()).then_some(bounded)
}

fn parse_numbers(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|' | '\n' | '\r'))
        .filter_map(|token| token.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(64)
        .collect()
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
}

fn nth(nums: &[f64], i: usize, default: f64) -> f64 {
    nums.get(i).copied().unwrap_or(default)
}

fn invoke_dual(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    local_message: String,
    args: serde_json::Value,
) {
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(cap_id, &local_message);
        super::interactions::show_tool_status(
            document,
            &label,
            &report.message,
            report.status_kind,
        );
        return;
    }
    super::interactions::show_tool_status(
        document,
        &label,
        &format!("Running {cap_id}…"),
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        match super::native_daemon::daemon_invoke(cap_id, args).await {
            Ok(response) if response.ok => {
                let report = super::tool_dual_path::live_ok(cap_id, &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    cap_id,
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("capability invoke failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied(cap_id, &error);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
        }
    });
}

// ── Local sketches (match Host scalar / Cl(3,0) algorithms) ───────────

const GA_EPS: f64 = 1e-6;

fn sketch_dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sketch_cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn sketch_normalize(v: [f64; 3]) -> [f64; 3] {
    let mag = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if mag > GA_EPS {
        [v[0] / mag, v[1] / mag, v[2] / mag]
    } else {
        v
    }
}

fn sketch_angle(a: [f64; 3], b: [f64; 3]) -> f64 {
    let mag_a = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    let mag_b = (b[0] * b[0] + b[1] * b[1] + b[2] * b[2]).sqrt();
    if mag_a > GA_EPS && mag_b > GA_EPS {
        (sketch_dot(a, b) / (mag_a * mag_b)).clamp(-1.0, 1.0).acos()
    } else {
        0.0
    }
}

/// Cl(3,0) geometric product of two pure vectors → scalar + bivector.
fn sketch_gp_vectors(a: [f64; 3], b: [f64; 3]) -> [f64; 8] {
    let c = sketch_cross(a, b);
    [
        sketch_dot(a, b),
        0.0,
        0.0,
        0.0,
        c[2],  // e12
        -c[1], // e13
        c[0],  // e23
        0.0,
    ]
}

/// Cl(3,0) outer product of two pure vectors → bivector.
fn sketch_op_vectors(a: [f64; 3], b: [f64; 3]) -> [f64; 8] {
    let mut out = sketch_gp_vectors(a, b);
    out[0] = 0.0;
    out
}

fn sketch_rotor(angle: f64, axis: [f64; 3]) -> [f64; 4] {
    let n = sketch_normalize(axis);
    let half = angle * 0.5;
    let s = half.sin();
    let c = half.cos();
    [c, -n[2] * s, n[1] * s, -n[0] * s]
}

fn sketch_apply_rotor(rotor: [f64; 4], v: [f64; 3]) -> [f64; 3] {
    // Recover θ, unit axis from Host rotor layout, then Rodrigues.
    let c = rotor[0].clamp(-1.0, 1.0);
    let half = c.acos();
    let angle = 2.0 * half;
    let s = half.sin();
    let axis = if s.abs() > GA_EPS {
        sketch_normalize([-rotor[3] / s, rotor[2] / s, -rotor[1] / s])
    } else {
        [0.0, 0.0, 1.0]
    };
    let (ca, sa) = (angle.cos(), angle.sin());
    let dot = sketch_dot(axis, v);
    let cross = sketch_cross(axis, v);
    [
        v[0] * ca + cross[0] * sa + axis[0] * dot * (1.0 - ca),
        v[1] * ca + cross[1] * sa + axis[1] * dot * (1.0 - ca),
        v[2] * ca + cross[2] * sa + axis[2] * dot * (1.0 - ca),
    ]
}

fn sketch_translator(d: [f64; 3]) -> [f64; 4] {
    [1.0, d[0] * 0.5, d[1] * 0.5, d[2] * 0.5]
}

fn sketch_apply_translator(t: [f64; 4], v: [f64; 3]) -> [f64; 3] {
    [v[0] + 2.0 * t[1], v[1] + 2.0 * t[2], v[2] + 2.0 * t[3]]
}

fn fmt8(c: &[f64; 8]) -> String {
    format!(
        "[{:.4}, {:.4}, {:.4}, {:.4}, {:.4}, {:.4}, {:.4}, {:.4}]",
        c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]
    )
}

fn fmt3(v: [f64; 3]) -> String {
    format!("[{:.6}, {:.6}, {:.6}]", v[0], v[1], v[2])
}

fn resolve_vec_pair(container: Option<&Element>, nums: &[f64]) -> ([f64; 3], [f64; 3]) {
    let a = [
        numeric_attr(container, "data-ax").unwrap_or_else(|| nth(nums, 0, 1.0)),
        numeric_attr(container, "data-ay").unwrap_or_else(|| nth(nums, 1, 0.0)),
        numeric_attr(container, "data-az").unwrap_or_else(|| nth(nums, 2, 0.0)),
    ];
    let b = [
        numeric_attr(container, "data-bx").unwrap_or_else(|| nth(nums, 3, 0.0)),
        numeric_attr(container, "data-by").unwrap_or_else(|| nth(nums, 4, 1.0)),
        numeric_attr(container, "data-bz").unwrap_or_else(|| nth(nums, 5, 0.0)),
    ];
    (a, b)
}

fn mv8_from_vec(v: [f64; 3]) -> Vec<f64> {
    vec![0.0, v[0], v[1], v[2], 0.0, 0.0, 0.0, 0.0]
}

/// `GeometricAlgebra.dot`
pub(super) fn run_dot(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (a, b) = resolve_vec_pair(container.as_ref(), &nums);
    let value = sketch_dot(a, b);
    invoke_dual(
        document,
        label,
        "GeometricAlgebra.dot",
        format!("Dot product sketch ≈ {value}"),
        json!({ "a": a.to_vec(), "b": b.to_vec() }),
    );
}

/// `GeometricAlgebra.cross_product`
pub(super) fn run_cross_product(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (a, b) = resolve_vec_pair(container.as_ref(), &nums);
    let c = sketch_cross(a, b);
    invoke_dual(
        document,
        label,
        "GeometricAlgebra.cross_product",
        format!("Cross product sketch ≈ {}", fmt3(c)),
        json!({ "a": a.to_vec(), "b": b.to_vec() }),
    );
}

/// `GeometricAlgebra.normalize_vector`
pub(super) fn run_normalize_vector(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let v = [
        numeric_attr(container.as_ref(), "data-vx").unwrap_or_else(|| nth(&nums, 0, 3.0)),
        numeric_attr(container.as_ref(), "data-vy").unwrap_or_else(|| nth(&nums, 1, 4.0)),
        numeric_attr(container.as_ref(), "data-vz").unwrap_or_else(|| nth(&nums, 2, 0.0)),
    ];
    let n = sketch_normalize(v);
    invoke_dual(
        document,
        label,
        "GeometricAlgebra.normalize_vector",
        format!("Normalize vector sketch ≈ {}", fmt3(n)),
        json!({ "v": v.to_vec() }),
    );
}

/// `GeometricAlgebra.angle_between_vectors`
pub(super) fn run_angle_between_vectors(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (a, b) = resolve_vec_pair(container.as_ref(), &nums);
    let radians = sketch_angle(a, b);
    invoke_dual(
        document,
        label,
        "GeometricAlgebra.angle_between_vectors",
        format!("Angle between vectors sketch ≈ {radians} rad"),
        json!({ "a": a.to_vec(), "b": b.to_vec() }),
    );
}

/// `GeometricAlgebra.geometric_product`
pub(super) fn run_geometric_product(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (a, b) = resolve_vec_pair(container.as_ref(), &nums);
    let coeffs = sketch_gp_vectors(a, b);
    invoke_dual(
        document,
        label,
        "GeometricAlgebra.geometric_product",
        format!("Geometric product (vector) sketch coeffs ≈ {}", fmt8(&coeffs)),
        json!({ "a": mv8_from_vec(a), "b": mv8_from_vec(b) }),
    );
}

/// `GeometricAlgebra.outer_product`
pub(super) fn run_outer_product(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (a, b) = resolve_vec_pair(container.as_ref(), &nums);
    let coeffs = sketch_op_vectors(a, b);
    invoke_dual(
        document,
        label,
        "GeometricAlgebra.outer_product",
        format!("Outer product (vector) sketch coeffs ≈ {}", fmt8(&coeffs)),
        json!({ "a": mv8_from_vec(a), "b": mv8_from_vec(b) }),
    );
}

/// `GeometricAlgebra.rotor_from_angle_axis`
pub(super) fn run_rotor_from_angle_axis(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let angle = numeric_attr(container.as_ref(), "data-angle")
        .unwrap_or_else(|| nth(&nums, 0, std::f64::consts::FRAC_PI_2));
    let axis = [
        numeric_attr(container.as_ref(), "data-nx").unwrap_or_else(|| nth(&nums, 1, 0.0)),
        numeric_attr(container.as_ref(), "data-ny").unwrap_or_else(|| nth(&nums, 2, 0.0)),
        numeric_attr(container.as_ref(), "data-nz").unwrap_or_else(|| nth(&nums, 3, 1.0)),
    ];
    let r = sketch_rotor(angle, axis);
    invoke_dual(
        document,
        label,
        "GeometricAlgebra.rotor_from_angle_axis",
        format!(
            "Rotor sketch components ≈ [{:.6}, {:.6}, {:.6}, {:.6}]",
            r[0], r[1], r[2], r[3]
        ),
        json!({ "angle": angle, "axis": axis.to_vec() }),
    );
}

/// `GeometricAlgebra.apply_rotor`
pub(super) fn run_apply_rotor(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    // Default: 90° about z applied to e1 → e2.
    let default_rotor = sketch_rotor(std::f64::consts::FRAC_PI_2, [0.0, 0.0, 1.0]);
    let rotor = [
        numeric_attr(container.as_ref(), "data-r0").unwrap_or_else(|| nth(&nums, 0, default_rotor[0])),
        numeric_attr(container.as_ref(), "data-r1").unwrap_or_else(|| nth(&nums, 1, default_rotor[1])),
        numeric_attr(container.as_ref(), "data-r2").unwrap_or_else(|| nth(&nums, 2, default_rotor[2])),
        numeric_attr(container.as_ref(), "data-r3").unwrap_or_else(|| nth(&nums, 3, default_rotor[3])),
    ];
    let vector = [
        numeric_attr(container.as_ref(), "data-vx").unwrap_or_else(|| nth(&nums, 4, 1.0)),
        numeric_attr(container.as_ref(), "data-vy").unwrap_or_else(|| nth(&nums, 5, 0.0)),
        numeric_attr(container.as_ref(), "data-vz").unwrap_or_else(|| nth(&nums, 6, 0.0)),
    ];
    let out = sketch_apply_rotor(rotor, vector);
    invoke_dual(
        document,
        label,
        "GeometricAlgebra.apply_rotor",
        format!("Apply rotor sketch ≈ {}", fmt3(out)),
        json!({ "rotor": rotor.to_vec(), "vector": vector.to_vec() }),
    );
}

/// `GeometricAlgebra.translator_from_displacement`
pub(super) fn run_translator_from_displacement(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let d = [
        numeric_attr(container.as_ref(), "data-dx").unwrap_or_else(|| nth(&nums, 0, 1.0)),
        numeric_attr(container.as_ref(), "data-dy").unwrap_or_else(|| nth(&nums, 1, 2.0)),
        numeric_attr(container.as_ref(), "data-dz").unwrap_or_else(|| nth(&nums, 2, 3.0)),
    ];
    let t = sketch_translator(d);
    invoke_dual(
        document,
        label,
        "GeometricAlgebra.translator_from_displacement",
        format!(
            "Translator sketch components ≈ [{:.6}, {:.6}, {:.6}, {:.6}]",
            t[0], t[1], t[2], t[3]
        ),
        json!({ "displacement": d.to_vec() }),
    );
}

/// `GeometricAlgebra.apply_translator`
pub(super) fn run_apply_translator(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let default_t = sketch_translator([1.0, 2.0, 3.0]);
    let translator = [
        numeric_attr(container.as_ref(), "data-t0").unwrap_or_else(|| nth(&nums, 0, default_t[0])),
        numeric_attr(container.as_ref(), "data-t1").unwrap_or_else(|| nth(&nums, 1, default_t[1])),
        numeric_attr(container.as_ref(), "data-t2").unwrap_or_else(|| nth(&nums, 2, default_t[2])),
        numeric_attr(container.as_ref(), "data-t3").unwrap_or_else(|| nth(&nums, 3, default_t[3])),
    ];
    let vector = [
        numeric_attr(container.as_ref(), "data-vx").unwrap_or_else(|| nth(&nums, 4, 0.0)),
        numeric_attr(container.as_ref(), "data-vy").unwrap_or_else(|| nth(&nums, 5, 0.0)),
        numeric_attr(container.as_ref(), "data-vz").unwrap_or_else(|| nth(&nums, 6, 0.0)),
    ];
    let out = sketch_apply_translator(translator, vector);
    invoke_dual(
        document,
        label,
        "GeometricAlgebra.apply_translator",
        format!("Apply translator sketch ≈ {}", fmt3(out)),
        json!({ "translator": translator.to_vec(), "vector": vector.to_vec() }),
    );
}

/// `GeometricAlgebra.is_simd_available`
pub(super) fn run_is_simd_available(document: &Document, label: &str) {
    let _ = selected_container(document);
    // AVX2 detection is Host-side; offline sketch reports false honestly.
    invoke_dual(
        document,
        label,
        "GeometricAlgebra.is_simd_available",
        "SIMD availability sketch: false (Host path authoritative)".into(),
        json!({}),
    );
}
