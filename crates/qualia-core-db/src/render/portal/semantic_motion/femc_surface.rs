//! Resident FEMC surface staging in the original mesh's orbit frame.

use wasm_bindgen::JsValue;

use crate::{
    etalon_render::surface_frames::tangent::tangents_from_uvs_into,
    render::{
        mesh_surface_frames::vertex_normals_from_indices_into,
        portal_load_10d::{decode_load_10d_in_frame, Load10dGpuBuffers},
        skeletal::semantic_motion::{FemcState, PreparedFemcMotion, SemanticMotionError},
    },
};

use super::motion_error;

#[derive(Clone)]
pub(super) struct SurfaceFrames {
    pub(super) positions: Vec<[f32; 3]>,
    pub(super) normals: Vec<[f32; 3]>,
    pub(super) tangents: Vec<[f32; 4]>,
}

pub(super) struct FemcSurface {
    rest: SurfaceFrames,
    pub(super) posed: SurfaceFrames,
    rest_binding: Vec<[f32; 3]>,
    deformed: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    indices: Vec<u32>,
    tangent_scratch: Vec<[f32; 6]>,
    scale: f32,
}

impl FemcSurface {
    pub(super) fn bind(asset: &[u8], raw: Load10dGpuBuffers, vertex_count: u32, controller: &PreparedFemcMotion) -> Result<Self, JsValue> {
        if raw.vertex_count != vertex_count {
            return Err(motion_error(SemanticMotionError::UnboundTarget));
        }
        let count = raw.positions.len();
        let mut rest_binding = vec![[0.0; 3]; count];
        controller.rest_surface(&raw.positions, &mut rest_binding).map_err(motion_error)?;
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for point in &raw.positions {
            for axis in 0..3 {
                min[axis] = min[axis].min(point[axis]);
                max[axis] = max[axis].max(point[axis]);
            }
        }
        let scale = 1.6 / (0..3).map(|axis| max[axis] - min[axis]).fold(1e-6, f32::max);
        let baked = decode_load_10d_in_frame(asset, true).map_err(|error| JsValue::from_str(&error))?;
        let uv = raw.uv0.unwrap_or_else(|| vec![[0.0; 2]; count]);
        let mut tangent_scratch = vec![[0.0; 6]; count];
        let normals = if let Some(normals) = raw.normals {
            normals
        } else {
            let mut normals = vec![[0.0; 3]; count];
            vertex_normals_from_indices_into(&baked.positions, &raw.indices, &mut normals)
                .map_err(|error| JsValue::from_str(&format!("FEMC source normals: {error:?}")))?;
            normals
        };
        let tangents = if let Some(tangents) = raw.tangents {
            tangents
        } else {
            let mut tangents = vec![[0.0; 4]; count];
            tangents_from_uvs_into(&baked.positions, &uv, &normals, &raw.indices, &mut tangents, &mut tangent_scratch)
                .map_err(|error| JsValue::from_str(&format!("FEMC source tangents: {error:?}")))?;
            tangents
        };
        let rest = SurfaceFrames { positions: baked.positions, normals, tangents };
        let posed = rest.clone();
        Ok(Self { rest, posed, rest_binding, deformed: vec![[0.0; 3]; count], uv, indices: raw.indices, tangent_scratch, scale })
    }

    pub(super) fn deform(&mut self, controller: &PreparedFemcMotion, state: &FemcState) -> Result<(), JsValue> {
        controller.deform(state, &mut self.deformed).map_err(motion_error)?;
        for i in 0..self.posed.positions.len() {
            for axis in 0..3 {
                let delta = self.deformed[i][axis] - self.rest_binding[i][axis];
                self.posed.positions[i][axis] = self.rest.positions[i][axis] + self.scale * delta;
            }
        }
        vertex_normals_from_indices_into(&self.posed.positions, &self.indices, &mut self.posed.normals)
            .map_err(|error| JsValue::from_str(&format!("FEMC deformed normals: {error:?}")))?;
        tangents_from_uvs_into(&self.posed.positions, &self.uv, &self.posed.normals, &self.indices,
            &mut self.posed.tangents, &mut self.tangent_scratch,
        ).map_err(|error| JsValue::from_str(&format!("FEMC deformed tangents: {error:?}")))?;
        Ok(())
    }

    pub(super) fn reset(&mut self) {
        self.posed.positions.copy_from_slice(&self.rest.positions);
        self.posed.normals.copy_from_slice(&self.rest.normals);
        self.posed.tangents.copy_from_slice(&self.rest.tangents);
    }
}
