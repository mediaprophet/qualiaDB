//! Graph-augmented retrieval — keyword-based triple retrieval.
//!
//! A tiny in-memory triple store with an entity → triple-index map. Queries
//! are ranked by term overlap between the query and each triple's components.
//! Deterministic, WASM-compatible, no embeddings.

use std::collections::HashMap;

/// One ranked retrieval result.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphRagResult {
    pub triple: (String, String, String),
    pub score: f64,
}

/// In-memory graph-RAG index.
#[derive(Debug, Clone, Default)]
pub struct GraphRagIndex {
    pub triples: Vec<(String, String, String)>,
    pub entity_index: HashMap<String, Vec<usize>>,
}

impl GraphRagIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a triple and index every component token.
    pub fn add_triple(&mut self, subject: &str, predicate: &str, object: &str) {
        let id = self.triples.len();
        let triple = (
            subject.to_string(),
            predicate.to_string(),
            object.to_string(),
        );
        for token in tokenize_triple(&triple) {
            self.entity_index.entry(token).or_default().push(id);
        }
        self.triples.push(triple);
    }

    /// Query the index: rank triples by overlap of query terms with triple
    /// components. Returns up to `k` results, highest score first.
    pub fn query(&self, query_text: &str, k: usize) -> Vec<GraphRagResult> {
        let q_terms: Vec<String> = query_text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_ascii_lowercase())
            .collect();
        if q_terms.is_empty() || self.triples.is_empty() {
            return Vec::new();
        }
        let mut scored: Vec<(usize, f64)> = self
            .triples
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let triple_terms: Vec<String> = tokenize_triple(t);
                let mut hits = 0usize;
                for q in &q_terms {
                    if triple_terms.iter().any(|tt| tt == q) {
                        hits += 1;
                    }
                }
                let score = hits as f64 / q_terms.len() as f64;
                (i, score)
            })
            .filter(|(_, s)| *s > 0.0)
            .collect();
        // Deterministic sort: score desc, then index asc for stable ties.
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });
        scored
            .into_iter()
            .take(k)
            .map(|(i, score)| GraphRagResult {
                triple: self.triples[i].clone(),
                score,
            })
            .collect()
    }
}

/// Lowercase alphanumeric tokens across all three components of a triple.
fn tokenize_triple(t: &(String, String, String)) -> Vec<String> {
    let mut out = Vec::new();
    for comp in [&t.0, &t.1, &t.2] {
        for tok in comp.split(|c: char| !c.is_alphanumeric()) {
            if !tok.is_empty() {
                out.push(tok.to_ascii_lowercase());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_and_query_basic() {
        let mut idx = GraphRagIndex::new();
        idx.add_triple("Paris", "locatedIn", "France");
        idx.add_triple("Socrates", "rdf:type", "philosopher");
        idx.add_triple("cat", "has", "tail");
        let results = idx.query("Paris France", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].triple.0, "Paris");
        assert_eq!(results[0].triple.2, "France");
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn query_ranks_by_overlap() {
        let mut idx = GraphRagIndex::new();
        idx.add_triple("dog", "has", "tail");
        idx.add_triple("cat", "has", "tail");
        idx.add_triple("bird", "has", "wings");
        let results = idx.query("cat tail", 10);
        // "cat has tail" matches both terms → score 1.0; "dog has tail" matches
        // one → 0.5; "bird has wings" matches none.
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].triple.0, "cat");
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn empty_query_returns_empty() {
        let mut idx = GraphRagIndex::new();
        idx.add_triple("a", "b", "c");
        assert!(idx.query("", 10).is_empty());
    }

    #[test]
    fn empty_index_returns_empty() {
        let idx = GraphRagIndex::new();
        assert!(idx.query("anything", 10).is_empty());
    }

    #[test]
    fn k_limits_results() {
        let mut idx = GraphRagIndex::new();
        idx.add_triple("a", "has", "x");
        idx.add_triple("b", "has", "x");
        idx.add_triple("c", "has", "x");
        let results = idx.query("x", 2);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn entity_index_populated() {
        let mut idx = GraphRagIndex::new();
        idx.add_triple("Paris", "locatedIn", "France");
        assert!(idx.entity_index.contains_key("paris"));
        assert!(idx.entity_index.contains_key("france"));
        assert!(idx.entity_index.contains_key("locatedin"));
    }
}
