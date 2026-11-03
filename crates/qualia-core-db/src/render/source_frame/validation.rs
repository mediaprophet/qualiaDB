use super::{SourceFrameBinding, SourceFrameError, SourceFrameReceipt, SourceMeshFrame};

/// One resident binding and its publication sequence. No clock or skeletal
/// state lives here. A detached next-state copy permits preflight before commit.
#[derive(Clone, Copy, Debug, Default)]
pub struct SourceFrameState {
    binding: Option<SourceFrameBinding>,
    last_sequence: u64,
    controlled: bool,
}
impl SourceFrameState {
    pub fn binding(&self) -> Option<SourceFrameBinding> {
        self.binding
    }
    pub fn externally_controlled(&self) -> bool {
        self.controlled
    }
    pub fn retire(&mut self) {
        self.binding = None;
    }
    pub fn bind(&mut self, binding: SourceFrameBinding) -> Result<(), SourceFrameError> {
        // A cold rebind on the same resident mesh could resurrect a retired
        // epoch-one job. New source/model admission must install a new mesh;
        // seek/clip/rate changes use advance_epoch instead.
        if self.controlled {
            return Err(SourceFrameError::Stale);
        }
        if binding.model_sha256 == [0; 32]
            || binding.source_sha256 == [0; 32]
            || binding.source_generation == 0
            || binding.mesh_generation == 0
            || binding.transport_epoch == 0
            || binding.vertex_count == 0
            || binding.index_count == 0
            || binding.index_count % 3 != 0
        {
            return Err(SourceFrameError::Input);
        }
        self.binding = Some(binding);
        self.controlled = true;
        self.last_sequence = 0;
        Ok(())
    }
    pub fn advance_epoch(
        &mut self,
        expected: SourceFrameBinding,
        next: u64,
    ) -> Result<SourceFrameBinding, SourceFrameError> {
        if self.binding != Some(expected) {
            return Err(SourceFrameError::Stale);
        }
        if next <= expected.transport_epoch {
            return Err(SourceFrameError::Sequence);
        }
        let binding = SourceFrameBinding {
            transport_epoch: next,
            ..expected
        };
        self.binding = Some(binding);
        self.last_sequence = 0;
        Ok(binding)
    }
    /// No mutation or allocation, including on failure. Backward ticks are
    /// permitted for explicit seeks; epoch and increasing sequence fence jobs.
    pub fn prepare(
        &self,
        frame: &SourceMeshFrame<'_>,
        mesh_generation: u64,
    ) -> Result<(Self, SourceFrameReceipt), SourceFrameError> {
        if self.binding != Some(frame.binding) || frame.binding.mesh_generation != mesh_generation {
            return Err(SourceFrameError::Stale);
        }
        if frame.frame_sequence == 0 || frame.frame_sequence <= self.last_sequence {
            return Err(SourceFrameError::Sequence);
        }
        if frame.timeline_tick > i64::MAX as u64 {
            return Err(SourceFrameError::Input);
        }
        let bounds = validate_pose(frame)?;
        let mut next = *self;
        next.last_sequence = frame.frame_sequence;
        Ok((
            next,
            SourceFrameReceipt {
                binding: frame.binding,
                frame_sequence: frame.frame_sequence,
                timeline_tick: frame.timeline_tick,
                bounds,
            },
        ))
    }
}

/// Whole-pose finite/unit/orthogonality validation before any CPU/GPU mutation.
pub fn validate_pose(frame: &SourceMeshFrame<'_>) -> Result<[[f32; 3]; 2], SourceFrameError> {
    let count = frame.binding.vertex_count as usize;
    if count == 0
        || frame.positions.len() != count
        || frame.normals.len() != count
        || frame.tangents.len() != count
    {
        return Err(SourceFrameError::Attributes);
    }
    let mut bounds = [[f32::INFINITY; 3], [f32::NEG_INFINITY; 3]];
    for ((position, normal), tangent) in frame
        .positions
        .iter()
        .zip(frame.normals)
        .zip(frame.tangents)
    {
        if position
            .iter()
            .chain(normal)
            .chain(tangent)
            .any(|v| !v.is_finite())
        {
            return Err(SourceFrameError::Attributes);
        }
        // Retain f32 headroom for bounds centers and squared distances used by
        // existing draw sorting/culling. This is a numerical, not physical cap.
        if position
            .iter()
            .any(|v| f64::from(*v).abs() > (f64::from(f32::MAX) / 12.).sqrt())
        {
            return Err(SourceFrameError::Attributes);
        }
        let n2: f64 = normal.iter().map(|v| f64::from(*v).powi(2)).sum();
        let t2: f64 = tangent[..3].iter().map(|v| f64::from(*v).powi(2)).sum();
        let dot: f64 = normal
            .iter()
            .zip(tangent)
            .map(|(n, t)| f64::from(*n) * f64::from(*t))
            .sum();
        // Numerical admission tolerance, not a visual quality measurement.
        if (n2 - 1.).abs() > 0.02
            || (t2 - 1.).abs() > 0.02
            || dot.abs() > 0.02
            || (tangent[3] != -1.0 && tangent[3] != 1.0)
        {
            return Err(SourceFrameError::Attributes);
        }
        for axis in 0..3 {
            bounds[0][axis] = bounds[0][axis].min(position[axis]);
            bounds[1][axis] = bounds[1][axis].max(position[axis]);
        }
    }
    Ok(bounds)
}
