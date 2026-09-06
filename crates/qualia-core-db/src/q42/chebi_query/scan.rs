//! Quin-slice scanners aligned with [`crate::q42::chebi_map`] encoding.

use crate::q42::chebi_map::{
    PRED_ACCESSION, PRED_FROM_RELEASE, PRED_HAS_NAME, PRED_HAS_PARENT,
};
use crate::{q_hash, NQuin};

use super::types::Uncertainty;

/// Cached predicate hashes for one query pass.
#[derive(Debug, Clone, Copy)]
pub struct PredHashes {
    pub accession: u64,
    pub has_name: u64,
    pub has_parent: u64,
    pub from_release: u64,
}

impl PredHashes {
    pub fn load() -> Self {
        Self {
            accession: q_hash(PRED_ACCESSION),
            has_name: q_hash(PRED_HAS_NAME),
            has_parent: q_hash(PRED_HAS_PARENT),
            from_release: q_hash(PRED_FROM_RELEASE),
        }
    }
}

/// Aggregated fields for one compound subject within an optional release context.
#[derive(Debug, Clone, Copy, Default)]
pub struct CompoundView {
    pub subject_hash: u64,
    pub name_hash: u64,
    pub parent_hash: Option<u64>,
    pub release_hash: u64,
    pub source_line: u32,
    pub has_accession: bool,
    pub has_provenance: bool,
    pub context: u64,
}

impl CompoundView {
    pub fn uncertainty(self) -> Uncertainty {
        if !self.has_accession {
            return Uncertainty::Unknown;
        }
        if !self.has_provenance || self.name_hash == 0 {
            return Uncertainty::Partial;
        }
        Uncertainty::Known
    }
}

/// Resolve licence note: caller string wins; else catalogue `chebi` stub; else fail-closed text.
pub fn resolve_licence_note(licence_note: &str) -> String {
    let trimmed = licence_note.trim();
    if !trimmed.is_empty() {
        return trimmed.to_owned();
    }
    if let Some(d) = crate::q42::source_catalogue::lookup("chebi") {
        return d.licence_note.to_owned();
    }
    "licence unknown — fail closed; do not redistribute without verified terms".to_owned()
}

/// Scan quins for a subject (optionally filtered by release context).
pub fn view_subject(
    quins: &[NQuin],
    preds: &PredHashes,
    subject: u64,
    context_filter: Option<u64>,
) -> CompoundView {
    let mut view = CompoundView {
        subject_hash: subject,
        ..CompoundView::default()
    };

    for q in quins {
        if q.subject != subject {
            continue;
        }
        if let Some(ctx) = context_filter {
            if q.context != ctx {
                continue;
            }
        }
        if view.context == 0 {
            view.context = q.context;
        }
        if q.predicate == preds.accession {
            view.has_accession = true;
            if view.context == 0 {
                view.context = q.context;
            }
        } else if q.predicate == preds.has_name {
            view.name_hash = q.object;
        } else if q.predicate == preds.has_parent {
            view.parent_hash = Some(q.object);
        } else if q.predicate == preds.from_release {
            view.release_hash = q.object;
            view.source_line = (q.metadata & 0xFFFF_FFFF) as u32;
            view.has_provenance = true;
            view.context = q.context;
        }
    }

    if view.release_hash == 0 && view.context != 0 {
        view.release_hash = view.context;
    }
    view
}

/// Count Quins and distinct accession subjects for a release context.
pub fn count_release(quins: &[NQuin], preds: &PredHashes, release_hash: u64) -> (usize, usize) {
    let mut quin_count = 0usize;
    let mut subjects = [0u64; 512];
    let mut subject_n = 0usize;

    for q in quins {
        if q.context != release_hash {
            continue;
        }
        quin_count += 1;
        if q.predicate == preds.accession {
            if !subjects[..subject_n].contains(&q.subject) && subject_n < subjects.len() {
                subjects[subject_n] = q.subject;
                subject_n += 1;
            }
        }
    }
    (subject_n, quin_count)
}

/// Find surface accession from optional record index by subject hash.
pub fn accession_from_records(
    records: Option<&[crate::q42::chebi_parse::ChebiRecord]>,
    subject: u64,
) -> Option<String> {
    let records = records?;
    for r in records {
        if q_hash(&r.accession) == subject {
            return Some(r.accession.clone());
        }
    }
    None
}
