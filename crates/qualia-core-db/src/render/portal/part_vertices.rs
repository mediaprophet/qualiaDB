//! Allocation-free authored-span updates, including compact replay lineage.
//! Extracted from the portal facade while repairing compatibility rendering.
use super::QualiaPortal;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl QualiaPortal {
    /// Write posed XYZ triples into an authored scene span. Compact backends
    /// receive the replay mesh's mapped positions, never original offsets.
    pub fn write_part_vertices(&mut self, start: u32, xyz: &[f32]) {
        let Ok(positions) = bytemuck::try_cast_slice::<f32, [f32; 3]>(xyz) else { return; };
        if positions.is_empty() || positions.iter().flatten().any(|v| !v.is_finite()) {
            return;
        }
        if let Some(body) = self.cpu_body.as_mut() {
            body.update_authored_span(start, positions);
        }
        let (start, positions) = if self.scene_animation.compact_gpu {
            let Some(body) = self.cpu_body.as_ref() else { return; };
            (0, body.positions.as_slice())
        } else {
            (start, positions)
        };
        #[cfg(target_arch = "wasm32")]
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.write_mesh_vertices(start, positions);
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(gl) = self.anatomy_webgl2.as_mut() {
            gl.write_vertices(start, positions);
        }
    }
}
