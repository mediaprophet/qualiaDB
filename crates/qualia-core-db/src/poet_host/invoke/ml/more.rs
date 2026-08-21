//! Additional ML invoke seams — KG embedding scores, ranking metrics,
//! polynomial regression, bootstrap CI, permutation tests, power analysis,
//! LOOCV, and multiple-testing rejection count.

use super::super::args;
use crate::solvers::learning as ml;
use poet_vibe::{Diagnostic, Span, Value};

// ── KG embedding scores ─────────────────────────────────────────────

/// `MachineLearning.complex_score` — ComplEx score.
/// Args: { h: [f64], r: [f64], t: [f64], k: u64 }
pub fn complex_score(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let h =
        args::rec_f64_list(args, "h").ok_or_else(|| args::bad(span, "complex_score needs h"))?;
    let r =
        args::rec_f64_list(args, "r").ok_or_else(|| args::bad(span, "complex_score needs r"))?;
    let t =
        args::rec_f64_list(args, "t").ok_or_else(|| args::bad(span, "complex_score needs t"))?;
    let k =
        args::rec_u64(args, "k").ok_or_else(|| args::bad(span, "complex_score needs k"))? as usize;
    Ok(Value::F64(ml::kg_embedding::score::complex_score(
        &h, &r, &t, k,
    )))
}

/// `MachineLearning.rotate_score` — RotatE score.
/// Args: { h: [f64], r: [f64], t: [f64], k: u64 }
pub fn rotate_score(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let h = args::rec_f64_list(args, "h").ok_or_else(|| args::bad(span, "rotate_score needs h"))?;
    let r = args::rec_f64_list(args, "r").ok_or_else(|| args::bad(span, "rotate_score needs r"))?;
    let t = args::rec_f64_list(args, "t").ok_or_else(|| args::bad(span, "rotate_score needs t"))?;
    let k =
        args::rec_u64(args, "k").ok_or_else(|| args::bad(span, "rotate_score needs k"))? as usize;
    Ok(Value::F64(ml::kg_embedding::score::rotate_score(
        &h, &r, &t, k,
    )))
}

// ── KG ranking metrics ──────────────────────────────────────────────

fn parse_embedding_table(v: &Value) -> Option<ml::kg_embedding::EmbeddingTable> {
    let model_str = args::rec_str(v, "model")?;
    let p = args::rec_u64(v, "p").unwrap_or(2) as u8;
    let model = match model_str {
        "TransE" => ml::kg_embedding::ScoreModel::TransE { p },
        "DistMult" => ml::kg_embedding::ScoreModel::DistMult,
        "ComplEx" => ml::kg_embedding::ScoreModel::ComplEx,
        "RotatE" => ml::kg_embedding::ScoreModel::RotatE,
        _ => return None,
    };
    let rank = args::rec_u64(v, "rank")? as usize;
    let n_entities = args::rec_u64(v, "n_entities")? as usize;
    let n_relations = args::rec_u64(v, "n_relations")? as usize;
    let entities = args::rec_f64_list(v, "entities")?;
    let relations = args::rec_f64_list(v, "relations")?;
    let (ent_dim, rel_dim) = ml::kg_embedding::ScoreModel::dims(model, rank);
    Some(ml::kg_embedding::EmbeddingTable {
        model,
        rank,
        ent_dim,
        rel_dim,
        n_entities,
        n_relations,
        entities,
        relations,
    })
}

fn parse_triples(v: &Value) -> Option<Vec<(usize, usize, usize)>> {
    let list = args::rec(v, "triples")?;
    let items = args::list(&list)?;
    let mut out = Vec::new();
    for item in items {
        let vals = args::f64s(item)?;
        if vals.len() >= 3 {
            out.push((vals[0] as usize, vals[1] as usize, vals[2] as usize));
        }
    }
    Some(out)
}

/// `MachineLearning.kg_mean_rank` — mean rank of test triples.
/// Args: { table: {...}, triples: [[u64; 3]], candidates: [u64] }
pub fn kg_mean_rank(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let table_val =
        args::rec(args, "table").ok_or_else(|| args::bad(span, "kg_mean_rank needs table"))?;
    let table = parse_embedding_table(table_val)
        .ok_or_else(|| args::bad(span, "kg_mean_rank: invalid embedding table"))?;
    let triples =
        parse_triples(args).ok_or_else(|| args::bad(span, "kg_mean_rank needs triples"))?;
    let candidates_u64 = args::rec_u64_list(args, "candidates").unwrap_or_default();
    let candidates: Vec<usize> = candidates_u64.iter().map(|&c| c as usize).collect();
    match ml::kg_embedding::predict::mean_rank(&table, &triples, &candidates) {
        Ok(mr) => Ok(Value::F64(mr)),
        Err(e) => Err(args::bad(span, format!("kg_mean_rank: {e:?}"))),
    }
}

/// `MachineLearning.kg_mean_reciprocal_rank` — MRR of test triples.
pub fn kg_mean_reciprocal_rank(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let table_val = args::rec(args, "table")
        .ok_or_else(|| args::bad(span, "kg_mean_reciprocal_rank needs table"))?;
    let table = parse_embedding_table(table_val)
        .ok_or_else(|| args::bad(span, "kg_mean_reciprocal_rank: invalid embedding table"))?;
    let triples = parse_triples(args)
        .ok_or_else(|| args::bad(span, "kg_mean_reciprocal_rank needs triples"))?;
    let candidates_u64 = args::rec_u64_list(args, "candidates").unwrap_or_default();
    let candidates: Vec<usize> = candidates_u64.iter().map(|&c| c as usize).collect();
    match ml::kg_embedding::predict::mean_reciprocal_rank(&table, &triples, &candidates) {
        Ok(mrr) => Ok(Value::F64(mrr)),
        Err(e) => Err(args::bad(span, format!("kg_mean_reciprocal_rank: {e:?}"))),
    }
}

/// `MachineLearning.kg_hits_at_k` — Hits@k of test triples.
/// Args: { table: {...}, triples: [[u64; 3]], candidates: [u64], k: u64 }
pub fn kg_hits_at_k(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let table_val =
        args::rec(args, "table").ok_or_else(|| args::bad(span, "kg_hits_at_k needs table"))?;
    let table = parse_embedding_table(table_val)
        .ok_or_else(|| args::bad(span, "kg_hits_at_k: invalid embedding table"))?;
    let triples =
        parse_triples(args).ok_or_else(|| args::bad(span, "kg_hits_at_k needs triples"))?;
    let candidates_u64 = args::rec_u64_list(args, "candidates").unwrap_or_default();
    let candidates: Vec<usize> = candidates_u64.iter().map(|&c| c as usize).collect();
    let k =
        args::rec_u64(args, "k").ok_or_else(|| args::bad(span, "kg_hits_at_k needs k"))? as usize;
    match ml::kg_embedding::predict::hits_at_k(&table, &triples, &candidates, k) {
        Ok(h) => Ok(Value::F64(h)),
        Err(e) => Err(args::bad(span, format!("kg_hits_at_k: {e:?}"))),
    }
}

// ── Polynomial regression ───────────────────────────────────────────

/// `MachineLearning.polynomial_regression` — degree-d polynomial fit.
/// Args: { x: [f64], y: [f64], n: u64, degree: u64 }
pub fn polynomial_regression(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64_list(args, "x")
        .ok_or_else(|| args::bad(span, "polynomial_regression needs x"))?;
    let y = args::rec_f64_list(args, "y")
        .ok_or_else(|| args::bad(span, "polynomial_regression needs y"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "polynomial_regression needs n"))? as usize;
    let degree = args::rec_u64(args, "degree")
        .ok_or_else(|| args::bad(span, "polynomial_regression needs degree"))?
        as usize;
    match ml::splines::polynomial_regression(&x, &y, n, degree) {
        Ok(spline) => Ok(args::record([
            ("degree", Value::U64(spline.degree as u64)),
            ("knots", args::f64_list_value(spline.knots.clone())),
            (
                "coefficients",
                args::f64_list_value(spline.coefficients.clone()),
            ),
        ])),
        Err(e) => Err(args::bad(span, format!("polynomial_regression: {e:?}"))),
    }
}

// ── Bootstrap CI ────────────────────────────────────────────────────

fn statistic_fn(name: &str) -> Box<dyn Fn(&[f64]) -> f64> {
    match name {
        "median" => Box::new(|d: &[f64]| {
            let mut s = d.to_vec();
            s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            crate::solvers::statistics::descriptive::median_sorted(&s).unwrap_or(0.0)
        }),
        "variance" => Box::new(|d: &[f64]| {
            crate::solvers::statistics::descriptive::variance(d, true).unwrap_or(0.0)
        }),
        _ => Box::new(|d: &[f64]| crate::solvers::statistics::descriptive::mean(d).unwrap_or(0.0)),
    }
}

/// `MachineLearning.bootstrap_estimate` — bootstrap standard error and bias.
/// Args: { data: [f64], b: u64, seed: u64, statistic: "mean"|"median"|"variance" }
pub fn bootstrap_estimate(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let data = args::rec_f64_list(args, "data")
        .ok_or_else(|| args::bad(span, "bootstrap_estimate needs data"))?;
    let b = args::rec_u64(args, "b").ok_or_else(|| args::bad(span, "bootstrap_estimate needs b"))?
        as usize;
    let seed = args::rec_u64(args, "seed").unwrap_or(0);
    let stat_name = args::rec_str(args, "statistic").unwrap_or("mean");
    let stat = statistic_fn(stat_name);
    match ml::resampling::bootstrap::bootstrap_estimate(&data, b, seed, |d| stat(d)) {
        Some(r) => Ok(args::record([
            ("estimate", Value::F64(r.estimate)),
            ("std_error", Value::F64(r.std_error)),
            ("bias", Value::F64(r.bias)),
        ])),
        None => Err(args::bad(span, "bootstrap_estimate: invalid input")),
    }
}

/// `MachineLearning.bootstrap_ci` — bootstrap confidence interval.
/// Args: { data: [f64], b: u64, alpha: f64, seed: u64, method: "percentile"|"bca", statistic: "mean"|"median"|"variance" }
pub fn bootstrap_ci(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let data = args::rec_f64_list(args, "data")
        .ok_or_else(|| args::bad(span, "bootstrap_ci needs data"))?;
    let b =
        args::rec_u64(args, "b").ok_or_else(|| args::bad(span, "bootstrap_ci needs b"))? as usize;
    let alpha = args::rec_f64(args, "alpha").unwrap_or(0.05);
    let seed = args::rec_u64(args, "seed").unwrap_or(0);
    let method_str = args::rec_str(args, "method").unwrap_or("percentile");
    let method = match method_str {
        "bca" | "Bca" | "BCA" => ml::resampling::bootstrap::CiMethod::Bca,
        _ => ml::resampling::bootstrap::CiMethod::Percentile,
    };
    let stat_name = args::rec_str(args, "statistic").unwrap_or("mean");
    let stat = statistic_fn(stat_name);
    match ml::resampling::bootstrap::bootstrap_ci(&data, b, alpha, seed, method, |d| stat(d)) {
        Some(ci) => Ok(args::record([
            ("lower", Value::F64(ci.lower)),
            ("upper", Value::F64(ci.upper)),
            ("confidence", Value::F64(ci.confidence)),
            ("method", Value::String(format!("{:?}", ci.method))),
        ])),
        None => Err(args::bad(span, "bootstrap_ci: invalid input")),
    }
}

// ── Permutation test ────────────────────────────────────────────────

/// `MachineLearning.permutation_test` — two-sample permutation test.
/// Args: { a: [f64], b: [f64], n_perm: u64, seed: u64, statistic: "mean"|"median"|"variance" }
pub fn permutation_test(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a =
        args::rec_f64_list(args, "a").ok_or_else(|| args::bad(span, "permutation_test needs a"))?;
    let b =
        args::rec_f64_list(args, "b").ok_or_else(|| args::bad(span, "permutation_test needs b"))?;
    let n_perm = args::rec_u64(args, "n_perm")
        .ok_or_else(|| args::bad(span, "permutation_test needs n_perm"))? as usize;
    let seed = args::rec_u64(args, "seed").unwrap_or(0);
    let stat_name = args::rec_str(args, "statistic").unwrap_or("mean");
    let stat = statistic_fn(stat_name);
    match ml::resampling::permutation::two_sample_test(&a, &b, n_perm, seed, |d| stat(d)) {
        Some(r) => Ok(args::record([
            ("observed", Value::F64(r.observed)),
            ("p_value", Value::F64(r.p_value)),
            ("n_permutations", Value::U64(r.n_permutations as u64)),
        ])),
        None => Err(args::bad(span, "permutation_test: invalid input")),
    }
}

// ── Power analysis ──────────────────────────────────────────────────

/// `MachineLearning.required_sample_size_two_proportion` — sample size for
/// two-proportion test.
/// Args: { p1: f64, p2: f64, alpha: f64, power: f64 }
pub fn required_sample_size_two_proportion(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p1 = args::rec_f64(args, "p1")
        .ok_or_else(|| args::bad(span, "required_sample_size_two_proportion needs p1"))?;
    let p2 = args::rec_f64(args, "p2")
        .ok_or_else(|| args::bad(span, "required_sample_size_two_proportion needs p2"))?;
    let alpha = args::rec_f64(args, "alpha").unwrap_or(0.05);
    let power = args::rec_f64(args, "power").unwrap_or(0.8);
    match ml::experiment::power::required_sample_size_two_proportion(p1, p2, alpha, power) {
        Some(n) => Ok(Value::U64(n as u64)),
        None => Err(args::bad(
            span,
            "required_sample_size_two_proportion: invalid inputs",
        )),
    }
}

// ── LOOCV ───────────────────────────────────────────────────────────

/// `MachineLearning.loocv` — leave-one-out cross-validation folds.
/// Args: { n: u64 }
pub fn loocv(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "loocv needs n"))? as usize;
    let folds = ml::resampling::folds::loocv(n);
    let fold_records: Vec<Value> = folds
        .iter()
        .map(|f| {
            args::record([
                (
                    "train",
                    Value::List(f.train.iter().map(|&i| Value::U64(i as u64)).collect()),
                ),
                (
                    "test",
                    Value::List(f.test.iter().map(|&i| Value::U64(i as u64)).collect()),
                ),
            ])
        })
        .collect();
    Ok(args::record([
        ("folds", Value::List(fold_records)),
        ("count", Value::U64(folds.len() as u64)),
    ]))
}

// ── Multiple testing ────────────────────────────────────────────────

/// `MachineLearning.n_rejected` — count rejections at level alpha.
/// Args: { adjusted: [f64], alpha: f64 }
pub fn n_rejected(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let adjusted = args::rec_f64_list(args, "adjusted")
        .ok_or_else(|| args::bad(span, "n_rejected needs adjusted"))?;
    let alpha = args::rec_f64(args, "alpha").unwrap_or(0.05);
    Ok(Value::U64(
        ml::multiple_testing::n_rejected(&adjusted, alpha) as u64,
    ))
}
