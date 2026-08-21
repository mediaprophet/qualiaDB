//! Active learning invoke seams — uncertainty sampling, density-weighted
//! querying, and query-by-committee disagreement measures.

use super::super::args;
use crate::solvers::learning::active as al;
use poet_vibe::{Diagnostic, Span, Value};

fn parse_strategy(v: &Value) -> al::Strategy {
    match args::rec_str(v, "strategy").unwrap_or("entropy") {
        "LeastConfidence" | "least_confidence" => al::Strategy::LeastConfidence,
        "Margin" | "margin" => al::Strategy::Margin,
        _ => al::Strategy::Entropy,
    }
}

fn parse_prob_rows(v: &Value, key: &str) -> Option<Vec<Vec<f64>>> {
    let list_val = args::rec(v, key)?;
    let items = args::list(&list_val)?;
    Some(items.iter().filter_map(|r| args::f64s(r)).collect())
}

fn parse_feature_rows(v: &Value, key: &str) -> Option<Vec<Vec<f64>>> {
    parse_prob_rows(v, key)
}

// ── Uncertainty sampling ────────────────────────────────────────────

/// `MachineLearning.al_row_score` — uncertainty score for a single row.
/// Args: { row: [f64], strategy: "LeastConfidence"|"Margin"|"Entropy" }
pub fn al_row_score(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let row =
        args::rec_f64_list(args, "row").ok_or_else(|| args::bad(span, "al_row_score needs row"))?;
    let strategy = parse_strategy(args);
    match al::uncertainty::row_score(&row, strategy) {
        Ok(s) => Ok(Value::F64(s)),
        Err(e) => Err(args::bad(span, format!("al_row_score: {e:?}"))),
    }
}

/// `MachineLearning.al_score` — uncertainty scores for a pool.
/// Args: { probs: [[f64]], strategy: "..." }
pub fn al_score(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let probs =
        parse_prob_rows(args, "probs").ok_or_else(|| args::bad(span, "al_score needs probs"))?;
    let strategy = parse_strategy(args);
    match al::uncertainty::score(&probs, strategy) {
        Ok(scores) => Ok(args::f64_list_value(scores)),
        Err(e) => Err(args::bad(span, format!("al_score: {e:?}"))),
    }
}

/// `MachineLearning.al_rank_informative` — rank pool by uncertainty (most first).
/// Args: { probs: [[f64]], strategy: "..." }
pub fn al_rank_informative(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let probs = parse_prob_rows(args, "probs")
        .ok_or_else(|| args::bad(span, "al_rank_informative needs probs"))?;
    let strategy = parse_strategy(args);
    match al::uncertainty::rank_informative(&probs, strategy) {
        Ok(ranks) => Ok(Value::List(
            ranks.iter().map(|&i| Value::U64(i as u64)).collect(),
        )),
        Err(e) => Err(args::bad(span, format!("al_rank_informative: {e:?}"))),
    }
}

/// `MachineLearning.al_most_informative` — index of the most informative sample.
/// Args: { probs: [[f64]], strategy: "..." }
pub fn al_most_informative(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let probs = parse_prob_rows(args, "probs")
        .ok_or_else(|| args::bad(span, "al_most_informative needs probs"))?;
    let strategy = parse_strategy(args);
    match al::uncertainty::most_informative(&probs, strategy) {
        Ok(idx) => Ok(Value::U64(idx as u64)),
        Err(e) => Err(args::bad(span, format!("al_most_informative: {e:?}"))),
    }
}

// ── Density-weighted sampling ───────────────────────────────────────

/// `MachineLearning.al_cosine_similarity` — cosine similarity of two vectors.
/// Args: { a: [f64], b: [f64] }
pub fn al_cosine_similarity(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64_list(args, "a")
        .ok_or_else(|| args::bad(span, "al_cosine_similarity needs a"))?;
    let b = args::rec_f64_list(args, "b")
        .ok_or_else(|| args::bad(span, "al_cosine_similarity needs b"))?;
    match al::density::cosine_similarity(&a, &b) {
        Ok(s) => Ok(Value::F64(s)),
        Err(e) => Err(args::bad(span, format!("al_cosine_similarity: {e:?}"))),
    }
}

/// `MachineLearning.al_representativeness` — mean representativeness per point.
/// Args: { features: [[f64]] }
pub fn al_representativeness(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let features = parse_feature_rows(args, "features")
        .ok_or_else(|| args::bad(span, "al_representativeness needs features"))?;
    match al::density::representativeness(&features) {
        Ok(scores) => Ok(args::f64_list_value(scores)),
        Err(e) => Err(args::bad(span, format!("al_representativeness: {e:?}"))),
    }
}

/// `MachineLearning.al_information_density` — uncertainty × representativeness^β.
/// Args: { uncertainty: [f64], features: [[f64]], beta: f64 }
pub fn al_information_density(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let uncertainty = args::rec_f64_list(args, "uncertainty")
        .ok_or_else(|| args::bad(span, "al_information_density needs uncertainty"))?;
    let features = parse_feature_rows(args, "features")
        .ok_or_else(|| args::bad(span, "al_information_density needs features"))?;
    let beta = args::rec_f64(args, "beta").unwrap_or(1.0);
    match al::density::information_density(&uncertainty, &features, beta) {
        Ok(scores) => Ok(args::f64_list_value(scores)),
        Err(e) => Err(args::bad(span, format!("al_information_density: {e:?}"))),
    }
}

/// `MachineLearning.al_rank_by_density` — rank pool by information density.
/// Args: { uncertainty: [f64], features: [[f64]], beta: f64 }
pub fn al_rank_by_density(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let uncertainty = args::rec_f64_list(args, "uncertainty")
        .ok_or_else(|| args::bad(span, "al_rank_by_density needs uncertainty"))?;
    let features = parse_feature_rows(args, "features")
        .ok_or_else(|| args::bad(span, "al_rank_by_density needs features"))?;
    let beta = args::rec_f64(args, "beta").unwrap_or(1.0);
    match al::density::rank_by_density(&uncertainty, &features, beta) {
        Ok(ranks) => Ok(Value::List(
            ranks.iter().map(|&i| Value::U64(i as u64)).collect(),
        )),
        Err(e) => Err(args::bad(span, format!("al_rank_by_density: {e:?}"))),
    }
}

// ── Query-by-committee ──────────────────────────────────────────────

/// `MachineLearning.al_vote_entropy` — vote entropy of a committee.
/// Args: { votes: [u64], n_classes: u64 }
pub fn al_vote_entropy(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let votes_u64 = args::rec_u64_list(args, "votes")
        .ok_or_else(|| args::bad(span, "al_vote_entropy needs votes"))?;
    let votes: Vec<usize> = votes_u64.iter().map(|&v| v as usize).collect();
    let n_classes = args::rec_u64(args, "n_classes")
        .ok_or_else(|| args::bad(span, "al_vote_entropy needs n_classes"))?
        as usize;
    match al::committee::vote_entropy(&votes, n_classes) {
        Ok(h) => Ok(Value::F64(h)),
        Err(e) => Err(args::bad(span, format!("al_vote_entropy: {e:?}"))),
    }
}

/// `MachineLearning.al_consensus` — consensus distribution of a committee.
/// Args: { members: [[f64]] }
pub fn al_consensus(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let members = parse_prob_rows(args, "members")
        .ok_or_else(|| args::bad(span, "al_consensus needs members"))?;
    match al::committee::consensus(&members) {
        Ok(cons) => Ok(args::f64_list_value(cons)),
        Err(e) => Err(args::bad(span, format!("al_consensus: {e:?}"))),
    }
}

/// `MachineLearning.al_consensus_entropy` — entropy of the consensus distribution.
/// Args: { members: [[f64]] }
pub fn al_consensus_entropy(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let members = parse_prob_rows(args, "members")
        .ok_or_else(|| args::bad(span, "al_consensus_entropy needs members"))?;
    match al::committee::consensus_entropy(&members) {
        Ok(h) => Ok(Value::F64(h)),
        Err(e) => Err(args::bad(span, format!("al_consensus_entropy: {e:?}"))),
    }
}

/// `MachineLearning.al_average_kl_disagreement` — mean KL(member || consensus).
/// Args: { members: [[f64]] }
pub fn al_average_kl_disagreement(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let members = parse_prob_rows(args, "members")
        .ok_or_else(|| args::bad(span, "al_average_kl_disagreement needs members"))?;
    match al::committee::average_kl_disagreement(&members) {
        Ok(kl) => Ok(Value::F64(kl)),
        Err(e) => Err(args::bad(
            span,
            format!("al_average_kl_disagreement: {e:?}"),
        )),
    }
}

/// `MachineLearning.al_rank_by_disagreement` — rank pool by KL disagreement.
/// Args: { pool: [[[f64]]] }  (n_samples × n_members × n_classes)
pub fn al_rank_by_disagreement(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let pool_val = args::rec(args, "pool")
        .ok_or_else(|| args::bad(span, "al_rank_by_disagreement needs pool"))?;
    let pool_items = args::list(pool_val)
        .ok_or_else(|| args::bad(span, "al_rank_by_disagreement: pool must be a list"))?;
    let mut pool: Vec<Vec<Vec<f64>>> = Vec::new();
    for sample in pool_items {
        let members = args::list(sample).ok_or_else(|| {
            args::bad(
                span,
                "al_rank_by_disagreement: each pool entry must be a list of member distributions",
            )
        })?;
        let member_rows: Vec<Vec<f64>> = members.iter().filter_map(|m| args::f64s(m)).collect();
        pool.push(member_rows);
    }
    match al::committee::rank_by_disagreement(&pool) {
        Ok(ranks) => Ok(Value::List(
            ranks.iter().map(|&i| Value::U64(i as u64)).collect(),
        )),
        Err(e) => Err(args::bad(span, format!("al_rank_by_disagreement: {e:?}"))),
    }
}
