//! P20: qualified dependency-set activation at a declared boundary.
//! Rolling back bytes restores the previous mesh/shader/transfer identities.
//! It does not undo already-consumed simulation state.

use crate::specialized_libs::computational_geometry::geometry_workspace::production_budget::ProductionBudget;
use crate::specialized_libs::computational_geometry::production_contracts::ContentId;

use super::RuntimeError;

/// Byte rollback restores identities; consumed simulation is not restored.
pub const ROLLBACK_DOES_NOT_UNDO_SIMULATION: bool = true;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationBoundary {
    Frame(u64),
    Job(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationFault {
    IncompleteAsset,
    InvalidatedEvidence,
    MixedVersion,
    DeviceLost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QualifiedSet {
    pub mesh: ContentId,
    pub shader: ContentId,
    pub transfer: ContentId,
    pub evidence: ContentId,
    pub mesh_generation: u64,
    pub shader_generation: u64,
    pub transfer_generation: u64,
    pub assets_complete: bool,
    pub evidence_valid: bool,
    pub boundary: ActivationBoundary,
}

impl QualifiedSet {
    pub fn generations_agree(self) -> bool {
        self.mesh_generation == self.shader_generation
            && self.shader_generation == self.transfer_generation
    }
}

pub struct ActivationStore {
    boundary: Option<ActivationBoundary>,
    current: Option<QualifiedSet>,
    previous: Option<QualifiedSet>,
    simulation_consumed: u64,
    device_lost: bool,
}

impl ActivationStore {
    pub fn new() -> Self {
        Self {
            boundary: None,
            current: None,
            previous: None,
            simulation_consumed: 0,
            device_lost: false,
        }
    }

    pub fn current(&self) -> Option<QualifiedSet> {
        self.current
    }
    pub fn previous(&self) -> Option<QualifiedSet> {
        self.previous
    }
    pub fn simulation_consumed(&self) -> u64 {
        self.simulation_consumed
    }
    pub fn device_lost(&self) -> bool {
        self.device_lost
    }

    pub fn declare_boundary(&mut self, boundary: ActivationBoundary) {
        self.boundary = Some(boundary);
    }

    /// Consumed simulation ticks are independent of mesh/shader byte rollback.
    pub fn consume_simulation(&mut self, steps: u64) -> Result<(), RuntimeError> {
        self.simulation_consumed = self
            .simulation_consumed
            .checked_add(steps)
            .ok_or(RuntimeError::Capacity)?;
        Ok(())
    }

    pub fn activate(
        &mut self,
        set: QualifiedSet,
        device_ok: bool,
        injected: Option<ActivationFault>,
        budget: &mut ProductionBudget<'_>,
    ) -> Result<(), RuntimeError> {
        budget.checkpoint(1)?;
        if !device_ok || injected == Some(ActivationFault::DeviceLost) {
            self.device_lost = true;
            return Err(RuntimeError::DeviceLost);
        }
        self.device_lost = false;
        if injected == Some(ActivationFault::IncompleteAsset) || !set.assets_complete {
            return Err(RuntimeError::MissingFallback);
        }
        if injected == Some(ActivationFault::InvalidatedEvidence) || !set.evidence_valid {
            return Err(RuntimeError::StaleIdentity);
        }
        if injected == Some(ActivationFault::MixedVersion) || !set.generations_agree() {
            return Err(RuntimeError::StaleIdentity);
        }
        match self.boundary {
            Some(boundary) if boundary == set.boundary => {}
            _ => return Err(RuntimeError::Unsupported),
        }
        self.previous = self.current;
        self.current = Some(set);
        Ok(())
    }

    /// Restore the previous qualified set. `simulation_consumed` is unchanged.
    pub fn rollback(&mut self, budget: &mut ProductionBudget<'_>) -> Result<QualifiedSet, RuntimeError> {
        budget.checkpoint(0)?;
        let restored = self.previous.ok_or(RuntimeError::MissingFallback)?;
        self.current = Some(restored);
        debug_assert!(ROLLBACK_DOES_NOT_UNDO_SIMULATION);
        Ok(restored)
    }
}

impl Default for ActivationStore {
    fn default() -> Self {
        Self::new()
    }
}
