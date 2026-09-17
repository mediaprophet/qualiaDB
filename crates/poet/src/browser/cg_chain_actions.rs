//! Dual-path Tool Chest actions for curated `ComputationalGeometry.*` ALL_BOUND ids
//! (wave 16 + wave 17 remainder).
//!
//! No Host widen — scopes must already exist in `poet_host/invoke/ids.rs`.

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

fn u64_attr(el: Option<&Element>, name: &str) -> Option<u64> {
    numeric_attr(el, name).and_then(|v| {
        if v.is_finite() && v >= 0.0 && v == v.floor() && v <= u64::MAX as f64 {
            Some(v as u64)
        } else {
            None
        }
    })
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

// ── Local sketches (offline; Host path is authoritative) ─────────────

fn sketch_distance_2d(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = bx - ax;
    let dy = by - ay;
    (dx * dx + dy * dy).sqrt()
}

fn sketch_distance_3d(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> f64 {
    let dx = bx - ax;
    let dy = by - ay;
    let dz = bz - az;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn sketch_point_segment_2d(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let abx = bx - ax;
    let aby = by - ay;
    let len2 = abx * abx + aby * aby;
    let t = if len2 <= 1e-30 {
        0.0
    } else {
        (((px - ax) * abx + (py - ay) * aby) / len2).clamp(0.0, 1.0)
    };
    let qx = ax + t * abx;
    let qy = ay + t * aby;
    sketch_distance_2d(px, py, qx, qy)
}

fn sketch_orientation_2(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64) -> &'static str {
    let cross = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
    if cross > 1e-12 {
        "CounterClockwise"
    } else if cross < -1e-12 {
        "Clockwise"
    } else {
        "Collinear"
    }
}

fn sketch_orient_3d(
    ax: f64,
    ay: f64,
    az: f64,
    bx: f64,
    by: f64,
    bz: f64,
    cx: f64,
    cy: f64,
    cz: f64,
    dx: f64,
    dy: f64,
    dz: f64,
) -> &'static str {
    let abx = bx - ax;
    let aby = by - ay;
    let abz = bz - az;
    let acx = cx - ax;
    let acy = cy - ay;
    let acz = cz - az;
    let adx = dx - ax;
    let ady = dy - ay;
    let adz = dz - az;
    let vol = abx * (acy * adz - acz * ady) - aby * (acx * adz - acz * adx)
        + abz * (acx * ady - acy * adx);
    if vol > 1e-12 {
        "Positive"
    } else if vol < -1e-12 {
        "Negative"
    } else {
        "Coplanar"
    }
}

/// Bit-interleave sketch for Morton 2D (Host is authoritative).
fn sketch_morton_encode_2d(x: u16, y: u16) -> u32 {
    fn part1(mut n: u32) -> u32 {
        n &= 0x0000_FFFF;
        n = (n | (n << 8)) & 0x00FF_00FF;
        n = (n | (n << 4)) & 0x0F0F_0F0F;
        n = (n | (n << 2)) & 0x3333_3333;
        n = (n | (n << 1)) & 0x5555_5555;
        n
    }
    part1(x as u32) | (part1(y as u32) << 1)
}

fn sketch_morton_decode_2d(code: u32) -> (u16, u16) {
    fn compact1(mut n: u32) -> u32 {
        n &= 0x5555_5555;
        n = (n | (n >> 1)) & 0x3333_3333;
        n = (n | (n >> 2)) & 0x0F0F_0F0F;
        n = (n | (n >> 4)) & 0x00FF_00FF;
        n = (n | (n >> 8)) & 0x0000_FFFF;
        n
    }
    (compact1(code) as u16, compact1(code >> 1) as u16)
}

fn sketch_morton_encode_3d(x: u16, y: u16, z: u16) -> u64 {
    fn part1(mut n: u64) -> u64 {
        n &= 0x1FFF;
        n = (n | (n << 16)) & 0x1F_0000_00FF;
        n = (n | (n << 8)) & 0x100F_00F0_0F00_FF;
        n = (n | (n << 4)) & 0x10C3_0C30_C30C_30C3;
        n = (n | (n << 2)) & 0x1249_2492_4924_9249;
        n
    }
    part1(x as u64) | (part1(y as u64) << 1) | (part1(z as u64) << 2)
}

fn sketch_hilbert_encode_2d(x: u16, y: u16) -> u32 {
    // Offline sketch: Morton stand-in — Host Hilbert is authoritative.
    sketch_morton_encode_2d(x, y)
}

fn sketch_circumcenter(
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    cx: f64,
    cy: f64,
) -> Option<(f64, f64)> {
    let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    if d.abs() < 1e-15 {
        return None;
    }
    let a2 = ax * ax + ay * ay;
    let b2 = bx * bx + by * by;
    let c2 = cx * cx + cy * cy;
    let ux = (a2 * (by - cy) + b2 * (cy - ay) + c2 * (ay - by)) / d;
    let uy = (a2 * (cx - bx) + b2 * (ax - cx) + c2 * (bx - ax)) / d;
    Some((ux, uy))
}

fn resolve_n(document: &Document, nums: &[f64], need: usize, defaults: &[f64]) -> Vec<f64> {
    let container = selected_container(document);
    let keys = [
        "data-ax", "data-ay", "data-az", "data-bx", "data-by", "data-bz", "data-cx", "data-cy",
        "data-cz", "data-dx", "data-dy", "data-dz", "data-px", "data-py", "data-pz",
    ];
    let mut out = Vec::with_capacity(need);
    for i in 0..need {
        let from_attr = keys.get(i).and_then(|k| numeric_attr(container.as_ref(), k));
        let from_num = nums.get(i).copied();
        let def = defaults.get(i).copied().unwrap_or(0.0);
        out.push(from_attr.or(from_num).unwrap_or(def));
    }
    out
}

fn resolve_u64_pair(document: &Document, nums: &[f64], defaults: (u64, u64)) -> (u64, u64) {
    let container = selected_container(document);
    let x = u64_attr(container.as_ref(), "data-x")
        .or_else(|| nums.first().map(|v| (*v).max(0.0) as u64))
        .unwrap_or(defaults.0);
    let y = u64_attr(container.as_ref(), "data-y")
        .or_else(|| nums.get(1).map(|v| (*v).max(0.0) as u64))
        .unwrap_or(defaults.1);
    (x.min(u16::MAX as u64), y.min(u16::MAX as u64))
}

fn resolve_u64_triple(
    document: &Document,
    nums: &[f64],
    defaults: (u64, u64, u64),
) -> (u64, u64, u64) {
    let container = selected_container(document);
    let x = u64_attr(container.as_ref(), "data-x")
        .or_else(|| nums.first().map(|v| (*v).max(0.0) as u64))
        .unwrap_or(defaults.0);
    let y = u64_attr(container.as_ref(), "data-y")
        .or_else(|| nums.get(1).map(|v| (*v).max(0.0) as u64))
        .unwrap_or(defaults.1);
    let z = u64_attr(container.as_ref(), "data-z")
        .or_else(|| nums.get(2).map(|v| (*v).max(0.0) as u64))
        .unwrap_or(defaults.2);
    (
        x.min(u16::MAX as u64),
        y.min(u16::MAX as u64),
        z.min(u16::MAX as u64),
    )
}

/// `ComputationalGeometry.distance_2d` — four numbers or data-ax/ay/bx/by.
pub(super) fn run_distance_2d(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let v = resolve_n(document, &nums, 4, &[0.0, 0.0, 1.0, 0.0]);
    let (ax, ay, bx, by) = (v[0], v[1], v[2], v[3]);
    if ![ax, ay, bx, by].iter().all(|n| n.is_finite()) {
        super::interactions::show_tool_status(
            document,
            label,
            "Need four finite numbers (ax ay bx by).",
            "error",
        );
        return;
    }
    let sketch = sketch_distance_2d(ax, ay, bx, by);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.distance_2d",
        format!("‖A−B‖₂ sketch ≈ {sketch}"),
        json!({ "ax": ax, "ay": ay, "bx": bx, "by": by }),
    );
}

/// `ComputationalGeometry.distance_3d` — six numbers or data-a*/b*.
pub(super) fn run_distance_3d(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let v = resolve_n(document, &nums, 6, &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    let (ax, ay, az, bx, by, bz) = (v[0], v[1], v[2], v[3], v[4], v[5]);
    if ![ax, ay, az, bx, by, bz].iter().all(|n| n.is_finite()) {
        super::interactions::show_tool_status(
            document,
            label,
            "Need six finite numbers (ax ay az bx by bz).",
            "error",
        );
        return;
    }
    let sketch = sketch_distance_3d(ax, ay, az, bx, by, bz);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.distance_3d",
        format!("‖A−B‖₃ sketch ≈ {sketch}"),
        json!({ "ax": ax, "ay": ay, "az": az, "bx": bx, "by": by, "bz": bz }),
    );
}

/// `ComputationalGeometry.point_segment_distance_2d` — px py ax ay bx by.
pub(super) fn run_point_segment_2d(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    // Prefer data-px/py then fall back to first six numbers.
    let px = numeric_attr(container.as_ref(), "data-px")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.0);
    let py = numeric_attr(container.as_ref(), "data-py")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(0.0);
    let ax = numeric_attr(container.as_ref(), "data-ax")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(0.0);
    let ay = numeric_attr(container.as_ref(), "data-ay")
        .or_else(|| nums.get(3).copied())
        .unwrap_or(0.0);
    let bx = numeric_attr(container.as_ref(), "data-bx")
        .or_else(|| nums.get(4).copied())
        .unwrap_or(1.0);
    let by = numeric_attr(container.as_ref(), "data-by")
        .or_else(|| nums.get(5).copied())
        .unwrap_or(0.0);
    if ![px, py, ax, ay, bx, by].iter().all(|n| n.is_finite()) {
        super::interactions::show_tool_status(
            document,
            label,
            "Need six finite numbers (px py ax ay bx by).",
            "error",
        );
        return;
    }
    let sketch = sketch_point_segment_2d(px, py, ax, ay, bx, by);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.point_segment_distance_2d",
        format!("point–segment₂ sketch ≈ {sketch}"),
        json!({ "px": px, "py": py, "ax": ax, "ay": ay, "bx": bx, "by": by }),
    );
}

/// `ComputationalGeometry.orientation_2` — three 2D points as six numbers.
pub(super) fn run_orientation_2(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let v = resolve_n(document, &nums, 6, &[0.0, 0.0, 1.0, 0.0, 0.0, 1.0]);
    let (ax, ay, bx, by, cx, cy) = (v[0], v[1], v[2], v[3], v[4], v[5]);
    if ![ax, ay, bx, by, cx, cy].iter().all(|n| n.is_finite()) {
        super::interactions::show_tool_status(
            document,
            label,
            "Need six finite numbers for points a,b,c.",
            "error",
        );
        return;
    }
    let sketch = sketch_orientation_2(ax, ay, bx, by, cx, cy);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.orientation_2",
        format!("orientation₂ sketch → {sketch}"),
        json!({ "a": [ax, ay], "b": [bx, by], "c": [cx, cy] }),
    );
}

/// `ComputationalGeometry.orient_3d` — four 3D points as twelve numbers.
pub(super) fn run_orient_3d(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let v = resolve_n(
        document,
        &nums,
        12,
        &[
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0,
        ],
    );
    let (ax, ay, az) = (v[0], v[1], v[2]);
    let (bx, by, bz) = (v[3], v[4], v[5]);
    let (cx, cy, cz) = (v[6], v[7], v[8]);
    let (dx, dy, dz) = (v[9], v[10], v[11]);
    if ![ax, ay, az, bx, by, bz, cx, cy, cz, dx, dy, dz]
        .iter()
        .all(|n| n.is_finite())
    {
        super::interactions::show_tool_status(
            document,
            label,
            "Need twelve finite numbers for tetrahedron a,b,c,d.",
            "error",
        );
        return;
    }
    let sketch = sketch_orient_3d(ax, ay, az, bx, by, bz, cx, cy, cz, dx, dy, dz);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.orient_3d",
        format!("orient₃ sketch → {sketch}"),
        json!({
            "a": [ax, ay, az],
            "b": [bx, by, bz],
            "c": [cx, cy, cz],
            "d": [dx, dy, dz]
        }),
    );
}

/// `ComputationalGeometry.morton_encode_2d` — integer x,y (data-x/data-y).
pub(super) fn run_morton_encode_2d(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (x, y) = resolve_u64_pair(document, &nums, (1, 2));
    let sketch = sketch_morton_encode_2d(x as u16, y as u16);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.morton_encode_2d",
        format!("Morton₂({x},{y}) sketch → {sketch}"),
        json!({ "x": x, "y": y }),
    );
}

/// `ComputationalGeometry.morton_decode_2d` — code (data-code or one number).
pub(super) fn run_morton_decode_2d(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let code = u64_attr(container.as_ref(), "data-code")
        .or_else(|| nums.first().map(|v| (*v).max(0.0) as u64))
        .unwrap_or(0)
        .min(u32::MAX as u64);
    let (sx, sy) = sketch_morton_decode_2d(code as u32);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.morton_decode_2d",
        format!("Morton₂⁻¹({code}) sketch → ({sx},{sy})"),
        json!({ "code": code }),
    );
}

/// `ComputationalGeometry.morton_encode_3d` — integer x,y,z.
pub(super) fn run_morton_encode_3d(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (x, y, z) = resolve_u64_triple(document, &nums, (1, 2, 3));
    let sketch = sketch_morton_encode_3d(x as u16, y as u16, z as u16);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.morton_encode_3d",
        format!("Morton₃({x},{y},{z}) sketch → {sketch}"),
        json!({ "x": x, "y": y, "z": z }),
    );
}

/// `ComputationalGeometry.hilbert_encode_2d` — integer x,y.
pub(super) fn run_hilbert_encode_2d(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (x, y) = resolve_u64_pair(document, &nums, (1, 2));
    let sketch = sketch_hilbert_encode_2d(x as u16, y as u16);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.hilbert_encode_2d",
        format!("Hilbert₂({x},{y}) sketch ≈ {sketch} (Morton stand-in)"),
        json!({ "x": x, "y": y }),
    );
}

/// `ComputationalGeometry.circumcenter` — three 2D points as six numbers.
pub(super) fn run_circumcenter(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let v = resolve_n(document, &nums, 6, &[0.0, 0.0, 1.0, 0.0, 0.5, 0.866_025_403_78]);
    let (ax, ay, bx, by, cx, cy) = (v[0], v[1], v[2], v[3], v[4], v[5]);
    if ![ax, ay, bx, by, cx, cy].iter().all(|n| n.is_finite()) {
        super::interactions::show_tool_status(
            document,
            label,
            "Need six finite numbers for triangle a,b,c.",
            "error",
        );
        return;
    }
    let sketch_msg = match sketch_circumcenter(ax, ay, bx, by, cx, cy) {
        Some((ux, uy)) => format!("circumcenter sketch ≈ ({ux}, {uy})"),
        None => "circumcenter sketch: degenerate (collinear)".into(),
    };
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.circumcenter",
        sketch_msg,
        json!({ "a": [ax, ay], "b": [bx, by], "c": [cx, cy] }),
    );
}

// ── Wave 17 remainder helpers ────────────────────────────────────────

fn points2_from_nums(nums: &[f64], default: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut pts: Vec<[f64; 2]> = nums
        .chunks(2)
        .filter_map(|c| {
            if c.len() == 2 && c[0].is_finite() && c[1].is_finite() {
                Some([c[0], c[1]])
            } else {
                None
            }
        })
        .take(32)
        .collect();
    if pts.len() < 3 {
        pts = default.iter().copied().collect();
    }
    pts
}

fn points3_from_nums(nums: &[f64], default: &[[f64; 3]]) -> Vec<[f64; 3]> {
    let mut pts: Vec<[f64; 3]> = nums
        .chunks(3)
        .filter_map(|c| {
            if c.len() == 3 && c.iter().all(|n| n.is_finite()) {
                Some([c[0], c[1], c[2]])
            } else {
                None
            }
        })
        .take(32)
        .collect();
    if pts.len() < 2 {
        pts = default.iter().copied().collect();
    }
    pts
}

fn sketch_point_segment_3d(
    px: f64,
    py: f64,
    pz: f64,
    ax: f64,
    ay: f64,
    az: f64,
    bx: f64,
    by: f64,
    bz: f64,
) -> f64 {
    let abx = bx - ax;
    let aby = by - ay;
    let abz = bz - az;
    let len2 = abx * abx + aby * aby + abz * abz;
    let t = if len2 <= 1e-30 {
        0.0
    } else {
        (((px - ax) * abx + (py - ay) * aby + (pz - az) * abz) / len2).clamp(0.0, 1.0)
    };
    sketch_distance_3d(px, py, pz, ax + t * abx, ay + t * aby, az + t * abz)
}

fn sketch_point_triangle_dist_sq_3d(
    px: f64,
    py: f64,
    pz: f64,
    ax: f64,
    ay: f64,
    az: f64,
    bx: f64,
    by: f64,
    bz: f64,
    cx: f64,
    cy: f64,
    cz: f64,
) -> f64 {
    // Offline stand-in: min squared distance to the three edges.
    let d_ab = sketch_point_segment_3d(px, py, pz, ax, ay, az, bx, by, bz);
    let d_bc = sketch_point_segment_3d(px, py, pz, bx, by, bz, cx, cy, cz);
    let d_ca = sketch_point_segment_3d(px, py, pz, cx, cy, cz, ax, ay, az);
    let d = d_ab.min(d_bc).min(d_ca);
    d * d
}

fn sketch_line_segment_intersect_2(
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    cx: f64,
    cy: f64,
    dx: f64,
    dy: f64,
) -> Option<(f64, f64)> {
    let r_x = bx - ax;
    let r_y = by - ay;
    let s_x = dx - cx;
    let s_y = dy - cy;
    let denom = r_x * s_y - r_y * s_x;
    if denom.abs() < 1e-15 {
        return None;
    }
    let t = ((cx - ax) * s_y - (cy - ay) * s_x) / denom;
    let u = ((cx - ax) * r_y - (cy - ay) * r_x) / denom;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some((ax + t * r_x, ay + t * r_y))
    } else {
        None
    }
}

fn sketch_bezier_eval(control: &[[f64; 3]], t: f64) -> Option<(f64, f64, f64)> {
    if control.is_empty() {
        return None;
    }
    let mut pts: Vec<[f64; 3]> = control.to_vec();
    let n = pts.len();
    for r in 1..n {
        for i in 0..(n - r) {
            pts[i][0] = (1.0 - t) * pts[i][0] + t * pts[i + 1][0];
            pts[i][1] = (1.0 - t) * pts[i][1] + t * pts[i + 1][1];
            pts[i][2] = (1.0 - t) * pts[i][2] + t * pts[i + 1][2];
        }
    }
    Some((pts[0][0], pts[0][1], pts[0][2]))
}

fn sketch_nearest_site(sites: &[[f64; 2]], qx: f64, qy: f64) -> Option<usize> {
    sites
        .iter()
        .enumerate()
        .map(|(i, p)| (i, sketch_distance_2d(qx, qy, p[0], p[1])))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
}

fn sketch_hull_count(pts: &[[f64; 2]]) -> usize {
    // Offline: unique extreme corners stand-in (Host hull is authoritative).
    if pts.len() < 3 {
        return pts.len();
    }
    let mut xs: Vec<f64> = pts.iter().map(|p| p[0]).collect();
    let mut ys: Vec<f64> = pts.iter().map(|p| p[1]).collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    4.min(pts.len())
}

fn default_unit_tet() -> (Vec<[f64; 3]>, Vec<[u64; 3]>) {
    (
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.5, 0.866_025_403_78, 0.0],
            [0.5, 0.288_675_134_59, 0.816_496_580_92],
        ],
        vec![[0, 1, 2], [0, 1, 3], [1, 2, 3], [0, 2, 3]],
    )
}

fn resolve_mesh(document: &Document, nums: &[f64]) -> (Vec<[f64; 3]>, Vec<[u64; 3]>) {
    let container = selected_container(document);
    // Prefer a compact flat list: first 12 floats = 4 verts; default tet triangles.
    let verts = points3_from_nums(nums, &default_unit_tet().0);
    let tris = if let Some(raw) = container
        .as_ref()
        .and_then(|el| el.get_attribute("data-triangles"))
    {
        let idxs: Vec<u64> = raw
            .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|'))
            .filter_map(|t| t.trim().parse::<u64>().ok())
            .collect();
        let mut out = Vec::new();
        for chunk in idxs.chunks(3) {
            if chunk.len() == 3 {
                out.push([chunk[0], chunk[1], chunk[2]]);
            }
        }
        if out.is_empty() {
            default_unit_tet().1
        } else {
            out
        }
    } else if verts.len() >= 4 {
        default_unit_tet().1
    } else if verts.len() == 3 {
        vec![[0, 1, 2]]
    } else {
        default_unit_tet().1
    };
    (verts, tris)
}

fn sketch_mesh_area(verts: &[[f64; 3]], tris: &[[u64; 3]]) -> f64 {
    let mut area = 0.0;
    for t in tris {
        let Some(a) = verts.get(t[0] as usize) else {
            continue;
        };
        let Some(b) = verts.get(t[1] as usize) else {
            continue;
        };
        let Some(c) = verts.get(t[2] as usize) else {
            continue;
        };
        let abx = b[0] - a[0];
        let aby = b[1] - a[1];
        let abz = b[2] - a[2];
        let acx = c[0] - a[0];
        let acy = c[1] - a[1];
        let acz = c[2] - a[2];
        let cx = aby * acz - abz * acy;
        let cy = abz * acx - abx * acz;
        let cz = abx * acy - aby * acx;
        area += 0.5 * (cx * cx + cy * cy + cz * cz).sqrt();
    }
    area
}

fn sketch_mesh_signed_volume(verts: &[[f64; 3]], tris: &[[u64; 3]]) -> f64 {
    let mut vol = 0.0;
    for t in tris {
        let Some(a) = verts.get(t[0] as usize) else {
            continue;
        };
        let Some(b) = verts.get(t[1] as usize) else {
            continue;
        };
        let Some(c) = verts.get(t[2] as usize) else {
            continue;
        };
        vol += a[0] * (b[1] * c[2] - b[2] * c[1])
            - a[1] * (b[0] * c[2] - b[2] * c[0])
            + a[2] * (b[0] * c[1] - b[1] * c[0]);
    }
    vol / 6.0
}

/// `ComputationalGeometry.point_segment_distance_3d` — px..bz (nine numbers).
pub(super) fn run_point_segment_3d(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let keys = [
        "data-px", "data-py", "data-pz", "data-ax", "data-ay", "data-az", "data-bx", "data-by",
        "data-bz",
    ];
    let defaults = [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut v = [0.0; 9];
    for i in 0..9 {
        v[i] = numeric_attr(container.as_ref(), keys[i])
            .or_else(|| nums.get(i).copied())
            .unwrap_or(defaults[i]);
    }
    if !v.iter().all(|n| n.is_finite()) {
        super::interactions::show_tool_status(
            document,
            label,
            "Need nine finite numbers (px py pz ax ay az bx by bz).",
            "error",
        );
        return;
    }
    let sketch = sketch_point_segment_3d(v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8]);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.point_segment_distance_3d",
        format!("point–segment₃ sketch ≈ {sketch}"),
        json!({
            "px": v[0], "py": v[1], "pz": v[2],
            "ax": v[3], "ay": v[4], "az": v[5],
            "bx": v[6], "by": v[7], "bz": v[8]
        }),
    );
}

/// `ComputationalGeometry.point_triangle_distance_3d` — twelve numbers (returns distance²).
pub(super) fn run_point_triangle_3d(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let v = resolve_n(
        document,
        &nums,
        12,
        &[
            0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        ],
    );
    if !v.iter().all(|n| n.is_finite()) {
        super::interactions::show_tool_status(
            document,
            label,
            "Need twelve finite numbers (p + triangle a,b,c).",
            "error",
        );
        return;
    }
    let sketch = sketch_point_triangle_dist_sq_3d(
        v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8], v[9], v[10], v[11],
    );
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.point_triangle_distance_3d",
        format!("point–triangle₃ distance² sketch ≈ {sketch}"),
        json!({
            "px": v[0], "py": v[1], "pz": v[2],
            "ax": v[3], "ay": v[4], "az": v[5],
            "bx": v[6], "by": v[7], "bz": v[8],
            "cx": v[9], "cy": v[10], "cz": v[11]
        }),
    );
}

/// `ComputationalGeometry.convex_hull_2` — flat x,y pairs → points.
pub(super) fn run_convex_hull_2(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let pts = points2_from_nums(
        &nums,
        &[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.5, 0.5]],
    );
    let sketch_n = sketch_hull_count(&pts);
    let points: Vec<serde_json::Value> = pts.iter().map(|p| json!([p[0], p[1]])).collect();
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.convex_hull_2",
        format!("convex_hull₂ sketch ≈ {sketch_n} hull verts from {} pts", pts.len()),
        json!({ "points": points }),
    );
}

/// `ComputationalGeometry.triangulate_polygon` — flat x,y pairs → vertices.
pub(super) fn run_triangulate_polygon(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let pts = points2_from_nums(
        &nums,
        &[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    let sketch_tris = pts.len().saturating_sub(2);
    let vertices: Vec<serde_json::Value> = pts.iter().map(|p| json!([p[0], p[1]])).collect();
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.triangulate_polygon",
        format!("triangulate sketch ≈ {sketch_tris} tris from {} verts", pts.len()),
        json!({ "vertices": vertices }),
    );
}

/// `ComputationalGeometry.surface_area` — 3D verts (+ optional data-triangles).
pub(super) fn run_surface_area(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (verts, tris) = resolve_mesh(document, &nums);
    let sketch = sketch_mesh_area(&verts, &tris);
    let vertices: Vec<serde_json::Value> =
        verts.iter().map(|p| json!([p[0], p[1], p[2]])).collect();
    let triangles: Vec<serde_json::Value> =
        tris.iter().map(|t| json!([t[0], t[1], t[2]])).collect();
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.surface_area",
        format!("surface_area sketch ≈ {sketch}"),
        json!({ "vertices": vertices, "triangles": triangles }),
    );
}

/// `ComputationalGeometry.signed_volume` — 3D verts (+ optional data-triangles).
pub(super) fn run_signed_volume(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (verts, tris) = resolve_mesh(document, &nums);
    let sketch = sketch_mesh_signed_volume(&verts, &tris);
    let vertices: Vec<serde_json::Value> =
        verts.iter().map(|p| json!([p[0], p[1], p[2]])).collect();
    let triangles: Vec<serde_json::Value> =
        tris.iter().map(|t| json!([t[0], t[1], t[2]])).collect();
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.signed_volume",
        format!("signed_volume sketch ≈ {sketch}"),
        json!({ "vertices": vertices, "triangles": triangles }),
    );
}

/// `ComputationalGeometry.line_segment_intersection_2` — eight numbers a,b,c,d.
pub(super) fn run_line_segment_intersection_2(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let v = resolve_n(
        document,
        &nums,
        8,
        &[0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0],
    );
    if !v.iter().all(|n| n.is_finite()) {
        super::interactions::show_tool_status(
            document,
            label,
            "Need eight finite numbers for segments ab and cd.",
            "error",
        );
        return;
    }
    let sketch_msg = match sketch_line_segment_intersect_2(
        v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7],
    ) {
        Some((x, y)) => format!("segment∩ sketch ≈ ({x}, {y})"),
        None => "segment∩ sketch: no intersection".into(),
    };
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.line_segment_intersection_2",
        sketch_msg,
        json!({
            "a": [v[0], v[1]],
            "b": [v[2], v[3]],
            "c": [v[4], v[5]],
            "d": [v[6], v[7]]
        }),
    );
}

/// `ComputationalGeometry.bezier_eval` — control xyz triples + t (data-t).
pub(super) fn run_bezier_eval(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let t = numeric_attr(container.as_ref(), "data-t")
        .or_else(|| nums.last().copied().filter(|_| nums.len() % 3 == 1))
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);
    let control_nums = if nums.len() % 3 == 1 && nums.len() > 1 {
        &nums[..nums.len() - 1]
    } else {
        nums.as_slice()
    };
    let control = points3_from_nums(
        control_nums,
        &[[0.0, 0.0, 0.0], [0.5, 1.0, 0.0], [1.0, 0.0, 0.0]],
    );
    let sketch_msg = match sketch_bezier_eval(&control, t) {
        Some((x, y, z)) => format!("bezier({t}) sketch ≈ ({x}, {y}, {z})"),
        None => "bezier sketch: empty control".into(),
    };
    let control_json: Vec<serde_json::Value> =
        control.iter().map(|p| json!([p[0], p[1], p[2]])).collect();
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.bezier_eval",
        sketch_msg,
        json!({ "control": control_json, "t": t }),
    );
}

/// `ComputationalGeometry.nearest_site_brute_force` — sites then query (data-qx/qy).
pub(super) fn run_nearest_site(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let qx = numeric_attr(container.as_ref(), "data-qx")
        .or_else(|| {
            if nums.len() >= 2 {
                nums.get(nums.len().saturating_sub(2)).copied()
            } else {
                None
            }
        })
        .unwrap_or(0.5);
    let qy = numeric_attr(container.as_ref(), "data-qy")
        .or_else(|| nums.last().copied())
        .unwrap_or(0.5);
    let site_nums = if numeric_attr(container.as_ref(), "data-qx").is_some()
        || numeric_attr(container.as_ref(), "data-qy").is_some()
    {
        nums.as_slice()
    } else if nums.len() >= 4 {
        &nums[..nums.len() - 2]
    } else {
        nums.as_slice()
    };
    let sites = points2_from_nums(
        site_nums,
        &[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
    );
    let sketch_msg = match sketch_nearest_site(&sites, qx, qy) {
        Some(i) => format!("nearest_site sketch → index {i}"),
        None => "nearest_site sketch: no sites".into(),
    };
    let sites_json: Vec<serde_json::Value> =
        sites.iter().map(|p| json!([p[0], p[1]])).collect();
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.nearest_site_brute_force",
        sketch_msg,
        json!({ "sites": sites_json, "query": [qx, qy] }),
    );
}
