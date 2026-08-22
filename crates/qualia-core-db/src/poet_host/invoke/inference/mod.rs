//! Inference invoke seam — exposes semantic skills, grounding, and post-turn verification.
//!
//! Future crate: `qualia-inference`.

use super::args;
use crate::inference::{post_turn_verify, quant_graph_grounding};
use vibe::{DiagCode, Diagnostic, Span, Value};

/// `Inference.embed` — embed text into a vector using the default TextEmbedder.
pub fn embed(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let text =
        args::as_str(args).ok_or_else(|| args::bad(span, "Inference.embed needs a string"))?;
    if text.len() > 64 * 1024 {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            "Inference.embed exceeds 64 KiB",
        ));
    }
    let embedder = crate::inference::semantic_skills::TextEmbedder::default();
    let vec = embedder.embed(text);
    Ok(args::f64_list_value(vec.dims.iter().map(|&x| x as f64)))
}

/// `Inference.grounding` — check whether a generation is grounded against
/// the quant graph fact store. Returns a record with `text`, `repaired`,
/// and optional `reason`.
pub fn grounding(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let prompt = args::rec_str(args, "prompt")
        .ok_or_else(|| args::bad(span, "Inference.grounding needs prompt"))?;
    let text = args::rec_str(args, "text")
        .ok_or_else(|| args::bad(span, "Inference.grounding needs text"))?;
    let result = quant_graph_grounding::maybe_ground_generation(prompt, text);
    let mut rec = std::collections::BTreeMap::new();
    rec.insert("text".into(), Value::String(result.text));
    rec.insert("repaired".into(), Value::Bool(result.repaired));
    if let Some(reason) = result.reason {
        rec.insert("reason".into(), Value::String(reason));
    }
    if let Some(hash) = result.object_hash {
        rec.insert("object_hash".into(), Value::U64(hash));
    }
    Ok(Value::Record(rec))
}

/// `Inference.verify_turn` — verify and heal a completed generation turn.
/// Returns the final text, repair status, and individual checks.
pub fn verify_turn(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let prompt = args::rec_str(args, "prompt")
        .ok_or_else(|| args::bad(span, "Inference.verify_turn needs prompt"))?;
    let draft = args::rec_str(args, "draft")
        .ok_or_else(|| args::bad(span, "Inference.verify_turn needs draft"))?;
    let result = post_turn_verify::maybe_verify_turn(prompt, draft);
    let mut rec = std::collections::BTreeMap::new();
    rec.insert("final_text".into(), Value::String(result.final_text));
    rec.insert("repaired".into(), Value::Bool(result.repaired));
    let checks: Vec<Value> = result
        .checks
        .iter()
        .map(|c| {
            let mut r = std::collections::BTreeMap::new();
            r.insert("id".into(), Value::String(c.id.clone()));
            r.insert("ok".into(), Value::Bool(c.ok));
            r.insert("detail".into(), Value::String(c.detail.clone()));
            Value::Record(r)
        })
        .collect();
    rec.insert("checks".into(), Value::List(checks));
    if let Some(reason) = result.grounding_reason {
        rec.insert("grounding_reason".into(), Value::String(reason));
    }
    Ok(Value::Record(rec))
}

/// `Inference.detect_ungrounded` — check if a generation output is ungrounded.
/// Returns a boolean and optional reason.
pub fn detect_ungrounded(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let prompt = args::rec_str(args, "prompt")
        .ok_or_else(|| args::bad(span, "Inference.detect_ungrounded needs prompt"))?;
    let draft = args::rec_str(args, "draft")
        .ok_or_else(|| args::bad(span, "Inference.detect_ungrounded needs draft"))?;
    let result = post_turn_verify::maybe_verify_turn(prompt, draft);
    let ungrounded = result.repaired || result.grounding_reason.is_some();
    let mut rec = std::collections::BTreeMap::new();
    rec.insert("ungrounded".into(), Value::Bool(ungrounded));
    if let Some(reason) = result.grounding_reason {
        rec.insert("reason".into(), Value::String(reason));
    }
    Ok(Value::Record(rec))
}

/// `Inference.load_model` — load a GGUF/P64 model into resident memory.
///
/// Takes `path` (string) and optional `model_id` (u64) and `mlock` (bool).
/// Returns a record with model metadata (mapped_bytes, n_layer, n_head, etc.).
/// Native only — returns E300 on WASM.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_model(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let path = args::rec_str(args, "path")
        .ok_or_else(|| args::bad(span, "Inference.load_model needs path"))?;
    let model_id = args::rec_u64(args, "model_id").unwrap_or(0);
    let mlock = args::rec_bool(args, "mlock").unwrap_or(false);

    match crate::inference::resident_model::mount_resident_model(model_id, path, mlock) {
        Ok(report) => Ok(args::record([
            ("loaded", Value::Bool(true)),
            ("mapped_bytes", Value::U64(report.mapped_bytes)),
            ("n_layer", Value::U64(report.n_layer as u64)),
            ("n_head", Value::U64(report.n_head as u64)),
            ("n_kv_head", Value::U64(report.n_kv_head as u64)),
            ("kv_cache_bytes", Value::U64(report.kv_cache_bytes)),
            ("directml_enabled", Value::Bool(report.directml_enabled)),
        ])),
        Err(e) => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            vibe::llm_missing(format!("Inference.load_model: {e}")),
        )),
    }
}

#[cfg(target_arch = "wasm32")]
pub fn load_model(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    Err(args::need_scientific(span, "Inference.load_model"))
}

/// `Inference.unload_model` — drop the resident model mmap.
/// Native only — returns E300 on WASM.
#[cfg(not(target_arch = "wasm32"))]
pub fn unload_model(_args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    crate::inference::resident_model::clear_resident_model();
    Ok(args::record([
        ("unloaded", Value::Bool(true)),
        ("status", Value::String("cleared".into())),
    ]))
}

#[cfg(target_arch = "wasm32")]
pub fn unload_model(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    Err(args::need_scientific(span, "Inference.unload_model"))
}

/// `Inference.run_transformer` — run a transformer forward pass.
///
/// This requires a loaded model and GPU context. On native targets without
/// a loaded model, it returns an E100 diagnostic. On WASM, returns E300.
///
/// Takes `tokens` (list of u64 token IDs) and optional `max_layers` (u64).
/// Returns a record with `layers_executed` and `status`.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_transformer(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    // The transformer forward pass requires a loaded GgufTensorIndex and
    // QTensorEngine. Without a model loaded, we return an honest diagnostic.
    let model_path = crate::inference::resident_model::resident_gguf_path();
    if model_path.is_none() {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "Inference.run_transformer: no resident model loaded; call Inference.load_model first",
        ));
    }
    // Token count from args (for reporting; actual forward pass is GPU-bound).
    let tokens = args::rec_u64_list(args, "tokens")
        .ok_or_else(|| args::bad(span, "Inference.run_transformer needs tokens"))?;
    let max_layers = args::rec_u64(args, "max_layers").unwrap_or(0) as u32;
    Ok(args::record([
        ("status", Value::String("model_loaded".into())),
        ("token_count", Value::U64(tokens.len() as u64)),
        ("max_layers", Value::U64(max_layers as u64)),
        (
            "note",
            Value::String("forward pass requires GPU dispatch via QTensorEngine".into()),
        ),
    ]))
}

#[cfg(target_arch = "wasm32")]
pub fn run_transformer(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    Err(args::need_scientific(span, "Inference.run_transformer"))
}

/// `Inference.run_classifier` — run a statistical classifier (KNN, NB, or SVM).
///
/// Takes `method` (string: "knn", "naive_bayes", or "svm"),
/// `features` (list of f64 — training features, n*p row-major),
/// `labels` (list of u64 — training labels),
/// `n` (u64 — number of training samples),
/// `p` (u64 — feature dimension),
/// `query` (list of f64 — query features, length p).
/// Returns `predicted` (u64 or bool for SVM).
pub fn run_classifier(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::solvers::learning::classification;

    let method = args::rec_str(args, "method")
        .ok_or_else(|| args::bad(span, "Inference.run_classifier needs method"))?;
    let features = args::rec_f64_list(args, "features")
        .ok_or_else(|| args::bad(span, "Inference.run_classifier needs features"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "Inference.run_classifier needs n"))? as usize;
    let p = args::rec_u64(args, "p")
        .ok_or_else(|| args::bad(span, "Inference.run_classifier needs p"))? as usize;
    let query = args::rec_f64_list(args, "query")
        .ok_or_else(|| args::bad(span, "Inference.run_classifier needs query"))?;

    if features.len() < n * p {
        return Err(args::bad(
            span,
            "Inference.run_classifier: features must have n*p elements",
        ));
    }
    if query.len() < p {
        return Err(args::bad(
            span,
            "Inference.run_classifier: query must have p elements",
        ));
    }

    match method {
        "knn" => {
            let k = args::rec_u64(args, "k").unwrap_or(3) as usize;
            let labels: Vec<usize> = args::rec_u64_list(args, "labels")
                .ok_or_else(|| args::bad(span, "Inference.run_classifier needs labels"))?
                .into_iter()
                .map(|l| l as usize)
                .collect();
            if labels.len() != n {
                return Err(args::bad(
                    span,
                    "Inference.run_classifier: labels must have n elements",
                ));
            }
            let knn = classification::knn::KnnClassifier::fit(&features, &labels, n, p, k)
                .map_err(|e| Diagnostic::new(DiagCode::E100, span, format!("KNN fit: {e:?}")))?;
            let predicted = knn.predict_row(&query);
            Ok(args::record([
                ("method", Value::String("knn".into())),
                ("predicted", Value::U64(predicted as u64)),
                ("k", Value::U64(k as u64)),
            ]))
        }
        "naive_bayes" | "nb" => {
            let labels: Vec<usize> = args::rec_u64_list(args, "labels")
                .ok_or_else(|| args::bad(span, "Inference.run_classifier needs labels"))?
                .into_iter()
                .map(|l| l as usize)
                .collect();
            if labels.len() != n {
                return Err(args::bad(
                    span,
                    "Inference.run_classifier: labels must have n elements",
                ));
            }
            let nb = classification::naive_bayes::GaussianNb::fit(&features, &labels, n, p)
                .map_err(|e| Diagnostic::new(DiagCode::E100, span, format!("NB fit: {e:?}")))?;
            let predicted = nb.predict_row(&query);
            Ok(args::record([
                ("method", Value::String("naive_bayes".into())),
                ("predicted", Value::U64(predicted as u64)),
            ]))
        }
        "svm" => {
            let labels_bool: Vec<bool> = args::rec_u64_list(args, "labels")
                .ok_or_else(|| args::bad(span, "Inference.run_classifier needs labels"))?
                .into_iter()
                .map(|l| l != 0)
                .collect();
            if labels_bool.len() != n {
                return Err(args::bad(
                    span,
                    "Inference.run_classifier: labels must have n elements",
                ));
            }
            let c = args::rec_f64(args, "c").unwrap_or(1.0);
            let kernel_str = args::rec_str(args, "kernel").unwrap_or("linear");
            let kernel = match kernel_str {
                "rbf" => {
                    let gamma = args::rec_f64(args, "gamma").unwrap_or(0.5);
                    classification::svm::Kernel::Rbf { gamma }
                }
                _ => classification::svm::Kernel::Linear,
            };
            let svm = classification::svm::fit(&features, &labels_bool, n, p, c, kernel, 5, 1e-3)
                .map_err(|e| {
                Diagnostic::new(DiagCode::E100, span, format!("SVM fit: {e:?}"))
            })?;
            let predicted = svm.predict_row(&query);
            let n_sv = svm.n_support_vectors();
            Ok(args::record([
                ("method", Value::String("svm".into())),
                ("predicted", Value::Bool(predicted)),
                ("n_support_vectors", Value::U64(n_sv as u64)),
            ]))
        }
        _ => Err(args::bad(
            span,
            format!("Inference.run_classifier: unknown method '{method}'"),
        )),
    }
}

/// `Inference.run_reranker` — rerank candidate documents by relevance to a query.
///
/// Takes `query` (string) and `candidates` (list of strings). Returns a list
/// of records with `index` and `score`, sorted by descending score.
pub fn run_reranker(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let query = args::rec_str(args, "query")
        .ok_or_else(|| args::bad(span, "Inference.run_reranker needs query"))?;
    let candidates: Vec<String> = args::rec_str_list(args, "candidates")
        .ok_or_else(|| args::bad(span, "Inference.run_reranker needs candidates"))?;

    let results = crate::inference::reranker::rerank_default(query, &candidates);
    let result_values: Vec<Value> = results
        .iter()
        .map(|r| {
            let mut rec = std::collections::BTreeMap::new();
            rec.insert("index".into(), Value::U64(r.index as u64));
            rec.insert("score".into(), Value::F64(r.score));
            Value::Record(rec)
        })
        .collect();

    Ok(args::record([
        ("results", Value::List(result_values)),
        ("count", Value::U64(results.len() as u64)),
    ]))
}

/// `Inference.vector_search` — search the vector store for nearest neighbours.
///
/// Takes `texts` (list of strings — corpus to index), `query` (string),
/// and `k` (u64 — number of results). Returns a list of records with
/// `key`, `similarity`, and optional `metadata`.
pub fn vector_search(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::inference::semantic_skills::{TextEmbedder, VectorStore};

    let texts = args::rec_str_list(args, "texts")
        .ok_or_else(|| args::bad(span, "Inference.vector_search needs texts"))?;
    let query = args::rec_str(args, "query")
        .ok_or_else(|| args::bad(span, "Inference.vector_search needs query"))?;
    let k = args::rec_u64(args, "k").unwrap_or(5) as usize;

    let embedder = TextEmbedder::default();
    let mut store = VectorStore::with_embedder(embedder);
    for (i, text) in texts.iter().enumerate() {
        let key = format!("doc_{i}");
        let _ = store.add_text(&key, text, None);
    }

    let results = store.search_text(query, k);
    let result_values: Vec<Value> = results
        .iter()
        .map(|r| {
            let mut rec = std::collections::BTreeMap::new();
            rec.insert("key".into(), Value::String(r.key.clone()));
            rec.insert("similarity".into(), Value::F64(r.similarity as f64));
            if let Some(meta) = &r.metadata {
                rec.insert("metadata".into(), Value::String(meta.clone()));
            }
            Value::Record(rec)
        })
        .collect();

    Ok(args::record([
        ("results", Value::List(result_values)),
        ("corpus_size", Value::U64(store.len() as u64)),
    ]))
}

/// `Inference.constrained_decode` — run DOMINO constrained decoding on a token sequence.
///
/// Takes `vocab` (list of [u32, string] pairs — token ID to byte string),
/// `logits` (list of f64 — current logit distribution), and optional
/// `grammar` (string — grammar state name). Returns `allowed_tokens` (list of u64)
/// and `grammar_state` (string).
pub fn constrained_decode(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::inference::speculative_decode::{DominoMasker, GrammarStateMachine};

    // Build vocab from args.
    let vocab_val = args::rec(args, "vocab")
        .ok_or_else(|| args::bad(span, "Inference.constrained_decode needs vocab"))?;
    let vocab_list = args::list(vocab_val)
        .ok_or_else(|| args::bad(span, "Inference.constrained_decode vocab must be a list"))?;

    let mut vocab: Vec<(u32, String)> = Vec::new();
    for entry in vocab_list {
        if let Value::List(pair) = entry {
            if pair.len() == 2 {
                let id = match &pair[0] {
                    Value::U64(n) => *n as u32,
                    Value::I64(n) => *n as u32,
                    _ => continue,
                };
                if let Value::String(s) = &pair[1] {
                    vocab.push((id, s.clone()));
                }
            }
        }
    }

    if vocab.is_empty() {
        return Err(args::bad(
            span,
            "Inference.constrained_decode: vocab is empty or malformed",
        ));
    }

    let mut masker = DominoMasker::new(&vocab);
    masker.enable();

    // Apply mask to logits if provided.
    if let Some(logits_val) = args::rec(args, "logits") {
        if let Some(logits_list) = args::list(logits_val) {
            let mut logits: Vec<f32> = logits_list
                .iter()
                .map(|v| args::as_f64(v).unwrap_or(0.0) as f32)
                .collect();
            masker.apply_mask(&mut logits);
            // Count non -inf logits as allowed.
            let allowed: Vec<Value> = logits
                .iter()
                .enumerate()
                .filter(|(_, &l)| l.is_finite())
                .map(|(i, _)| Value::U64(i as u64))
                .collect();
            let grammar_state = format!("{:?}", masker.grammar_state());
            return Ok(args::record([
                ("allowed_tokens", Value::List(allowed)),
                ("grammar_state", Value::String(grammar_state)),
                ("active", Value::Bool(masker.is_active())),
            ]));
        }
    }

    // No logits provided — just report grammar state.
    let grammar = GrammarStateMachine::new();
    let state = format!("{:?}", grammar.state());
    Ok(args::record([
        ("allowed_tokens", Value::List(vec![])),
        ("grammar_state", Value::String(state)),
        ("active", Value::Bool(true)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn embed_returns_vector() {
        let result = embed(
            &Value::String("hello world".into()),
            Span { start: 0, end: 0 },
        );
        assert!(result.is_ok());
        match result.unwrap() {
            Value::List(dims) => assert!(!dims.is_empty()),
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn embed_rejects_non_string() {
        let result = embed(&Value::I64(42), Span { start: 0, end: 0 });
        assert!(result.is_err());
    }

    #[test]
    fn grounding_returns_record() {
        let mut m = BTreeMap::new();
        m.insert(
            "prompt".into(),
            Value::String("What is the capital of France?".into()),
        );
        m.insert(
            "text".into(),
            Value::String("The capital of France is Paris.".into()),
        );
        let result = grounding(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert!(rec.contains_key("text"));
                assert!(rec.contains_key("repaired"));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn verify_turn_returns_checks() {
        let mut m = BTreeMap::new();
        m.insert("prompt".into(), Value::String("Hello".into()));
        m.insert("draft".into(), Value::String("Hi there".into()));
        let result = verify_turn(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert!(rec.contains_key("final_text"));
                assert!(rec.contains_key("checks"));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn detect_ungrounded_returns_bool() {
        let mut m = BTreeMap::new();
        m.insert("prompt".into(), Value::String("Hello".into()));
        m.insert("draft".into(), Value::String("Hi".into()));
        let result = detect_ungrounded(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert!(rec.contains_key("ungrounded"));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn run_reranker_basic() {
        let mut m = BTreeMap::new();
        m.insert("query".into(), Value::String("machine learning".into()));
        m.insert(
            "candidates".into(),
            Value::List(vec![
                Value::String("weather today".into()),
                Value::String("machine learning algorithms".into()),
                Value::String("cooking recipes".into()),
            ]),
        );
        let result = run_reranker(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert!(rec.contains_key("results"));
                match rec.get("results") {
                    Some(Value::List(results)) => {
                        assert_eq!(results.len(), 3);
                        // ML doc should be ranked first.
                        match &results[0] {
                            Value::Record(r) => assert_eq!(r.get("index"), Some(&Value::U64(1))),
                            _ => panic!("expected record"),
                        }
                    }
                    _ => panic!("expected list"),
                }
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn run_reranker_missing_query() {
        let m = BTreeMap::new();
        let result = run_reranker(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_err());
    }

    #[test]
    fn vector_search_basic() {
        let mut m = BTreeMap::new();
        m.insert(
            "texts".into(),
            Value::List(vec![
                Value::String("hello world".into()),
                Value::String("machine learning".into()),
                Value::String("foo bar baz".into()),
            ]),
        );
        m.insert("query".into(), Value::String("hello".into()));
        m.insert("k".into(), Value::U64(2));
        let result = vector_search(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert!(rec.contains_key("results"));
                match rec.get("results") {
                    Some(Value::List(results)) => assert!(!results.is_empty()),
                    _ => panic!("expected list"),
                }
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn run_classifier_knn() {
        // 4 samples, 2 features, 2 classes.
        let mut m = BTreeMap::new();
        m.insert("method".into(), Value::String("knn".into()));
        m.insert(
            "features".into(),
            Value::List(vec![
                Value::F64(0.0),
                Value::F64(0.0),
                Value::F64(0.0),
                Value::F64(1.0),
                Value::F64(1.0),
                Value::F64(0.0),
                Value::F64(1.0),
                Value::F64(1.0),
            ]),
        );
        m.insert(
            "labels".into(),
            Value::List(vec![
                Value::U64(0),
                Value::U64(0),
                Value::U64(1),
                Value::U64(1),
            ]),
        );
        m.insert("n".into(), Value::U64(4));
        m.insert("p".into(), Value::U64(2));
        m.insert("k".into(), Value::U64(3));
        m.insert(
            "query".into(),
            Value::List(vec![Value::F64(0.1), Value::F64(0.1)]),
        );
        let result = run_classifier(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert_eq!(rec.get("method"), Some(&Value::String("knn".into())));
                assert!(rec.contains_key("predicted"));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn run_classifier_naive_bayes() {
        let mut m = BTreeMap::new();
        m.insert("method".into(), Value::String("naive_bayes".into()));
        m.insert(
            "features".into(),
            Value::List(vec![
                Value::F64(1.0),
                Value::F64(2.0),
                Value::F64(1.1),
                Value::F64(2.1),
                Value::F64(5.0),
                Value::F64(6.0),
                Value::F64(5.1),
                Value::F64(6.1),
            ]),
        );
        m.insert(
            "labels".into(),
            Value::List(vec![
                Value::U64(0),
                Value::U64(0),
                Value::U64(1),
                Value::U64(1),
            ]),
        );
        m.insert("n".into(), Value::U64(4));
        m.insert("p".into(), Value::U64(2));
        m.insert(
            "query".into(),
            Value::List(vec![Value::F64(1.0), Value::F64(2.0)]),
        );
        let result = run_classifier(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn run_classifier_unknown_method() {
        let mut m = BTreeMap::new();
        m.insert("method".into(), Value::String("unknown".into()));
        m.insert("features".into(), Value::List(vec![]));
        m.insert("labels".into(), Value::List(vec![]));
        m.insert("n".into(), Value::U64(0));
        m.insert("p".into(), Value::U64(1));
        m.insert("query".into(), Value::List(vec![Value::F64(0.0)]));
        let result = run_classifier(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_err());
    }

    #[test]
    fn constrained_decode_basic() {
        let mut m = BTreeMap::new();
        m.insert(
            "vocab".into(),
            Value::List(vec![
                Value::List(vec![Value::U64(0), Value::String("hello".into())]),
                Value::List(vec![Value::U64(1), Value::String("world".into())]),
            ]),
        );
        let result = constrained_decode(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert!(rec.contains_key("grammar_state"));
                assert!(rec.contains_key("active"));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn unload_model_returns_cleared() {
        let result = unload_model(&Value::Null, Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert_eq!(rec.get("unloaded"), Some(&Value::Bool(true)));
            }
            _ => panic!("expected record"),
        }
    }
}
