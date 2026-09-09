//! Caller-backed adjacency and `plan_routes` before any forwarding publish.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::route::hysteresis::{admit_healed_routes, HopHoldDown};
use crate::net::qdnf::route::{
    insert_edge, plan_routes, AdjacencyIndex, CandidatePath, ForwardingGeneration, PathConstraint,
    SpfTable, ValidatedEdge,
};

use super::{
    MultiHop, DEST_NODE, DEST_SLOT, MIDDLE_NODE, MIDDLE_SLOT, NODE_COUNT, ORIGIN_NODE, ORIGIN_SLOT,
};

pub(super) const PLAN_NOW: u64 = 1;
const SCOPE: u64 = 1;
const EXPIRY: u64 = 100;
pub(super) const FORBIDDEN_REALM: u8 = 1;
pub(super) const PERMITTED_REALM: u8 = 0;
const CHEAP_COST: u16 = 1;
const ALLOWED_COST: u16 = 50;
const MIN_PROFILE: u8 = 2;

fn edge(
    origin: u8,
    from: u8,
    to: u8,
    seq: u32,
    cost: u16,
    realm: u8,
    profile: u8,
) -> ValidatedEdge {
    ValidatedEdge {
        origin,
        from,
        to,
        sequence: seq,
        expiry_unix: EXPIRY,
        withdrawn: false,
        bidirectional: true,
        cost,
        latency_ms: 1,
        energy_uj: 1,
        energy_known: true,
        realm_bit: realm,
        profile,
        failure_domain: from,
        scope: SCOPE,
    }
}

/// Production 1→2→3 chain. Node 0 is never used (`plan_routes` returns `Range`).
pub(super) fn line_index() -> Result<AdjacencyIndex, QdnfError> {
    let mut index = AdjacencyIndex::new(SCOPE);
    insert_edge(&mut index, edge(1, ORIGIN_NODE, MIDDLE_NODE, 1, 1, 0, 0))?;
    insert_edge(&mut index, edge(2, MIDDLE_NODE, DEST_NODE, 1, 1, 0, 0))?;
    Ok(index)
}

/// Cheap 1→3 shortcut (forbidden realm / P0) plus expensive permitted 1→2→3.
pub(super) fn shortcut_index() -> Result<AdjacencyIndex, QdnfError> {
    let mut index = AdjacencyIndex::new(SCOPE);
    insert_edge(
        &mut index,
        edge(1, ORIGIN_NODE, DEST_NODE, 1, CHEAP_COST, FORBIDDEN_REALM, 0),
    )?;
    insert_edge(
        &mut index,
        edge(
            1,
            ORIGIN_NODE,
            MIDDLE_NODE,
            2,
            ALLOWED_COST,
            PERMITTED_REALM,
            MIN_PROFILE,
        ),
    )?;
    insert_edge(
        &mut index,
        edge(
            2,
            MIDDLE_NODE,
            DEST_NODE,
            1,
            ALLOWED_COST,
            PERMITTED_REALM,
            MIN_PROFILE,
        ),
    )?;
    Ok(index)
}

pub(super) fn realm_constraint() -> PathConstraint {
    PathConstraint {
        permitted_realms: 1u64 << PERMITTED_REALM,
        min_profile: 0,
        max_cost: u16::MAX,
        deadline_ms: 0,
        require_loop_free: true,
        max_energy_uj: 0,
    }
}

pub(super) fn profile_constraint() -> PathConstraint {
    PathConstraint {
        permitted_realms: u64::MAX,
        min_profile: MIN_PROFILE,
        max_cost: u16::MAX,
        deadline_ms: 0,
        require_loop_free: true,
        max_energy_uj: 0,
    }
}

fn table_from_planned(path: &CandidatePath) -> Result<SpfTable, QdnfError> {
    if path.dest == 0 || path.first_hop == 0 {
        return Err(QdnfError::Range);
    }
    if path.hop_len == 0 {
        return Err(QdnfError::Incomplete);
    }
    let mut table = SpfTable::EMPTY;
    table.nodes = 1;
    table.dest[0] = path.dest;
    table.hop_counts[0] = 1;
    table.hops[0][0].node = path.first_hop;
    table.hops[0][0].cost = path.metrics.cost;
    Ok(table)
}

fn local_table(node: u8) -> Result<SpfTable, QdnfError> {
    if node == 0 {
        return Err(QdnfError::Range);
    }
    let mut table = SpfTable::EMPTY;
    table.nodes = 1;
    table.dest[0] = node;
    table.hop_counts[0] = 0;
    Ok(table)
}

fn plan_one(
    index: &AdjacencyIndex,
    origin: u8,
    dest: u8,
    constraint: &PathConstraint,
) -> Result<CandidatePath, QdnfError> {
    let mut out = [CandidatePath::EMPTY; 3];
    let n = plan_routes(index, origin, dest, constraint, PLAN_NOW, &mut out)?;
    if n == 0 {
        return Err(QdnfError::NoRoute);
    }
    Ok(out[0])
}

/// Publish per-node generations only after `plan_routes` succeeds.
pub(super) fn publish_planned(
    gens: &mut [ForwardingGeneration; NODE_COUNT],
    index: &AdjacencyIndex,
    constraint: &PathConstraint,
) -> Result<u8, QdnfError> {
    let origin = plan_one(index, ORIGIN_NODE, DEST_NODE, constraint)?;
    if origin.first_hop != MIDDLE_NODE {
        return Err(QdnfError::NoRoute);
    }
    gens[ORIGIN_SLOT].publish(table_from_planned(&origin)?)?;
    let middle = plan_one(index, MIDDLE_NODE, DEST_NODE, constraint)?;
    gens[MIDDLE_SLOT].publish(table_from_planned(&middle)?)?;
    gens[DEST_SLOT].publish(local_table(DEST_NODE)?)?;
    Ok(origin.hop_len)
}

pub(super) fn publish_line(m: &mut MultiHop) -> Result<(), QdnfError> {
    let index = line_index()?;
    let origin = plan_one(&index, ORIGIN_NODE, DEST_NODE, &PathConstraint::UNRESTRICTED)?;
    m.planned_hops = publish_planned(&mut m.gens, &index, &PathConstraint::UNRESTRICTED)?;
    m.current_path = origin;
    m.last_change_unix = PLAN_NOW;
    m.held = HopHoldDown::EMPTY;
    Ok(())
}

fn unpublish(m: &mut MultiHop) {
    m.gens = [
        ForwardingGeneration::empty(1),
        ForwardingGeneration::empty(2),
        ForwardingGeneration::empty(3),
    ];
    m.planned_hops = 0;
}

/// Partition the middle next-hop. Forwarding is unpublished and the hop is held down.
pub(super) fn partition_next_hop(m: &mut MultiHop, now_unix: u64) -> Result<(), QdnfError> {
    m.held = HopHoldDown {
        hop: MIDDLE_NODE,
        last_down_unix: now_unix,
    };
    m.current_path = CandidatePath::EMPTY;
    m.last_change_unix = now_unix;
    unpublish(m);
    Ok(())
}

/// Heal 1→2→3 advertisements. Hold-down still blocks the flapping first hop.
pub(super) fn heal_next_hop(m: &mut MultiHop, now_unix: u64) -> Result<(), QdnfError> {
    let index = line_index()?;
    let constraint = PathConstraint::UNRESTRICTED;
    let mut planned = [CandidatePath::EMPTY; 3];
    let n = match plan_routes(
        &index,
        ORIGIN_NODE,
        DEST_NODE,
        &constraint,
        now_unix.max(PLAN_NOW),
        &mut planned,
    ) {
        Ok(k) => k,
        Err(QdnfError::NoRoute) => 0,
        Err(e) => return Err(e),
    };
    let mut admitted = [CandidatePath::EMPTY; 3];
    match admit_healed_routes(
        &m.current_path,
        &planned[..n],
        &[m.held],
        &constraint,
        &m.policy,
        now_unix,
        m.last_change_unix,
        &mut admitted,
    ) {
        Ok(k) if k > 0 && admitted[0].first_hop == MIDDLE_NODE => {
            m.planned_hops = publish_admitted(&mut m.gens, &index, &admitted[0], &constraint)?;
            m.current_path = admitted[0];
            m.last_change_unix = now_unix;
            Ok(())
        }
        Ok(_) | Err(QdnfError::NoRoute) => {
            unpublish(m);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn publish_admitted(
    gens: &mut [ForwardingGeneration; NODE_COUNT],
    index: &AdjacencyIndex,
    admitted: &CandidatePath,
    constraint: &PathConstraint,
) -> Result<u8, QdnfError> {
    if admitted.first_hop != MIDDLE_NODE {
        return Err(QdnfError::NoRoute);
    }
    gens[ORIGIN_SLOT].publish(table_from_planned(admitted)?)?;
    let middle = plan_one(index, MIDDLE_NODE, DEST_NODE, constraint)?;
    gens[MIDDLE_SLOT].publish(table_from_planned(&middle)?)?;
    gens[DEST_SLOT].publish(local_table(DEST_NODE)?)?;
    Ok(admitted.hop_len)
}

pub(super) fn plan_unrestricted(index: &AdjacencyIndex) -> Result<CandidatePath, QdnfError> {
    plan_one(index, ORIGIN_NODE, DEST_NODE, &PathConstraint::UNRESTRICTED)
}

pub(super) fn plan_with(
    index: &AdjacencyIndex,
    constraint: &PathConstraint,
) -> Result<CandidatePath, QdnfError> {
    plan_one(index, ORIGIN_NODE, DEST_NODE, constraint)
}
