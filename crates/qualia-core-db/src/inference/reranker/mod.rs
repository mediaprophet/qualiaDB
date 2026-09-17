//! Cross-encoder reranker for candidate re-scoring.
//!
//! A reranker takes a query and a list of candidate documents and re-scores
//! them based on query-document relevance. This implementation uses a
//! deterministic symbolic scoring approach (no neural network required):
//!
//! 1. **Token overlap**: Jaccard similarity between query and document tokens.
//! 2. **BM25-style scoring**: Term frequency / inverse document frequency.
//! 3. **Phrase bonus**: Bonus for multi-word query phrases appearing in the document.
//! 4. **Position bonus**: Bonus for matches appearing early in the document.
//!
//! The final score is a weighted combination of these signals. Candidates
//! are returned sorted by descending score.

use std::collections::HashMap;

/// A single reranked result.
#[derive(Debug, Clone, PartialEq)]
pub struct RerankResult {
    /// Original index of the candidate in the input list.
    pub index: usize,
    /// Relevance score (higher = more relevant).
    pub score: f64,
}

/// Configuration for the reranker.
#[derive(Debug, Clone)]
pub struct RerankConfig {
    /// Weight for token overlap (Jaccard) signal. Default: 0.3.
    pub overlap_weight: f64,
    /// Weight for BM25 signal. Default: 0.5.
    pub bm25_weight: f64,
    /// Weight for phrase bonus. Default: 0.15.
    pub phrase_weight: f64,
    /// Weight for position bonus. Default: 0.05.
    pub position_weight: f64,
    /// BM25 k1 parameter. Default: 1.2.
    pub bm25_k1: f64,
    /// BM25 b parameter. Default: 0.75.
    pub bm25_b: f64,
}

impl Default for RerankConfig {
    fn default() -> Self {
        Self {
            overlap_weight: 0.3,
            bm25_weight: 0.5,
            phrase_weight: 0.15,
            position_weight: 0.05,
            bm25_k1: 1.2,
            bm25_b: 0.75,
        }
    }
}

/// Tokenise a string into lowercase alphanumeric tokens.
fn tokenise(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_ascii_lowercase())
        .collect()
}

/// Compute term frequencies for a token list.
fn term_freqs(tokens: &[String]) -> HashMap<String, f64> {
    let mut freqs = HashMap::new();
    let n = tokens.len() as f64;
    for t in tokens {
        *freqs.entry(t.clone()).or_insert(0.0) += 1.0;
    }
    // Normalise by document length.
    for v in freqs.values_mut() {
        *v /= n.max(1.0);
    }
    freqs
}

/// Compute Jaccard similarity between two token sets.
fn jaccard(a: &[String], b: &[String]) -> f64 {
    let set_a: std::collections::BTreeSet<&String> = a.iter().collect();
    let set_b: std::collections::BTreeSet<&String> = b.iter().collect();
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Compute BM25 score for a single document given query terms.
fn bm25_score(
    query_terms: &[String],
    doc_tokens: &[String],
    doc_freqs: &HashMap<String, f64>,
    avg_doc_len: f64,
    config: &RerankConfig,
) -> f64 {
    let doc_len = doc_tokens.len() as f64;
    let mut score = 0.0;
    let n_docs = 1.0; // Single-doc scoring; IDF computed externally if needed.

    for term in query_terms {
        let tf = doc_freqs.get(term).copied().unwrap_or(0.0);
        if tf == 0.0 {
            continue;
        }
        // Simplified IDF: since we're scoring one doc at a time, use a
        // fixed IDF of 1.0 (all terms equally rare). A full implementation
        // would compute IDF across the corpus.
        let idf = 1.0 + (n_docs + 0.5) / (tf + 0.5);
        let tf_component = (tf * (config.bm25_k1 + 1.0))
            / (tf
                + config.bm25_k1
                    * (1.0 - config.bm25_b + config.bm25_b * (doc_len / avg_doc_len.max(1.0))));
        score += idf * tf_component;
    }
    score
}

/// Check if a multi-word phrase from the query appears in the document.
fn phrase_bonus(query: &str, doc: &str) -> f64 {
    let query_lower = query.to_ascii_lowercase();
    let doc_lower = doc.to_ascii_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();
    if query_words.len() < 2 {
        return 0.0;
    }
    // Check for 2-grams and 3-grams from the query in the document.
    let mut bonus = 0.0;
    for n in 2..=3.min(query_words.len()) {
        for window in query_words.windows(n) {
            let phrase = window.join(" ");
            if doc_lower.contains(&phrase) {
                bonus += 0.1 * n as f64;
            }
        }
    }
    bonus.min(1.0)
}

/// Position bonus: matches early in the document get higher scores.
fn position_bonus(query_terms: &[String], doc_tokens: &[String]) -> f64 {
    if doc_tokens.is_empty() {
        return 0.0;
    }
    let doc_len = doc_tokens.len();
    let mut earliest_match = doc_len;
    for (i, token) in doc_tokens.iter().enumerate() {
        if query_terms.contains(token) {
            earliest_match = earliest_match.min(i);
        }
    }
    if earliest_match == doc_len {
        0.0
    } else {
        // Score decreases linearly with position.
        1.0 - (earliest_match as f64 / doc_len as f64)
    }
}

/// Rerank candidates given a query.
///
/// Returns results sorted by descending score, each with the original index.
pub fn rerank(query: &str, candidates: &[String], config: &RerankConfig) -> Vec<RerankResult> {
    let query_tokens = tokenise(query);
    if query_tokens.is_empty() || candidates.is_empty() {
        return candidates
            .iter()
            .enumerate()
            .map(|(i, _)| RerankResult {
                index: i,
                score: 0.0,
            })
            .collect();
    }

    let candidate_tokens: Vec<Vec<String>> = candidates.iter().map(|c| tokenise(c)).collect();
    let avg_doc_len =
        candidate_tokens.iter().map(|t| t.len() as f64).sum::<f64>() / candidates.len() as f64;

    let mut results = Vec::with_capacity(candidates.len());
    for (i, (candidate, doc_tokens)) in candidates.iter().zip(candidate_tokens.iter()).enumerate() {
        let overlap = jaccard(&query_tokens, doc_tokens);
        let doc_freqs = term_freqs(doc_tokens);
        let bm25 = bm25_score(&query_tokens, doc_tokens, &doc_freqs, avg_doc_len, config);
        let phrase = phrase_bonus(query, candidate);
        let position = position_bonus(&query_tokens, doc_tokens);

        let score = config.overlap_weight * overlap
            + config.bm25_weight * (bm25 / (bm25 + 1.0)) // Normalise BM25 to [0,1]
            + config.phrase_weight * phrase
            + config.position_weight * position;

        results.push(RerankResult { index: i, score });
    }

    // Sort by descending score.
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

/// Rerank candidates with default configuration.
pub fn rerank_default(query: &str, candidates: &[String]) -> Vec<RerankResult> {
    rerank(query, candidates, &RerankConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rerank_basic_ordering() {
        let query = "machine learning algorithms";
        let candidates = vec![
            "The weather is nice today".to_string(),
            "Machine learning algorithms are powerful tools".to_string(),
            "I like pizza".to_string(),
        ];
        let results = rerank_default(query, &candidates);
        assert_eq!(results.len(), 3);
        // The ML document should be ranked first.
        assert_eq!(results[0].index, 1);
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn rerank_empty_query() {
        let candidates = vec!["doc one".to_string(), "doc two".to_string()];
        let results = rerank_default("", &candidates);
        assert_eq!(results.len(), 2);
        // All scores should be 0.
        assert!(results.iter().all(|r| r.score == 0.0));
    }

    #[test]
    fn rerank_empty_candidates() {
        let results = rerank_default("query", &[]);
        assert!(results.is_empty());
    }

    #[test]
    fn rerank_identical_candidates() {
        let query = "hello world";
        let candidates = vec!["hello world".to_string(), "hello world".to_string()];
        let results = rerank_default(query, &candidates);
        assert_eq!(results.len(), 2);
        // Both should have the same score.
        assert!((results[0].score - results[1].score).abs() < 1e-9);
    }

    #[test]
    fn rerank_phrase_bonus() {
        let query = "machine learning";
        let candidates = vec![
            "machine and learning separately".to_string(),
            "machine learning together".to_string(),
        ];
        let results = rerank_default(query, &candidates);
        // The phrase match should rank higher.
        assert_eq!(results[0].index, 1);
    }

    #[test]
    fn rerank_position_bonus() {
        let query = "important";
        let candidates = vec![
            "not relevant text here important at end".to_string(),
            "important at the start of the document".to_string(),
        ];
        let results = rerank_default(query, &candidates);
        // Earlier match should rank higher.
        assert_eq!(results[0].index, 1);
    }

    #[test]
    fn rerank_custom_config() {
        let query = "test query";
        let candidates = vec!["test document".to_string()];
        let config = RerankConfig {
            overlap_weight: 1.0,
            bm25_weight: 0.0,
            phrase_weight: 0.0,
            position_weight: 0.0,
            ..Default::default()
        };
        let results = rerank(query, &candidates, &config);
        assert_eq!(results.len(), 1);
        // With only overlap weight, score should be the Jaccard similarity.
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn rerank_scores_in_descending_order() {
        let query = "alpha beta gamma";
        let candidates = vec![
            "alpha".to_string(),
            "alpha beta gamma delta".to_string(),
            "alpha beta".to_string(),
        ];
        let results = rerank_default(query, &candidates);
        for i in 1..results.len() {
            assert!(
                results[i - 1].score >= results[i].score,
                "results not in descending order at index {i}"
            );
        }
    }

    #[test]
    fn tokenise_handles_punctuation() {
        let tokens = tokenise("Hello, world! Foo-Bar");
        assert_eq!(tokens, vec!["hello", "world", "foo", "bar"]);
    }

    #[test]
    fn jaccard_identical() {
        let a = tokenise("hello world");
        let b = tokenise("hello world");
        assert_eq!(jaccard(&a, &b), 1.0);
    }

    #[test]
    fn jaccard_disjoint() {
        let a = tokenise("hello world");
        let b = tokenise("foo bar");
        assert_eq!(jaccard(&a, &b), 0.0);
    }

    #[test]
    fn phrase_bonus_multiword() {
        let bonus = phrase_bonus("machine learning models", "machine learning is great");
        assert!(bonus > 0.0);
    }

    #[test]
    fn phrase_bonus_single_word() {
        let bonus = phrase_bonus("hello", "hello world");
        assert_eq!(bonus, 0.0);
    }

    #[test]
    fn position_bonus_early_match() {
        let query = tokenise("important");
        let doc = tokenise("important text follows here");
        let bonus = position_bonus(&query, &doc);
        assert!(bonus > 0.9); // Very early match
    }

    #[test]
    fn position_bonus_no_match() {
        let query = tokenise("missing");
        let doc = tokenise("some text here");
        let bonus = position_bonus(&query, &doc);
        assert_eq!(bonus, 0.0);
    }
}
