//! `Render.scene` — build a renderer contract from Qualia kernels.
//!
//! Cold construction. The output is a serialisable node/edge/face record the
//! desktop host maps onto `webizen_render::RenderScene`. No wgpu here.

use super::super::{args, clinical, engineering, geometry};
use crate::poet_host::PoetSnapshot;
use poet_vibe::{Diagnostic, Span, Value};

pub fn scene(snap: &PoetSnapshot, args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let kind = args::rec_str(args_v, "kind")
        .or_else(|| args::as_str(args_v))
        .unwrap_or("research");
    match kind {
        "media" => media_scene(span),
        "social" => social_scene(snap),
        "health" => health_scene(span),
        "settings" => Ok(ring_scene("settings", "#94a3b8", 8, 0.28)),
        "vibe" => Ok(vibe_scene()),
        "map" | "research" | "submanifold" => map_scene(args_v, span),
        other => map_scene(&named_kind(other), span),
    }
}

fn named_kind(kind: &str) -> Value {
    args::record([("kind", Value::String(kind.into()))])
}

fn map_scene(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let points = custom_points(args_v).unwrap_or_else(catchment_points);
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for (i, (x, y, z)) in points.iter().copied().enumerate() {
        let projected = super::super::manifold::project(
            &args::record([
                ("x", Value::F64(x)),
                ("y", Value::F64(y)),
                ("z", Value::F64(z)),
                ("level", Value::U64(2)),
                ("entity", Value::U64(i as u64 + 1)),
            ]),
            span,
        )?;
        let (px, py, pz) = xyz_of(&projected).unwrap_or((x, y, z));
        nodes.push(node(
            format!("site-{i}"),
            px,
            py,
            pz,
            "#4ade80",
            7.0,
            0.42 + (i as f64) * 0.05,
        ));
        if i > 0 {
            let (ax, ay, az) = points[i - 1];
            edges.push(edge(ax, ay, az, px, py, pz, "#166534", 1.4));
        }
    }
    let mut faces = Vec::new();
    let hull_in = Value::List(
        points
            .iter()
            .map(|(x, y, _)| args::f64_list_value([*x, *y]))
            .collect(),
    );
    if let Ok(Value::List(hull)) = geometry::hull2(&hull_in, span) {
        let verts: Vec<Value> = hull
            .iter()
            .filter_map(|p| {
                let xy = args::f64s(p)?;
                if xy.len() < 2 {
                    return None;
                }
                Some(args::record([
                    ("x", Value::F64(xy[0])),
                    ("y", Value::F64(xy[1])),
                    ("z", Value::F64(0.0)),
                ]))
            })
            .collect();
        if verts.len() >= 3 {
            faces.push(args::record([
                ("vertices", Value::List(verts)),
                ("color", Value::String("#14532d".into())),
                ("alpha", Value::F64(0.28)),
            ]));
        }
    }
    Ok(pack(
        "map",
        "#07090e",
        nodes,
        edges,
        faces,
        [0.0, 0.0, 2.4],
        "live",
    ))
}

fn media_scene(span: Span) -> Result<Value, Diagnostic> {
    let kin = engineering::kinematics(
        &args::record([
            ("x0", Value::F64(0.12)),
            ("v0", Value::F64(0.10)),
            ("a", Value::F64(0.015)),
            ("t", args::f64_list_value([0.0, 1.0, 2.0, 3.0, 4.0, 5.0])),
        ]),
        span,
    );
    let xs = match kin {
        Ok(Value::Record(r)) => match r.get("positions") {
            Some(Value::List(v)) => v.iter().filter_map(args::as_f64).collect(),
            _ => default_path(),
        },
        _ => default_path(),
    };
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let n = xs.len().max(1);
    for (i, x) in xs.iter().copied().enumerate() {
        let nx = (x * 0.55 + 0.22).clamp(0.08, 0.92);
        let ny = 0.50 + 0.22 * ((i as f64) * 0.9).sin();
        let nz = (i as f64) / n as f64 * 0.2;
        nodes.push(node(
            format!("pose-{i}"),
            nx,
            ny,
            nz,
            "#00d2ff",
            6.0 + (i as f64) * 0.4,
            0.55 + (i as f64) * 0.07,
        ));
        if i > 0 {
            if let Value::Record(prev) = &nodes[i - 1] {
                if let (Some(ax), Some(ay), Some(az)) =
                    (rec_num(prev, "x"), rec_num(prev, "y"), rec_num(prev, "z"))
                {
                    edges.push(edge(ax, ay, az, nx, ny, nz, "#0891b2", 1.6));
                }
            }
        }
    }
    Ok(pack(
        "media",
        "#05070c",
        nodes,
        edges,
        Vec::new(),
        [0.15, 0.1, 2.2],
        "live",
    ))
}

fn social_scene(snap: &PoetSnapshot) -> Result<Value, Diagnostic> {
    let stats = super::super::graph::stats(snap);
    let count = match &stats {
        Value::Record(r) => rec_num(r, "quin_count").unwrap_or(1.0),
        _ => 1.0,
    };
    let n = ((count as usize) + 3).clamp(3, 10);
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for i in 0..n {
        let a = (i as f64) / (n as f64) * std::f64::consts::TAU;
        let x = 0.50 + 0.28 * a.cos();
        let y = 0.50 + 0.28 * a.sin();
        nodes.push(node(
            format!("peer-{i}"),
            x,
            y,
            0.0,
            "#a78bfa",
            6.5,
            0.35 + (i as f64) * 0.04,
        ));
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let (ax, ay, _) = xyz_tuple(&nodes[i]);
        let (bx, by, _) = xyz_tuple(&nodes[j]);
        edges.push(edge(ax, ay, 0.0, bx, by, 0.0, "#6d28d9", 1.2));
    }
    Ok(pack(
        "social",
        "#0b0614",
        nodes,
        edges,
        Vec::new(),
        [0.0, 0.0, 2.6],
        snap.honesty(),
    ))
}

fn health_scene(span: Span) -> Result<Value, Diagnostic> {
    let score = clinical::framingham(&Value::Record(Default::default()), span);
    let (risk, category) = match score {
        Ok(Value::Record(r)) => (
            rec_num(&r, "risk_10yr").unwrap_or(0.12),
            match r.get("category") {
                Some(Value::String(s)) => s.clone(),
                _ => "reference".into(),
            },
        ),
        _ => (0.12, "unavailable".into()),
    };
    let nodes = vec![
        node(
            "reference-profile".into(),
            0.50,
            0.48,
            0.0,
            "#fb7185",
            8.0 + risk * 18.0,
            0.2 + risk,
        ),
        node("context-a".into(), 0.28, 0.68, 0.0, "#fda4af", 5.0, 0.15),
        node("context-b".into(), 0.72, 0.32, 0.0, "#fecdd3", 5.0, 0.15),
    ];
    let honesty = if category == "unavailable" {
        "partial"
    } else {
        "live"
    };
    let mut rec = match pack(
        "health",
        "#14080c",
        nodes,
        vec![
            edge(0.50, 0.48, 0.0, 0.28, 0.68, 0.0, "#9f1239", 1.1),
            edge(0.50, 0.48, 0.0, 0.72, 0.32, 0.0, "#9f1239", 1.1),
        ],
        Vec::new(),
        [0.0, 0.1, 2.3],
        honesty,
    ) {
        Value::Record(m) => m,
        other => return Ok(other),
    };
    rec.insert("risk_10yr".into(), Value::F64(risk));
    rec.insert("category".into(), Value::String(category));
    rec.insert(
        "note".into(),
        Value::String("reference adult profile — not a named person".into()),
    );
    Ok(Value::Record(rec))
}

fn vibe_scene() -> Value {
    let nodes = vec![
        node("parse".into(), 0.28, 0.68, 0.0, "#ffb834", 8.0, 0.5),
        node("check".into(), 0.72, 0.68, 0.0, "#00d2ff", 8.0, 0.55),
        node("eval".into(), 0.50, 0.30, 0.0, "#4ade80", 9.0, 0.7),
    ];
    pack(
        "vibe",
        "#07090e",
        nodes,
        vec![
            edge(0.28, 0.68, 0.0, 0.72, 0.68, 0.0, "#334155", 1.5),
            edge(0.28, 0.68, 0.0, 0.50, 0.30, 0.0, "#334155", 1.5),
            edge(0.72, 0.68, 0.0, 0.50, 0.30, 0.0, "#334155", 1.5),
        ],
        Vec::new(),
        [0.0, 0.0, 2.1],
        "live",
    )
}

fn ring_scene(kind: &'static str, color: &str, n: usize, radius: f64) -> Value {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for i in 0..n {
        let a = (i as f64) / (n as f64) * std::f64::consts::TAU;
        let x = 0.50 + radius * a.cos();
        let y = 0.50 + radius * a.sin();
        nodes.push(node(
            format!("{kind}-{i}"),
            x,
            y,
            0.0,
            color,
            5.5,
            0.3 + (i as f64) * 0.04,
        ));
        if i > 0 {
            let (ax, ay, _) = xyz_tuple(&nodes[i - 1]);
            edges.push(edge(ax, ay, 0.0, x, y, 0.0, color, 1.0));
        }
    }
    if n > 2 {
        let (ax, ay, _) = xyz_tuple(&nodes[n - 1]);
        let (bx, by, _) = xyz_tuple(&nodes[0]);
        edges.push(edge(ax, ay, 0.0, bx, by, 0.0, color, 1.0));
    }
    pack(
        kind,
        "#07090e",
        nodes,
        edges,
        Vec::new(),
        [0.0, 0.0, 2.5],
        "live",
    )
}

fn catchment_points() -> Vec<(f64, f64, f64)> {
    vec![
        (0.20, 0.64, 0.0),
        (0.34, 0.30, 0.02),
        (0.58, 0.22, 0.01),
        (0.80, 0.46, 0.03),
        (0.66, 0.76, 0.02),
        (0.38, 0.82, 0.01),
    ]
}

fn default_path() -> Vec<f64> {
    vec![0.12, 0.22, 0.34, 0.48, 0.64, 0.82]
}

fn custom_points(args_v: &Value) -> Option<Vec<(f64, f64, f64)>> {
    let rows = args::rec(args_v, "points").and_then(args::list)?;
    let mut out = Vec::new();
    for row in rows {
        let xy = args::f64s(row)?;
        if xy.len() < 2 {
            return None;
        }
        out.push((xy[0], xy[1], xy.get(2).copied().unwrap_or(0.0)));
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn node(id: String, x: f64, y: f64, z: f64, color: &str, radius: f64, sigma: f64) -> Value {
    args::record([
        ("id", Value::String(id)),
        ("x", Value::F64(x)),
        ("y", Value::F64(y)),
        ("z", Value::F64(z)),
        ("color", Value::String(color.into())),
        ("radius", Value::F64(radius)),
        ("sigma", Value::F64(sigma)),
        ("alpha", Value::F64(0.92)),
    ])
}

fn edge(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64, color: &str, width: f64) -> Value {
    args::record([
        ("from", args::f64_list_value([ax, ay, az])),
        ("to", args::f64_list_value([bx, by, bz])),
        ("color", Value::String(color.into())),
        ("width", Value::F64(width)),
        ("alpha", Value::F64(0.8)),
    ])
}

fn pack(
    kind: &str,
    background: &str,
    nodes: Vec<Value>,
    edges: Vec<Value>,
    faces: Vec<Value>,
    camera: [f64; 3],
    honesty: &'static str,
) -> Value {
    args::record([
        ("kind", Value::String(kind.into())),
        ("honesty", Value::String(honesty.into())),
        ("background", Value::String(background.into())),
        ("nodes", Value::List(nodes)),
        ("edges", Value::List(edges)),
        ("faces", Value::List(faces)),
        (
            "camera",
            args::record([
                ("x", Value::F64(camera[0])),
                ("y", Value::F64(camera[1])),
                ("z", Value::F64(camera[2])),
                ("fov", Value::F64(58.0)),
            ]),
        ),
        (
            "contract",
            Value::String("webizen_render::scene_contract::RenderScene".into()),
        ),
    ])
}

fn xyz_of(v: &Value) -> Option<(f64, f64, f64)> {
    match v {
        Value::Record(m) => Some((
            rec_num(m, "x")?,
            rec_num(m, "y")?,
            rec_num(m, "z").unwrap_or(0.0),
        )),
        _ => None,
    }
}

fn xyz_tuple(v: &Value) -> (f64, f64, f64) {
    xyz_of(v).unwrap_or((0.5, 0.5, 0.0))
}

fn rec_num(m: &std::collections::BTreeMap<String, Value>, k: &str) -> Option<f64> {
    m.get(k).and_then(args::as_f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_emits_nodes_and_contract() {
        let snap = PoetSnapshot::with_demo_seed();
        match scene(&snap, &named_kind("map"), Span { start: 0, end: 0 }).unwrap() {
            Value::Record(r) => {
                assert_eq!(
                    r.get("contract"),
                    Some(&Value::String(
                        "webizen_render::scene_contract::RenderScene".into()
                    ))
                );
                match r.get("nodes") {
                    Some(Value::List(xs)) => assert!(xs.len() >= 3),
                    other => panic!("{other:?}"),
                }
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn social_uses_graph_size() {
        let snap = PoetSnapshot::with_demo_seed();
        match scene(&snap, &named_kind("social"), Span { start: 0, end: 0 }).unwrap() {
            Value::Record(r) => match r.get("nodes") {
                Some(Value::List(xs)) => assert!(xs.len() >= 3),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn health_is_not_a_named_person() {
        let snap = PoetSnapshot::with_demo_seed();
        match scene(&snap, &named_kind("health"), Span { start: 0, end: 0 }).unwrap() {
            Value::Record(r) => match r.get("note") {
                Some(Value::String(s)) => assert!(s.contains("not a named person")),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }
}
