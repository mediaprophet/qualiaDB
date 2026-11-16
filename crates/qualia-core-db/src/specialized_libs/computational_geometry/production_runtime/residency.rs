//! P17: GPU-less residency state machine, pinned coarse fallback, eviction.
//! Visibility is never an access-control input. Actual device-loss is P22.

use crate::specialized_libs::computational_geometry::geometry_workspace::production_budget::ProductionBudget;

use super::{RuntimeError, ValidatedHierarchy};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageState {
    Absent,
    Inflight { transfer_id: u32 },
    Resident,
    Pinned,
    Cancelled,
}

pub struct ResidencyTable<'a> {
    states: &'a mut [PageState],
    bytes: &'a mut [u32],
    used_bytes: u32,
    cap_bytes: u32,
}

impl<'a> ResidencyTable<'a> {
    pub fn admit(
        hierarchy: &ValidatedHierarchy<'_>,
        states: &'a mut [PageState],
        bytes: &'a mut [u32],
        cap_bytes: u32,
        budget: &mut ProductionBudget<'_>,
    ) -> Result<Self, RuntimeError> {
        let n = hierarchy.nodes().len();
        if states.len() < n || bytes.len() < n || cap_bytes == 0 {
            return Err(RuntimeError::Capacity);
        }
        budget.checkpoint(n as u64)?;
        states[..n].fill(PageState::Absent);
        let mut used = 0u32;
        for &root in hierarchy.roots() {
            let size = bytes[root as usize];
            if size == 0 {
                return Err(RuntimeError::MissingFallback);
            }
            used = used.checked_add(size).ok_or(RuntimeError::Capacity)?;
            if used > cap_bytes {
                return Err(RuntimeError::Capacity);
            }
            states[root as usize] = PageState::Pinned;
        }
        Ok(Self {
            states: &mut states[..n],
            bytes: &mut bytes[..n],
            used_bytes: used,
            cap_bytes,
        })
    }

    pub fn used_bytes(&self) -> u32 {
        self.used_bytes
    }
    pub fn cap_bytes(&self) -> u32 {
        self.cap_bytes
    }
    pub fn state(&self, index: usize) -> PageState {
        self.states[index]
    }
    pub fn is_ready(&self, index: usize) -> bool {
        matches!(self.states[index], PageState::Resident | PageState::Pinned)
    }

    pub fn begin_transfer(
        &mut self,
        index: usize,
        transfer_id: u32,
        size: u32,
        budget: &mut ProductionBudget<'_>,
    ) -> Result<(), RuntimeError> {
        budget.checkpoint(1)?;
        if index >= self.states.len() || size == 0 {
            return Err(RuntimeError::InvalidHierarchy);
        }
        match self.states[index] {
            PageState::Absent | PageState::Cancelled => {}
            _ => return Err(RuntimeError::StaleIdentity),
        }
        let next = self
            .used_bytes
            .checked_add(size)
            .ok_or(RuntimeError::Capacity)?;
        if next > self.cap_bytes {
            return Err(RuntimeError::Capacity);
        }
        self.used_bytes = next;
        self.bytes[index] = size;
        self.states[index] = PageState::Inflight { transfer_id };
        Ok(())
    }

    pub fn complete_transfer(
        &mut self,
        index: usize,
        transfer_id: u32,
        budget: &mut ProductionBudget<'_>,
    ) -> Result<(), RuntimeError> {
        budget.checkpoint(1)?;
        match self.states.get(index).copied() {
            Some(PageState::Inflight { transfer_id: id }) if id == transfer_id => {
                self.states[index] = PageState::Resident;
                Ok(())
            }
            _ => Err(RuntimeError::StaleIdentity),
        }
    }

    /// Simulated inflight cancellation. Reserved bytes are released; the page is not activated.
    pub fn cancel_transfer(
        &mut self,
        index: usize,
        transfer_id: u32,
        budget: &mut ProductionBudget<'_>,
    ) -> Result<(), RuntimeError> {
        budget.checkpoint(1)?;
        match self.states.get(index).copied() {
            Some(PageState::Inflight { transfer_id: id }) if id == transfer_id => {
                self.used_bytes = self.used_bytes.saturating_sub(self.bytes[index]);
                self.bytes[index] = 0;
                self.states[index] = PageState::Cancelled;
                Ok(())
            }
            _ => Err(RuntimeError::StaleIdentity),
        }
    }

    pub fn evict(
        &mut self,
        hierarchy: &ValidatedHierarchy<'_>,
        index: usize,
        budget: &mut ProductionBudget<'_>,
    ) -> Result<(), RuntimeError> {
        budget.checkpoint(1)?;
        if index >= self.states.len() {
            return Err(RuntimeError::InvalidHierarchy);
        }
        match self.states[index] {
            PageState::Pinned => return Err(RuntimeError::MissingFallback),
            PageState::Resident => {}
            _ => return Err(RuntimeError::StaleIdentity),
        }
        let parent = hierarchy.parent_of(index).ok_or(RuntimeError::MissingFallback)?;
        if !self.is_ready(parent as usize) {
            return Err(RuntimeError::MissingFallback);
        }
        self.used_bytes = self.used_bytes.saturating_sub(self.bytes[index]);
        self.bytes[index] = 0;
        self.states[index] = PageState::Absent;
        Ok(())
    }

    fn replacement_ready_inner(
        &self,
        hierarchy: &ValidatedHierarchy<'_>,
        parent: usize,
    ) -> bool {
        self.is_ready(parent)
            && hierarchy
                .children_of(parent)
                .iter()
                .all(|&child| self.is_ready(child as usize))
    }

    pub fn replacement_ready(
        &self,
        hierarchy: &ValidatedHierarchy<'_>,
        parent: usize,
        budget: &mut ProductionBudget<'_>,
    ) -> Result<bool, RuntimeError> {
        budget.checkpoint(1)?;
        let children = hierarchy.children_of(parent);
        if !children.is_empty() {
            budget.checkpoint(children.len() as u64)?;
        }
        Ok(self.replacement_ready_inner(hierarchy, parent))
    }

    /// Never activate a replacement whose child dependencies are incomplete.
    pub fn activate_replacement(
        &self,
        hierarchy: &ValidatedHierarchy<'_>,
        parent: usize,
        budget: &mut ProductionBudget<'_>,
    ) -> Result<(), RuntimeError> {
        if !self.replacement_ready(hierarchy, parent, budget)? {
            return Err(RuntimeError::MissingFallback);
        }
        Ok(())
    }

    /// Finest fully-resident legal cut. Roots must remain pinned/resident.
    pub fn renderable_cut(
        &self,
        hierarchy: &ValidatedHierarchy<'_>,
        stack: &mut [u32],
        out_cut: &mut [u32],
        budget: &mut ProductionBudget<'_>,
    ) -> Result<usize, RuntimeError> {
        for &root in hierarchy.roots() {
            if !self.is_ready(root as usize) {
                return Err(RuntimeError::MissingFallback);
            }
        }
        hierarchy.cpu_cut_by_refine(
            |index| Ok(self.replacement_ready_inner(hierarchy, index)),
            stack,
            out_cut,
            budget,
        )
    }
}
