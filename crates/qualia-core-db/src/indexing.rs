use crate::NQuin;
use std::collections::HashMap;

/// In-memory inverted index over a `NQuin` collection.
///
/// Provides O(1) average lookup by subject, predicate, object, or context.
/// Built once from a slice of quins; not designed for incremental updates.
pub struct QuinIndex {
    quins: Vec<NQuin>,
    by_subject: HashMap<u64, Vec<usize>>,
    by_predicate: HashMap<u64, Vec<usize>>,
    by_object: HashMap<u64, Vec<usize>>,
    by_context: HashMap<u64, Vec<usize>>,
}

impl QuinIndex {
    /// Build an index from a slice of quins (copied into the index).
    pub fn from_slice(quins: &[NQuin]) -> Self {
        let mut idx = Self {
            quins: quins.to_vec(),
            by_subject: HashMap::new(),
            by_predicate: HashMap::new(),
            by_object: HashMap::new(),
            by_context: HashMap::new(),
        };
        for (i, q) in idx.quins.iter().enumerate() {
            idx.by_subject.entry(q.subject).or_default().push(i);
            idx.by_predicate.entry(q.predicate).or_default().push(i);
            idx.by_object.entry(q.object).or_default().push(i);
            idx.by_context.entry(q.context).or_default().push(i);
        }
        idx
    }

    /// Build an empty index and populate via `insert()`.
    pub fn new() -> Self {
        Self {
            quins: Vec::new(),
            by_subject: HashMap::new(),
            by_predicate: HashMap::new(),
            by_object: HashMap::new(),
            by_context: HashMap::new(),
        }
    }

    /// Insert a single quin into the index.
    pub fn insert(&mut self, quin: NQuin) {
        let i = self.quins.len();
        self.by_subject.entry(quin.subject).or_default().push(i);
        self.by_predicate.entry(quin.predicate).or_default().push(i);
        self.by_object.entry(quin.object).or_default().push(i);
        self.by_context.entry(quin.context).or_default().push(i);
        self.quins.push(quin);
    }

    pub fn len(&self) -> usize {
        self.quins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.quins.is_empty()
    }

    pub fn by_subject(&self, id: u64) -> Vec<NQuin> {
        self.lookup(&self.by_subject, id)
    }

    pub fn by_predicate(&self, id: u64) -> Vec<NQuin> {
        self.lookup(&self.by_predicate, id)
    }

    pub fn by_object(&self, id: u64) -> Vec<NQuin> {
        self.lookup(&self.by_object, id)
    }

    pub fn by_context(&self, id: u64) -> Vec<NQuin> {
        self.lookup(&self.by_context, id)
    }

    /// Returns all quins where subject==s AND predicate==p.
    pub fn by_subject_and_predicate(&self, s: u64, p: u64) -> Vec<NQuin> {
        let Some(rows) = self.by_subject.get(&s) else {
            return vec![];
        };
        rows.iter()
            .filter_map(|&i| {
                let q = &self.quins[i];
                if q.predicate == p { Some(*q) } else { None }
            })
            .collect()
    }

    fn lookup(&self, map: &HashMap<u64, Vec<usize>>, key: u64) -> Vec<NQuin> {
        map.get(&key)
            .map(|indices| indices.iter().map(|&i| self.quins[i]).collect())
            .unwrap_or_default()
    }
}

impl Default for QuinIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_quin(s: u64, p: u64, o: u64, c: u64) -> NQuin {
        NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: c,
            metadata: 0,
            parity: NQuin::calculate_parity(s, p, o, c, 0),
        }
    }

    #[test]
    fn index_lookup_by_subject() {
        let quins = vec![
            make_quin(1, 10, 100, 1000),
            make_quin(2, 20, 200, 2000),
            make_quin(1, 30, 300, 3000),
        ];
        let idx = QuinIndex::from_slice(&quins);
        let hits = idx.by_subject(1);
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|q| q.subject == 1));
    }

    #[test]
    fn index_lookup_by_context() {
        let quins = vec![
            make_quin(1, 10, 100, 42),
            make_quin(2, 20, 200, 42),
            make_quin(3, 30, 300, 99),
        ];
        let idx = QuinIndex::from_slice(&quins);
        assert_eq!(idx.by_context(42).len(), 2);
        assert_eq!(idx.by_context(99).len(), 1);
        assert_eq!(idx.by_context(0).len(), 0);
    }

    #[test]
    fn index_incremental_insert() {
        let mut idx = QuinIndex::new();
        idx.insert(make_quin(5, 6, 7, 8));
        idx.insert(make_quin(5, 9, 10, 11));
        assert_eq!(idx.len(), 2);
        assert_eq!(idx.by_subject(5).len(), 2);
    }

    #[test]
    fn index_subject_and_predicate() {
        let quins = vec![
            make_quin(1, 10, 100, 1000),
            make_quin(1, 20, 200, 2000),
            make_quin(2, 10, 300, 3000),
        ];
        let idx = QuinIndex::from_slice(&quins);
        let hits = idx.by_subject_and_predicate(1, 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].object, 100);
    }
}
