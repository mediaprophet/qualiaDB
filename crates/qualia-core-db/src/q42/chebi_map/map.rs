//! ChEBI record → evidence-preserving Quin projection (AST-04).
//!
//! # Predicate IRIs (via [`crate::q_hash`], 60-bit)
//!
//! | Role | IRI |
//! |------|-----|
//! | Type | `rdf:type` → class `chebi:Compound` |
//! | Accession | `chebi:accession` → 60-bit hash of accession string |
//! | Name | `chebi:hasName` → 60-bit hash of display name |
//! | Parent | `chebi:hasParent` → 60-bit hash of `CHEBI:{parent_id}` |
//! | Provenance | `chebi:fromRelease` → 60-bit hash of `release_label`; `metadata` low 32 bits = `source_line` |
//!
//! # Layout per accepted record
//!
//! Always emits **4** Quins (type, accession, hasName, fromRelease). When
//! `parent_id` is `Some`, emits one more (`hasParent`). Subject for all is
//! `q_hash(accession)`. Graph `context` is `q_hash(release_label)`.
//!
//! Parity is the AGENTS.md XOR fold: `subject ^ predicate ^ object ^ context`.
//! Object IRI/name hashes use the same 60-bit mask as `q_hash` (top 4 bits free
//! for inline tags — not set here).
//!
//! Duplicate `id` or `accession` in one batch → [`MapConflict`], first mapping
//! retained (no silent overwrite).

use crate::q42::chebi_parse::ChebiRecord;
use crate::{q_hash, NQuin};

use super::report::{LexiconEntry, MapBudgets, MapConflict, MapError, MapReport};

/// Quins emitted for every mapped record (type, accession, name, provenance).
pub const QUINS_PER_RECORD_BASE: usize = 4;

/// Predicate: RDF typing.
pub const PRED_RDF_TYPE: &str = "rdf:type";
/// Object class for a ChEBI compound.
pub const CLASS_CHEBI_COMPOUND: &str = "chebi:Compound";
/// Predicate: accession string evidence.
pub const PRED_ACCESSION: &str = "chebi:accession";
/// Predicate: display name (hashed; surface in lexicon note).
pub const PRED_HAS_NAME: &str = "chebi:hasName";
/// Predicate: parent compound accession hash.
pub const PRED_HAS_PARENT: &str = "chebi:hasParent";
/// Predicate: release provenance; source_line in metadata.
pub const PRED_FROM_RELEASE: &str = "chebi:fromRelease";

/// How many Quins one record would emit (including optional parent).
#[inline]
pub fn quins_for_record(record: &ChebiRecord) -> usize {
    QUINS_PER_RECORD_BASE + usize::from(record.parent_id.is_some())
}

/// Format `CHEBI:{id}` into a caller stack buffer; returns the UTF-8 prefix used.
fn format_chebi_accession(id: u64, buf: &mut [u8; 32]) -> &str {
    const PREFIX: &[u8] = b"CHEBI:";
    buf[..PREFIX.len()].copy_from_slice(PREFIX);
    let mut n = id;
    let mut digits = [0u8; 20];
    let mut dlen = 0usize;
    if n == 0 {
        digits[0] = b'0';
        dlen = 1;
    } else {
        while n > 0 {
            digits[dlen] = b'0' + (n % 10) as u8;
            dlen += 1;
            n /= 10;
        }
        digits[..dlen].reverse();
    }
    let total = PREFIX.len() + dlen;
    debug_assert!(total <= buf.len());
    buf[PREFIX.len()..total].copy_from_slice(&digits[..dlen]);
    // SAFETY: PREFIX + ASCII digits are always valid UTF-8.
    core::str::from_utf8(&buf[..total]).unwrap_or("CHEBI:0")
}

#[inline]
fn parity_fold(q: &NQuin) -> u64 {
    q.subject ^ q.predicate ^ q.object ^ q.context
}

#[inline]
fn write_quin(
    out: &mut [NQuin],
    idx: usize,
    subject: u64,
    predicate: u64,
    object: u64,
    context: u64,
    metadata: u64,
) {
    let mut q = NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity: 0,
    };
    q.parity = parity_fold(&q);
    out[idx] = q;
}

fn push_lexicon(lexicon: &mut Vec<LexiconEntry>, hash: u64, surface: &str, kind: &'static str) {
    lexicon.push(LexiconEntry {
        hash,
        surface: surface.to_owned(),
        kind,
    });
}

fn push_conflict(
    conflicts: &mut Vec<MapConflict>,
    budgets: MapBudgets,
    accession: &str,
    reason: &'static str,
) -> Result<(), MapError> {
    if conflicts.len() >= budgets.max_conflicts {
        return Err(MapError::ConflictBudgetExceeded {
            used: conflicts.len(),
            max: budgets.max_conflicts,
        });
    }
    conflicts.push(MapConflict {
        accession: accession.to_owned(),
        reason,
    });
    Ok(())
}

/// Project accepted ChEBI records into caller-buffered Quins.
///
/// Hot projection writes only into `out`. The returned [`MapReport`] is cold
/// (heap) and carries conflicts, lexicon notes, and counts. On
/// [`MapError::OutputFull`] / [`MapError::ConflictBudgetExceeded`], Quins already
/// written remain in `out` up to the last successful record.
pub fn map_records_into(
    records: &[ChebiRecord],
    release_label: &str,
    budgets: MapBudgets,
    out: &mut [NQuin],
) -> Result<MapReport, MapError> {
    let budgets = budgets.validate()?;
    let capacity = out.len().min(budgets.max_quins);

    let pred_type = q_hash(PRED_RDF_TYPE);
    let class_compound = q_hash(CLASS_CHEBI_COMPOUND);
    let pred_accession = q_hash(PRED_ACCESSION);
    let pred_has_name = q_hash(PRED_HAS_NAME);
    let pred_has_parent = q_hash(PRED_HAS_PARENT);
    let pred_from_release = q_hash(PRED_FROM_RELEASE);
    let context = q_hash(release_label);
    let release_hash = context;

    let mut written = 0usize;
    let mut records_mapped = 0usize;
    let mut conflicts: Vec<MapConflict> = Vec::new();
    let mut lexicon: Vec<LexiconEntry> = Vec::new();

    // Seen keys for this batch only (cold construction).
    let mut seen_accessions: Vec<String> = Vec::new();
    let mut seen_ids: Vec<u64> = Vec::new();

    push_lexicon(&mut lexicon, release_hash, release_label, "release");

    for record in records {
        let needed = quins_for_record(record);

        // Identifier collision: never overwrite an earlier mapping.
        let dup_acc = seen_accessions.iter().any(|a| a == &record.accession);
        let dup_id = seen_ids.iter().any(|&id| id == record.id);
        if dup_acc || dup_id {
            let reason = if dup_acc && dup_id {
                "duplicate_accession_and_id"
            } else if dup_acc {
                "duplicate_accession"
            } else {
                "duplicate_id"
            };
            push_conflict(&mut conflicts, budgets, &record.accession, reason)?;
            continue;
        }

        if written + needed > capacity {
            return Err(MapError::OutputFull {
                written,
                needed,
                capacity,
            });
        }

        let subject = q_hash(&record.accession);
        let accession_obj = subject; // accession string hash == subject identity
        let name_obj = q_hash(&record.name);

        // 1. Identity / type
        write_quin(
            out,
            written,
            subject,
            pred_type,
            class_compound,
            context,
            0,
        );
        written += 1;

        // 2. Accession evidence
        write_quin(
            out,
            written,
            subject,
            pred_accession,
            accession_obj,
            context,
            0,
        );
        written += 1;

        // 3. Name (hash + lexicon note)
        write_quin(
            out,
            written,
            subject,
            pred_has_name,
            name_obj,
            context,
            0,
        );
        written += 1;

        // 4. Optional parent
        if let Some(parent_id) = record.parent_id {
            let mut parent_buf = [0u8; 32];
            let parent_acc = format_chebi_accession(parent_id, &mut parent_buf);
            let parent_obj = q_hash(parent_acc);
            write_quin(
                out,
                written,
                subject,
                pred_has_parent,
                parent_obj,
                context,
                0,
            );
            written += 1;
            push_lexicon(&mut lexicon, parent_obj, parent_acc, "parent");
        }

        // 5. Provenance: release hash + source_line in metadata low 32 bits
        let metadata = record.source_line & 0xFFFF_FFFF;
        write_quin(
            out,
            written,
            subject,
            pred_from_release,
            release_hash,
            context,
            metadata,
        );
        written += 1;

        push_lexicon(&mut lexicon, accession_obj, &record.accession, "accession");
        push_lexicon(&mut lexicon, name_obj, &record.name, "name");

        seen_accessions.push(record.accession.clone());
        seen_ids.push(record.id);
        records_mapped += 1;
    }

    Ok(MapReport {
        quins_written: written,
        records_mapped,
        conflicts,
        lexicon,
        release_label: release_label.to_owned(),
    })
}

/// True when Quin parity matches the AGENTS.md XOR fold.
#[inline]
pub fn quin_parity_valid(q: &NQuin) -> bool {
    q.parity == parity_fold(q)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42::chebi_parse::ChebiRecord;

    fn rec(
        id: u64,
        accession: &str,
        name: &str,
        parent_id: Option<u64>,
        source_line: u64,
    ) -> ChebiRecord {
        ChebiRecord {
            id,
            status: "C".into(),
            accession: accession.into(),
            source: String::new(),
            parent_id,
            name: name.into(),
            definition: String::new(),
            source_line,
        }
    }

    fn budgets(max_quins: usize, max_conflicts: usize) -> MapBudgets {
        MapBudgets {
            max_quins,
            max_conflicts,
        }
    }

    #[test]
    fn two_records_expected_count_and_parity() {
        let records = [
            rec(1, "CHEBI:1", "water", None, 2),
            rec(2, "CHEBI:2", "ethanol", Some(1), 3),
        ];
        // 4 + (4+1) = 9
        let mut out = [NQuin::default(); 16];
        let report = map_records_into(&records, "chebi-test-2026", budgets(16, 8), &mut out)
            .expect("map");
        assert_eq!(report.records_mapped, 2);
        assert_eq!(report.quins_written, 9);
        assert!(report.conflicts.is_empty());
        for q in &out[..report.quins_written] {
            assert!(quin_parity_valid(q), "parity invalid: {q:?}");
        }
        // Lexicon includes release + per-record accession/name + one parent
        assert!(report.lexicon.iter().any(|e| e.kind == "release"));
        assert!(report.lexicon.iter().any(|e| e.kind == "parent"));
        assert_eq!(
            report
                .lexicon
                .iter()
                .filter(|e| e.kind == "accession")
                .count(),
            2
        );
    }

    #[test]
    fn duplicate_accession_reports_conflict_keeps_first() {
        let records = [
            rec(10, "CHEBI:10", "first", None, 2),
            rec(10, "CHEBI:10", "second-overwrite-attempt", None, 3),
        ];
        let mut out = [NQuin::default(); 16];
        let report =
            map_records_into(&records, "rel", budgets(16, 8), &mut out).expect("map");
        assert_eq!(report.records_mapped, 1);
        assert_eq!(report.quins_written, QUINS_PER_RECORD_BASE);
        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.conflicts[0].accession, "CHEBI:10");
        assert_eq!(report.conflicts[0].reason, "duplicate_accession_and_id");

        let name_pred = q_hash(PRED_HAS_NAME);
        let name_quin = out[..report.quins_written]
            .iter()
            .find(|q| q.predicate == name_pred)
            .expect("name quin");
        assert_eq!(name_quin.object, q_hash("first"));
        assert_ne!(name_quin.object, q_hash("second-overwrite-attempt"));
    }

    #[test]
    fn parent_relationship_when_parent_id_set() {
        let records = [rec(99, "CHEBI:99", "child", Some(7), 5)];
        let mut out = [NQuin::default(); 8];
        let report =
            map_records_into(&records, "rel-p", budgets(8, 4), &mut out).expect("map");
        assert_eq!(report.quins_written, 5);
        let parent_pred = q_hash(PRED_HAS_PARENT);
        let parent_q = out[..report.quins_written]
            .iter()
            .find(|q| q.predicate == parent_pred)
            .expect("parent quin");
        assert_eq!(parent_q.object, q_hash("CHEBI:7"));
        assert_eq!(parent_q.subject, q_hash("CHEBI:99"));
    }

    #[test]
    fn output_buffer_too_small_errors() {
        let records = [
            rec(1, "CHEBI:1", "a", None, 2),
            rec(2, "CHEBI:2", "b", None, 3),
        ];
        // First record needs 4; buffer of 4 succeeds first then fails on second.
        let mut out = [NQuin::default(); 4];
        let err = map_records_into(&records, "rel", budgets(64, 8), &mut out).unwrap_err();
        assert_eq!(
            err,
            MapError::OutputFull {
                written: 4,
                needed: 4,
                capacity: 4,
            }
        );
    }

    #[test]
    fn empty_input_writes_zero() {
        let mut out = [NQuin::default(); 4];
        let report = map_records_into(&[], "rel", budgets(4, 1), &mut out).expect("map");
        assert_eq!(report.quins_written, 0);
        assert_eq!(report.records_mapped, 0);
        assert!(report.conflicts.is_empty());
        // Release lexicon note still recorded for provenance of the pass.
        assert_eq!(report.lexicon.len(), 1);
        assert_eq!(report.lexicon[0].kind, "release");
    }

    #[test]
    fn deterministic_same_records_same_quin_bytes() {
        let records = [
            rec(3, "CHEBI:3", "glucose", Some(2), 10),
            rec(4, "CHEBI:4", "fructose", None, 11),
        ];
        let mut a = [NQuin::default(); 16];
        let mut b = [NQuin::default(); 16];
        let ra = map_records_into(&records, "chebi-det", budgets(16, 4), &mut a).unwrap();
        let rb = map_records_into(&records, "chebi-det", budgets(16, 4), &mut b).unwrap();
        assert_eq!(ra.quins_written, rb.quins_written);
        for i in 0..ra.quins_written {
            assert_eq!(a[i], b[i], "quin {i} differs");
        }
    }

    #[test]
    fn provenance_encodes_source_line_and_release() {
        let records = [rec(5, "CHEBI:5", "named", None, 42)];
        let mut out = [NQuin::default(); 8];
        let report =
            map_records_into(&records, "release-label-x", budgets(8, 2), &mut out).unwrap();
        let pred = q_hash(PRED_FROM_RELEASE);
        let q = out[..report.quins_written]
            .iter()
            .find(|q| q.predicate == pred)
            .expect("provenance");
        assert_eq!(q.object, q_hash("release-label-x"));
        assert_eq!(q.metadata & 0xFFFF_FFFF, 42);
        assert_eq!(q.context, q_hash("release-label-x"));
    }

    #[test]
    fn duplicate_id_different_accession_conflicts() {
        let records = [
            rec(1, "CHEBI:1", "one", None, 2),
            rec(1, "CHEBI:999", "spoof", None, 3),
        ];
        let mut out = [NQuin::default(); 16];
        let report = map_records_into(&records, "rel", budgets(16, 4), &mut out).unwrap();
        assert_eq!(report.records_mapped, 1);
        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.conflicts[0].reason, "duplicate_id");
        assert_eq!(report.conflicts[0].accession, "CHEBI:999");
    }

    #[test]
    fn predicate_hashes_are_sixty_bit() {
        for iri in [
            PRED_RDF_TYPE,
            CLASS_CHEBI_COMPOUND,
            PRED_ACCESSION,
            PRED_HAS_NAME,
            PRED_HAS_PARENT,
            PRED_FROM_RELEASE,
        ] {
            let h = q_hash(iri);
            assert_eq!(h & !0x0FFF_FFFF_FFFF_FFFF, 0, "{iri}");
        }
    }

    #[test]
    fn format_chebi_accession_matches_string() {
        let mut buf = [0u8; 32];
        assert_eq!(format_chebi_accession(15377, &mut buf), "CHEBI:15377");
        assert_eq!(format_chebi_accession(0, &mut buf), "CHEBI:0");
    }
}
