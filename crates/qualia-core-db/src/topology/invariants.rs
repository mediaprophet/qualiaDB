//! Non-recursive, zero-heap topological, modal, and homological invariant verifiers.
//!
//! Tier-1 Zero-Heap (`AGENTS.md` §0 & §0-A): all graph traversals, reachability checks,
//! cycle detectors, and Betti/Euler calculators operate strictly on caller-supplied
//! `visited_bits: &mut [u64]` and `work_queue: &mut [u32]` slices without recursion or heap allocation.

use crate::frame_layout::DEFEATER_BIT;
use crate::modalities::logic::deontic::OP_FORBID;
use crate::tensor::Tensor10D;
use crate::topology::csr_view::HarbouredCsrView;
use crate::topology::delta_ring::{DeltaOpKind, MergedGraphView};
use crate::topology::geometry_view::PoetGeometryView;
use crate::topology::header::{
    AnchorDomain, GraphMorphology, TopologyError, TopologyInvariants,
};

/// Belnap 4-valued paraconsistent `Both` (`⊤` contradiction) opcode byte in `NQuin::predicate[0..7]`.
pub const OP_BELNAP_BOTH: u8 = 0x32;

/// Summary of 1-skeleton homological invariants for a `HarbouredCsrView`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct HomologySummary {
    pub euler_characteristic: i32,
    pub betti_0: u32,
    pub betti_1: u32,
    pub root_count: u32,
    pub sink_count: u32,
}

/// Non-recursive zero-heap reachability gate verifying that inserting `u -> v` into `view`
/// does not introduce a directed cycle (`v -*-> u`).
pub fn check_acyclic_insert<const CAP: usize>(
    view: &MergedGraphView<'_, CAP>,
    u: u32,
    v: u32,
    visited_bits: &mut [u64],
    queue: &mut [u32],
) -> Result<(), TopologyError> {
    let v_count = view.vertex_count();
    if u >= v_count {
        return Err(TopologyError::VertexOutOfRange {
            vertex: u,
            count: v_count,
        });
    }
    if v >= v_count {
        return Err(TopologyError::VertexOutOfRange {
            vertex: v,
            count: v_count,
        });
    }
    if u == v {
        return Err(TopologyError::CycleDetected {
            source: u,
            target: v,
        });
    }

    let n = v_count as usize;
    let words = (n + 63) / 64;
    if visited_bits.len() < words || queue.len() < n {
        return Err(TopologyError::BufferTooSmall);
    }

    visited_bits[..words].fill(0);
    set_bit(visited_bits, v as usize);
    queue[0] = v;
    let mut head = 0usize;
    let mut tail = 1usize;

    while head < tail {
        let curr = queue[head];
        head += 1;

        // 1. Base CSR outgoing neighbours not tombstoned in the delta ring.
        let base_nbrs = view.base().out_neighbors(curr)?;
        let mut i = 0usize;
        while i < base_nbrs.len() {
            let w = base_nbrs[i];
            i += 1;
            if !is_edge_live_in_ring(view, curr, w, true) {
                continue;
            }
            if w == u {
                return Err(TopologyError::CycleDetected {
                    source: u,
                    target: v,
                });
            }
            if !test_bit(visited_bits, w as usize) {
                set_bit(visited_bits, w as usize);
                queue[tail] = w;
                tail += 1;
            }
        }

        // 2. Delta-ring inserted/reified outgoing neighbours from `curr`.
        let entries = view.ring().entries();
        let mut j = 0usize;
        while j < entries.len() {
            let e = &entries[j];
            j += 1;
            if e.graph_slot() as u16 != view.graph_slot() || e.source_vertex != curr {
                continue;
            }
            if matches!(e.op_kind(), DeltaOpKind::Insert | DeltaOpKind::Reify) {
                let w = e.target_vertex;
                if w >= v_count {
                    return Err(TopologyError::VertexOutOfRange {
                        vertex: w,
                        count: v_count,
                    });
                }
                if !is_edge_live_in_ring(view, curr, w, false) {
                    continue;
                }
                if w == u {
                    return Err(TopologyError::CycleDetected {
                        source: u,
                        target: v,
                    });
                }
                if !test_bit(visited_bits, w as usize) {
                    set_bit(visited_bits, w as usize);
                    queue[tail] = w;
                    tail += 1;
                }
            }
        }
    }

    Ok(())
}

/// Verify all active `TopologyInvariants` on `csr` and compute its `HomologySummary`
/// ($\chi, \beta_0, \beta_1$, root count, sink count) with zero heap allocation.
pub fn verify_graph_invariants(
    csr: &HarbouredCsrView<'_>,
    quins: &[crate::NQuin],
    coords_10d: Option<&[Tensor10D]>,
    visited_bits: &mut [u64],
    work_queue: &mut [u32],
) -> Result<HomologySummary, TopologyError> {
    let v_count = csr.vertex_count() as usize;
    let e_count = csr.edge_count() as usize;
    let words = (v_count + 63) / 64;

    if v_count > 0 && (visited_bits.len() < words || work_queue.len() < v_count) {
        return Err(TopologyError::BufferTooSmall);
    }

    let invariants = csr.header().invariants;
    let is_dag = invariants.contains(TopologyInvariants::ACYCLIC)
        || csr.header().morphology == GraphMorphology::DeterministicDAG as u8;

    if is_dag && v_count > 0 {
        verify_acyclic_kahn(csr, visited_bits, work_queue)?;
    }

    let check_temporal = invariants.contains(TopologyInvariants::TEMPORAL_MONOTONIC);
    let check_deontic = invariants.contains(TopologyInvariants::DEONTIC_GUARDED);
    let check_paraconsistent = invariants.contains(TopologyInvariants::PARACONSISTENT_ISOLATED);
    let check_sensitivity = invariants.contains(TopologyInvariants::SENSITIVITY_MONOTONIC);
    let max_sens = csr.header().max_sensitivity_class();

    if check_temporal || check_deontic || check_paraconsistent || check_sensitivity {
        let mut u = 0u32;
        while (u as usize) < v_count {
            let nbrs = csr.out_neighbors(u)?;
            let e_anchors = csr.out_edge_anchors(u)?;
            let mut k = 0usize;
            while k < nbrs.len() {
                let v = nbrs[k];
                let edge_anchor_idx = e_anchors[k];
                k += 1;

                if check_temporal {
                    if let Some(coords) = coords_10d {
                        if (u as usize) < coords.len()
                            && (v as usize) < coords.len()
                            && coords[u as usize].t > coords[v as usize].t
                        {
                            return Err(TopologyError::TemporalMonotonicityViolation {
                                source: u,
                                target: v,
                            });
                        }
                    }
                    if let ( Some(clock_u), Some(clock_v) ) = (
                        vertex_lamport_clock(csr, quins, u),
                        vertex_lamport_clock(csr, quins, v),
                    ) {
                        if clock_u > clock_v {
                            return Err(TopologyError::TemporalMonotonicityViolation {
                                source: u,
                                target: v,
                            });
                        }
                    }
                }

                if let Some(q) = quins.get(edge_anchor_idx as usize) {
                    let opcode = (q.predicate & 0xFF) as u8;
                    if check_deontic && opcode == OP_FORBID && (q.predicate & DEFEATER_BIT) == 0 {
                        return Err(TopologyError::DeonticForbidViolation { edge_anchor_idx });
                    }
                    if check_paraconsistent && opcode == OP_BELNAP_BOTH {
                        return Err(TopologyError::ParaconsistentContagion { edge_anchor_idx });
                    }
                    if check_sensitivity {
                        let edge_sens = (q.context >> 56) as u8;
                        if edge_sens > max_sens {
                            return Err(TopologyError::SensitivityEscalation {
                                source_class: max_sens,
                                target_class: edge_sens,
                            });
                        }
                    }
                }
            }
            u += 1;
        }
    }

    let mut root_count = 0u32;
    let mut sink_count = 0u32;
    let mut v = 0u32;
    while (v as usize) < v_count {
        if csr.in_degree(v)? == 0 {
            root_count += 1;
        }
        if csr.out_degree(v)? == 0 {
            sink_count += 1;
        }
        v += 1;
    }

    let betti_0 = compute_connected_components(csr, visited_bits, work_queue)?;
    let euler_characteristic = (v_count as i32) - (e_count as i32);
    let betti_1 = ((e_count as i32) - (v_count as i32) + (betti_0 as i32)).max(0) as u32;

    Ok(HomologySummary {
        euler_characteristic,
        betti_0,
        betti_1,
        root_count,
        sink_count,
    })
}

/// Verify that the host Geometry Complex's stamped `euler_characteristic` equals
/// its cellular alternating sum $\chi = N_0 - N_1 + N_2 - N_3$.
pub fn verify_geometry_complex_homology(geom: &PoetGeometryView<'_>) -> Result<i32, TopologyError> {
    let hdr = geom.header();
    let computed = hdr.computed_euler_characteristic();
    if hdr.euler_characteristic != computed {
        return Err(TopologyError::ChecksumMismatch {
            expected: computed as u32,
            actual: hdr.euler_characteristic as u32,
        });
    }
    Ok(computed)
}

fn verify_acyclic_kahn(
    csr: &HarbouredCsrView<'_>,
    visited_bits: &mut [u64],
    in_deg_work: &mut [u32],
) -> Result<(), TopologyError> {
    let v_count = csr.vertex_count() as usize;
    let words = (v_count + 63) / 64;
    visited_bits[..words].fill(0);

    let mut u = 0usize;
    while u < v_count {
        let nbrs = csr.out_neighbors(u as u32)?;
        let mut k = 0usize;
        while k < nbrs.len() {
            if nbrs[k] == u as u32 {
                return Err(TopologyError::CycleDetected {
                    source: u as u32,
                    target: u as u32,
                });
            }
            k += 1;
        }
        in_deg_work[u] = csr.in_degree(u as u32)?;
        u += 1;
    }

    let mut eliminated = 0usize;
    while eliminated < v_count {
        let mut progress = false;
        let mut v = 0usize;
        while v < v_count {
            if !test_bit(visited_bits, v) && in_deg_work[v] == 0 {
                set_bit(visited_bits, v);
                eliminated += 1;
                progress = true;
                let nbrs = csr.out_neighbors(v as u32)?;
                let mut k = 0usize;
                while k < nbrs.len() {
                    let w = nbrs[k] as usize;
                    in_deg_work[w] = in_deg_work[w].saturating_sub(1);
                    k += 1;
                }
            }
            v += 1;
        }
        if !progress {
            let mut src = 0u32;
            let mut dst = 0u32;
            let mut s = 0usize;
            'find_cycle: while s < v_count {
                if !test_bit(visited_bits, s) {
                    let nbrs = csr.out_neighbors(s as u32)?;
                    let mut k = 0usize;
                    while k < nbrs.len() {
                        let w = nbrs[k] as usize;
                        if !test_bit(visited_bits, w) {
                            src = s as u32;
                            dst = w as u32;
                            break 'find_cycle;
                        }
                        k += 1;
                    }
                }
                s += 1;
            }
            return Err(TopologyError::CycleDetected {
                source: src,
                target: dst,
            });
        }
    }
    Ok(())
}

fn compute_connected_components(
    csr: &HarbouredCsrView<'_>,
    visited_bits: &mut [u64],
    work_queue: &mut [u32],
) -> Result<u32, TopologyError> {
    let v_count = csr.vertex_count() as usize;
    if v_count == 0 {
        return Ok(0);
    }
    let words = (v_count + 63) / 64;
    visited_bits[..words].fill(0);

    let mut components = 0u32;
    let mut start = 0usize;
    while start < v_count {
        if test_bit(visited_bits, start) {
            start += 1;
            continue;
        }
        components += 1;
        set_bit(visited_bits, start);
        work_queue[0] = start as u32;
        let mut head = 0usize;
        let mut tail = 1usize;

        while head < tail {
            let curr = work_queue[head];
            head += 1;

            let out_nbrs = csr.out_neighbors(curr)?;
            let mut i = 0usize;
            while i < out_nbrs.len() {
                let w = out_nbrs[i] as usize;
                if !test_bit(visited_bits, w) {
                    set_bit(visited_bits, w);
                    work_queue[tail] = w as u32;
                    tail += 1;
                }
                i += 1;
            }

            let in_nbrs = csr.in_neighbors(curr)?;
            let mut j = 0usize;
            while j < in_nbrs.len() {
                let w = in_nbrs[j] as usize;
                if !test_bit(visited_bits, w) {
                    set_bit(visited_bits, w);
                    work_queue[tail] = w as u32;
                    tail += 1;
                }
                j += 1;
            }
        }
        start += 1;
    }

    Ok(components)
}

#[inline]
fn vertex_lamport_clock(csr: &HarbouredCsrView<'_>, quins: &[crate::NQuin], v: u32) -> Option<u64> {
    if quins.is_empty() {
        return None;
    }
    let anchor = csr.cell_anchor(v).ok()?;
    if anchor.anchor_domain == AnchorDomain::SemanticQuin as u8 {
        if let Some(q) = quins.get(anchor.cell_index as usize) {
            if anchor.entity_hash == 0 || q.subject == anchor.entity_hash {
                return Some((q.metadata >> 32) & 0x1FFF_FFFF);
            }
        }
    }
    if anchor.entity_hash != 0 {
        let mut i = 0usize;
        while i < quins.len() {
            if quins[i].subject == anchor.entity_hash {
                return Some((quins[i].metadata >> 32) & 0x1FFF_FFFF);
            }
            i += 1;
        }
    }
    None
}

#[inline]
fn is_edge_live_in_ring<const CAP: usize>(
    view: &MergedGraphView<'_, CAP>,
    u: u32,
    v: u32,
    mut live: bool,
) -> bool {
    let entries = view.ring().entries();
    let mut i = 0usize;
    while i < entries.len() {
        let e = &entries[i];
        if e.graph_slot() as u16 == view.graph_slot()
            && e.source_vertex == u
            && e.target_vertex == v
        {
            match e.op_kind() {
                DeltaOpKind::Tombstone => live = false,
                DeltaOpKind::Insert | DeltaOpKind::Reify => live = true,
                DeltaOpKind::AnchorBind => {}
            }
        }
        i += 1;
    }
    live
}

#[inline]
fn test_bit(bits: &[u64], idx: usize) -> bool {
    (bits[idx >> 6] & (1u64 << (idx & 63))) != 0
}

#[inline]
fn set_bit(bits: &mut [u64], idx: usize) {
    bits[idx >> 6] |= 1u64 << (idx & 63);
}

