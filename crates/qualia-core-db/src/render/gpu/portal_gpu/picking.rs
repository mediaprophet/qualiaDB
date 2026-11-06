//! Semantic draw adaptation for canonical GPU picking ownership.
use super::super::{picking, PortalGpu};
impl PortalGpu {
    pub fn queue_pick(&mut self, x: f32, y: f32) {
        self.pick_readback.queue(x, y, self.width, self.height);
        self.pick_semantic_result = None;
    }
    pub fn poll_pick_readback(&mut self) -> Option<u32> {
        if self.is_device_lost() {
            self.pick_readback.notify_device_lost();
            self.pick_semantic_result = None;
            return None;
        }
        match self.pick_readback.poll(&self.device)? {
            etalon_render::picking::PickResult::Tensor(index) => Some(index),
            etalon_render::picking::PickResult::Mesh(id) => {
                self.pick_semantic_result = Some(id);
                None
            }
            _ => None,
        }
    }
    /// Return the full semantic identity from the most recently completed mesh pick.
    ///
    /// Tensor picks continue to be returned by `poll_pick_readback`; this separate accessor keeps
    /// the existing tensor API stable while allowing IDs wider than the R32Uint target.
    pub fn poll_semantic_pick_readback(&mut self) -> Option<u64> {
        self.pick_semantic_result.take()
    }

    /// `PortalGpu::pick_readback_pending`.
    pub fn pick_readback_pending(&self) -> bool {
        self.pick_readback.is_pending()
    }

    /// `record_picking_pass`.
    pub(super) fn record_picking_pass(&mut self, encoder: &mut etalon::CommandEncoder) {
        if !self.pick_readback.needs_recording() {
            return;
        }

        let tensor_bind = self.projector_tensor_bind.as_ref();
        // Static semantic-node positions do not follow externally authored skin
        // frames. Keep their data, but pick only the actual posed mesh until a
        // new mesh upload leaves this external-control lifecycle.
        let tensor_count = if self.source_motion.externally_controlled() {
            0
        } else {
            self.tensor_node_count
        };
        let mesh_count = self.visible_mesh_instances.count();
        let semantics = if self.mesh.is_some() {
            &self.visible_instance_scratch[..mesh_count as usize]
        } else {
            &[]
        };
        if !self
            .pick_readback
            .snapshot(semantics, |record| record.semantic_id())
        {
            return;
        }

        etalon_render::picking_readback::record_pass(
            encoder,
            &self.picking_view,
            self.scene_depth.view(),
            |pass| {
                if let Some(tensor_bind) =
                    tensor_bind.filter(|_| tensor_count < picking::MESH_PICK_FLAG)
                {
                    if tensor_count > 0 {
                        pass.set_pipeline(&self.picking_pipeline);
                        pass.set_bind_group(0, &self.projector_camera_bind, &[]);
                        pass.set_bind_group(1, tensor_bind, &[]);
                        pass.draw(0..6, 0..tensor_count);
                    }
                }

                if let Some(mesh) = self.mesh.as_ref().filter(|_| mesh_count > 0) {
                    pass.set_pipeline(&self.mesh_picking_pipeline);
                    pass.set_bind_group(0, &self.mesh_frame_bind, &[]);
                    pass.set_bind_group(1, &self.mesh_model_bind, &[]);
                    pass.set_bind_group(3, self.visible_mesh_instances.bind_group(), &[]);
                    pass.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
                    pass.set_vertex_buffer(1, mesh.color_buf.slice(..));
                    pass.set_vertex_buffer(2, mesh.normal_buf.slice(..));
                    pass.set_vertex_buffer(3, mesh.tangent_buf.slice(..));
                    pass.set_vertex_buffer(4, mesh.uv_buf.slice(..));
                    pass.set_index_buffer(mesh.index_buf.slice(..), etalon::IndexFormat::Uint32);
                    for draw in &mesh.material_gpu.draws {
                        pass.set_bind_group(
                            2,
                            &mesh.material_gpu.material_bind_groups[draw.material_index],
                            &[draw.material_offset],
                        );
                        pass.draw_indexed(
                            draw.first_index..draw.first_index + draw.index_count,
                            0,
                            0..mesh_count,
                        );
                    }
                }
            },
        );
    }

    pub(super) fn record_pick_copy(&mut self, encoder: &mut etalon::CommandEncoder) {
        self.pick_readback
            .record_copy(encoder, &self.picking_texture);
    }
}
