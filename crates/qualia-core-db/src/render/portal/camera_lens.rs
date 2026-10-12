use super::*;
#[wasm_bindgen]
impl QualiaPortal {
    /// Shared native lens state for hardware rendering and CPU picking.
    pub fn set_camera_near_plane(&mut self,near:f32)->Result<(),JsValue> {
        if !near.is_finite()||!(0.00001..=1.0).contains(&near) {
            return Err(JsValue::from_str("camera near plane outside admitted range"));
        }
        self.camera.near_plane=near;
        #[cfg(target_arch="wasm32")]
        if let Some(gpu)=self.gpu.as_mut() {gpu.set_camera_near_plane(near);}
        Ok(())
    }
}
