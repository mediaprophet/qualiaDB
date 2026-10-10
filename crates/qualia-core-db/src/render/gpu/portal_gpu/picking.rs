//! Semantic picking pass and readback implementation for PortalGpu.

use super::super::{padded_bytes_per_row, picking, PortalGpu};

impl PortalGpu {
    pub fn queue_pick(&mut self, x: f32, y: f32) {
        // The staging buffer cannot be copied into while an earlier map is
        // unresolved. A later click can be queued after the next readback.
        if self.pick_map_rx.is_some() {
            return;
        }
        // Treat pointer coordinates as continuous canvas positions and select
        // the containing top-left-origin texel, matching the CPU oracle.
        let px = x.floor().max(0.0) as u32;
        let py = y.floor().max(0.0) as u32;
        self.pending_pick = Some((
            px.min(self.width.saturating_sub(1)),
            py.min(self.height.saturating_sub(1)),
        ));
        self.pick_copy_submitted = false;
        self.pick_result = None;
        self.pick_semantic_result = None;
    }

    pub fn poll_pick_readback(&mut self) -> Option<u32> {
        if let Some(idx) = self.pick_result.take() {
            return Some(idx);
        }
        if !self.pick_copy_submitted {
            return None;
        }
        if self.pick_map_rx.is_none() {
            let slice = self.pick_staging_buf.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result.is_ok());
            });
            self.pick_map_rx = Some(rx);
        }
        // Browser WebGPU mapAsync resolves on a later event-loop turn. A
        // blocking poll on that same thread cannot deliver its callback.
        #[cfg(not(target_arch = "wasm32"))]
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let mapped_ok = match self.pick_map_rx.as_ref().unwrap().try_recv() {
            Ok(ok) => ok,
            Err(std::sync::mpsc::TryRecvError::Empty) => return None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => false,
        };
        self.pick_map_rx = None;
        if !mapped_ok {
            self.pick_copy_submitted = false;
            return None;
        }
        let slice = self.pick_staging_buf.slice(..);
        let mapped = slice
            .get_mapped_range()
            .expect("wgpu buffer map_range failed");
        let raw = if mapped.len() >= 4 {
            Some(u32::from_le_bytes(mapped[0..4].try_into().unwrap()))
        } else {
            None
        };
        drop(mapped);
        self.pick_staging_buf.unmap();
        self.pick_copy_submitted = false;
        // Zero is the portable R32Uint clear value and is reserved for "no hit". Tensor IDs
        // are biased by one in WGSL so tensor index zero remains distinguishable from a clear.
        let raw = raw.filter(|&id| id != 0)?;
        if let Some(slot) = picking::decode_mesh_slot(raw) {
            if slot < self.pick_semantic_count {
                self.pick_semantic_result = Some(self.pick_instance_semantics[slot]);
            }
            None
        } else {
            Some(raw - 1)
        }
    }

    /// Return the full semantic identity from the most recently completed mesh pick.
    ///
    /// Tensor picks continue to be returned by `poll_pick_readback`; this separate accessor keeps
    /// the existing tensor API stable while allowing IDs wider than the R32Uint target.
    pub fn poll_semantic_pick_readback(&mut self) -> Option<u64> {
        self.pick_semantic_result.take()
    }

    pub fn pick_readback_pending(&self) -> bool {
        self.pending_pick.is_some() || self.pick_copy_submitted || self.pick_map_rx.is_some()
    }

    pub(super) fn record_picking_pass(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if self.pending_pick.is_none() {
            return;
        }

        let tensor_bind = self.projector_tensor_bind.as_ref();
        let tensor_count = self.tensor_node_count;
        let mesh_count = self.visible_mesh_instances.count();
        self.pick_semantic_count = if self.mesh.is_some() {
            picking::snapshot_semantic_ids(
                &self.visible_instance_scratch[..mesh_count as usize],
                &mut self.pick_instance_semantics,
            )
            .unwrap_or(0)
        } else {
            0
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("portal-picking-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.picking_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: self.scene_depth.view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            multiview_mask: None,
            timestamp_writes: None,
        });
        if let Some(tensor_bind) = tensor_bind.filter(|_| tensor_count < picking::MESH_PICK_FLAG) {
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
            pass.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
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
    }

    pub(super) fn record_pick_copy(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let Some((px, py)) = self.pending_pick.take() else {
            return;
        };
        // Canvas pointer pixels and WebGPU texture-copy origins are both
        // top-left based. Flipping here samples the opposite side of the view.
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.picking_texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: px, y: py, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.pick_staging_buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row(1)),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        self.pick_copy_submitted = true;
    }
}
