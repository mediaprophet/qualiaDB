//! Full authored-mesh publication through the existing PortalGpu buffers.
use super::PortalGpu;
use crate::render::source_frame::{
    decode_bound_model, validate_resident_mesh, SourceFrameBindLimits, SourceFrameBinding,
    SourceFrameError, SourceFrameReceipt, SourceMeshFrame,
};

impl PortalGpu {
    pub fn bind_source_motion_model(
        &mut self,
        model: &[u8],
        source_sha256: [u8; 32],
        source_generation: u64,
        limits: SourceFrameBindLimits,
    ) -> Result<SourceFrameBinding, SourceFrameError> {
        self.bind_source_motion_model_at_epoch(model, source_sha256, source_generation, 1, limits)
    }

    /// Commit the caller's current transport epoch with cold model admission;
    /// no fallible epoch-advance step is needed after binding has mutated state.
    pub fn bind_source_motion_model_at_epoch(
        &mut self,
        model: &[u8],
        source_sha256: [u8; 32],
        source_generation: u64,
        transport_epoch: u64,
        limits: SourceFrameBindLimits,
    ) -> Result<SourceFrameBinding, SourceFrameError> {
        if transport_epoch == 0 {
            return Err(SourceFrameError::Input);
        }
        if self.is_device_lost() {
            return Err(SourceFrameError::Backend);
        }
        if self.source_motion.externally_controlled() {
            return Err(SourceFrameError::Stale);
        }
        // First profile is one full authored mesh, not instanced copies or an
        // independently driven artefact joint layered over the native player.
        if !self.source_motion_instance_supported() || self.artefact_joint.is_some() {
            return Err(SourceFrameError::Backend);
        }
        if source_generation == 0 || source_sha256 == [0; 32] {
            return Err(SourceFrameError::Input);
        }
        let mesh = self.mesh.as_ref().ok_or(SourceFrameError::Backend)?;
        let (decoded, model_sha256) = decode_bound_model(model, limits)?;
        validate_resident_mesh(&decoded, &mesh.cpu_positions, &mesh.cpu_indices)?;
        let binding = SourceFrameBinding {
            model_sha256,
            source_sha256,
            source_generation,
            mesh_generation: self.mesh_generation,
            transport_epoch,
            vertex_count: mesh.vertex_count,
            index_count: mesh.cpu_indices.len() as u32,
        };
        // Binding is the last operation; failed cold admission leaves prior state.
        self.source_motion.bind(binding)?;
        self.invalidate_source_pose_picks(false);
        Ok(binding)
    }

    pub fn source_motion_binding(&self) -> Option<SourceFrameBinding> {
        if self.is_device_lost() {
            None
        } else {
            self.source_motion.binding()
        }
    }

    #[allow(dead_code)]
    pub(crate) fn source_motion_mesh_counts(&self) -> Option<(u32, u32)> {
        self.mesh
            .as_ref()
            .map(|mesh| (mesh.vertex_count, mesh.cpu_indices.len() as u32))
    }
    pub fn retire_source_motion_binding(&mut self) {
        self.source_motion.retire();
        self.invalidate_source_pose_picks(false);
    }
    pub fn advance_source_motion_epoch(
        &mut self,
        expected: SourceFrameBinding,
        next_epoch: u64,
    ) -> Result<SourceFrameBinding, SourceFrameError> {
        if self.is_device_lost() {
            return Err(SourceFrameError::Backend);
        }
        if expected.mesh_generation != self.mesh_generation {
            return Err(SourceFrameError::Stale);
        }
        let next = self.source_motion.advance_epoch(expected, next_epoch)?;
        self.invalidate_source_pose_picks(false);
        self.temporal_reset_pending = true;
        Ok(next)
    }
    pub fn apply_source_motion_frame(
        &mut self,
        frame: SourceMeshFrame<'_>,
    ) -> Result<SourceFrameReceipt, SourceFrameError> {
        if self.is_device_lost() {
            return Err(SourceFrameError::Backend);
        }
        if !self.source_motion_instance_supported() || self.artefact_joint.is_some() {
            return Err(SourceFrameError::Backend);
        }
        let (next, receipt) = self.source_motion.prepare(&frame, self.mesh_generation)?;
        let mesh = self.mesh.as_ref().ok_or(SourceFrameError::Backend)?;
        if mesh.vertex_count != frame.binding.vertex_count
            || mesh.cpu_indices.len() != frame.binding.index_count as usize
        {
            return Err(SourceFrameError::ResidentMismatch);
        }
        // Complete validation precedes the existing no-fail upload commit.
        if !self.write_mesh_animated_frames(0, frame.positions, frame.normals, frame.tangents) {
            return Err(SourceFrameError::Upload);
        }
        self.source_motion = next;
        self.invalidate_source_pose_picks(true);
        // No per-vertex previous-pose motion vectors are supplied by this seam.
        // Reset history rather than pretending camera-only vectors describe skinning.
        self.temporal_reset_pending = true;
        Ok(receipt)
    }

    fn source_motion_instance_supported(&self) -> bool {
        const IDENTITY: [[f32; 4]; 4] = [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ];
        let [instance] = self.mesh_instance_source.as_slice() else {
            return false;
        };
        let expected = crate::render::instance_culling::GpuInstanceRecord::new(IDENTITY, 0);
        instance.world_from_local == expected.world_from_local
            && instance.normal_from_local == expected.normal_from_local
            && instance.orientation_sign == expected.orientation_sign
    }

    fn invalidate_source_pose_picks(&mut self, retain_unsubmitted_pointer: bool) {
        // Pointer coordinates not submitted yet may target the new pose. Old
        // readbacks/results cannot claim to describe it.
        self.pick_readback.invalidate(retain_unsubmitted_pointer);
        self.pick_semantic_result = None;
    }
}
