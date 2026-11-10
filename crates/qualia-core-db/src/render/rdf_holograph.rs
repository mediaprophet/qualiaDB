//! RDF / NQuin → holographic semantic-graph layout.
//!
//! Hot path is zero-heap: stack `QuinCollector`, open-addressed id table, and a
//! caller-owned [`RdfHolographScene`] of fixed arrays. GPU residency (SSBO /
//! RGBA32F) is the larger graphics budget; this file does not `Vec` / `BTreeMap`
//! / `String` on layout, atlas pack, or framing.
//!
//! Projects 48-byte NQuins into the 96-byte [`HolographicNodeGpu`] contract used by
//! the Holographic 3D Engine (WebGPU SSBO) and the WebGL2 GLSL ES 300 theatre
//! (RGBA32F node texture, 6 texels per node). No new `.10d` / `.emf` magics.

use std::io::Cursor;

use crate::render::gpu::holographic::{HolographicNodeGpu, HolographicPrimitive};
use crate::sparql_library::rdf_formats::{parse_rdf, QuinCollector, RdfFormat};
use crate::{q_hash, NQuin};

/// Six RGBA32F texels pack one 96-byte holographic node.
pub const HOLOGRAPH_TEXELS_PER_NODE: usize = 6;
/// Mobile-safe instance cap for the WebGL2 RGBA32F atlas (height = node count).
pub const MAX_HOLOGRAPH_NODES: usize = 2048;
const ID_HASH_CAP: usize = 4096;
const ID_HASH_EMPTY: u16 = u16::MAX;

const RDF_TYPE: u64 = q_hash("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");

const COLOR_CLASS: [f32; 4] = [0.52, 0.68, 1.0, 1.0];
const COLOR_INSTANCE: [f32; 4] = [0.28, 0.88, 1.0, 1.0];
const COLOR_LITERAL: [f32; 4] = [1.0, 0.78, 0.32, 1.0];
const COLOR_TYPE_EDGE: [f32; 4] = [1.0, 0.86, 0.34, 1.0];

/// One labelled holographic entity after 10D → 3D projection.
#[derive(Debug, Clone, Copy)]
pub struct RdfHolographLabel {
    pub id: u64,
    pub kind: &'static str,
    pub degree: u32,
    pub position: [f32; 3],
    pub name: [u8; 24],
    pub name_len: u8,
}

impl Default for RdfHolographLabel {
    fn default() -> Self {
        Self {
            id: 0,
            kind: "",
            degree: 0,
            position: [0.0; 3],
            name: [0; 24],
            name_len: 0,
        }
    }
}

impl RdfHolographLabel {
    fn with_kind(id: u64, kind: &'static str, degree: u32, position: [f32; 3]) -> Self {
        Self {
            id,
            kind,
            degree,
            position,
            name: [0; 24],
            name_len: 0,
        }
    }

    /// UTF-8 display name filled at ingest; empty when unknown.
    pub fn name_str(&self) -> &str {
        let n = self.name_len as usize;
        core::str::from_utf8(&self.name[..n.min(24)]).unwrap_or("")
    }

    /// Copy a display name. Ingest only; ignored when already set.
    pub fn set_name(&mut self, s: &str) {
        if self.name_len != 0 || s.is_empty() {
            return;
        }
        let b = s.as_bytes();
        let n = b.len().min(24);
        self.name[..n].copy_from_slice(&b[..n]);
        self.name_len = n as u8;
    }
}

/// Laid-out holographic RDF scene. Fixed arrays; caller owns residency.
pub struct RdfHolographScene {
    nodes: [HolographicNodeGpu; MAX_HOLOGRAPH_NODES],
    labels: [RdfHolographLabel; MAX_HOLOGRAPH_NODES],
    node_count: u16,
    label_count: u16,
    pub triple_count: u32,
    pub truncated: bool,
}

impl RdfHolographScene {
    /// Zeroed resident scene. One-time fill; not a per-frame allocation.
    pub fn empty() -> Self {
        Self {
            nodes: [HolographicNodeGpu::default(); MAX_HOLOGRAPH_NODES],
            labels: [RdfHolographLabel::default(); MAX_HOLOGRAPH_NODES],
            node_count: 0,
            label_count: 0,
            triple_count: 0,
            truncated: false,
        }
    }

    /// Heap-resident scene for WASM portal construction (avoids a 261 KiB stack frame).
    pub fn empty_box() -> Box<Self> {
        Box::new(Self::empty())
    }

    /// Drops occupancy; keeps the resident arrays.
    pub fn clear(&mut self) {
        self.node_count = 0;
        self.label_count = 0;
        self.triple_count = 0;
        self.truncated = false;
    }

    #[inline]
    pub fn node_count(&self) -> usize {
        self.node_count as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.node_count == 0
    }

    #[inline]
    pub fn nodes(&self) -> &[HolographicNodeGpu] {
        &self.nodes[..self.node_count as usize]
    }

    #[inline]
    pub fn labels(&self) -> &[RdfHolographLabel] {
        &self.labels[..self.label_count as usize]
    }

    /// Mutable labels for ingest name fill. Not a paint-path API.
    #[inline]
    pub fn labels_mut(&mut self) -> &mut [RdfHolographLabel] {
        &mut self.labels[..self.label_count as usize]
    }

    #[inline]
    fn push_node(&mut self, node: HolographicNodeGpu, label: RdfHolographLabel) -> bool {
        let i = self.node_count as usize;
        if i >= MAX_HOLOGRAPH_NODES {
            self.truncated = true;
            return false;
        }
        self.nodes[i] = node;
        self.labels[i] = label;
        self.node_count += 1;
        self.label_count += 1;
        true
    }
}

fn unit(id: u64, salt: u64) -> f32 {
    let x = id.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(salt);
    (x >> 40) as f32 / 16_777_215.0
}

/// 10D address [q,v,w,x,y,z,t,α,μ,σ] folded to a cinematic 3-space with context as height.
pub fn project_10d(id: u64, context: u64, degree: u32) -> [f32; 3] {
    let u = unit(id, 0xA11CE);
    let v = unit(id, 0xB0B);
    let w = unit(context, 0xC0DE);
    let theta = u * std::f32::consts::TAU;
    let phi = (2.0 * v - 1.0).clamp(-1.0, 1.0).acos();
    let radius = 2.15 + (degree as f32 + 1.0).ln() * 0.28 + w * 0.35;
    let x = radius * phi.sin() * theta.cos();
    let y = radius * phi.cos() * 0.58 + (w - 0.5) * 0.70;
    let z = radius * phi.sin() * theta.sin();
    [x, y, z]
}

fn cool_predicate_color(pred: u64) -> [f32; 4] {
    let t = unit(pred, 0xC01A);
    [
        0.18 + t * 0.22,
        0.55 + unit(pred, 0x5EED) * 0.35,
        0.82 + unit(pred, 0xF00D) * 0.16,
        1.0,
    ]
}

struct IdTable {
    slot: [u16; ID_HASH_CAP],
    id: [u64; MAX_HOLOGRAPH_NODES],
    degree: [u32; MAX_HOLOGRAPH_NODES],
    context: [u64; MAX_HOLOGRAPH_NODES],
    flags: [u8; MAX_HOLOGRAPH_NODES],
    count: u16,
}

impl IdTable {
    fn new() -> Self {
        Self {
            slot: [ID_HASH_EMPTY; ID_HASH_CAP],
            id: [0; MAX_HOLOGRAPH_NODES],
            degree: [0; MAX_HOLOGRAPH_NODES],
            context: [0; MAX_HOLOGRAPH_NODES],
            flags: [0; MAX_HOLOGRAPH_NODES],
            count: 0,
        }
    }

    fn lookup_or_insert(&mut self, id: u64) -> Option<usize> {
        let mut h = (id as usize) & (ID_HASH_CAP - 1);
        for _ in 0..ID_HASH_CAP {
            let s = self.slot[h];
            if s == ID_HASH_EMPTY {
                if self.count as usize >= MAX_HOLOGRAPH_NODES {
                    return None;
                }
                let i = self.count as usize;
                self.slot[h] = self.count;
                self.id[i] = id;
                self.count += 1;
                return Some(i);
            }
            if self.id[s as usize] == id {
                return Some(s as usize);
            }
            h = (h + 1) & (ID_HASH_CAP - 1);
        }
        None
    }

    fn get(&self, id: u64) -> Option<usize> {
        let mut h = (id as usize) & (ID_HASH_CAP - 1);
        for _ in 0..ID_HASH_CAP {
            let s = self.slot[h];
            if s == ID_HASH_EMPTY {
                return None;
            }
            if self.id[s as usize] == id {
                return Some(s as usize);
            }
            h = (h + 1) & (ID_HASH_CAP - 1);
        }
        None
    }
}

const FLAG_SUBJECT: u8 = 1;
const FLAG_CLASS: u8 = 2;

fn kind_of(flags: u8, degree: u32) -> (&'static str, HolographicPrimitive, [f32; 4], f32) {
    if flags & FLAG_SUBJECT == 0 && flags & FLAG_CLASS == 0 {
        ("literal", HolographicPrimitive::RoundedBox, COLOR_LITERAL, 0.20)
    } else if flags & FLAG_CLASS != 0 || degree >= 6 {
        (
            "class",
            HolographicPrimitive::RoundedBox,
            COLOR_CLASS,
            0.46 + (degree as f32).ln_1p() * 0.06,
        )
    } else {
        (
            "instance",
            HolographicPrimitive::Sphere,
            COLOR_INSTANCE,
            0.30 + (degree as f32).ln_1p() * 0.04,
        )
    }
}

/// Push overlapping resource crystals apart. Zero-heap Jacobi; n ≤ 2048.
fn separate_resources(scene: &mut RdfHolographScene, resource_count: usize) {
    let n = resource_count.min(scene.node_count as usize);
    for _ in 0..4 {
        for i in 0..n {
            for j in (i + 1)..n {
                let pi = scene.nodes[i].position_scale;
                let pj = scene.nodes[j].position_scale;
                let dx = pi[0] - pj[0];
                let dy = pi[1] - pj[1];
                let dz = pi[2] - pj[2];
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                let min_d = (pi[3] + pj[3]) * 1.18 + 0.22;
                if dist >= min_d {
                    continue;
                }
                let inv = ((min_d - dist) * 0.42) / dist.max(1e-4) * 0.5;
                scene.nodes[i].position_scale[0] += dx * inv;
                scene.nodes[i].position_scale[1] += dy * inv;
                scene.nodes[i].position_scale[2] += dz * inv;
                scene.nodes[j].position_scale[0] -= dx * inv;
                scene.nodes[j].position_scale[1] -= dy * inv;
                scene.nodes[j].position_scale[2] -= dz * inv;
                scene.labels[i].position = [
                    scene.nodes[i].position_scale[0],
                    scene.nodes[i].position_scale[1],
                    scene.nodes[i].position_scale[2],
                ];
                scene.labels[j].position = [
                    scene.nodes[j].position_scale[0],
                    scene.nodes[j].position_scale[1],
                    scene.nodes[j].position_scale[2],
                ];
            }
        }
    }
}

/// Layout NQuins into a caller-owned scene. No heap.
pub fn layout_nquins_into(quins: &[NQuin], scene: &mut RdfHolographScene) {
    scene.clear();
    scene.triple_count = quins.len() as u32;

    let mut table = IdTable::new();
    for q in quins {
        match table.lookup_or_insert(q.subject) {
            Some(i) => {
                table.degree[i] = table.degree[i].saturating_add(1);
                if table.context[i] == 0 {
                    table.context[i] = q.context;
                }
                table.flags[i] |= FLAG_SUBJECT;
            }
            None => scene.truncated = true,
        }
        match table.lookup_or_insert(q.object) {
            Some(i) => {
                table.degree[i] = table.degree[i].saturating_add(1);
                if table.context[i] == 0 {
                    table.context[i] = q.context;
                }
                if q.predicate == RDF_TYPE {
                    table.flags[i] |= FLAG_CLASS;
                }
            }
            None => scene.truncated = true,
        }
    }

    for i in 0..table.count as usize {
        if !scene.push_node(
            {
                let id = table.id[i];
                let deg = table.degree[i].max(1);
                let pos = project_10d(id, table.context[i], deg);
                let (kind, prim, color, scale) = kind_of(table.flags[i], deg);
                let mut node = HolographicNodeGpu::new(
                    pos,
                    scale,
                    [scale, scale, if kind == "literal" { scale * 1.8 } else { scale }],
                    prim,
                    0.35 + unit(id, 7) * 0.9,
                    0.85,
                    unit(id, 13) * std::f32::consts::TAU,
                    1.15,
                    unit(id, 0xE915),
                    color,
                );
                node.manifold_tensor[1] = manifold_code(kind, false);
                let _ = kind;
                node
            },
            {
                let id = table.id[i];
                let deg = table.degree[i].max(1);
                let pos = project_10d(id, table.context[i], deg);
                let (kind, _, _, _) = kind_of(table.flags[i], deg);
                RdfHolographLabel::with_kind(id, kind, deg, pos)
            },
        ) {
            break;
        }
    }

    separate_resources(scene, table.count as usize);

    for q in quins {
        let Some(si) = table.get(q.subject) else {
            continue;
        };
        let Some(oi) = table.get(q.object) else {
            continue;
        };
        let from = [
            scene.nodes[si].position_scale[0],
            scene.nodes[si].position_scale[1],
            scene.nodes[si].position_scale[2],
        ];
        let to = [
            scene.nodes[oi].position_scale[0],
            scene.nodes[oi].position_scale[1],
            scene.nodes[oi].position_scale[2],
        ];
        let mid = [
            (from[0] + to[0]) * 0.5,
            (from[1] + to[1]) * 0.5,
            (from[2] + to[2]) * 0.5,
        ];
        let dx = to[0] - from[0];
        let dy = to[1] - from[1];
        let dz = to[2] - from[2];
        let span = (dx * dx + dy * dy + dz * dz).sqrt().max(0.2);
        let is_type = q.predicate == RDF_TYPE;
        let color = if is_type {
            COLOR_TYPE_EDGE
        } else {
            cool_predicate_color(q.predicate)
        };
        let radius = if is_type { 0.10 } else { 0.062 };
        // Tight capsule AABB plus a halo pad. Fat isotropic padding drew ghost cubes.
        let pad = 0.42;
        let hx = dx.abs() * 0.5 + radius + pad;
        let hy = dy.abs() * 0.5 + radius + pad;
        let hz = dz.abs() * 0.5 + radius + pad;
        let mut node = HolographicNodeGpu::new(
            mid,
            span * 0.5,
            [hx.max(0.18), hy.max(0.18), hz.max(0.18)],
            HolographicPrimitive::InterferenceBridge,
            0.8 + unit(q.predicate, 3) * 1.4,
            1.0,
            (q.metadata & 0xFFFF_FFFF) as f32 * 0.0001,
            0.7,
            0.35,
            color,
        )
        .with_interference_bridge(to, 0.9);
        node.manifold_tensor[1] = manifold_code("predicate", is_type);
        if !scene.push_node(
            node,
            RdfHolographLabel::with_kind(q.predicate, "predicate", 1, mid),
        ) {
            break;
        }
    }
    recenter_on_resource_centroid(scene);

    let resource_end = (table.count as usize).min(scene.node_count as usize);
    for i in 0..resource_end {
        if scene.labels[i].kind != "class" {
            continue;
        }
        let pos = [
            scene.nodes[i].position_scale[0],
            scene.nodes[i].position_scale[1],
            scene.nodes[i].position_scale[2],
        ];
        let well = (scene.nodes[i].position_scale[3] * 2.2).max(1.4);
        let mut node = HolographicNodeGpu::new(
            pos,
            well,
            [well, well, well],
            HolographicPrimitive::Metaball,
            0.12,
            0.40,
            0.0,
            0.40,
            0.20,
            [0.16, 0.24, 0.55, 0.35],
        );
        node.manifold_tensor[1] = manifold_code("plane", false);
        if !scene.push_node(
            node,
            RdfHolographLabel::with_kind(
                scene.labels[i].id,
                "plane",
                scene.labels[i].degree,
                pos,
            ),
        ) {
            break;
        }
    }
}

/// Test helper: stack/box scene filled from quins.
pub fn layout_nquins(quins: &[NQuin]) -> Box<RdfHolographScene> {
    let mut scene = Box::new(RdfHolographScene::empty());
    layout_nquins_into(quins, &mut scene);
    scene
}

fn recenter_on_resource_centroid(scene: &mut RdfHolographScene) {
    let mut cx = 0.0f32;
    let mut cy = 0.0f32;
    let mut cz = 0.0f32;
    let mut n = 0.0f32;
    for i in 0..scene.label_count as usize {
        let label = &scene.labels[i];
        if label.kind == "predicate" || label.kind == "plane" {
            continue;
        }
        cx += label.position[0];
        cy += label.position[1];
        cz += label.position[2];
        n += 1.0;
    }
    if n <= 0.0 {
        return;
    }
    cx /= n;
    cy /= n;
    cz /= n;
    for i in 0..scene.node_count as usize {
        scene.nodes[i].position_scale[0] -= cx;
        scene.nodes[i].position_scale[1] -= cy;
        scene.nodes[i].position_scale[2] -= cz;
        scene.nodes[i].connection_endpoint[0] -= cx;
        scene.nodes[i].connection_endpoint[1] -= cy;
        scene.nodes[i].connection_endpoint[2] -= cz;
    }
    for i in 0..scene.label_count as usize {
        scene.labels[i].position[0] -= cx;
        scene.labels[i].position[1] -= cy;
        scene.labels[i].position[2] -= cz;
    }
}

/// Orbit distance that fits resource crystals in the vertical FOV.
/// Predicate capsules store half-span in `position_scale.w`; counting that as a
/// sphere radius pulled the camera so far the field collapsed to a cluster.
/// k=1.22 with a 4.2–16 clamp keeps a small constellation (FOAF/Star) in frame
/// instead of filling the viewport with a single glass close-up.
pub fn framing_zoom(scene: &RdfHolographScene) -> f32 {
    let mut max_r = 1.0f32;
    for (i, node) in scene.nodes().iter().enumerate() {
        if scene
            .labels()
            .get(i)
            .is_some_and(|l| l.kind == "predicate" || l.kind == "plane")
        {
            continue;
        }
        let r = (node.position_scale[0].hypot(node.position_scale[2]))
            .hypot(node.position_scale[1])
            + node.position_scale[3] * 1.05
            + 0.22;
        max_r = max_r.max(r);
    }
    (max_r / crate::render::camera::CAMERA_TAN_HALF_FOV * 1.22).clamp(4.2, 16.0)
}

/// Classify a resource for packing. Quoted lexical forms that never appear as
/// a subject are literals; rdf:type objects / high-degree hubs are classes;
/// remaining IRIs are instances even when they only appear as objects.
pub fn classify_resource_kind(
    is_subject: bool,
    is_class: bool,
    degree: u32,
    quoted_literal: bool,
) -> &'static str {
    if is_class || degree >= 6 {
        "class"
    } else if quoted_literal && !is_subject {
        "literal"
    } else {
        "instance"
    }
}

/// Intern a vibe/JS kind string to a `'static` label used on the hot path.
pub fn intern_kind(kind: &str) -> &'static str {
    match kind {
        "class" => "class",
        "literal" => "literal",
        "predicate" | "edge" | "filament" => "predicate",
        "plane" | "well" => "plane",
        _ => "instance",
    }
}

/// Shader `manifold_tensor[1]` code for an interned kind.
/// 0 instance, 1 class, 2 literal, 3 rdf:type filament, 4 property filament, 5 plane.
pub fn manifold_code(kind: &str, is_type: bool) -> f32 {
    match intern_kind(kind) {
        "class" => 1.0,
        "literal" => 2.0,
        "predicate" => {
            if is_type {
                3.0
            } else {
                4.0
            }
        }
        "plane" => 5.0,
        _ => 0.0,
    }
}

/// Local display token of an expanded IRI or literal lexical form.
pub fn display_local_name(lexical: &str) -> &str {
    lexical
        .rsplit(['#', '/'])
        .next()
        .unwrap_or(lexical)
        .split(':')
        .next_back()
        .unwrap_or(lexical)
}

/// Y-rotate a world point by the holographic W-angle.
pub fn rotate_y_w(p: [f32; 3], angle: f32) -> [f32; 3] {
    let c = angle.cos();
    let s = angle.sin();
    [p[0] * c - p[2] * s, p[1], p[0] * s + p[2] * c]
}

/// Rotate by W then project to NDC. `None` when clip.w is degenerate.
pub fn project_world_ndc(
    world: [f32; 3],
    view_projection: &[[f32; 4]; 4],
    w_angle: f32,
) -> Option<[f32; 3]> {
    let p = rotate_y_w(world, w_angle);
    let clip = mul_col(view_projection, [p[0], p[1], p[2], 1.0]);
    if clip[3].abs() < 1e-5 {
        return None;
    }
    Some([clip[0] / clip[3], clip[1] / clip[3], clip[2] / clip[3]])
}

/// NDC xy → canvas pixels (origin top-left).
pub fn ndc_to_canvas(ndc_x: f32, ndc_y: f32, canvas_w: f32, canvas_h: f32) -> [f32; 2] {
    [
        (ndc_x * 0.5 + 0.5) * canvas_w,
        (0.5 - ndc_y * 0.5) * canvas_h,
    ]
}

/// Fill empty label names at ingest. Already-set names win.
pub fn fill_label_names<'a>(
    scene: &mut RdfHolographScene,
    mut name_of: impl FnMut(u64) -> Option<&'a str>,
) {
    for label in scene.labels_mut() {
        if let Some(n) = name_of(label.id) {
            label.set_name(n);
        }
    }
}

/// One in-field name chip after camera projection. Caller owns the output slot.
#[derive(Debug, Clone, Copy)]
pub struct HolographOverlay {
    pub index: u16,
    pub kind: &'static str,
    pub sx: f32,
    pub sy: f32,
    pub name: [u8; 24],
    pub name_len: u8,
}

impl Default for HolographOverlay {
    fn default() -> Self {
        Self {
            index: 0,
            kind: "",
            sx: 0.0,
            sy: 0.0,
            name: [0; 24],
            name_len: 0,
        }
    }
}

impl HolographOverlay {
    pub fn name_str(&self) -> &str {
        let n = self.name_len as usize;
        core::str::from_utf8(&self.name[..n.min(24)]).unwrap_or("")
    }
}

/// Project named resource crystals to canvas pixels. Skips filaments and wells.
/// `out` is caller-owned; returns the number of chips written.
pub fn overlay_chips_into(
    scene: &RdfHolographScene,
    view_projection: &[[f32; 4]; 4],
    w_angle: f32,
    canvas_w: f32,
    canvas_h: f32,
    out: &mut [HolographOverlay],
) -> usize {
    if canvas_w < 1.0 || canvas_h < 1.0 || out.is_empty() {
        return 0;
    }
    let mut n = 0usize;
    for (i, label) in scene.labels().iter().enumerate() {
        if n >= out.len() {
            break;
        }
        if label.kind == "predicate" || label.kind == "plane" || label.name_len == 0 {
            continue;
        }
        let Some(ndc) = project_world_ndc(label.position, view_projection, w_angle) else {
            continue;
        };
        if ndc[0].abs() > 1.15 || ndc[1].abs() > 1.15 {
            continue;
        }
        let xy = ndc_to_canvas(ndc[0], ndc[1], canvas_w, canvas_h);
        let mut chip = HolographOverlay {
            index: i as u16,
            kind: label.kind,
            sx: xy[0],
            sy: xy[1],
            name: [0; 24],
            name_len: label.name_len,
        };
        let len = label.name_len as usize;
        chip.name[..len.min(24)].copy_from_slice(&label.name[..len.min(24)]);
        out[n] = chip;
        n += 1;
    }
    n
}

/// One vibe-placed holographic entity. Ingest may allocate the slice; packing
/// into [`RdfHolographScene`] stays on the caller-owned fixed arrays.
#[derive(Debug, Clone, Copy)]
pub struct PlacedHolograph {
    pub id: u64,
    pub kind: &'static str,
    pub degree: u32,
    pub position: [f32; 3],
    pub endpoint: [f32; 3],
    pub scale: f32,
    pub is_type: bool,
    pub name: [u8; 24],
    pub name_len: u8,
}

impl Default for PlacedHolograph {
    fn default() -> Self {
        Self {
            id: 0,
            kind: "instance",
            degree: 1,
            position: [0.0; 3],
            endpoint: [0.0; 3],
            scale: 0.30,
            is_type: false,
            name: [0; 24],
            name_len: 0,
        }
    }
}

impl PlacedHolograph {
    /// Fill the 24-byte display name from ingest.
    pub fn set_name(&mut self, s: &str) {
        if self.name_len != 0 || s.is_empty() {
            return;
        }
        let b = s.as_bytes();
        let n = b.len().min(24);
        self.name[..n].copy_from_slice(&b[..n]);
        self.name_len = n as u8;
    }
}

fn pack_placed(placed: &PlacedHolograph) -> (HolographicNodeGpu, RdfHolographLabel) {
    let kind = intern_kind(placed.kind);
    let scale = placed.scale.max(0.05);
    let pos = placed.position;
    let manifold = manifold_code(kind, placed.is_type);
    let (prim, color, half) = match kind {
        "class" => (
            HolographicPrimitive::RoundedBox,
            COLOR_CLASS,
            [scale, scale, scale],
        ),
        "literal" => (
            HolographicPrimitive::RoundedBox,
            COLOR_LITERAL,
            [scale, scale, scale * 1.8],
        ),
        "predicate" => {
            let color = if placed.is_type {
                COLOR_TYPE_EDGE
            } else {
                cool_predicate_color(placed.id)
            };
            (
                HolographicPrimitive::InterferenceBridge,
                color,
                [scale, scale, scale],
            )
        }
        "plane" => (
            HolographicPrimitive::Metaball,
            [0.16, 0.24, 0.55, 0.35],
            [scale, scale, scale],
        ),
        _ => (
            HolographicPrimitive::Sphere,
            COLOR_INSTANCE,
            [scale, scale, scale],
        ),
    };
    let mut node = if kind == "predicate" {
        let to = placed.endpoint;
        let from = [
            2.0 * pos[0] - to[0],
            2.0 * pos[1] - to[1],
            2.0 * pos[2] - to[2],
        ];
        let dx = to[0] - from[0];
        let dy = to[1] - from[1];
        let dz = to[2] - from[2];
        let span = (dx * dx + dy * dy + dz * dz).sqrt().max(0.2);
        let radius = if placed.is_type { 0.10 } else { 0.062 };
        let pad = 0.42;
        let hx = dx.abs() * 0.5 + radius + pad;
        let hy = dy.abs() * 0.5 + radius + pad;
        let hz = dz.abs() * 0.5 + radius + pad;
        HolographicNodeGpu::new(
            pos,
            span * 0.5,
            [hx.max(0.18), hy.max(0.18), hz.max(0.18)],
            prim,
            0.8 + unit(placed.id, 3) * 1.4,
            1.0,
            0.0,
            0.7,
            0.35,
            color,
        )
        .with_interference_bridge(to, 0.9)
    } else if kind == "plane" {
        HolographicNodeGpu::new(
            pos,
            scale,
            half,
            prim,
            0.12,
            0.40,
            0.0,
            0.40,
            0.20,
            color,
        )
    } else {
        HolographicNodeGpu::new(
            pos,
            scale,
            half,
            prim,
            0.35 + unit(placed.id, 7) * 0.9,
            0.85,
            unit(placed.id, 13) * std::f32::consts::TAU,
            1.15,
            unit(placed.id, 0xE915),
            color,
        )
    };
    node.manifold_tensor[1] = manifold;
    let mut label = RdfHolographLabel::with_kind(placed.id, kind, placed.degree.max(1), pos);
    if placed.name_len > 0 {
        let n = placed.name_len as usize;
        label.name[..n.min(24)].copy_from_slice(&placed.name[..n.min(24)]);
        label.name_len = placed.name_len;
    }
    (node, label)
}

/// Pack vibe-placed records into a caller-owned scene. No heap.
pub fn layout_placed_into(
    placed: &[PlacedHolograph],
    triple_count: u32,
    scene: &mut RdfHolographScene,
) {
    scene.clear();
    scene.triple_count = triple_count;
    for p in placed {
        let (node, label) = pack_placed(p);
        if !scene.push_node(node, label) {
            break;
        }
    }
    recenter_on_resource_centroid(scene);
}

fn mul_col(m: &[[f32; 4]; 4], v: [f32; 4]) -> [f32; 4] {
    [
        m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2] + m[3][0] * v[3],
        m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2] + m[3][1] * v[3],
        m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2] + m[3][2] * v[3],
        m[0][3] * v[0] + m[1][3] * v[1] + m[2][3] * v[2] + m[3][3] * v[3],
    ]
}

/// Screen-space pick of a resource crystal. Skips filaments and semantic planes.
/// `w_angle` matches the shader / overlay Y-rotation about W.
pub fn pick_resource_at(
    scene: &RdfHolographScene,
    view_projection: &[[f32; 4]; 4],
    w_angle: f32,
    canvas_x: f32,
    canvas_y: f32,
    canvas_w: f32,
    canvas_h: f32,
) -> i32 {
    if canvas_w < 1.0 || canvas_h < 1.0 {
        return -1;
    }
    let mut best = -1i32;
    let mut best_d = 44.0f32;
    for (i, node) in scene.nodes().iter().enumerate() {
        if scene
            .labels()
            .get(i)
            .is_some_and(|l| l.kind == "predicate" || l.kind == "plane")
        {
            continue;
        }
        let world = [
            node.position_scale[0],
            node.position_scale[1],
            node.position_scale[2],
        ];
        let Some(ndc) = project_world_ndc(world, view_projection, w_angle) else {
            continue;
        };
        if ndc[0].abs() > 1.35 || ndc[1].abs() > 1.35 {
            continue;
        }
        let xy = ndc_to_canvas(ndc[0], ndc[1], canvas_w, canvas_h);
        let dist = (xy[0] - canvas_x).hypot(xy[1] - canvas_y);
        if dist < best_d {
            best_d = dist;
            best = i as i32;
        }
    }
    best
}

/// Parse an RDF document into a caller-owned scene. Parse errors are static.
pub fn layout_rdf_into(
    format: RdfFormat,
    payload: &[u8],
    scene: &mut RdfHolographScene,
) -> Result<(), &'static str> {
    let mut collector = QuinCollector::new();
    parse_rdf(format, Cursor::new(payload), 0, &mut collector)
        .map_err(|_| "rdf_holograph_parse_failed")?;
    layout_nquins_into(collector.as_slice(), scene);
    scene.truncated = scene.truncated || collector.truncated;
    Ok(())
}

/// Parse by Content-Type or Qualia format id (`turtle`, `text/turtle`, …).
pub fn layout_rdf_bytes_into(
    content_type: &str,
    payload: &[u8],
    scene: &mut RdfHolographScene,
) -> Result<(), &'static str> {
    let format = RdfFormat::from_media_type(content_type)
        .or_else(|| RdfFormat::from_str(content_type))
        .ok_or("unsupported RDF Content-Type / format")?;
    layout_rdf_into(format, payload, scene)
}

/// Parse a UTF-8 RDF document by Content-Type or Qualia format id.
pub fn layout_rdf_str_into(
    content_type: &str,
    payload: &str,
    scene: &mut RdfHolographScene,
) -> Result<(), &'static str> {
    layout_rdf_bytes_into(content_type, payload.as_bytes(), scene)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::mul_col;

    fn triple(s: u64, p: u64, o: u64) -> NQuin {
        NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 7,
            metadata: 3,
            parity: NQuin::calculate_parity(s, p, o, 7, 3),
        }
    }

    #[test]
    fn empty_layout_is_empty() {
        let scene = layout_nquins(&[]);
        assert_eq!(scene.node_count(), 0);
        assert_eq!(scene.triple_count, 0);
        assert!(scene.nodes().is_empty());
    }

    #[test]
    fn one_triple_emits_two_resources_and_one_bridge() {
        let scene = layout_nquins(&[triple(11, 22, 33)]);
        assert_eq!(scene.triple_count, 1);
        assert_eq!(scene.node_count(), 3);
        assert_eq!(
            scene.labels().iter().filter(|l| l.kind != "predicate").count(),
            2
        );
        assert_eq!(
            scene.labels().iter().filter(|l| l.kind == "predicate").count(),
            1
        );
        let prim = scene.nodes()[2].half_extents_primitive[3] as u32;
        assert_eq!(prim, HolographicPrimitive::InterferenceBridge as u32);
        assert!(scene
            .nodes()
            .iter()
            .all(|n| n.position_scale.iter().all(|v| v.is_finite())));
        let mut cx = 0.0f32;
        let mut cy = 0.0f32;
        let mut cz = 0.0f32;
        let mut n = 0.0f32;
        for label in scene.labels().iter().filter(|l| l.kind != "predicate") {
            cx += label.position[0];
            cy += label.position[1];
            cz += label.position[2];
            n += 1.0;
        }
        cx /= n;
        cy /= n;
        cz /= n;
        assert!(cx.abs() < 1e-5 && cy.abs() < 1e-5 && cz.abs() < 1e-5);
        let zoom = framing_zoom(&scene);
        assert!(zoom >= 4.2 && zoom <= 16.0);
        let packed: &[f32] = bytemuck::cast_slice(scene.nodes());
        assert_eq!(packed.len(), scene.node_count() * 24);
    }

    #[test]
    fn cinema_framing_places_resources_in_ndc() {
        let ttl = br#"@prefix foaf: <http://xmlns.com/foaf/0.1/> .
<http://ex/alice> foaf:knows <http://ex/bob> .
<http://ex/bob> foaf:knows <http://ex/carol> .
<http://ex/carol> foaf:knows <http://ex/alice> .
<http://ex/alice> foaf:name "Alice" .
"#;
        let mut scene = Box::new(RdfHolographScene::empty());
        layout_rdf_into(RdfFormat::Turtle, ttl, &mut scene).expect("turtle");
        let zoom = framing_zoom(&scene);
        let vp = crate::render::camera::orbit_view_projection_with_near(
            0.55,
            0.38,
            zoom,
            [0.0, 0.0, 0.0],
            16.0 / 9.0,
            0.05,
        );
        let mut in_frame = 0u32;
        for (i, node) in scene.nodes().iter().enumerate() {
            if scene.labels()[i].kind == "predicate" {
                let he = node.half_extents_primitive;
                let mid = node.position_scale;
                let end = node.connection_endpoint;
                assert!(
                    (end[0] - mid[0]).abs() <= he[0] + 0.02
                        && (end[1] - mid[1]).abs() <= he[1] + 0.02
                        && (end[2] - mid[2]).abs() <= he[2] + 0.02,
                    "bridge AABB must cover the capsule endpoint"
                );
                continue;
            }
            let clip = mul_col(&vp, [node.position_scale[0], node.position_scale[1], node.position_scale[2], 1.0]);
            assert!(clip[3].abs() > 1e-5, "clip.w");
            let ndc = [clip[0] / clip[3], clip[1] / clip[3], clip[2] / clip[3]];
            if ndc[0].abs() < 1.25 && ndc[1].abs() < 1.25 && (0.0..1.0).contains(&ndc[2]) {
                in_frame += 1;
            }
        }
        assert!(
            in_frame >= 3,
            "cinema framing must put resource crystals in frame (in_frame={in_frame}, zoom={zoom})"
        );
    }

    #[test]
    fn turtle_layout_parses_and_projects() {
        let ttl = br#"@prefix foaf: <http://xmlns.com/foaf/0.1/> .
<http://ex/alice> foaf:knows <http://ex/bob> .
<http://ex/bob> foaf:knows <http://ex/carol> .
"#;
        let mut scene = Box::new(RdfHolographScene::empty());
        layout_rdf_into(RdfFormat::Turtle, ttl, &mut scene).expect("turtle");
        assert!(scene.triple_count >= 2);
        assert!(scene.node_count() >= 5);
        let packed: &[f32] = bytemuck::cast_slice(scene.nodes());
        assert_eq!(packed.len(), scene.node_count() * HOLOGRAPH_TEXELS_PER_NODE * 4);
    }

    #[test]
    fn class_hub_emits_semantic_plane() {
        let ttl = br#"@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
<http://ex/n> rdf:type <http://ex/Core> .
"#;
        let mut scene = Box::new(RdfHolographScene::empty());
        layout_rdf_into(RdfFormat::Turtle, ttl, &mut scene).expect("turtle");
        assert!(scene.labels().iter().any(|l| l.kind == "class"));
        assert_eq!(
            scene.labels().iter().filter(|l| l.kind == "plane").count(),
            1
        );
        let plane = scene
            .nodes()
            .iter()
            .zip(scene.labels())
            .find(|(_, l)| l.kind == "plane")
            .unwrap()
            .0;
        assert!((plane.manifold_tensor[1] - 5.0).abs() < 1e-5);
        assert!(plane.position_scale[3] >= 1.4);
    }

    #[test]
    fn project_10d_is_stable_and_bounded() {
        let a = project_10d(42, 1, 3);
        let b = project_10d(42, 1, 3);
        assert_eq!(a, b);
        assert!(a.iter().all(|v| v.is_finite() && v.abs() < 20.0));
        let c = project_10d(42, 99, 3);
        assert_ne!(a[1], c[1]);
    }

    #[test]
    fn layout_nquins_into_reuses_resident_scene() {
        let mut scene = Box::new(RdfHolographScene::empty());
        layout_nquins_into(&[triple(1, 2, 3)], &mut scene);
        let first = scene.node_count();
        layout_nquins_into(&[triple(4, 5, 6), triple(6, 7, 4)], &mut scene);
        assert_eq!(first, 3);
        // 4 and 6 are resources; two predicates. 7 is a predicate, not a node.
        assert_eq!(scene.node_count(), 4);
        assert_eq!(scene.triple_count, 2);
    }

    #[test]
    fn layout_placed_into_packs_named_triangle() {
        let mut a = PlacedHolograph {
            id: 1,
            kind: "instance",
            degree: 2,
            position: [1.0, 0.0, 0.0],
            scale: 0.30,
            ..PlacedHolograph::default()
        };
        a.set_name("alice");
        let mut b = PlacedHolograph {
            id: 2,
            kind: "instance",
            degree: 2,
            position: [-0.5, 0.0, 0.87],
            scale: 0.30,
            ..PlacedHolograph::default()
        };
        b.set_name("bob");
        let mut e = PlacedHolograph {
            id: 9,
            kind: "predicate",
            degree: 1,
            position: [0.25, 0.0, 0.435],
            endpoint: [-0.5, 0.0, 0.87],
            scale: 0.05,
            ..PlacedHolograph::default()
        };
        e.set_name("knows");
        let mut scene = Box::new(RdfHolographScene::empty());
        layout_placed_into(&[a, b, e], 1, &mut scene);
        assert_eq!(scene.node_count(), 3);
        assert_eq!(scene.triple_count, 1);
        assert_eq!(scene.labels()[0].name_str(), "alice");
        assert_eq!(scene.labels()[1].name_str(), "bob");
        assert_eq!(scene.labels()[2].kind, "predicate");
        let pa = scene.labels()[0].position;
        let pb = scene.labels()[1].position;
        let dx = pa[0] - pb[0];
        let dz = pa[2] - pb[2];
        assert!(dx * dx + dz * dz > 0.5, "placed resources stay apart");
        let zoom = framing_zoom(&scene);
        assert!(zoom >= 4.2 && zoom <= 16.0);
        let vp = crate::render::camera::orbit_view_projection_with_near(
            0.55,
            0.48,
            zoom,
            [0.0, 0.0, 0.0],
            16.0 / 9.0,
            0.05,
        );
        let mut chips = [HolographOverlay::default(); 8];
        let n = overlay_chips_into(&scene, &vp, 0.0, 1280.0, 720.0, &mut chips);
        assert_eq!(n, 2);
        assert_eq!(chips[0].name_str(), "alice");
        assert_eq!(intern_kind("filament"), "predicate");
        assert_eq!(intern_kind("well"), "plane");
        assert_eq!(
            classify_resource_kind(false, false, 1, true),
            "literal"
        );
        assert_eq!(
            classify_resource_kind(false, false, 1, false),
            "instance"
        );
        assert_eq!(classify_resource_kind(false, true, 1, false), "class");
        assert_eq!(display_local_name("http://ex/alice"), "alice");
        assert_eq!(manifold_code("class", false), 1.0);
        assert_eq!(manifold_code("filament", true), 3.0);
        assert_eq!(manifold_code("well", false), 5.0);
        let hit = pick_resource_at(&scene, &vp, 0.0, chips[0].sx, chips[0].sy, 1280.0, 720.0);
        assert_eq!(hit, chips[0].index as i32);
        fill_label_names(&mut scene, |_| Some("skip-set"));
        assert_eq!(scene.labels()[0].name_str(), "alice");
        fill_label_names(&mut scene, |id| if id == 9 { Some("knows") } else { None });
        assert_eq!(scene.labels()[2].name_str(), "knows");
    }
}
