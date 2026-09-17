//! Bounded subgraph export from a seed accession (AST-05).

use crate::q_hash;
use crate::NQuin;

use super::access::normalize_accession_query;
use super::error::QueryError;
use super::scan::PredHashes;
use super::types::QueryLimits;

/// Export Quins for the seed compound and related compounds within `max_depth`.
///
/// Walk follows `chebi:hasParent` edges in both directions (parent and child).
/// Depth is hops from the seed (`0` = seed only expansions blocked when
/// `max_depth == 0` is rejected by [`QueryLimits::validate`]; `max_depth == 1`
/// includes direct neighbours). Neighbours beyond `max_depth` are omitted
/// (soft ceiling). Output overflow fails closed with [`QueryError::OutputFull`].
pub fn export_subgraph_into(
    quins: &[NQuin],
    seed_accession: &str,
    limits: QueryLimits,
    out: &mut [NQuin],
) -> Result<usize, QueryError> {
    let limits = limits.validate()?;
    if quins.is_empty() {
        return Err(QueryError::EmptyInput);
    }
    let accession = normalize_accession_query(seed_accession).ok_or(QueryError::EmptyQuery)?;
    let seed = q_hash(&accession);
    let preds = PredHashes::load();

    let seed_present = quins
        .iter()
        .any(|q| q.predicate == preds.accession && q.subject == seed);
    if !seed_present {
        return Err(QueryError::NotFound);
    }

    const MAX_VISIT: usize = 256;
    let mut visited = [0u64; MAX_VISIT];
    let mut depths = [0u8; MAX_VISIT];
    let mut visit_n = 1usize;

    let mut queue = [0usize; MAX_VISIT];
    let mut qh = 0usize;
    let mut qt = 1usize;

    visited[0] = seed;
    depths[0] = 0;
    queue[0] = 0;

    while qh < qt {
        let vi = queue[qh];
        qh += 1;
        let subj = visited[vi];
        let depth = depths[vi] as usize;

        if depth >= limits.max_depth {
            continue;
        }

        for q in quins {
            if q.predicate != preds.has_parent {
                continue;
            }
            let next = if q.subject == subj {
                q.object
            } else if q.object == subj {
                q.subject
            } else {
                continue;
            };
            if contains_u64(&visited[..visit_n], next) {
                continue;
            }
            if visit_n >= MAX_VISIT {
                return Err(QueryError::LimitExceeded {
                    limit: MAX_VISIT,
                    needed: visit_n + 1,
                });
            }
            visited[visit_n] = next;
            depths[visit_n] = (depth + 1) as u8;
            queue[qt] = visit_n;
            visit_n += 1;
            qt += 1;
        }
    }

    let capacity = out.len().min(limits.max_export_quins);
    let mut written = 0usize;
    let mut needed = 0usize;
    for q in quins {
        if !contains_u64(&visited[..visit_n], q.subject) {
            continue;
        }
        needed += 1;
        if written >= capacity {
            continue;
        }
        out[written] = *q;
        written += 1;
    }

    if needed > capacity {
        return Err(QueryError::OutputFull { written, capacity });
    }
    Ok(written)
}

#[inline]
fn contains_u64(slice: &[u64], v: u64) -> bool {
    slice.iter().any(|&x| x == v)
}
