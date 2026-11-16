//! Bounded scan fallback: deliberately trades throughput for explicit storage/work bounds.
use super::super::normalized_cost::{normalized_cost, NormalizedCostTerms};
use super::super::{face_plane, interpolate_semantic_attributes, Quadric4};
use super::*;
use crate::specialized_libs::computational_geometry::production_contracts::{
    CollapseJournal, CollapseNeighborhood, CollapseProposal, CornerId,
};

#[derive(Clone, Copy)]
struct Ranked {
    proposal: CollapseProposal,
    terms: NormalizedCostTerms,
}

fn better(a: Ranked, b: Ranked) -> bool {
    a.terms
        .total
        .total_cmp(&b.terms.total)
        .then_with(|| a.proposal.stable_endpoint_ids.cmp(&b.proposal.stable_endpoint_ids))
        .then_with(|| a.proposal.t_from_removed_to_kept.total_cmp(&b.proposal.t_from_removed_to_kept))
        .is_lt()
}
pub(super) fn hard_compatible(input: &AdmissionInput<'_>, s: &[AdmissionSlot], lo: usize, hi: usize) -> bool {
    let a = s[lo].attributes;
    let b = s[hi].attributes;
    input.domains[lo] == input.domains[hi]
        && !input.domains[lo].immovable()
        && !a.cluster_border_locked
        && !b.cluster_border_locked
        && !a.affordance_boundary
        && !b.affordance_boundary
        && a.subpart_tag == b.subpart_tag
        && a.skeleton_joint_id == b.skeleton_joint_id
}
fn neighborhood(
    s: &[AdmissionSlot],
    m: usize,
    p: &CollapseProposal,
    faces: &mut [FaceDelta],
    corners: &mut [CornerDelta],
    budget: &mut ProductionBudget<'_>,
) -> Result<(usize, usize), AdmissionError> {
    let mut nf = 0;
    let mut nc = 0;
    for f in 0..m {
        budget.checkpoint(1)?;
        if !s[f].face_alive {
            continue;
        }
        let before = s[f].triangle;
        if !before.contains(&p.removed_vertex) && !before.contains(&p.kept_vertex) {
            continue;
        }
        let after = topology::after(before, p.removed_vertex, p.kept_vertex);
        faces[nf] = FaceDelta { original_face: f as u32, before, after };
        nf += 1;
        budget.checkpoint(3)?;
        for slot in 0..3_u8 {
            let source = CornerId { face: f as u32, slot };
            corners[nc] = CornerDelta { source, target: after.map(|_| source) };
            nc += 1;
        }
    }
    Ok((nf, nc))
}
fn initialize_quadrics(
    s: &mut [AdmissionSlot],
    m: usize,
    budget: &mut ProductionBudget<'_>,
) -> Result<(), AdmissionError> {
    for f in 0..m {
        budget.checkpoint(1)?;
        let t = s[f].triangle;
        let Some((a, b, c, d, area)) =
            face_plane(s[t[0] as usize].position, s[t[1] as usize].position, s[t[2] as usize].position)
        else {
            return Err(error(ReasonCode::InvalidInput, "face_plane"));
        };
        let q = Quadric4::from_plane(a, b, c, d, area);
        for v in t {
            s[v as usize].quadric.add(&q);
        }
        for (i, j) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            if topology::incidence(s, m, i, j, budget)?.0 != 1 {
                continue;
            }
            let p = s[i as usize].position;
            let r = s[j as usize].position;
            let (x, y, z) = (r.x - p.x, r.y - p.y, r.z - p.z);
            let (nx, ny, nz) = (y * c - z * b, z * a - x * c, x * b - y * a);
            let length = (nx * nx + ny * ny + nz * nz).sqrt();
            if length > 0.0 && length.is_finite() {
                let (nx, ny, nz) = (nx / length, ny / length, nz / length);
                let q = Quadric4::from_plane(nx, ny, nz, -(nx * p.x + ny * p.y + nz * p.z), 1000.0 * area);
                s[i as usize].quadric.add(&q);
                s[j as usize].quadric.add(&q);
            }
        }
    }
    Ok(())
}

fn terminal_error(e: AdmissionError) -> Result<Termination, AdmissionError> {
    match e {
        AdmissionError::Budget(BudgetError::Cancelled) => Ok(Termination::Cancelled),
        AdmissionError::Budget(BudgetError::WorkLimitExceeded { .. }) => Ok(Termination::WorkExhausted),
        other => Err(other),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run<K: GeometryKernel, T: TransferAdmission>(
    kernel: &K,
    input: AdmissionInput<'_>,
    policy: AdmissionPolicy,
    identity: AdmissionIdentity,
    _limits: AdmissionLimits,
    scratch: AdmissionScratch<'_>,
    output: AdmissionOutput<'_>,
    transfer: &mut T,
    budget: &mut ProductionBudget<'_>,
) -> Result<AdmissionReport, AdmissionError> {
    let n = input.vertices.len();
    let m = input.triangles.len();
    initialize_quadrics(scratch.slots, m, budget)?;
    let mut journal = CollapseJournal::new(output.events);
    let mut live_faces = m;
    let mut collapses = 0_usize;
    let mut largest = NormalizedCostTerms::default();
    let mut rejected = RejectionCounts::default();
    let mut termination = 'outer: loop {
        if live_faces <= policy.target_faces {
            break Termination::ReachedTarget;
        }
        let search = (|| -> Result<Option<Ranked>, AdmissionError> {
            let mut best: Option<Ranked> = None;
            for f in 0..m {
                budget.checkpoint(1)?;
                if !scratch.slots[f].face_alive {
                    continue;
                }
                let tri = scratch.slots[f].triangle;
                for (a, b) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
                    budget.checkpoint(1)?;
                    let (lo, hi) = if input.stable_vertex_ids[a as usize] < input.stable_vertex_ids[b as usize] {
                        (a, b)
                    } else {
                        (b, a)
                    };
                    if !hard_compatible(&input, scratch.slots, lo as usize, hi as usize) {
                        rejected.semantic += 1;
                        continue;
                    }
                    let pl = scratch.slots[lo as usize].position;
                    let ph = scratch.slots[hi as usize].position;
                    let mid = Point3::new(0.5 * pl.x + 0.5 * ph.x, 0.5 * pl.y + 0.5 * ph.y, 0.5 * pl.z + 0.5 * ph.z);
                    for (target, t) in [(mid, 0.5), (pl, 0.0), (ph, 1.0)] {
                        budget.checkpoint(1)?;
                        if !topology::legal(
                            kernel,
                            scratch.slots,
                            n,
                            m,
                            lo,
                            hi,
                            target,
                            policy.preserve_boundary_vertices,
                            policy.cost.length_scale,
                            budget,
                        )? {
                            rejected.topology += 1;
                            continue;
                        }
                        let proposal = CollapseProposal {
                            run: identity.run,
                            source: identity.source,
                            policy: identity.policy,
                            sequence: collapses as u64,
                            topology_generation: collapses as u64,
                            removed_vertex: lo,
                            kept_vertex: hi,
                            stable_endpoint_ids: [
                                input.stable_vertex_ids[lo as usize],
                                input.stable_vertex_ids[hi as usize],
                            ],
                            endpoint_generations: [
                                scratch.slots[lo as usize].generation,
                                scratch.slots[hi as usize].generation,
                            ],
                            target,
                            t_from_removed_to_kept: t,
                        };
                        proposal.validate()?;
                        let (nf, nc) =
                            neighborhood(scratch.slots, m, &proposal, scratch.faces, scratch.corners, budget)?;
                        budget.checkpoint(transfer.work_bound(nf, nc)?)?;
                        let view =
                            CollapseNeighborhood { faces: &scratch.faces[..nf], corners: &scratch.corners[..nc] };
                        match transfer.prepare(&proposal, view, scratch.transfer) {
                            Ok(_) => {}
                            Err(e) if e.code == ReasonCode::UnsatisfiedConstraint => {
                                rejected.transfer += 1;
                                continue;
                            }
                            Err(e) => return Err(e.into()),
                        }
                        let mut q = scratch.slots[lo as usize].quadric;
                        q.add(&scratch.slots[hi as usize].quadric);
                        let Some(terms) = normalized_cost(
                            pl,
                            ph,
                            target,
                            q.error_at(target),
                            &scratch.slots[lo as usize].attributes,
                            &scratch.slots[hi as usize].attributes,
                            &policy.cost,
                        ) else {
                            rejected.numeric += 1;
                            continue;
                        };
                        let ranked = Ranked { proposal, terms };
                        if best.is_none_or(|previous| better(ranked, previous)) {
                            best = Some(ranked);
                        }
                    }
                }
            }
            Ok(best)
        })();
        let best = match search {
            Ok(Some(best)) => best,
            Ok(None) => break Termination::ConstraintLimited,
            Err(AdmissionError::Contract(e)) => break Termination::TransferFailure(e.code),
            Err(e) => break terminal_error(e)?,
        };
        if policy.maximum_normalized_cost.is_some_and(|limit| best.terms.total > limit) {
            break Termination::CostLimited;
        }
        let p = best.proposal;
        let reservation = match journal.reserve() {
            Ok(slot) => slot,
            Err(_) => break Termination::EventCapacityExhausted,
        };
        // Repeat every mutable-state admission and prepare immediately before atomic mutation.
        let prepare = (|| -> Result<(T::Prepared, usize), AdmissionError> {
            budget.checkpoint(1)?;
            if !hard_compatible(&input, scratch.slots, p.removed_vertex as usize, p.kept_vertex as usize)
                || !topology::legal(
                    kernel,
                    scratch.slots,
                    n,
                    m,
                    p.removed_vertex,
                    p.kept_vertex,
                    p.target,
                    policy.preserve_boundary_vertices,
                    policy.cost.length_scale,
                    budget,
                )?
            {
                return Err(error(ReasonCode::OracleMismatch, "winner_revalidation"));
            }
            let (nf, nc) = neighborhood(scratch.slots, m, &p, scratch.faces, scratch.corners, budget)?;
            budget.checkpoint(transfer.work_bound(nf, nc)?)?;
            let view = CollapseNeighborhood { faces: &scratch.faces[..nf], corners: &scratch.corners[..nc] };
            let prepared = transfer.prepare(&p, view, scratch.transfer)?;
            transfer.revalidate(&p, view, &prepared, scratch.transfer)?;
            let commit_work = (nf as u64) + 1;
            budget.checkpoint(commit_work)?;
            Ok((prepared, nf))
        })();
        let (prepared, nf) = match prepare {
            Ok(value) => value,
            Err(AdmissionError::Contract(e)) => break 'outer Termination::TransferFailure(e.code),
            Err(e) => break 'outer terminal_error(e)?,
        };
        let transfer_identity = transfer.prepared_identity(&prepared);
        // No fallible checks or cancellation inside this complete commit.
        transfer.commit(&p, prepared, scratch.transfer);
        let lo = p.removed_vertex as usize;
        let hi = p.kept_vertex as usize;
        scratch.slots[hi].position = p.target;
        scratch.slots[hi].attributes = interpolate_semantic_attributes(
            &scratch.slots[lo].attributes,
            &scratch.slots[hi].attributes,
            p.t_from_removed_to_kept,
        );
        let q = scratch.slots[lo].quadric;
        scratch.slots[hi].quadric.add(&q);
        scratch.slots[lo].vertex_alive = false;
        scratch.slots[lo].lineage = hi as u32;
        scratch.slots[lo].generation += 1;
        scratch.slots[hi].generation += 1;
        for delta in &scratch.faces[..nf] {
            let slot = &mut scratch.slots[delta.original_face as usize];
            match delta.after {
                Some(t) => slot.triangle = t,
                None => {
                    slot.face_alive = false;
                    live_faces -= 1;
                }
            }
        }
        reservation.commit(CollapseEvent { proposal: p, transfer: transfer_identity });
        collapses += 1;
        if best.terms.total > largest.total {
            largest = best.terms;
        }
    };
    if matches!(budget.checkpoint(0), Err(BudgetError::Cancelled)) {
        termination = Termination::Cancelled;
    }
    let mut vc = 0;
    let mut fc = 0;
    for v in 0..n {
        output.remaps.surviving_vertex_compaction[v] = u32::MAX;
        if scratch.slots[v].vertex_alive {
            output.vertices[vc] = scratch.slots[v].position;
            output.attributes[vc] = scratch.slots[v].attributes;
            output.domains[vc] = input.domains[v];
            output.remaps.surviving_vertex_compaction[v] = vc as u32;
            vc += 1;
        }
    }
    for v in 0..n {
        let mut root = v as u32;
        for _ in 0..n {
            let next = scratch.slots[root as usize].lineage;
            if next == root {
                break;
            }
            root = next;
        }
        output.remaps.vertex_lineage[v] = root;
    }
    for f in 0..m {
        output.remaps.face_compaction[f] = u32::MAX;
        if scratch.slots[f].face_alive {
            output.triangles[fc] =
                scratch.slots[f].triangle.map(|v| output.remaps.surviving_vertex_compaction[v as usize]);
            output.remaps.face_compaction[f] = fc as u32;
            fc += 1;
        }
        for slot in 0..3_u8 {
            output.remaps.corner_compaction[f * 3 + slot as usize] = if scratch.slots[f].face_alive {
                Some(CornerId { face: output.remaps.face_compaction[f], slot })
            } else {
                None
            };
        }
    }
    let termination = if matches!(termination, Termination::ReachedTarget) {
        match budget.checkpoint(0) {
            Ok(()) => termination,
            Err(BudgetError::Cancelled) => Termination::Cancelled,
            Err(_) => Termination::WorkExhausted,
        }
    } else {
        termination
    };
    Ok(AdmissionReport {
        termination,
        vertices: vc,
        faces: fc,
        committed_collapses: collapses,
        rejections: rejected,
        largest_cost: largest,
        work_used: budget.work_used(),
        admitted_bytes: budget.capacity_plan().total_bytes()?,
        derived_frames_require_recomputation: collapses != 0,
    })
}
