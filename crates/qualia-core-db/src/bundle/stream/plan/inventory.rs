//! Authenticated mixed-HMC inventory. Family is declared metadata, not a suffix.
use super::PlanError;
use crate::bundle::stream::descriptor::CandidateIdentity;
use crate::net::qdnf::types::StrongDigest;
pub const MAX_PLAN_ENTRIES: usize = 64;
pub const MAX_PLAN_EDGES: usize = 256;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryFamily {
    TenD,
    Q42,
    Emf,
    Qac,
    M64,
    P64,
    Opaque,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Obligation {
    Mandatory,
    Optional,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepresentationClass {
    Control,
    Metadata,
    Base,
    Enhancement,
    Alternative,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DependencyReason {
    Frame,
    Calibration,
    Clock,
    Geometry,
    Query,
}
/// Exact source-declared interval. Equal ticks do not mint a shared clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RationalInterval {
    start_num: i64,
    start_den: u64,
    end_num: i64,
    end_den: u64,
}
impl RationalInterval {
    pub fn declared(
        start_num: i64,
        start_den: u64,
        end_num: i64,
        end_den: u64,
    ) -> Result<Self, PlanError> {
        if start_den == 0 || end_den == 0 {
            return Err(PlanError::Field);
        }
        Ok(Self {
            start_num,
            start_den,
            end_num,
            end_den,
        })
    }
    pub const fn start_num(self) -> i64 {
        self.start_num
    }
    pub const fn start_den(self) -> u64 {
        self.start_den
    }
    pub const fn end_num(self) -> i64 {
        self.end_num
    }
    pub const fn end_den(self) -> u64 {
        self.end_den
    }
}
/// Authenticated timebase identity. Absence means no clock, not epoch zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimebaseId {
    identity: StrongDigest,
}
impl TimebaseId {
    pub fn declared(identity: StrongDigest) -> Result<Self, PlanError> {
        if identity.is_zero() {
            return Err(PlanError::Field);
        }
        Ok(Self { identity })
    }
    pub const fn identity(self) -> StrongDigest {
        self.identity
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ResourceEstimate {
    bytes: u64,
    work: u64,
    io: u64,
}
impl ResourceEstimate {
    pub const fn new(bytes: u64, work: u64, io: u64) -> Self {
        Self { bytes, work, io }
    }
    pub const fn bytes(self) -> u64 {
        self.bytes
    }
    pub const fn work(self) -> u64 {
        self.work
    }
    pub const fn io(self) -> u64 {
        self.io
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryRef {
    candidate: CandidateIdentity,
    entry_id: u32,
}
impl EntryRef {
    pub const fn new(candidate: CandidateIdentity, entry_id: u32) -> Self {
        Self {
            candidate,
            entry_id,
        }
    }
    pub const fn candidate(self) -> CandidateIdentity {
        self.candidate
    }
    pub const fn entry_id(self) -> u32 {
        self.entry_id
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanEntrySpec {
    pub candidate: CandidateIdentity,
    pub entry_id: u32,
    pub family: EntryFamily,
    pub role: RepresentationClass,
    pub interval: Option<RationalInterval>,
    pub timebase: Option<TimebaseId>,
    pub estimate: ResourceEstimate,
    pub obligation: Obligation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanEntry {
    candidate: CandidateIdentity,
    entry_id: u32,
    family: EntryFamily,
    role: RepresentationClass,
    interval: Option<RationalInterval>,
    timebase: Option<TimebaseId>,
    estimate: ResourceEstimate,
    obligation: Obligation,
}
impl PlanEntry {
    pub fn new(spec: PlanEntrySpec) -> Result<Self, PlanError> {
        if spec.candidate.descriptor_id().is_zero() || spec.candidate.context_id().is_zero() {
            return Err(PlanError::Identity);
        }
        Ok(Self {
            candidate: spec.candidate,
            entry_id: spec.entry_id,
            family: spec.family,
            role: spec.role,
            interval: spec.interval,
            timebase: spec.timebase,
            estimate: spec.estimate,
            obligation: spec.obligation,
        })
    }
    pub const fn candidate(&self) -> CandidateIdentity {
        self.candidate
    }
    pub const fn entry_id(&self) -> u32 {
        self.entry_id
    }
    pub const fn family(&self) -> EntryFamily {
        self.family
    }
    pub const fn role(&self) -> RepresentationClass {
        self.role
    }
    pub const fn interval(&self) -> Option<RationalInterval> {
        self.interval
    }
    pub const fn timebase(&self) -> Option<TimebaseId> {
        self.timebase
    }
    pub const fn estimate(&self) -> ResourceEstimate {
        self.estimate
    }
    pub const fn obligation(&self) -> Obligation {
        self.obligation
    }
    pub const fn key(&self) -> EntryRef {
        EntryRef::new(self.candidate, self.entry_id)
    }
}
/// `from` depends on `to`. Bind by candidate identity, never by filename.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DependencyEdge {
    from: EntryRef,
    to: EntryRef,
    obligation: Obligation,
    reason: DependencyReason,
}
impl DependencyEdge {
    pub fn bind(
        from: &PlanEntry,
        to: &PlanEntry,
        obligation: Obligation,
        reason: DependencyReason,
    ) -> Result<Self, PlanError> {
        if from.candidate() != to.candidate() {
            return Err(PlanError::Identity);
        }
        if from.entry_id() == to.entry_id() {
            return Err(PlanError::Graph);
        }
        if reason == DependencyReason::Clock && !declared_shared_clock(from, to) {
            return Err(PlanError::Refused);
        }
        Ok(Self {
            from: from.key(),
            to: to.key(),
            obligation,
            reason,
        })
    }
    pub const fn from_ref(self) -> EntryRef {
        self.from
    }
    pub const fn to_ref(self) -> EntryRef {
        self.to
    }
    pub const fn from_id(self) -> u32 {
        self.from.entry_id
    }
    pub const fn to_id(self) -> u32 {
        self.to.entry_id
    }
    pub const fn obligation(self) -> Obligation {
        self.obligation
    }
    pub const fn reason(self) -> DependencyReason {
        self.reason
    }
}
fn declared_shared_clock(a: &PlanEntry, b: &PlanEntry) -> bool {
    match (a.timebase(), b.timebase()) {
        (Some(ta), Some(tb)) => ta == tb,
        _ => false,
    }
}
pub struct Inventory<'a> {
    candidate: CandidateIdentity,
    entries: &'a [PlanEntry],
    edges: &'a [DependencyEdge],
}
impl<'a> Inventory<'a> {
    pub fn compile(
        candidate: CandidateIdentity,
        entries: &'a [PlanEntry],
        edges: &'a [DependencyEdge],
    ) -> Result<Self, PlanError> {
        if candidate.descriptor_id().is_zero() || candidate.context_id().is_zero() {
            return Err(PlanError::Identity);
        }
        if entries.is_empty() || entries.len() > MAX_PLAN_ENTRIES || edges.len() > MAX_PLAN_EDGES {
            return Err(PlanError::Limit);
        }
        for entry in entries {
            if entry.candidate() != candidate {
                return Err(PlanError::Identity);
            }
        }
        for i in 0..entries.len() {
            for j in 0..i {
                if entries[i].entry_id() == entries[j].entry_id() {
                    return Err(PlanError::DuplicateId);
                }
            }
        }
        for (i, edge) in edges.iter().enumerate() {
            if edge.from.candidate != candidate || edge.to.candidate != candidate {
                return Err(PlanError::Identity);
            }
            let from = index_of(entries, edge.from_id()).ok_or(PlanError::Graph)?;
            let to = index_of(entries, edge.to_id()).ok_or(PlanError::Graph)?;
            if from == to {
                return Err(PlanError::Graph);
            }
            for prior in &edges[..i] {
                if prior.from_id() == edge.from_id() && prior.to_id() == edge.to_id() {
                    return Err(PlanError::Graph);
                }
            }
            if edge.reason == DependencyReason::Clock
                && !declared_shared_clock(&entries[from], &entries[to])
            {
                return Err(PlanError::Refused);
            }
        }
        acyclic(entries, edges)?;
        Ok(Self {
            candidate,
            entries,
            edges,
        })
    }
    pub const fn candidate(&self) -> CandidateIdentity {
        self.candidate
    }
    pub const fn entries(&self) -> &'a [PlanEntry] {
        self.entries
    }
    pub const fn edges(&self) -> &'a [DependencyEdge] {
        self.edges
    }
    pub const fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn entry(&self, id: u32) -> Option<&'a PlanEntry> {
        self.entries.iter().find(|e| e.entry_id() == id)
    }
    pub fn index_of(&self, id: u32) -> Option<usize> {
        index_of(self.entries, id)
    }
    /// True only when both entries named the same authenticated timebase.
    pub fn share_clock(&self, a: u32, b: u32) -> bool {
        match (self.entry(a), self.entry(b)) {
            (Some(x), Some(y)) => declared_shared_clock(x, y),
            _ => false,
        }
    }
}
fn index_of(entries: &[PlanEntry], id: u32) -> Option<usize> {
    entries.iter().position(|e| e.entry_id() == id)
}
fn acyclic(entries: &[PlanEntry], edges: &[DependencyEdge]) -> Result<(), PlanError> {
    let n = entries.len();
    let mut indeg = [0u16; MAX_PLAN_ENTRIES];
    for edge in edges {
        let to = index_of(entries, edge.to_id()).ok_or(PlanError::Graph)?;
        indeg[to] = indeg[to].checked_add(1).ok_or(PlanError::Limit)?;
    }
    let mut queue = [0usize; MAX_PLAN_ENTRIES];
    let mut qh = 0usize;
    let mut qt = 0usize;
    for i in 0..n {
        if indeg[i] == 0 {
            queue[qt] = i;
            qt += 1;
        }
    }
    let mut seen = 0usize;
    while qh < qt {
        let i = queue[qh];
        qh += 1;
        seen += 1;
        let id = entries[i].entry_id();
        for edge in edges {
            if edge.from_id() != id {
                continue;
            }
            let to = index_of(entries, edge.to_id()).ok_or(PlanError::Graph)?;
            indeg[to] = indeg[to].checked_sub(1).ok_or(PlanError::Graph)?;
            if indeg[to] == 0 {
                queue[qt] = to;
                qt += 1;
            }
        }
    }
    if seen != n {
        return Err(PlanError::Cycle);
    }
    Ok(())
}
