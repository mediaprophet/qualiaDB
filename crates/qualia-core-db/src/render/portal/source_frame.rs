//! Browser projection of the same typed native frame submission. The caller is
//! the existing source-owned Rust session, not a JS-owned animation evaluator.
use super::{BodyRendererBackend, QualiaPortal};
use crate::render::source_frame::{
    SourceFrameBindLimits, SourceFrameBinding, SourceFrameError, SourceFrameReceipt,
    SourceMeshFrame,
};

impl QualiaPortal {
    fn source_motion_backend(&self) -> Result<(), SourceFrameError> {
        if self.body_renderer != BodyRendererBackend::WebGpu
            || self.scene_animation.compact_gpu
            || !self.preserve_authored_frame
            || self.gpu.is_none()
        {
            return Err(SourceFrameError::Backend);
        }
        let counts = self
            .gpu
            .as_ref()
            .and_then(|g| g.source_motion_mesh_counts());
        if counts != Some((self.body_vertex_count, self.body_index_count)) {
            return Err(SourceFrameError::ResidentMismatch);
        }
        Ok(())
    }
    pub fn bind_source_motion_model(
        &mut self,
        model: &[u8],
        source_sha256: [u8; 32],
        source_generation: u64,
        limits: SourceFrameBindLimits,
    ) -> Result<SourceFrameBinding, SourceFrameError> {
        self.bind_source_motion_model_at_epoch(model, source_sha256, source_generation, 1, limits)
    }

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
        self.source_motion_backend()?;
        let gpu = self.gpu.as_mut().ok_or(SourceFrameError::Backend)?;
        let binding = gpu.bind_source_motion_model_at_epoch(
            model,
            source_sha256,
            source_generation,
            transport_epoch,
            limits,
        )?;
        // Exactly one authoritative source player now owns pose. Retired handles
        // do not resume the old generic player; a new scene installation resets it.
        self.scene_animation.external_controlled = true;
        // The source session retains the authoritative contact-aware player.
        // Drop now-inactive generic actor players instead of retaining a second
        // skeletal runtime whose snapshots would describe a different pose.
        self.scene_animation.actors.clear();
        self.scene_animation.resident_bytes = 0;
        gpu.set_tensor_projection_enabled(false);
        self.tensor_projection_enabled = false;
        self.clear_source_pose_selection();
        Ok(binding)
    }
    pub fn source_motion_binding(&self) -> Option<SourceFrameBinding> {
        if self.source_motion_backend().is_err() {
            return None;
        }
        self.gpu.as_ref().and_then(|g| g.source_motion_binding())
    }
    pub fn retire_source_motion_binding(&mut self) {
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.retire_source_motion_binding();
        }
        self.clear_source_pose_selection();
    }
    pub fn advance_source_motion_epoch(
        &mut self,
        expected: SourceFrameBinding,
        next_epoch: u64,
    ) -> Result<SourceFrameBinding, SourceFrameError> {
        self.source_motion_backend()?;
        let binding = self
            .gpu
            .as_mut()
            .ok_or(SourceFrameError::Backend)?
            .advance_source_motion_epoch(expected, next_epoch)?;
        self.clear_source_pose_selection();
        Ok(binding)
    }
    pub fn apply_source_motion_frame(
        &mut self,
        frame: SourceMeshFrame<'_>,
    ) -> Result<SourceFrameReceipt, SourceFrameError> {
        self.source_motion_backend()?;
        let positions = frame.positions;
        let receipt = self
            .gpu
            .as_mut()
            .ok_or(SourceFrameError::Backend)?
            .apply_source_motion_frame(frame)?;
        // update_authored_span's only rejection is overflow or nonfinite input.
        // Start=0, capped matching vertex count, and whole-pose validation above
        // establish those preconditions before GPU mutation; this is commit only.
        if let Some(body) = self.cpu_body.as_mut() {
            body.update_authored_span(0, positions);
        }
        self.clear_source_pose_selection();
        // GPU pose commit retains an unsubmitted pointer for the next posed
        // mesh pass. Keep the portal polling lease consistent with that queue,
        // while never retaining the previous pose's selected result.
        self.pending_gpu_pick = self
            .gpu
            .as_ref()
            .is_some_and(|gpu| gpu.pick_readback_pending());
        Ok(receipt)
    }
    fn clear_source_pose_selection(&mut self) {
        self.selected_node = None;
        self.selected_semantic_id = None;
        self.pending_gpu_pick = false;
    }
}
