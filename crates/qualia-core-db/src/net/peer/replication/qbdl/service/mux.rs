//! One session walks a compiled mixed-HMC [`Schedule`]: at most one record unit
//! per `next`. Control is never starved. Shared dependencies are charged once.
//! This module does not mint Kind 13+, copy Open/Credit/RequestLeaves, or admit
//! codecs — it transfers already-selected leaves over [`transfer_one_leaf`]'s body.
use super::{map_policy, Error, PlainClient, PlainProvider, ServiceIo};
use crate::bundle::stream::plan::{
    evaluate, EntryFamily, Inventory, Obligation, PlanError, PrincipalSelection, Readiness,
    RecordUnit, Region, Schedule, ScheduledUnit, WorkClass, MAX_PLAN_ENTRIES, MAX_SCHEDULE_ITEMS,
};
use crate::bundle::stream::policy::{LocalEffectControl, LocalWholeSourceGrant};
use crate::crypto::network::authority::PinAuthority;

fn map_plan(e: PlanError) -> Error {
    match e {
        PlanError::Capacity | PlanError::Limit => Error::Limit,
        PlanError::DuplicateId | PlanError::Identity | PlanError::Cycle | PlanError::Graph => {
            Error::Identity
        }
        PlanError::Field | PlanError::Unsupported => Error::Unsupported,
        PlanError::Refused => Error::Admission,
    }
}

fn deferred_family(family: Option<EntryFamily>) -> bool {
    matches!(family, Some(EntryFamily::M64 | EntryFamily::P64))
}

/// Compiled fair schedule plus the readiness gate for this walk.
pub struct MuxTransfer {
    schedule: Schedule,
    blocked: Option<Error>,
    shared_reused: u16,
    cancelled_skipped: u16,
    deferred_pending: u16,
    deferred_ids: [u32; MAX_PLAN_ENTRIES],
    deferred_len: usize,
}

/// One selected publication entry bound to its plain provider.
pub struct BoundEntry<'src, 'sel, 'nodes> {
    pub entry_id: u32,
    pub provider: PlainProvider<'src, 'sel, 'nodes>,
}

/// Counts from one `transfer_scheduled` walk. No `Debug` — tests `match` errors.
pub struct MuxReport {
    pub transferred: u16,
    pub control: u16,
    pub delayed_deferred: u16,
    pub cancelled_skipped: u16,
    pub shared_reused: u16,
    pub order: [Option<EntryFamily>; MAX_SCHEDULE_ITEMS],
    pub order_len: u16,
}

impl MuxTransfer {
    pub fn compile(
        inventory: &Inventory<'_>,
        selections: &[PrincipalSelection<'_>],
        present: &[u32],
    ) -> Result<Self, Error> {
        let (blocked, shared_reused, deferred_ids, deferred_len) =
            inspect(inventory, selections, present);
        let schedule = Schedule::compile(inventory, selections, present).map_err(map_plan)?;
        Ok(Self {
            schedule,
            blocked,
            shared_reused,
            cancelled_skipped: 0,
            deferred_pending: deferred_len as u16,
            deferred_ids,
            deferred_len,
        })
    }

    pub fn next_unit(&mut self) -> Option<ScheduledUnit> {
        self.schedule.next()
    }

    pub fn acknowledge(&mut self, unit: ScheduledUnit) -> Result<(), Error> {
        self.schedule
            .acknowledge_complete(unit.principal(), unit.entry_id(), unit.unit().leaf())
            .map_err(map_plan)
    }

    pub fn acknowledge_complete(
        &mut self,
        principal: u64,
        entry_id: Option<u32>,
        leaf: u64,
    ) -> Result<(), Error> {
        self.schedule
            .acknowledge_complete(principal, entry_id, leaf)
            .map_err(map_plan)
    }

    pub fn cancel(&mut self, entry_id: u32, region: Option<Region>) -> usize {
        let n = self.schedule.cancel(entry_id, region);
        if n > 0 {
            self.cancelled_skipped = self.cancelled_skipped.saturating_add(n as u16);
            if self.is_deferred_id(entry_id) {
                self.deferred_pending = self.deferred_pending.saturating_sub(1);
            }
        }
        n
    }

    pub fn occupancy_retained(&self, principal: u64, entry_id: Option<u32>, leaf: u64) -> bool {
        self.schedule.occupancy_retained(principal, entry_id, leaf)
    }

    pub fn push_control(&mut self, principal: u64, unit: RecordUnit) -> Result<(), Error> {
        self.schedule.push_control(principal, unit).map_err(map_plan)
    }

    pub fn is_charged(&self, entry_id: u32) -> bool {
        self.schedule.is_charged(self.schedule.candidate(), entry_id)
    }

    pub const fn cancelled_skipped(&self) -> u16 {
        self.cancelled_skipped
    }

    pub const fn shared_reused(&self) -> u16 {
        self.shared_reused
    }

    pub const fn len(&self) -> usize {
        self.schedule.len()
    }

    fn is_deferred_id(&self, entry_id: u32) -> bool {
        self.deferred_ids[..self.deferred_len]
            .iter()
            .any(|id| *id == entry_id)
    }
}

/// Drive `next` until empty. Control acknowledges with no payload. Entry units
/// look up [`BoundEntry`] by id, then the `transfer_one_leaf` body for that leaf.
/// One grant scopes the walk: `LocalWholeSourceGrant` is one-shot, so the loop
/// calls `run_one_leaf` (the same body) and drops the effect at the end.
#[allow(clippy::too_many_arguments)]
pub fn transfer_scheduled<PS: ServiceIo, CS: ServiceIo>(
    mux: &mut MuxTransfer,
    entries: &[BoundEntry<'_, '_, '_>],
    clients: &mut [PlainClient<'_>],
    provider_session: &mut PS,
    client_session: &mut CS,
    grant: &mut LocalWholeSourceGrant<'_, '_>,
    pins: &PinAuthority,
    clock: impl Fn() -> u64,
    payload_out: &mut [u8],
) -> Result<MuxReport, Error> {
    if let Some(e) = mux.blocked {
        return Err(e);
    }
    match grant.accept_local_scoped(pins, clock, |authority| {
        Ok(drive_scheduled(
            mux,
            entries,
            clients,
            provider_session,
            client_session,
            authority,
            payload_out,
        ))
    }) {
        Ok(inner) => inner,
        Err(e) => Err(map_policy(e)),
    }
}

fn drive_scheduled<PS: ServiceIo, CS: ServiceIo>(
    mux: &mut MuxTransfer,
    entries: &[BoundEntry<'_, '_, '_>],
    clients: &mut [PlainClient<'_>],
    provider_session: &mut PS,
    client_session: &mut CS,
    authority: &mut LocalEffectControl<'_, '_>,
    payload_out: &mut [u8],
) -> Result<MuxReport, Error> {
    let mut report = MuxReport {
        transferred: 0,
        control: 0,
        delayed_deferred: 0,
        cancelled_skipped: mux.cancelled_skipped,
        shared_reused: mux.shared_reused,
        order: [None; MAX_SCHEDULE_ITEMS],
        order_len: 0,
    };
    let mut sent = [None; MAX_SCHEDULE_ITEMS];
    let mut sent_len = 0usize;
    while let Some(unit) = mux.next_unit() {
        push_order(&mut report, unit.family());
        if unit.entry_id().is_none() || unit.class() == WorkClass::Control {
            if mux.deferred_pending > 0 {
                report.delayed_deferred = report.delayed_deferred.saturating_add(1);
            }
            mux.acknowledge(unit)?;
            report.control = report.control.saturating_add(1);
            continue;
        }
        let id = match unit.entry_id() {
            Some(id) => id,
            None => return Err(Error::Identity),
        };
        let leaf = unit.unit().leaf();
        if sent[..sent_len].iter().any(|slot| *slot == Some((id, leaf))) {
            report.shared_reused = report.shared_reused.saturating_add(1);
            mux.acknowledge(unit)?;
            continue;
        }
        let idx = match entries.iter().position(|e| e.entry_id == id) {
            Some(i) => i,
            None => return Err(Error::Identity),
        };
        if idx >= clients.len() {
            return Err(Error::Identity);
        }
        let deferred = deferred_family(unit.family());
        if !deferred && mux.deferred_pending > 0 {
            report.delayed_deferred = report.delayed_deferred.saturating_add(1);
        }
        let served = super::run_one_leaf(
            &entries[idx].provider,
            &clients[idx],
            provider_session,
            client_session,
            authority,
            leaf,
            payload_out,
        )?;
        if served.leaf != leaf || served.entry != id {
            return Err(Error::Identity);
        }
        if sent_len < MAX_SCHEDULE_ITEMS {
            sent[sent_len] = Some((id, leaf));
            sent_len += 1;
        }
        if deferred {
            mux.deferred_pending = mux.deferred_pending.saturating_sub(1);
        }
        mux.acknowledge(unit)?;
        report.transferred = report.transferred.saturating_add(1);
    }
    Ok(report)
}

fn push_order(report: &mut MuxReport, family: Option<EntryFamily>) {
    let i = report.order_len as usize;
    if i < MAX_SCHEDULE_ITEMS {
        report.order[i] = family;
        report.order_len = report.order_len.saturating_add(1);
    }
}

fn inspect(
    inventory: &Inventory<'_>,
    selections: &[PrincipalSelection<'_>],
    present: &[u32],
) -> (Option<Error>, u16, [u32; MAX_PLAN_ENTRIES], usize) {
    let mut blocked = None;
    let mut cover = [0u8; MAX_PLAN_ENTRIES];
    for sel in selections {
        let mut seen = [false; MAX_PLAN_ENTRIES];
        for intent in sel.request().intents() {
            match evaluate(
                inventory,
                intent.entry_id(),
                Some(intent.family()),
                present,
            ) {
                Readiness::Refused | Readiness::Incomplete { .. } => {
                    blocked = Some(Error::Admission);
                }
                Readiness::Ready | Readiness::Opaque => {}
            }
            walk_ready(
                inventory,
                intent.entry_id(),
                present,
                &mut seen,
                &mut cover,
            );
        }
    }
    let mut shared = 0u16;
    for c in cover {
        if c > 1 {
            shared = shared.saturating_add(u16::from(c - 1));
        }
    }
    let mut deferred_ids = [0u32; MAX_PLAN_ENTRIES];
    let mut deferred_len = 0usize;
    for (i, entry) in inventory.entries().iter().enumerate() {
        if cover[i] == 0 || !deferred_family(Some(entry.family())) {
            continue;
        }
        if deferred_len < MAX_PLAN_ENTRIES {
            deferred_ids[deferred_len] = entry.entry_id();
            deferred_len += 1;
        }
    }
    (blocked, shared, deferred_ids, deferred_len)
}

fn walk_ready(
    inventory: &Inventory<'_>,
    start: u32,
    present: &[u32],
    seen: &mut [bool; MAX_PLAN_ENTRIES],
    cover: &mut [u8; MAX_PLAN_ENTRIES],
) {
    let mut stack = [0u32; MAX_PLAN_ENTRIES];
    let mut sp = 0usize;
    push_id(inventory, &mut stack, &mut sp, seen, start);
    while sp > 0 {
        sp -= 1;
        let id = stack[sp];
        let Some(entry) = inventory.entry(id) else {
            continue;
        };
        let readiness = evaluate(inventory, id, Some(entry.family()), present);
        if readiness == Readiness::Ready {
            if let Some(idx) = inventory.index_of(id) {
                cover[idx] = cover[idx].saturating_add(1);
            }
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
                push_id(inventory, &mut stack, &mut sp, seen, dep);
            }
        }
    }
}

fn push_id(
    inventory: &Inventory<'_>,
    stack: &mut [u32; MAX_PLAN_ENTRIES],
    sp: &mut usize,
    seen: &mut [bool; MAX_PLAN_ENTRIES],
    id: u32,
) {
    let Some(idx) = inventory.index_of(id) else {
        return;
    };
    if seen[idx] {
        return;
    }
    if *sp >= MAX_PLAN_ENTRIES {
        return;
    }
    seen[idx] = true;
    stack[*sp] = id;
    *sp += 1;
}

#[cfg(test)]
#[path = "mux_tests.rs"]
mod mux_tests;
