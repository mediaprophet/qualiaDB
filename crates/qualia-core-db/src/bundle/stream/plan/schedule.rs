//! Fair record-boundary schedule. Occupancy is an intent flag, not a ledger.
use super::inventory::{
    EntryFamily, EntryRef, Inventory, Obligation, PlanEntry, RepresentationClass, MAX_PLAN_ENTRIES,
};
use super::readiness::{evaluate, Readiness};
use super::PlanError;
use crate::bundle::stream::descriptor::CandidateIdentity;
pub const MAX_SCHEDULE_ITEMS: usize = 64;
pub const MAX_PRINCIPALS: usize = 8;
/// Semantic class for this planner; not `net::peer::runtime::WorkClass`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkClass {
    Control,
    Metadata,
    BaseRepresentation,
    Enhancement,
}
impl WorkClass {
    fn of(entry: &PlanEntry) -> Self {
        match entry.role() {
            RepresentationClass::Control => Self::Control,
            RepresentationClass::Metadata => Self::Metadata,
            RepresentationClass::Base => Self::BaseRepresentation,
            RepresentationClass::Enhancement | RepresentationClass::Alternative => {
                Self::Enhancement
            }
        }
    }
}
fn deferred_family(family: Option<EntryFamily>) -> bool {
    matches!(family, Some(EntryFamily::M64 | EntryFamily::P64))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    start: u64,
    end: u64,
}
impl Region {
    pub fn declared(start: u64, end: u64) -> Result<Self, PlanError> {
        if end <= start {
            return Err(PlanError::Field);
        }
        Ok(Self { start, end })
    }
    pub const fn start(self) -> u64 {
        self.start
    }
    pub const fn end(self) -> u64 {
        self.end
    }
    pub const fn contains(self, leaf: u64) -> bool {
        leaf >= self.start && leaf < self.end
    }
}
/// One admitted record-boundary unit. `next` never yields more than one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordUnit {
    leaf: u64,
}
impl RecordUnit {
    pub const fn at(leaf: u64) -> Self {
        Self { leaf }
    }
    pub const fn leaf(self) -> u64 {
        self.leaf
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectionIntent {
    entry_id: u32,
    family: EntryFamily,
    interval: Option<super::RationalInterval>,
    region: Option<Region>,
}
impl SelectionIntent {
    pub fn scene(
        entry_id: u32,
        family: EntryFamily,
        interval: Option<super::RationalInterval>,
        region: Option<Region>,
    ) -> Result<Self, PlanError> {
        Ok(Self {
            entry_id,
            family,
            interval,
            region,
        })
    }
    pub const fn entry_id(self) -> u32 {
        self.entry_id
    }
    pub const fn family(self) -> EntryFamily {
        self.family
    }
    pub const fn interval(self) -> Option<super::RationalInterval> {
        self.interval
    }
    pub const fn region(self) -> Option<Region> {
        self.region
    }
}
/// Scene or analysis intent. Not an M64 path or filename.
#[derive(Clone, Copy, Debug)]
pub struct SelectionRequest<'a> {
    intents: &'a [SelectionIntent],
}
impl<'a> SelectionRequest<'a> {
    pub fn scene(intents: &'a [SelectionIntent]) -> Result<Self, PlanError> {
        if intents.is_empty() || intents.len() > MAX_PLAN_ENTRIES {
            return Err(PlanError::Limit);
        }
        for i in 0..intents.len() {
            for j in 0..i {
                if intents[i].entry_id == intents[j].entry_id
                    && intents[i].region == intents[j].region
                {
                    return Err(PlanError::DuplicateId);
                }
            }
        }
        Ok(Self { intents })
    }
    pub const fn intents(&self) -> &'a [SelectionIntent] {
        self.intents
    }
}
#[derive(Clone, Copy, Debug)]
pub struct PrincipalSelection<'a> {
    principal: u64,
    request: SelectionRequest<'a>,
}
impl<'a> PrincipalSelection<'a> {
    pub fn new(principal: u64, request: SelectionRequest<'a>) -> Result<Self, PlanError> {
        if principal == 0 {
            return Err(PlanError::Field);
        }
        Ok(Self { principal, request })
    }
    pub const fn principal(&self) -> u64 {
        self.principal
    }
    pub const fn request(&self) -> &SelectionRequest<'a> {
        &self.request
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScheduledUnit {
    principal: u64,
    class: WorkClass,
    family: Option<EntryFamily>,
    entry_id: Option<u32>,
    unit: RecordUnit,
}
impl ScheduledUnit {
    pub const fn principal(self) -> u64 {
        self.principal
    }
    pub const fn class(self) -> WorkClass {
        self.class
    }
    pub const fn family(self) -> Option<EntryFamily> {
        self.family
    }
    pub const fn entry_id(self) -> Option<u32> {
        self.entry_id
    }
    pub const fn unit(self) -> RecordUnit {
        self.unit
    }
}
#[derive(Clone, Copy)]
struct WorkItem {
    principal: u64,
    entry_id: Option<u32>,
    family: Option<EntryFamily>,
    class: WorkClass,
    unit: RecordUnit,
    key: Option<EntryRef>,
    cancelled: bool,
    occupancy: bool,
    issued: bool,
    ready: bool,
}
impl WorkItem {
    const VACANT: Self = Self {
        principal: 0,
        entry_id: None,
        family: None,
        class: WorkClass::Control,
        unit: RecordUnit::at(0),
        key: None,
        cancelled: false,
        occupancy: false,
        issued: false,
        ready: false,
    };
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum DeferFilter {
    Live,
    Deferred,
}
pub struct Schedule {
    candidate: CandidateIdentity,
    items: [WorkItem; MAX_SCHEDULE_ITEMS],
    len: usize,
    principals: [u64; MAX_PRINCIPALS],
    principal_count: usize,
    principal_cursor: usize,
    charged: [Option<EntryRef>; MAX_SCHEDULE_ITEMS],
    charged_len: usize,
    charged_bytes: u64,
}
impl Schedule {
    pub fn compile(
        inventory: &Inventory<'_>,
        selections: &[PrincipalSelection<'_>],
        present: &[u32],
    ) -> Result<Self, PlanError> {
        if selections.is_empty() || selections.len() > MAX_PRINCIPALS {
            return Err(PlanError::Limit);
        }
        let mut plan = Self {
            candidate: inventory.candidate(),
            items: [WorkItem::VACANT; MAX_SCHEDULE_ITEMS],
            len: 0,
            principals: [0; MAX_PRINCIPALS],
            principal_count: 0,
            principal_cursor: 0,
            charged: [None; MAX_SCHEDULE_ITEMS],
            charged_len: 0,
            charged_bytes: 0,
        };
        for sel in selections {
            plan.ensure_principal(sel.principal())?;
            let mut leaves = [0u64; MAX_PLAN_ENTRIES];
            for intent in sel.request().intents() {
                let Some(entry) = inventory.entry(intent.entry_id()) else {
                    return Err(PlanError::Graph);
                };
                if entry.family() != intent.family() {
                    return Err(PlanError::Identity);
                }
                if let (Some(want), Some(have)) = (intent.interval(), entry.interval()) {
                    if want != have {
                        return Err(PlanError::Field);
                    }
                }
                if let Some(idx) = inventory.index_of(intent.entry_id()) {
                    if let Some(region) = intent.region() {
                        leaves[idx] = region.start();
                    }
                }
                close_selection(
                    &mut plan,
                    inventory,
                    sel.principal(),
                    intent,
                    present,
                    &leaves,
                )?;
            }
        }
        Ok(plan)
    }
    pub fn enqueue_unit(
        &mut self,
        principal: u64,
        entry: &PlanEntry,
        unit: RecordUnit,
        ready: bool,
    ) -> Result<(), PlanError> {
        if entry.candidate() != self.candidate {
            return Err(PlanError::Identity);
        }
        self.ensure_principal(principal)?;
        self.push_item(
            principal,
            Some(entry.entry_id()),
            Some(entry.family()),
            WorkClass::of(entry),
            unit,
            Some(entry.key()),
            ready,
            entry.estimate().bytes(),
        )
    }
    pub fn push_control(&mut self, principal: u64, unit: RecordUnit) -> Result<(), PlanError> {
        self.ensure_principal(principal)?;
        self.push_item(
            principal,
            None,
            None,
            WorkClass::Control,
            unit,
            None,
            true,
            0,
        )
    }
    /// Mark overlapping units cancelled. Occupancy stays until `acknowledge_complete`.
    pub fn cancel(&mut self, entry_id: u32, region: Option<Region>) -> usize {
        let mut n = 0usize;
        for i in 0..self.len {
            let item = &mut self.items[i];
            if item.entry_id != Some(entry_id) || item.cancelled {
                continue;
            }
            let hit = match region {
                None => true,
                Some(r) => r.contains(item.unit.leaf()),
            };
            if hit {
                item.cancelled = true;
                n += 1;
            }
        }
        n
    }
    /// At most one admitted record-boundary unit. Control is never starved.
    /// M64/P64 wait while Ready EMF/QAC/10D (and other live families) remain.
    pub fn next(&mut self) -> Option<ScheduledUnit> {
        const ORDER: [(WorkClass, DeferFilter); 6] = [
            (WorkClass::Metadata, DeferFilter::Live),
            (WorkClass::BaseRepresentation, DeferFilter::Live),
            (WorkClass::Enhancement, DeferFilter::Live),
            (WorkClass::Metadata, DeferFilter::Deferred),
            (WorkClass::BaseRepresentation, DeferFilter::Deferred),
            (WorkClass::Enhancement, DeferFilter::Deferred),
        ];
        if let Some(i) = self.find_fair(WorkClass::Control, None) {
            return self.take(i);
        }
        for (class, filter) in ORDER {
            if let Some(i) = self.find_fair(class, Some(filter)) {
                return self.take(i);
            }
        }
        None
    }
    pub fn occupancy_retained(&self, principal: u64, entry_id: Option<u32>, leaf: u64) -> bool {
        self.items[..self.len].iter().any(|item| {
            item.principal == principal
                && item.entry_id == entry_id
                && item.unit.leaf() == leaf
                && item.occupancy
        })
    }
    pub fn acknowledge_complete(
        &mut self,
        principal: u64,
        entry_id: Option<u32>,
        leaf: u64,
    ) -> Result<(), PlanError> {
        for item in &mut self.items[..self.len] {
            if item.principal == principal
                && item.entry_id == entry_id
                && item.unit.leaf() == leaf
                && item.occupancy
            {
                item.occupancy = false;
                return Ok(());
            }
        }
        Err(PlanError::Graph)
    }
    pub fn is_charged(&self, candidate: CandidateIdentity, entry_id: u32) -> bool {
        let key = EntryRef::new(candidate, entry_id);
        self.charged[..self.charged_len]
            .iter()
            .any(|slot| *slot == Some(key))
    }
    pub const fn charged_bytes(&self) -> u64 {
        self.charged_bytes
    }
    pub const fn candidate(&self) -> CandidateIdentity {
        self.candidate
    }
    pub const fn len(&self) -> usize {
        self.len
    }
    fn ensure_principal(&mut self, principal: u64) -> Result<usize, PlanError> {
        if principal == 0 {
            return Err(PlanError::Field);
        }
        if let Some(i) = self.principals[..self.principal_count]
            .iter()
            .position(|p| *p == principal)
        {
            return Ok(i);
        }
        if self.principal_count >= MAX_PRINCIPALS {
            return Err(PlanError::Capacity);
        }
        let i = self.principal_count;
        self.principals[i] = principal;
        self.principal_count += 1;
        Ok(i)
    }
    #[allow(clippy::too_many_arguments)]
    fn push_item(
        &mut self,
        principal: u64,
        entry_id: Option<u32>,
        family: Option<EntryFamily>,
        class: WorkClass,
        unit: RecordUnit,
        key: Option<EntryRef>,
        ready: bool,
        bytes: u64,
    ) -> Result<(), PlanError> {
        for item in &self.items[..self.len] {
            if item.principal == principal
                && item.entry_id == entry_id
                && item.unit.leaf() == unit.leaf()
                && item.class == class
            {
                return Ok(());
            }
        }
        if let Some(key) = key {
            if self.items[..self.len]
                .iter()
                .any(|item| item.key == Some(key) && item.unit.leaf() == unit.leaf())
            {
                self.charge(key, bytes)?;
                return Ok(());
            }
            self.charge(key, bytes)?;
        }
        if self.len >= MAX_SCHEDULE_ITEMS {
            return Err(PlanError::Capacity);
        }
        self.items[self.len] = WorkItem {
            principal,
            entry_id,
            family,
            class,
            unit,
            key,
            cancelled: false,
            occupancy: true,
            issued: false,
            ready,
        };
        self.len += 1;
        Ok(())
    }
    fn charge(&mut self, key: EntryRef, bytes: u64) -> Result<(), PlanError> {
        if self.charged[..self.charged_len]
            .iter()
            .any(|slot| *slot == Some(key))
        {
            return Ok(());
        }
        if self.charged_len >= MAX_SCHEDULE_ITEMS {
            return Err(PlanError::Capacity);
        }
        self.charged[self.charged_len] = Some(key);
        self.charged_len += 1;
        self.charged_bytes = self
            .charged_bytes
            .checked_add(bytes)
            .ok_or(PlanError::Limit)?;
        Ok(())
    }
    fn find_fair(&self, class: WorkClass, filter: Option<DeferFilter>) -> Option<usize> {
        if self.principal_count == 0 {
            return None;
        }
        for step in 0..self.principal_count {
            let p = (self.principal_cursor + step) % self.principal_count;
            let principal = self.principals[p];
            let mut best: Option<usize> = None;
            for i in 0..self.len {
                let item = &self.items[i];
                if item.principal != principal
                    || item.class != class
                    || item.cancelled
                    || item.issued
                    || !item.ready
                {
                    continue;
                }
                if let Some(filter) = filter {
                    let deferred = deferred_family(item.family);
                    match filter {
                        DeferFilter::Live if deferred => continue,
                        DeferFilter::Deferred if !deferred => continue,
                        _ => {}
                    }
                }
                best = Some(match best {
                    None => i,
                    Some(j) if precedes(item, &self.items[j]) => i,
                    Some(j) => j,
                });
            }
            if best.is_some() {
                return best;
            }
        }
        None
    }
    fn take(&mut self, index: usize) -> Option<ScheduledUnit> {
        let item = self.items.get_mut(index)?;
        item.issued = true;
        item.occupancy = true;
        let unit = ScheduledUnit {
            principal: item.principal,
            class: item.class,
            family: item.family,
            entry_id: item.entry_id,
            unit: item.unit,
        };
        if let Some(i) = self.principals[..self.principal_count]
            .iter()
            .position(|p| *p == unit.principal)
        {
            self.principal_cursor = (i + 1) % self.principal_count.max(1);
        }
        Some(unit)
    }
}
fn precedes(a: &WorkItem, b: &WorkItem) -> bool {
    match (a.entry_id, b.entry_id) {
        (Some(x), Some(y)) if x != y => x < y,
        _ => a.unit.leaf() < b.unit.leaf(),
    }
}
fn close_selection(
    plan: &mut Schedule,
    inventory: &Inventory<'_>,
    principal: u64,
    intent: &SelectionIntent,
    present: &[u32],
    leaves: &[u64; MAX_PLAN_ENTRIES],
) -> Result<(), PlanError> {
    let mut stack = [0u32; MAX_PLAN_ENTRIES];
    let mut sp = 0usize;
    let mut seen = [false; MAX_PLAN_ENTRIES];
    push_id(inventory, &mut stack, &mut sp, &mut seen, intent.entry_id())?;
    while sp > 0 {
        sp -= 1;
        let id = stack[sp];
        let entry = inventory.entry(id).ok_or(PlanError::Graph)?;
        let readiness = evaluate(inventory, id, Some(entry.family()), present);
        match readiness {
            Readiness::Refused => return Err(PlanError::Refused),
            Readiness::Ready => {
                let leaf = inventory.index_of(id).map(|idx| leaves[idx]).unwrap_or(0);
                plan.enqueue_unit(principal, entry, RecordUnit::at(leaf), true)?;
            }
            Readiness::Incomplete { .. } | Readiness::Opaque => {}
        }
        for edge in inventory.edges() {
            if edge.from_id() != id {
                continue;
            }
            let dep = edge.to_id();
            let include = match edge.obligation() {
                Obligation::Mandatory => true,
                Obligation::Optional => evaluate(inventory, dep, None, present) == Readiness::Ready,
            };
            if include {
                push_id(inventory, &mut stack, &mut sp, &mut seen, dep)?;
            }
        }
    }
    Ok(())
}
fn push_id(
    inventory: &Inventory<'_>,
    stack: &mut [u32; MAX_PLAN_ENTRIES],
    sp: &mut usize,
    seen: &mut [bool; MAX_PLAN_ENTRIES],
    id: u32,
) -> Result<(), PlanError> {
    let idx = inventory.index_of(id).ok_or(PlanError::Graph)?;
    if seen[idx] {
        return Ok(());
    }
    if *sp >= MAX_PLAN_ENTRIES {
        return Err(PlanError::Capacity);
    }
    seen[idx] = true;
    stack[*sp] = id;
    *sp += 1;
    Ok(())
}
