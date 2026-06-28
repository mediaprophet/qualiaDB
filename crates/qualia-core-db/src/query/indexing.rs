use crate::NQuin;
use std::collections::HashMap;

/// In-memory inverted index over a `NQuin` collection.
///
/// Provides O(1) average lookup by subject, predicate, object, or context. Build once
/// from a slice (`from_slice`) or grow incrementally (`insert`); the zero-alloc
/// `iter_*` / `object_of` accessors yield copies, so they stay valid across the `Vec`
/// reallocation an `insert` can trigger. Designed to live per 512 MB cell; wiring it to
/// BIDX/demand-paging across cells is separate (task #22).
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
                if q.predicate == p {
                    Some(*q)
                } else {
                    None
                }
            })
            .collect()
    }

    // ── Zero-allocation accessors (the modal-kind resolution hot path, task #22) ──
    // The `by_*` methods above each return `Vec<NQuin>` — one heap allocation per
    // call, unacceptable for continuous resolution. These yield `NQuin` BY VALUE (it
    // is `Copy`, 48 bytes) while borrowing the index: no per-call heap alloc, and —
    // because they yield copies, not `&NQuin` into the backing `Vec` — they remain
    // valid across the incremental `insert()` that may reallocate it. Keep them OUT of
    // the SIMD/GPU vectorized loop (random-access gather): this is the CPU/logic-layer
    // resolution path (see `frame_layout` "Tag policy").

    /// Zero-alloc: every quin with this subject, yielded by copy.
    pub fn iter_by_subject(&self, s: u64) -> impl Iterator<Item = NQuin> + '_ {
        self.by_subject
            .get(&s)
            .map(Vec::as_slice)
            .unwrap_or(&[])
            .iter()
            .map(move |&i| self.quins[i])
    }

    /// Zero-alloc: every quin matching subject AND predicate, yielded by copy.
    pub fn iter_by_subject_and_predicate(
        &self,
        s: u64,
        p: u64,
    ) -> impl Iterator<Item = NQuin> + '_ {
        self.iter_by_subject(s).filter(move |q| q.predicate == p)
    }

    /// Zero-alloc modal-kind resolution primitive: the first object of `(s, p)`.
    /// e.g. `object_of(identifier, has_modality_kind)` resolves an identifier's kind
    /// in one point lookup with no heap allocation.
    pub fn object_of(&self, s: u64, p: u64) -> Option<u64> {
        self.iter_by_subject_and_predicate(s, p)
            .next()
            .map(|q| q.object)
    }

    /// Zero-copy raw backing-store row indices for a subject, for callers that gather
    /// into their own contiguous scratch buffer (pair with `quin_at`).
    pub fn rows_by_subject(&self, s: u64) -> &[usize] {
        self.by_subject.get(&s).map(Vec::as_slice).unwrap_or(&[])
    }

    /// The quin at a backing-store row index. Copy, no allocation.
    #[inline]
    pub fn quin_at(&self, i: usize) -> NQuin {
        self.quins[i]
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

    #[test]
    fn incremental_insert_survives_reallocation_with_zero_alloc_accessors() {
        // Grow well past initial capacity so the backing Vec reallocates, then confirm
        // the copy-yielding accessors still resolve correctly (the basis for per-cell
        // incremental indexing — task #22).
        let mut idx = QuinIndex::new();
        // Kind subjects are OUTSIDE the growth range (0..2000) so each appears once.
        idx.insert(make_quin(5000, 101, 12345, 0)); // before the growth
        for i in 0..2000u64 {
            idx.insert(make_quin(i, 200, i + 1, 0));
        }
        idx.insert(make_quin(6000, 101, 67890, 0)); // and one after
        assert_eq!(idx.len(), 2002);

        // object_of / iter_* yield copies, so they survive the reallocation above.
        assert_eq!(idx.object_of(5000, 101), Some(12345));
        assert_eq!(idx.object_of(6000, 101), Some(67890));
        assert_eq!(idx.iter_by_subject(5000).count(), 1);
        assert_eq!(idx.iter_by_subject_and_predicate(6000, 101).count(), 1);
        assert!(idx.object_of(5000, 999).is_none());
    }
}
