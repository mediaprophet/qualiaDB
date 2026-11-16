use super::*;

#[wasm_bindgen]
impl QualiaPortal {
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
        let spans = Array::new();
        for i in 0..organs.length() {
            let organ = organs.get(i);
            let before = accum.positions.len() as u32;
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
            let first_index = accum.indices.len() as u32;
            accum.append_organ_10d(&bytes, [r, g, b, a]);
            let count = accum.positions.len() as u32 - before;
            if count > 0 {
                let id = Reflect::get(&organ, &JsValue::from_str("id"))
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();
                let span = js_sys::Object::new();
                Reflect::set(&span, &JsValue::from_str("id"), &JsValue::from_str(&id))?;
                Reflect::set(
                    &span,
                    &JsValue::from_str("vertex_start"),
                    &JsValue::from_f64(before as f64),
                )?;
                Reflect::set(
                    &span,
                    &JsValue::from_str("vertex_count"),
                    &JsValue::from_f64(count as f64),
                )?;
                Reflect::set(
                    &span,
                    &JsValue::from_str("first_index"),
                    &JsValue::from_f64(first_index as f64),
                )?;
                Reflect::set(
                    &span,
                    &JsValue::from_str("index_count"),
                    &JsValue::from_f64((accum.indices.len() as u32 - first_index) as f64),
                )?;
                spans.push(&span);
            }
        }
        let result = self.finish_body_mesh_upload(accum)?;
        Reflect::set(&result, &JsValue::from_str("parts"), &spans)?;
        self.present_misses = 0;
        Ok(result)
    }
}
