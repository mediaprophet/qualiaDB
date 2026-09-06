//! Core ChEBI query operations over an in-memory Quin slice (AST-05).

use crate::q_hash;
use crate::NQuin;

use super::access::normalize_accession_query;
use super::error::QueryError;
use super::scan::{
    accession_from_records, count_release, resolve_licence_note, view_subject, PredHashes,
};
use super::types::{
    ChemicalHit, EvidenceHit, QueryLimits, RelationHit, ReleaseDescription, Uncertainty,
};
use crate::q42::chebi_parse::ChebiRecord;

/// Describe one release / asset slice: counts + licence obligation stub.
///
/// Empty Quin input yields zero counts (still returns a description). Licence
/// note comes from `licence_note` when non-empty, otherwise the catalogue
/// `chebi` stub.
pub fn describe_release(
    quins: &[NQuin],
    release_label: &str,
    licence_note: &str,
) -> ReleaseDescription {
    let preds = PredHashes::load();
    let release_hash = q_hash(release_label);
    let (record_count, quin_count) = count_release(quins, &preds, release_hash);
    let note = resolve_licence_note(licence_note);
    let licence_obligation_present = !note.is_empty();
    ReleaseDescription {
        release_label: release_label.to_owned(),
        release_hash,
        record_count,
        quin_count,
        licence_note: note,
        licence_obligation_present,
    }
}

/// Resolve a chemical by accession (`CHEBI:{id}`) or bare numeric id into `out`.
///
/// Fail-closed:
/// - empty quins → [`QueryError::EmptyInput`]
/// - empty / bad query → [`QueryError::EmptyQuery`]
/// - zero hits → [`QueryError::NotFound`]
/// - more than one distinct (subject, context) → [`QueryError::Ambiguous`]
/// - capacity / max_hits too small → [`QueryError::LimitExceeded`]
pub fn resolve_chemical_into(
    quins: &[NQuin],
    query: &str,
    licence_note: &str,
    limits: QueryLimits,
    records: Option<&[ChebiRecord]>,
    out: &mut [ChemicalHit],
) -> Result<usize, QueryError> {
    let limits = limits.validate()?;
    if quins.is_empty() {
        return Err(QueryError::EmptyInput);
    }
    let accession = normalize_accession_query(query).ok_or(QueryError::EmptyQuery)?;
    let subject = q_hash(&accession);
    let preds = PredHashes::load();
    let note = resolve_licence_note(licence_note);

    let mut matches = [(0u64, 0u64); 32];
    let mut match_n = 0usize;
    for q in quins {
        if q.predicate != preds.accession || q.subject != subject {
            continue;
        }
        let pair = (q.subject, q.context);
        if matches[..match_n].contains(&pair) {
            continue;
        }
        if match_n >= matches.len() {
            return Err(QueryError::LimitExceeded {
                limit: matches.len(),
                needed: match_n + 1,
            });
        }
        matches[match_n] = pair;
        match_n += 1;
    }

    if match_n == 0 {
        return Err(QueryError::NotFound);
    }
    if match_n > 1 {
        return Err(QueryError::Ambiguous { hits: match_n });
    }

    let capacity = out.len().min(limits.max_hits);
    if capacity == 0 {
        return Err(QueryError::LimitExceeded {
            limit: 0,
            needed: 1,
        });
    }

    let (subj, ctx) = matches[0];
    let view = view_subject(quins, &preds, subj, Some(ctx));
    let surface = accession_from_records(records, subj).unwrap_or(accession);
    out[0] = ChemicalHit {
        subject_hash: view.subject_hash,
        name_hash: view.name_hash,
        parent_hash: view.parent_hash,
        release_hash: if view.release_hash != 0 {
            view.release_hash
        } else {
            ctx
        },
        source_line: view.source_line,
        uncertainty: view.uncertainty(),
        accession: surface,
        licence_note: note,
    };
    Ok(1)
}

/// Look up parent edges (`chebi:hasParent`) for a child accession.
pub fn lookup_parents_into(
    quins: &[NQuin],
    child_accession: &str,
    licence_note: &str,
    limits: QueryLimits,
    records: Option<&[ChebiRecord]>,
    out: &mut [RelationHit],
) -> Result<usize, QueryError> {
    let limits = limits.validate()?;
    if quins.is_empty() {
        return Err(QueryError::EmptyInput);
    }
    let accession = normalize_accession_query(child_accession).ok_or(QueryError::EmptyQuery)?;
    let child = q_hash(&accession);
    let preds = PredHashes::load();
    let note = resolve_licence_note(licence_note);
    let capacity = out.len().min(limits.max_hits);

    let mut written = 0usize;
    let mut needed = 0usize;
    for q in quins {
        if q.predicate != preds.has_parent || q.subject != child {
            continue;
        }
        needed += 1;
        if written >= capacity {
            continue;
        }
        let child_view = view_subject(quins, &preds, child, Some(q.context));
        let parent_surface =
            accession_from_records(records, q.object).unwrap_or_else(|| {
                // Parent object is q_hash("CHEBI:{id}"); recover surface from child record.
                records
                    .and_then(|rs| {
                        rs.iter()
                            .find(|r| q_hash(&r.accession) == child)
                            .and_then(|r| r.parent_id)
                    })
                    .map(|pid| {
                        let mut buf = [0u8; 32];
                        super::access::format_chebi_accession(pid, &mut buf).to_owned()
                    })
                    .unwrap_or_default()
            });
        out[written] = RelationHit {
            child_hash: child,
            parent_hash: q.object,
            release_hash: if child_view.release_hash != 0 {
                child_view.release_hash
            } else {
                q.context
            },
            source_line: child_view.source_line,
            uncertainty: if child_view.has_provenance {
                Uncertainty::Known
            } else {
                Uncertainty::Partial
            },
            child_accession: accession.clone(),
            parent_accession: parent_surface,
            licence_note: note.clone(),
        };
        written += 1;
    }

    if needed == 0 {
        return Err(QueryError::NotFound);
    }
    if needed > capacity {
        return Err(QueryError::LimitExceeded {
            limit: capacity,
            needed,
        });
    }
    Ok(written)
}

/// Look up children that declare `chebi:hasParent` pointing at `parent_accession`.
pub fn lookup_children_into(
    quins: &[NQuin],
    parent_accession: &str,
    licence_note: &str,
    limits: QueryLimits,
    records: Option<&[ChebiRecord]>,
    out: &mut [RelationHit],
) -> Result<usize, QueryError> {
    let limits = limits.validate()?;
    if quins.is_empty() {
        return Err(QueryError::EmptyInput);
    }
    let accession = normalize_accession_query(parent_accession).ok_or(QueryError::EmptyQuery)?;
    let parent = q_hash(&accession);
    let preds = PredHashes::load();
    let note = resolve_licence_note(licence_note);
    let capacity = out.len().min(limits.max_hits);

    let mut written = 0usize;
    let mut needed = 0usize;
    for q in quins {
        if q.predicate != preds.has_parent || q.object != parent {
            continue;
        }
        needed += 1;
        if written >= capacity {
            continue;
        }
        let child_view = view_subject(quins, &preds, q.subject, Some(q.context));
        let child_surface = accession_from_records(records, q.subject).unwrap_or_default();
        out[written] = RelationHit {
            child_hash: q.subject,
            parent_hash: parent,
            release_hash: if child_view.release_hash != 0 {
                child_view.release_hash
            } else {
                q.context
            },
            source_line: child_view.source_line,
            uncertainty: if child_view.has_provenance {
                Uncertainty::Known
            } else {
                Uncertainty::Partial
            },
            child_accession: child_surface,
            parent_accession: accession.clone(),
            licence_note: note.clone(),
        };
        written += 1;
    }

    if needed == 0 {
        return Err(QueryError::NotFound);
    }
    if needed > capacity {
        return Err(QueryError::LimitExceeded {
            limit: capacity,
            needed,
        });
    }
    Ok(written)
}

/// Look up `chebi:fromRelease` provenance for an accession.
pub fn lookup_evidence_into(
    quins: &[NQuin],
    accession_query: &str,
    licence_note: &str,
    limits: QueryLimits,
    out: &mut [EvidenceHit],
) -> Result<usize, QueryError> {
    let limits = limits.validate()?;
    if quins.is_empty() {
        return Err(QueryError::EmptyInput);
    }
    let accession = normalize_accession_query(accession_query).ok_or(QueryError::EmptyQuery)?;
    let subject = q_hash(&accession);
    let preds = PredHashes::load();
    let note = resolve_licence_note(licence_note);
    let capacity = out.len().min(limits.max_hits);

    let mut written = 0usize;
    let mut needed = 0usize;
    for q in quins {
        if q.predicate != preds.from_release || q.subject != subject {
            continue;
        }
        needed += 1;
        if written >= capacity {
            continue;
        }
        out[written] = EvidenceHit {
            subject_hash: subject,
            release_hash: q.object,
            source_line: (q.metadata & 0xFFFF_FFFF) as u32,
            uncertainty: Uncertainty::Known,
            accession: accession.clone(),
            licence_note: note.clone(),
        };
        written += 1;
    }

    if needed == 0 {
        return Err(QueryError::NotFound);
    }
    if needed > capacity {
        return Err(QueryError::LimitExceeded {
            limit: capacity,
            needed,
        });
    }
    Ok(written)
}
