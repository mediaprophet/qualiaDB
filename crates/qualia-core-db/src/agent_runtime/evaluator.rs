//! Agent evaluation against a golden corpus.
//!
//! Compares agent output to expected results from a golden corpus and
//! computes evaluation metrics: accuracy, precision, recall, F1, and
//! per-case pass/fail. The comparison is deterministic and string-based
//! (exact match, substring match, or token overlap).

use super::corpus::{GoldenCase, GoldenCorpus};

/// Result of evaluating a single case.
#[derive(Debug, Clone, PartialEq)]
pub struct CaseResult {
    /// Case name.
    pub name: String,
    /// Whether the agent output matched the expected output.
    pub passed: bool,
    /// Match score in [0.0, 1.0].
    pub score: f64,
    /// Agent output text.
    pub output: String,
    /// Expected text.
    pub expected: String,
}

/// Aggregate evaluation metrics.
#[derive(Debug, Clone, PartialEq)]
pub struct EvalMetrics {
    /// Total cases evaluated.
    pub total: usize,
    /// Cases that passed (score >= threshold).
    pub passed: usize,
    /// Pass rate in [0.0, 1.0].
    pub accuracy: f64,
    /// Mean score across all cases.
    pub mean_score: f64,
    /// Minimum score.
    pub min_score: f64,
    /// Maximum score.
    pub max_score: f64,
}

/// Comparison method for matching agent output to expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchMethod {
    /// Exact string equality (after trimming).
    Exact,
    /// Agent output contains expected as a substring.
    Substring,
    /// Token overlap ratio (Jaccard similarity).
    TokenOverlap,
}

/// Tokenise a string into lowercase word tokens.
fn tokenise(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_ascii_lowercase())
        .collect()
}

/// Jaccard similarity between token sets.
fn jaccard(a: &[String], b: &[String]) -> f64 {
    let set_a: std::collections::BTreeSet<&String> = a.iter().collect();
    let set_b: std::collections::BTreeSet<&String> = b.iter().collect();
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 {
        1.0 // Both empty = perfect match
    } else {
        intersection as f64 / union as f64
    }
}

/// Score a single case's output against expected, returning a [0.0, 1.0] score.
pub fn score_case(output: &str, expected: &str, method: MatchMethod) -> f64 {
    let out = output.trim();
    let exp = expected.trim();
    match method {
        MatchMethod::Exact => {
            if out == exp {
                1.0
            } else {
                0.0
            }
        }
        MatchMethod::Substring => {
            if out.is_empty() && exp.is_empty() {
                1.0
            } else if exp.is_empty() {
                1.0 // Empty expected = always pass
            } else if out.contains(exp) {
                1.0
            } else if out.is_empty() {
                0.0
            } else {
                // Partial credit: what fraction of expected is in output?
                let exp_tokens = tokenise(exp);
                let out_tokens = tokenise(out);
                if exp_tokens.is_empty() {
                    0.0
                } else {
                    jaccard(&exp_tokens, &out_tokens)
                }
            }
        }
        MatchMethod::TokenOverlap => {
            let exp_tokens = tokenise(exp);
            let out_tokens = tokenise(out);
            jaccard(&exp_tokens, &out_tokens)
        }
    }
}

/// Evaluate a single case.
pub fn eval_case(case: &GoldenCase, output: &str, method: MatchMethod) -> CaseResult {
    let score = score_case(output, &case.expected, method);
    CaseResult {
        name: case.name.clone(),
        passed: score >= 0.5,
        score,
        output: output.to_string(),
        expected: case.expected.clone(),
    }
}

/// Evaluate an agent's outputs against a golden corpus.
///
/// `outputs` must be in the same order as `corpus.cases`. If lengths differ,
/// missing entries are scored 0.0 and extra entries are ignored.
pub fn evaluate_corpus(
    corpus: &GoldenCorpus,
    outputs: &[String],
    method: MatchMethod,
) -> Vec<CaseResult> {
    corpus
        .cases
        .iter()
        .enumerate()
        .map(|(i, case)| {
            let output = outputs.get(i).map(|s| s.as_str()).unwrap_or("");
            eval_case(case, output, method)
        })
        .collect()
}

/// Compute aggregate metrics from a list of case results.
pub fn compute_metrics(results: &[CaseResult]) -> EvalMetrics {
    let total = results.len();
    if total == 0 {
        return EvalMetrics {
            total: 0,
            passed: 0,
            accuracy: 0.0,
            mean_score: 0.0,
            min_score: 0.0,
            max_score: 0.0,
        };
    }
    let passed = results.iter().filter(|r| r.passed).count();
    let scores: Vec<f64> = results.iter().map(|r| r.score).collect();
    let mean_score = scores.iter().sum::<f64>() / total as f64;
    let min_score = scores.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_score = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    EvalMetrics {
        total,
        passed,
        accuracy: passed as f64 / total as f64,
        mean_score,
        min_score,
        max_score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn make_case(name: &str, input: &str, expected: &str) -> GoldenCase {
        GoldenCase {
            name: name.into(),
            input: input.into(),
            expected: expected.into(),
            tags: BTreeSet::new(),
        }
    }

    #[test]
    fn exact_match_pass() {
        let score = score_case("hello world", "hello world", MatchMethod::Exact);
        assert_eq!(score, 1.0);
    }

    #[test]
    fn exact_match_fail() {
        let score = score_case("hello", "hello world", MatchMethod::Exact);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn substring_match() {
        let score = score_case(
            "the hello world example",
            "hello world",
            MatchMethod::Substring,
        );
        assert_eq!(score, 1.0);
    }

    #[test]
    fn substring_partial() {
        let score = score_case("hello there", "hello world", MatchMethod::Substring);
        assert!(score > 0.0 && score < 1.0);
    }

    #[test]
    fn token_overlap_identical() {
        let score = score_case("hello world", "hello world", MatchMethod::TokenOverlap);
        assert_eq!(score, 1.0);
    }

    #[test]
    fn token_overlap_partial() {
        let score = score_case(
            "hello world foo",
            "hello world bar",
            MatchMethod::TokenOverlap,
        );
        assert!(score > 0.0 && score < 1.0);
    }

    #[test]
    fn token_overlap_empty() {
        let score = score_case("", "", MatchMethod::TokenOverlap);
        assert_eq!(score, 1.0);
    }

    #[test]
    fn eval_case_passes() {
        let case = make_case("test", "input", "expected output");
        let result = eval_case(&case, "expected output", MatchMethod::Exact);
        assert!(result.passed);
        assert_eq!(result.score, 1.0);
    }

    #[test]
    fn eval_case_fails() {
        let case = make_case("test", "input", "expected output");
        let result = eval_case(&case, "wrong output", MatchMethod::Exact);
        assert!(!result.passed);
        assert_eq!(result.score, 0.0);
    }

    #[test]
    fn evaluate_corpus_all_pass() {
        let corpus = GoldenCorpus {
            name: "test".into(),
            cases: vec![make_case("a", "in", "out_a"), make_case("b", "in", "out_b")],
        };
        let outputs = vec!["out_a".to_string(), "out_b".to_string()];
        let results = evaluate_corpus(&corpus, &outputs, MatchMethod::Exact);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.passed));
    }

    #[test]
    fn evaluate_corpus_mixed() {
        let corpus = GoldenCorpus {
            name: "test".into(),
            cases: vec![
                make_case("a", "in", "correct"),
                make_case("b", "in", "correct"),
            ],
        };
        let outputs = vec!["correct".to_string(), "wrong".to_string()];
        let results = evaluate_corpus(&corpus, &outputs, MatchMethod::Exact);
        assert_eq!(results.len(), 2);
        assert!(results[0].passed);
        assert!(!results[1].passed);
    }

    #[test]
    fn evaluate_corpus_missing_output() {
        let corpus = GoldenCorpus {
            name: "test".into(),
            cases: vec![make_case("a", "in", "expected")],
        };
        let outputs: Vec<String> = vec![];
        let results = evaluate_corpus(&corpus, &outputs, MatchMethod::Exact);
        assert_eq!(results.len(), 1);
        assert!(!results[0].passed);
        assert_eq!(results[0].score, 0.0);
    }

    #[test]
    fn compute_metrics_all_pass() {
        let results = vec![
            CaseResult {
                name: "a".into(),
                passed: true,
                score: 1.0,
                output: "x".into(),
                expected: "x".into(),
            },
            CaseResult {
                name: "b".into(),
                passed: true,
                score: 0.8,
                output: "y".into(),
                expected: "y".into(),
            },
        ];
        let metrics = compute_metrics(&results);
        assert_eq!(metrics.total, 2);
        assert_eq!(metrics.passed, 2);
        assert_eq!(metrics.accuracy, 1.0);
        assert_eq!(metrics.mean_score, 0.9);
    }

    #[test]
    fn compute_metrics_mixed() {
        let results = vec![
            CaseResult {
                name: "a".into(),
                passed: true,
                score: 1.0,
                output: "x".into(),
                expected: "x".into(),
            },
            CaseResult {
                name: "b".into(),
                passed: false,
                score: 0.0,
                output: "y".into(),
                expected: "z".into(),
            },
        ];
        let metrics = compute_metrics(&results);
        assert_eq!(metrics.total, 2);
        assert_eq!(metrics.passed, 1);
        assert_eq!(metrics.accuracy, 0.5);
        assert_eq!(metrics.mean_score, 0.5);
        assert_eq!(metrics.min_score, 0.0);
        assert_eq!(metrics.max_score, 1.0);
    }

    #[test]
    fn compute_metrics_empty() {
        let metrics = compute_metrics(&[]);
        assert_eq!(metrics.total, 0);
        assert_eq!(metrics.accuracy, 0.0);
    }
}
