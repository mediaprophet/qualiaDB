use super::super::primitives::Point3;
use super::{ContentId, ContractError, ReasonCode};

pub const REMOVED_INDEX: u32 = u32::MAX;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollapseProposal {
    pub run: ContentId,
    pub source: ContentId,
    pub policy: ContentId,
    pub sequence: u64,
    pub topology_generation: u64,
    pub removed_vertex: u32,
    pub kept_vertex: u32,
    pub stable_endpoint_ids: [u64; 2],
    pub endpoint_generations: [u64; 2],
    pub target: Point3,
    pub t_from_removed_to_kept: f64,
}
impl CollapseProposal {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.removed_vertex == self.kept_vertex
            || self.removed_vertex == REMOVED_INDEX
            || self.kept_vertex == REMOVED_INDEX
            || self.stable_endpoint_ids[0] == self.stable_endpoint_ids[1]
            || ![
                self.target.x,
                self.target.y,
                self.target.z,
                self.t_from_removed_to_kept,
            ]
            .iter()
            .all(|x| x.is_finite())
            || !(0.0..=1.0).contains(&self.t_from_removed_to_kept)
        {
            return Err(ContractError::new(
                ReasonCode::InvalidInput,
                "collapse_proposal",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CornerId {
    pub face: u32,
    pub slot: u8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaceDelta {
    pub original_face: u32,
    pub before: [u32; 3],
    pub after: Option<[u32; 3]>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CornerDelta {
    pub source: CornerId,
    pub target: Option<CornerId>,
}
#[derive(Debug, Clone, Copy)]
pub struct CollapseNeighborhood<'a> {
    pub faces: &'a [FaceDelta],
    pub corners: &'a [CornerDelta],
}

/// Prepared state is caller-owned scratch, not a permission to change the mesh.
/// The driver revalidates after ranking, reserves every journal/map capacity,
/// and commits transfer, mesh and journal without cancellation inside that step.
pub trait TransferAdmission {
    type Prepared: Copy;
    fn additional_storage_bytes(&self) -> Result<usize, ContractError>;
    fn scratch_bytes(&self) -> Result<usize, ContractError>;
    fn work_bound(&self, faces: usize, corners: usize) -> Result<u64, ContractError>;
    fn prepare(
        &self,
        proposal: &CollapseProposal,
        neighborhood: CollapseNeighborhood<'_>,
        scratch: &mut [u8],
    ) -> Result<Self::Prepared, ContractError>;
    fn revalidate(
        &self,
        proposal: &CollapseProposal,
        neighborhood: CollapseNeighborhood<'_>,
        prepared: &Self::Prepared,
        scratch: &[u8],
    ) -> Result<(), ContractError>;
    fn prepared_identity(&self, prepared: &Self::Prepared) -> ContentId;
    fn commit(&mut self, proposal: &CollapseProposal, prepared: Self::Prepared, scratch: &mut [u8]);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollapseEvent {
    pub proposal: CollapseProposal,
    pub transfer: ContentId,
}

/// These are distinct maps: lineage is not interpolation or correspondence.
pub struct RemapBuffers<'a> {
    pub vertex_lineage: &'a mut [u32],
    pub surviving_vertex_compaction: &'a mut [u32],
    pub face_compaction: &'a mut [u32],
    pub corner_compaction: &'a mut [Option<CornerId>],
}
impl RemapBuffers<'_> {
    pub fn validate_capacity(&self, vertices: usize, faces: usize) -> Result<(), ContractError> {
        let corners = faces.checked_mul(3).ok_or(ContractError::new(
            ReasonCode::CapacityExhausted,
            "corner_count",
        ))?;
        if self.vertex_lineage.len() < vertices
            || self.surviving_vertex_compaction.len() < vertices
            || self.face_compaction.len() < faces
            || self.corner_compaction.len() < corners
        {
            return Err(ContractError::new(
                ReasonCode::CapacityExhausted,
                "remap_capacity",
            ));
        }
        Ok(())
    }
}

pub struct CollapseJournal<'a> {
    events: &'a mut [Option<CollapseEvent>],
    used: usize,
}
impl<'a> CollapseJournal<'a> {
    pub fn new(events: &'a mut [Option<CollapseEvent>]) -> Self {
        Self { events, used: 0 }
    }
    pub fn committed(&self) -> &[Option<CollapseEvent>] {
        &self.events[..self.used]
    }
    pub fn reserve(&mut self) -> Result<ReservedCollapseEvent<'_>, ContractError> {
        if self.used == self.events.len() {
            return Err(ContractError::new(
                ReasonCode::CapacityExhausted,
                "collapse_events",
            ));
        }
        Ok(ReservedCollapseEvent {
            slot: &mut self.events[self.used],
            used: &mut self.used,
        })
    }
}
#[derive(Debug)]
pub struct ReservedCollapseEvent<'a> {
    slot: &'a mut Option<CollapseEvent>,
    used: &'a mut usize,
}
impl ReservedCollapseEvent<'_> {
    pub fn commit(self, event: CollapseEvent) {
        *self.slot = Some(event);
        *self.used += 1;
    }
}
