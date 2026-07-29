//! Bounded graph query — map journal record IDs to materialized quin counts.

use qualia_core_db::NQuin;
use wellfare_core::record::q_hash_str;

use super::journal::JournalEntry;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GraphCoverageRow {
    pub record_id: String,
    pub kind: String,
    pub quin_count: usize,
}

/// Count quins whose subject field matches the FNV-1a hash of `record_id`.
pub fn count_quins_for_record(quins: &[NQuin], record_id: &str) -> usize {
    let subject = q_hash_str(record_id);
    quins.iter().filter(|q| q.subject == subject).count()
}

/// For each journal row, report how many materialized quins reference that record id.
pub fn coverage_for_journal(journal: &[JournalEntry], quins: &[NQuin]) -> Vec<GraphCoverageRow> {
    journal
        .iter()
        .map(|entry| GraphCoverageRow {
            record_id: entry.id.clone(),
            kind: entry.kind.clone(),
            quin_count: count_quins_for_record(quins, &entry.id),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::NQuin;

    #[test]
    fn count_quins_matches_subject_hash() {
        let record_id = "urn:wellfair:weight:abc";
        let subject = q_hash_str(record_id);
        let quins = [
            NQuin {
                subject,
                predicate: 1,
                object: 2,
                context: 3,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 99,
                predicate: 1,
                object: 2,
                context: 3,
                metadata: 0,
                parity: 0,
            },
        ];
        assert_eq!(count_quins_for_record(&quins, record_id), 1);
    }
}
