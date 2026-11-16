use super::*;

#[wasm_bindgen]
impl QualiaPortal {
    /// Bind exact prototype triangle spans to disjoint native instance-record spans.
    pub fn set_mesh_instance_groups(
        &mut self,
        abi_version: u32,
        descriptors: &[u32],
        packed_records: &[u8],
    ) -> Result<JsValue, JsValue> {
        if !self.preserve_authored_frame {
            return Err(JsValue::from_str(
                "prototype groups require the preserved authored scene frame",
            ));
        }
        if abi_version != crate::render::instance_culling::GPU_INSTANCE_RECORD_ABI_VERSION {
            return Err(JsValue::from_str(
                "mesh_instance_record_abi_version_unsupported",
            ));
        }
        let gpu = self
            .gpu
            .as_mut()
            .ok_or_else(|| JsValue::from_str("native GPU instance groups unavailable"))?;
        gpu.set_mesh_instance_groups_packed(descriptors, packed_records)
            .map_err(|e| JsValue::from_str(&e))?;
        let (groups, source, visible, triangles) = gpu.mesh_instance_group_counts();
        let receipt = js_sys::Object::new();
        for (key, value) in [
            ("prototype_groups", groups as f64),
            ("source_instances", source as f64),
            ("visible_instances", visible as f64),
            ("expanded_triangles", triangles as f64),
        ] {
            Reflect::set(&receipt, &key.into(), &JsValue::from_f64(value))?;
        }
        Reflect::set(
            &receipt,
            &"frame".into(),
            &"preserved authored coordinates".into(),
        )?;
        Ok(receipt.into())
    }
}
