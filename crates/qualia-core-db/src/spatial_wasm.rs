//! Spatial + tensor WASM exports for the Qualia portal (browser hot path).

#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::gpu_context::{global_vram_ledger, record_bake_pulse, sample_ambient_telemetry};
#[cfg(target_arch = "wasm32")]
use crate::q_hash;
#[cfg(target_arch = "wasm32")]
use crate::tensor::buffer_export::write_tensor_buffer;
use crate::tensor::Tensor10D;

// geometry_wasm.rs lives at the crate root (sibling of this file), not under spatial_wasm/.
// Needs `specialized_libs` (wasm-scientific / native) — not available on portal-slim.
#[cfg(all(target_arch = "wasm32", feature = "wasm-scientific"))]
#[path = "geometry_wasm.rs"]
pub mod geometry_wasm;
#[cfg(all(target_arch = "wasm32", feature = "wasm-scientific"))]
pub use geometry_wasm::*;

#[cfg(target_arch = "wasm32")]
const MAX_ENCODE_VERTICES: usize = 8192;

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct SpatialEncodeRequest {
    #[serde(rename = "type")]
    pub geo_type: String,
    pub detail: u32,
}

#[cfg(target_arch = "wasm32")]
#[derive(Serialize)]
pub struct QuinJson {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub context: String,
    pub metadata: String,
    pub parity: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Serialize)]
pub struct SpatialEncodeResponse {
    pub vertex_count: usize,
    pub quin_count: usize,
    pub memory_kb: f64,
    pub quins: Vec<QuinJson>,
    pub backend: &'static str,
    pub tensor_bytes: usize,
}

#[cfg(target_arch = "wasm32")]
fn pack_coord(x: f32, y: f32, z: f32) -> u64 {
    let xi = (x * 1000.0).round() as i64 & 0xfffff;
    let yi = (y * 1000.0).round() as i64 & 0xfffff;
    let zi = (z * 1000.0).round() as i64 & 0xfffff;
    ((xi as u64) << 40) | ((yi as u64) << 20) | (zi as u64)
}

#[cfg(target_arch = "wasm32")]
fn hex_u64(v: u64) -> String {
    format!("0x{:016x}", v)
}

#[cfg(target_arch = "wasm32")]
fn sample_vertices(geo_type: &str, detail: u32) -> Vec<[f32; 3]> {
    let n = match geo_type {
        "icosahedron" => 12 + detail as usize * 20,
        "cube" => 8 + detail as usize * 12,
        "sphere" => (detail * 8 + 8) as usize * (detail * 6 + 6) as usize,
        "torus" => (detail * 8 + 8) as usize * (detail * 6 + 6) as usize,
        "knot" => ((detail * 32 + 32) as usize).min(MAX_ENCODE_VERTICES),
        _ => 12,
    }
    .min(MAX_ENCODE_VERTICES);

    let mut out = Vec::with_capacity(n);
    let phi = 1.618_033_988_75_f32;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let theta = t * std::f32::consts::TAU * 3.0;
        let r = 5.0 + (detail as f32 * 0.5);
        let x = r * theta.cos() * (t * phi).sin();
        let y = r * (t * 2.0).sin();
        let z = r * theta.sin() * (t * phi).cos();
        out.push([x, y, z]);
    }
    out
}

#[cfg(target_arch = "wasm32")]
fn vertices_to_tensors(verts: &[[f32; 3]]) -> Vec<Tensor10D> {
    let mut tensors = Vec::with_capacity(verts.len());
    for (i, [x, y, z]) in verts.iter().enumerate() {
        let nx = (*x / 10.0).clamp(-1.0, 1.0);
        let ny = (*y / 10.0).clamp(-1.0, 1.0);
        let nz = (*z / 10.0).clamp(-1.0, 1.0);
        let sigma = (i as f32 / verts.len() as f32).fract();
        let t_coord = i as f32 / verts.len().max(1) as f32;
        // Demo manifold fan-out (w) and sandbox spin (q) for Phase 1 PGA validation.
        let w = (i % 5) as f32;
        let q = if i % 4 == 0 {
            0.0
        } else {
            0.12 + (i % 6) as f32 * 0.06
        };
        // mu = 2 tags bilateral nodes for Phase 2c T_pull (EnforceBilateralMicroCommons).
        let mu = if i % 3 == 0 { 2.0 } else { 0.0 };
        // Phase 3 v-band demo: Euclidean / cyclic / hyperbolic / boundary clique mix.
        let v = match i % 4 {
            0 => 0.0,
            1 => 1.5,
            2 => 2.5,
            _ => 3.2,
        };
        tensors.push(Tensor10D::new(q, v, w, nx, ny, nz, t_coord, 1.0, mu, sigma));
    }
    tensors
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn spatial_encode_wasm(json: &str) -> Result<JsValue, JsValue> {
    let req: SpatialEncodeRequest =
        serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let verts = sample_vertices(&req.geo_type, req.detail);
    let geom_hash = q_hash(&format!("geo:{}:{}", req.geo_type, req.detail));
    let ctx_hash = q_hash("ctx:spatial-demo");
    let pred_vertex = q_hash("geo:hasVertex");

    let mut quins = Vec::with_capacity(verts.len());
    for (i, [x, y, z]) in verts.iter().enumerate() {
        let object = pack_coord(*x, *y, *z);
        let metadata = i as u64;
        let parity = geom_hash ^ pred_vertex ^ object ^ ctx_hash ^ metadata;
        quins.push(QuinJson {
            subject: hex_u64(geom_hash),
            predicate: hex_u64(pred_vertex),
            object: hex_u64(object),
            context: hex_u64(ctx_hash),
            metadata: hex_u64(metadata),
            parity: hex_u64(parity),
        });
    }

    let tensors = vertices_to_tensors(&verts);
    let tensor_bytes = crate::tensor::buffer_export::TensorBufferHeader::total_bytes(tensors.len());
    let ledger = global_vram_ledger();
    let fabric = crate::compute_universe::UniverseFabric::current(ledger);
    if !fabric.can_pin_tensor(ledger, tensor_bytes as u64) {
        return Err(JsValue::from_str("U1 tensor pin denied (VRAM ledger cap)"));
    }
    ledger.record_tensor(tensor_bytes as u64);
    record_bake_pulse();
    let loaded = crate::tensor::resident_substrate::global_resident_substrate()
        .load_from_tensors(&tensors, geom_hash)
        .unwrap_or(0);
    crate::tensor::kv_provenance::rebuild_prompt_provenance(loaded, loaded, 0);
    if let Some(anchor) = tensors.first() {
        crate::compute_universe::publish_query_tensor(*anchor, geom_hash);
    }
    for (i, [x, y, z]) in verts.iter().take(8).enumerate() {
        let _ = crate::compute_universe::push_tensor_context(
            crate::compute_universe::ContextInjectToken {
                tensor_index: i as u32,
                subject_hash: geom_hash,
                distance: (x * x + y * y + z * z).sqrt(),
                manifold_w: 0.0,
            },
        );
    }

    let resp = SpatialEncodeResponse {
        vertex_count: verts.len(),
        quin_count: quins.len(),
        memory_kb: ((quins.len() * 48) as f64 / 1024.0 * 100.0).round() / 100.0,
        quins,
        backend: "wasm",
        tensor_bytes,
    };
    serde_wasm_bindgen::to_value(&resp).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct GeosparqlRequest {
    #[serde(rename = "geoA")]
    pub geo_a: String,
    #[serde(rename = "geoB")]
    pub geo_b: String,
    pub op: String,
    #[serde(default = "default_crs")]
    pub crs: String,
}

#[cfg(target_arch = "wasm32")]
fn default_crs() -> String {
    "4326".to_string()
}

#[cfg(target_arch = "wasm32")]
fn parse_point(wkt: &str) -> Result<(f64, f64), String> {
    let s = wkt.trim();
    let inner = s
        .strip_prefix("POINT(")
        .and_then(|t| t.strip_suffix(')'))
        .ok_or("expected POINT(x y)")?;
    let mut parts = inner.split_whitespace();
    let x: f64 = parts
        .next()
        .ok_or("missing x")?
        .parse()
        .map_err(|e: std::num::ParseFloatError| e.to_string())?;
    let y: f64 = parts
        .next()
        .ok_or("missing y")?
        .parse()
        .map_err(|e: std::num::ParseFloatError| e.to_string())?;
    Ok((x, y))
}

#[cfg(target_arch = "wasm32")]
fn parse_polygon(wkt: &str) -> Result<Vec<(f64, f64)>, String> {
    let s = wkt.trim();
    let inner = s
        .strip_prefix("POLYGON((")
        .and_then(|t| t.strip_suffix("))"))
        .ok_or("expected POLYGON((...))")?;
    inner
        .split(',')
        .map(|pair| {
            let mut p = pair.trim().split_whitespace();
            let x: f64 = p
                .next()
                .unwrap()
                .parse()
                .map_err(|e: std::num::ParseFloatError| e.to_string())?;
            let y: f64 = p
                .next()
                .unwrap()
                .parse()
                .map_err(|e: std::num::ParseFloatError| e.to_string())?;
            Ok((x, y))
        })
        .collect()
}

#[cfg(target_arch = "wasm32")]
fn point_in_polygon(x: f64, y: f64, ring: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let mut j = ring.len().saturating_sub(1);
    for i in 0..ring.len() {
        let (xi, yi) = ring[i];
        let (xj, yj) = ring[j];
        let intersect = (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi + 1e-12) + xi;
        if intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn geosparql_operation_wasm(json: &str) -> Result<JsValue, JsValue> {
    let req: GeosparqlRequest =
        serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let poly = parse_polygon(&req.geo_a).map_err(|e| JsValue::from_str(&e))?;
    let (px, py) = parse_point(&req.geo_b).map_err(|e| JsValue::from_str(&e))?;
    let within = point_in_polygon(px, py, &poly);
    let dist = ((px - poly[0].0).powi(2) + (py - poly[0].1).powi(2)).sqrt();

    #[derive(Serialize)]
    struct GeoResult {
        operation: String,
        crs: String,
        geometry_a: String,
        geometry_b: String,
        result: serde_json::Value,
        predicate: String,
        elapsed_ms: f64,
        backend: &'static str,
    }

    let (result, predicate) = match req.op.as_str() {
        "within" => (serde_json::json!(within), "geo:sfWithin"),
        "contains" => (serde_json::json!(within), "geo:sfContains"),
        "intersects" => (serde_json::json!(within), "geo:sfIntersects"),
        "touches" => (serde_json::json!(false), "geo:sfTouches"),
        "overlaps" => (serde_json::json!(false), "geo:sfOverlaps"),
        "distance" => (
            serde_json::json!({ "value": dist, "unit": "coordinate-units" }),
            "geo:distance",
        ),
        other => return Err(JsValue::from_str(&format!("unknown op: {other}"))),
    };

    crate::gpu_context::record_logic_flash();
    let out = GeoResult {
        operation: req.op,
        crs: format!("EPSG:{}", req.crs),
        geometry_a: req.geo_a,
        geometry_b: req.geo_b,
        result,
        predicate: predicate.to_string(),
        elapsed_ms: 0.0,
        backend: "wasm",
    };
    serde_wasm_bindgen::to_value(&out).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn sample_browser_telemetry_wasm() -> Result<JsValue, JsValue> {
    let samples = sample_ambient_telemetry();
    #[derive(Serialize)]
    struct BrowserTelemetry {
        memory_pressure: f32,
        network_ripple: f32,
        baking_crystallization: f32,
        logic_flashes: f32,
        llm_heat: f32,
        quantum_activity: f32,
        spectral_shift: f32,
        temporal_pulse: f32,
        epistemic_density: f32,
        manifold_pressure: f32,
        operational_mode: u8,
    }
    let t = BrowserTelemetry {
        memory_pressure: samples[0],
        network_ripple: samples[1],
        baking_crystallization: samples[2],
        logic_flashes: samples[3],
        llm_heat: samples[4],
        quantum_activity: samples[5],
        spectral_shift: samples[6],
        temporal_pulse: samples[7],
        epistemic_density: samples[8],
        manifold_pressure: samples[9],
        operational_mode: global_vram_ledger().mode() as u8,
    };
    serde_wasm_bindgen::to_value(&t).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn export_tensor_slice_wasm(max_nodes: u32) -> Result<JsValue, JsValue> {
    use crate::tensor::buffer_export::write_tensor_slice_from_resident;
    use crate::tensor::resident_substrate::{global_resident_substrate, MAX_RESIDENT_NODES};

    let max = max_nodes as usize;
    if max == 0 || max > MAX_RESIDENT_NODES {
        return Err(JsValue::from_str("invalid max_nodes"));
    }
    let substrate = global_resident_substrate();
    let count = substrate.node_count() as usize;
    if count == 0 {
        return Err(JsValue::from_str("no resident tensor substrate"));
    }
    let export_n = count.min(max);
    let need = crate::tensor::buffer_export::TensorBufferHeader::total_bytes(export_n);
    let mut buf = vec![0u8; need];
    write_tensor_slice_from_resident(substrate, export_n, &mut buf)
        .map_err(|e| JsValue::from_str(e))?;
    let u8arr = js_sys::Uint8Array::new_with_length(need as u32);
    u8arr.copy_from(&buf);
    Ok(u8arr.into())
}

#[cfg(target_arch = "wasm32")]
fn tensors_from_encode_json(json: &str) -> Result<Vec<Tensor10D>, JsValue> {
    if json.contains("\"parts\"") {
        let doc: crate::design_encode::DesignDocument =
            serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        crate::design_encode::design_to_tensors(&doc)
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))
    } else {
        let req: SpatialEncodeRequest =
            serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let verts = sample_vertices(&req.geo_type, req.detail);
        Ok(vertices_to_tensors(&verts))
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Serialize)]
struct DesignEncodeWasmResponse {
    part_count: usize,
    relation_count: usize,
    tensor_count: usize,
    quin_count: usize,
    design_hash: String,
    tensor_bytes: usize,
    quins: Vec<QuinJson>,
    backend: &'static str,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn design_encode_wasm(json: &str) -> Result<JsValue, JsValue> {
    let doc: crate::design_encode::DesignDocument =
        serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let tensors = crate::design_encode::design_to_tensors(&doc)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
    let quins_raw = crate::design_encode::design_to_quins(&doc)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;

    let design_hash = crate::design_encode::design_context_hash(&doc);
    let geom_hash = design_hash;

    let mut quins_json = Vec::with_capacity(quins_raw.len());
    for q in &quins_raw {
        quins_json.push(QuinJson {
            subject: hex_u64(q.subject),
            predicate: hex_u64(q.predicate),
            object: hex_u64(q.object),
            context: hex_u64(q.context),
            metadata: hex_u64(q.metadata),
            parity: hex_u64(q.parity),
        });
    }

    let tensor_bytes = crate::tensor::buffer_export::TensorBufferHeader::total_bytes(tensors.len());
    let ledger = global_vram_ledger();
    let fabric = crate::compute_universe::UniverseFabric::current(ledger);
    if !fabric.can_pin_tensor(ledger, tensor_bytes as u64) {
        return Err(JsValue::from_str("U1 tensor pin denied (VRAM ledger cap)"));
    }
    ledger.record_tensor(tensor_bytes as u64);
    record_bake_pulse();
    let loaded = crate::tensor::resident_substrate::global_resident_substrate()
        .load_from_tensors(&tensors, geom_hash)
        .unwrap_or(0);
    crate::tensor::kv_provenance::rebuild_prompt_provenance(loaded, loaded, 0);
    if let Some(anchor) = tensors.first() {
        crate::compute_universe::publish_query_tensor(*anchor, geom_hash);
    }
    for (i, t) in tensors.iter().take(8).enumerate() {
        let _ = crate::compute_universe::push_tensor_context(
            crate::compute_universe::ContextInjectToken {
                tensor_index: i as u32,
                subject_hash: geom_hash,
                distance: (t.x * t.x + t.y * t.y + t.z * t.z).sqrt(),
                manifold_w: t.w,
            },
        );
    }

    let resp = DesignEncodeWasmResponse {
        part_count: doc.parts.len(),
        relation_count: doc.relations.len(),
        tensor_count: tensors.len(),
        quin_count: quins_raw.len(),
        design_hash: hex_u64(design_hash),
        tensor_bytes,
        quins: quins_json,
        backend: "wasm-design",
    };
    serde_wasm_bindgen::to_value(&resp).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn export_tensor_buffer_wasm(json: &str) -> Result<JsValue, JsValue> {
    let tensors = tensors_from_encode_json(json)?;
    let need = crate::tensor::buffer_export::TensorBufferHeader::total_bytes(tensors.len());
    let mut buf = vec![0u8; need];
    write_tensor_buffer(&tensors, &mut buf).map_err(|e| JsValue::from_str(e))?;
    record_bake_pulse();
    let u8arr = js_sys::Uint8Array::new_with_length(need as u32);
    u8arr.copy_from(&buf);
    Ok(u8arr.into())
}
