//! Semantic draw adaptation for canonical GPU picking ownership.
use super::super::{picking, PortalGpu};
use crate::render::portal_load_10d::{PickSurfaceRegion, SemanticSurfacePick};
use etalon_render::picking_readback::MeshPickMetadata;
use std::collections::BTreeMap;
impl PortalGpu {
    pub fn queue_pick(&mut self, x: f32, y: f32) {
        self.pick_readback.queue(x, y, self.width, self.height);
        self.pick_semantic_result = None;
    }

    /// Queue a screen-space search over the currently admitted SDFB section.
    pub fn queue_nearest_section_pick(
        &mut self,
        x: f32,
        y: f32,
        radius_px: u8,
        surface_region: PickSurfaceRegion,
    ) -> Result<(), String> {
        if self.is_device_lost() {
            return Err("Section picking requires an active GPU device".into());
        }
        if self.pick_group_iris.is_empty() {
            return Err("Section picking requires resident source-admitted SDFB metadata".into());
        }
        self.pick_readback
            .queue_nearest_mesh_region(
                x,
                y,
                self.width,
                self.height,
                radius_px,
                surface_region as u8,
            )
            .map_err(|error| format!("Section pick request: {error:?}"))?;
        self.pick_semantic_result = None;
        Ok(())
    }

    pub fn poll_pick_readback(&mut self) -> Option<u32> {
        if self.is_device_lost() {
            self.pick_readback.notify_device_lost();
            self.pick_semantic_result = None;
            return None;
        }
        let require_section = self.pick_readback.is_mesh_region_request();
        match self.pick_readback.poll_with_metadata(&self.device)? {
            (etalon_render::picking::PickResult::Tensor(index), _) => Some(index),
            (etalon_render::picking::PickResult::Mesh(semantic_id), Some(metadata)) => {
                let surface_region = PickSurfaceRegion::from_u8(metadata.surface_region)?;
                let pick = SemanticSurfacePick {
                    instance_slot: metadata.source_slot,
                    semantic_id,
                    surface_region,
                };
                if !require_section || self.section_iri_for_pick(pick).is_some() {
                    self.pick_semantic_result = Some(pick);
                }
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
        self.poll_semantic_pick_with_metadata()
            .map(|pick| pick.semantic_id)
    }

    /// Return the frame's original instance slot and semantic region as well as
    /// its full identity. Uncut SDFB faces carry their wall IRI with region 00.
    pub fn poll_semantic_pick_with_metadata(&mut self) -> Option<SemanticSurfacePick> {
        self.pick_semantic_result.take()
    }

    pub(crate) fn set_section_pick_metadata(
        &mut self,
        fields: Vec<(u32, PickSurfaceRegion)>,
        iris: BTreeMap<u64, String>,
    ) -> Result<(), String> {
        if self.mesh_instance_groups.is_empty() || self.mesh_instance_source.len() != fields.len() {
            return Err("Section picking metadata does not cover the native source stream".into());
        }
        let mut submitted = BTreeMap::new();
        for instance in &self.mesh_instance_source {
            let hash = instance.semantic_id();
            if submitted.contains_key(&hash) {
                continue;
            }
            let iri = iris
                .get(&hash)
                .ok_or("Section draw is missing its source-admitted IRI")?;
            if hash == 0 || crate::q_hash(iri) != hash {
                return Err("Section draw lexicon conflicts with its admitted q_hash".into());
            }
            submitted.insert(hash, iri.clone());
        }
        self.pick_group_metadata = fields
            .into_iter()
            .map(|(source_slot, region)| MeshPickMetadata {
                source_slot,
                surface_region: region as u8,
            })
            .collect();
        self.pick_group_iris = submitted;
        Ok(())
    }

    /// Resolve the exact resident section tuple, without consulting another
    /// asset or treating a hash collision as identity.
    pub fn section_iri_for_pick(&self, selected: SemanticSurfacePick) -> Option<&str> {
        let iri = self.pick_group_iris.get(&selected.semantic_id)?;
        self.pick_group_metadata
            .iter()
            .zip(&self.mesh_instance_source)
            .any(|(metadata, instance)| {
                metadata.source_slot == selected.instance_slot
                    && metadata.surface_region == selected.surface_region as u8
                    && instance.semantic_id() == selected.semantic_id
            })
            .then_some(iri.as_str())
    }

    /// Resolve a canonical IRI to every distinct resident section tuple.
    pub fn section_picks_for_iri(&self, iri: &str) -> Vec<SemanticSurfacePick> {
        let semantic_id = crate::q_hash(iri);
        if self.pick_group_iris.get(&semantic_id).map(String::as_str) != Some(iri) {
            return Vec::new();
        }
        let mut seen = std::collections::BTreeSet::new();
        self.pick_group_metadata
            .iter()
            .zip(&self.mesh_instance_source)
            .filter_map(|(metadata, instance)| {
                if instance.semantic_id() != semantic_id
                    || !seen.insert((metadata.source_slot, metadata.surface_region))
                {
                    return None;
                }
                Some(SemanticSurfacePick {
                    instance_slot: metadata.source_slot,
                    semantic_id,
                    surface_region: PickSurfaceRegion::from_u8(metadata.surface_region)?,
                })
            })
            .collect()
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
        let region_scissor = self.pick_readback.mesh_region_scissor();
        let section_query = region_scissor.is_some();
        if section_query && self.pick_group_iris.is_empty() {
            self.pick_readback.invalidate(false);
            return;
        }

        let tensor_bind = self.projector_tensor_bind.as_ref();
        // Static semantic-node positions do not follow externally authored skin
        // frames. Keep their data, but pick only the actual posed mesh until a
        // new mesh upload leaves this external-control lifecycle.
        let tensor_count = if self.source_motion.externally_controlled()
            || (section_query && !self.tensor_projection_enabled)
        {
            0
        } else {
            self.tensor_node_count
        };
        let mesh_count = if self.last_visibility_key.is_some() && self.mesh.is_some() {
            self.visible_mesh_instances.count()
        } else {
            0
        };
        let semantics = if self.mesh.is_some() {
            &self.visible_instance_scratch[..mesh_count as usize]
        } else {
            &[]
        };
        let selected = &self.visible_instance_indices;
        let groups = &self.mesh_instance_groups;
        let source_count = self.mesh_instance_source.len();
        let metadata = &self.pick_group_metadata;
        if !self.pick_readback.snapshot_with_metadata(
            semantics,
            |record| record.semantic_id(),
            |slot| {
                let source_slot =
                    picking::visible_to_source_slot(slot, selected, groups, source_count)?;
                if metadata.is_empty() {
                    Some(MeshPickMetadata {
                        source_slot,
                        surface_region: 0,
                    })
                } else {
                    metadata.get(source_slot as usize).copied()
                }
            },
        ) {
            return;
        }

        etalon_render::picking_readback::record_pass(
            encoder,
            &self.picking_view,
            self.scene_depth.view(),
            |pass| {
                if let Some([x, y, width, height]) = region_scissor {
                    pass.set_scissor_rect(x, y, width, height);
                }
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
                        for (indices, instances) in super::super::instance_groups::draw_ranges(
                            &self.mesh_instance_groups,
                            draw.first_index,
                            draw.index_count,
                            mesh_count,
                            true,
                        ) {
                            pass.draw_indexed(indices, 0, instances);
                        }
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
