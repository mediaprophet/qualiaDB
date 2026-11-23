//! Native SDFB cutaway installation and physical-slot semantic pick binding.

use super::super::PortalGpu;
use crate::render::{
    instance_culling::GpuInstanceRecord,
    portal_emf_binding::BoundLeafEmfCompanion,
    portal_emf_state::LoadedEmf,
    portal_load_10d::{
        decode_hmc_leaf_cutaway_with_emf, decode_sdfb_cutaway_with_emf, resolve_hmc_section_iris,
        PickSurfaceRegion, SectionLeafOccurrence, SectionPlane,
    },
};

impl PortalGpu {
    /// Use one world-space plane and preserve terminal occurrence slots across
    /// per-region native draw groups.
    pub fn set_hmc_section_plane(
        &mut self,
        bundle: &crate::bundle::BundleReader<'_>,
        assembly_key: &str,
        plane: SectionPlane,
        leaf_slots: &[u32],
    ) -> Result<[u32; 4], String> {
        self.set_hmc_section_plane_with_occurrences(bundle, assembly_key, plane, leaf_slots)
            .map(|(counts, _)| counts)
    }

    pub(crate) fn set_hmc_section_plane_with_occurrences(
        &mut self,
        bundle: &crate::bundle::BundleReader<'_>,
        assembly_key: &str,
        plane: SectionPlane,
        leaf_slots: &[u32],
    ) -> Result<([u32; 4], Vec<SectionLeafOccurrence>), String> {
        self.set_hmc_section_plane_with_emf(bundle, assembly_key, plane, leaf_slots, None)
    }

    pub fn set_hmc_section_plane_with_emf(
        &mut self,
        bundle: &crate::bundle::BundleReader<'_>,
        assembly_key: &str,
        plane: SectionPlane,
        leaf_slots: &[u32],
        admitted_emf: Option<&LoadedEmf>,
    ) -> Result<([u32; 4], Vec<SectionLeafOccurrence>), String> {
        let section =
            decode_hmc_leaf_cutaway_with_emf(bundle, assembly_key, plane, leaf_slots, admitted_emf)?;
        if section.draw.indices.is_empty() {
            self.upload_mesh_colored_with_frames_and_materials(&[], &[], None, None, &[], &[], &[])?;
            return Ok(([0; 4], section.leaf_occurrences));
        }
        let iris = resolve_hmc_section_iris(bundle, &section)?;
        section.draw.upload(self)?;
        if let Err(error) = self.set_section_pick_metadata(section.pick_metadata, iris) {
            self.upload_mesh(&[], &[]);
            return Err(error);
        }
        Ok((
            [
                section.draw.vertex_count,
                section.draw.triangle_count,
                section.cut_wall_triangles,
                section.lumen_triangles,
            ],
            section.leaf_occurrences,
        ))
    }

    /// Build and install a section plane over a verified standalone SDFB asset.
    /// The asset remains immutable; native GPU draw groups distinguish solid
    /// cut tissues from positive, enclosed lumen apertures.
    pub fn set_section_plane(
        &mut self,
        sdfb_10d: &[u8],
        plane: SectionPlane,
    ) -> Result<[u32; 4], String> {
        self.set_section_plane_with_emf(sdfb_10d, plane, None)
    }

    /// Build and install a section plane over a verified standalone SDFB asset,
    /// evaluating an optional bound optical `.emf` companion in authored metre coordinates
    /// on both exterior and cut-tissue surfaces while keeping lumen openings distinct.
    pub fn set_section_plane_with_emf(
        &mut self,
        sdfb_10d: &[u8],
        plane: SectionPlane,
        companion: Option<&BoundLeafEmfCompanion<'_>>,
    ) -> Result<[u32; 4], String> {
        let cut = decode_sdfb_cutaway_with_emf(sdfb_10d, plane, companion)?;
        if cut.mesh.triangles.is_empty() {
            self.upload_mesh_colored_with_frames_and_materials(&[], &[], None, None, &[], &[], &[])?;
            return Ok([0; 4]);
        }
        let scale = 1.6 / cut.source_span_m;
        let positions: Vec<_> = cut
            .mesh
            .positions
            .iter()
            .map(|point| {
                std::array::from_fn(|axis| (point[axis] - cut.source_center_m[axis]) * scale)
            })
            .collect();
        let indices: Vec<_> = cut.mesh.triangles.iter().flatten().copied().collect();
        let identity = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let mut descriptors = Vec::with_capacity(cut.ranges.len());
        let mut instances = Vec::with_capacity(cut.ranges.len());
        let mut metadata = Vec::with_capacity(cut.ranges.len());
        let (mut wall_triangles, mut lumen_triangles) = (0, 0);
        for (slot, (range, region)) in cut.ranges.iter().zip(cut.regions).enumerate() {
            descriptors.push([range.first_index, range.index_count, slot as u32, 1]);
            instances.push(GpuInstanceRecord::new(identity, range.semantic_id));
            metadata.push((0, region));
            match region {
                PickSurfaceRegion::UncutSurface => {}
                PickSurfaceRegion::CutTissueWall => wall_triangles += range.index_count / 3,
                PickSurfaceRegion::InternalCavityLumen => lumen_triangles += range.index_count / 3,
            }
        }
        self.upload_mesh_colored_with_frames_and_materials_and_uv0(
            &positions,
            &cut.colors,
            None,
            None,
            None,
            &indices,
            &cut.materials,
            &cut.ranges,
        )?;
        if let Err(error) = self
            .set_mesh_instance_groups_packed(
                bytemuck::cast_slice(&descriptors),
                bytemuck::cast_slice(&instances),
            )
            .and_then(|()| self.set_section_pick_metadata(metadata, cut.semantic_iris))
        {
            self.upload_mesh(&[], &[]);
            return Err(error);
        }
        Ok([
            positions.len() as u32,
            (indices.len() / 3) as u32,
            wall_triangles,
            lumen_triangles,
        ])
    }
}
