//! Dual-path Tool Chest actions for curated Host-bound `Inference.*` ids (wave 20).
//!
//! Includes wave-19 activation / RMS-norm Host binds plus earlier Host embed /
//! classifier / vector_search. No Host widen — scopes must already exist in
//! `poet_host/invoke/ids.rs`. Local sketches mirror Host CPU algebra.

use serde_json::json;
use web_sys::{Document, Element};

const EMBED_DIM: usize = 256;
const NGRAM: usize = 3;

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn selected_source(document: &Document) -> Option<String> {
    let container = selected_container(document)?;
    let text = container
        .query_selector(".vibe-editor, .vibe-editor-textarea, .doc-editor, .sheet-grid")
        .ok()
        .flatten()
        .and_then(|editor| editor.text_content())
        .or_else(|| container.text_content())?;
    let bounded: String = text.chars().take(16_384).collect();
    (!bounded.trim().is_empty()).then_some(bounded)
}

fn parse_numbers(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|' | '\n' | '\r'))
        .filter_map(|token| token.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(256)
        .collect()
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn string_attr(el: Option<&Element>, name: &str) -> Option<String> {
    el.and_then(|e| e.get_attribute(name))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn format_vec(v: &[f64], max: usize) -> String {
    let n = v.len().min(max);
    let body: Vec<String> = v[..n].iter().map(|x| format!("{x:.4}")).collect();
    if v.len() > max {
        format!("[{}, …] (n={})", body.join(", "), v.len())
    } else {
        format!("[{}]", body.join(", "))
    }
}

fn invoke_dual(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    local_message: String,
    args: serde_json::Value,
) {
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(cap_id, &local_message);
        super::interactions::show_tool_status(
            document,
            &label,
            &report.message,
            report.status_kind,
        );
        return;
    }
    super::interactions::show_tool_status(
        document,
        &label,
        &format!("Running {cap_id}…"),
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        match super::native_daemon::daemon_invoke(cap_id, args).await {
            Ok(response) if response.ok => {
                let report = super::tool_dual_path::live_ok(cap_id, &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    cap_id,
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("capability invoke failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied(cap_id, &error);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
        }
    });
}

fn resolve_x(document: &Document, fallback: &[f64]) -> Vec<f64> {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    if nums.is_empty() {
        fallback.to_vec()
    } else {
        nums
    }
}

// ── Local sketches (match Host `solvers::activation` / semantic embed) ──

fn local_relu(x: &mut [f64]) {
    for v in x.iter_mut() {
        if *v < 0.0 {
            *v = 0.0;
        }
    }
}

fn local_sigmoid(x: &mut [f64]) {
    for v in x.iter_mut() {
        *v = 1.0 / (1.0 + (-*v).exp());
    }
}

fn local_gelu(x: &mut [f64]) {
    const C: f64 = 0.797_884_560_802_865_4;
    for v in x.iter_mut() {
        let x3 = *v * *v * *v;
        *v = 0.5 * *v * (1.0 + (C * (*v + 0.044_715 * x3)).tanh());
    }
}

fn local_softmax(x: &mut [f64]) {
    if x.is_empty() {
        return;
    }
    let mut max = f64::NEG_INFINITY;
    for &v in x.iter() {
        if v > max {
            max = v;
        }
    }
    let mut sum = 0.0;
    for v in x.iter_mut() {
        *v = (*v - max).exp();
        sum += *v;
    }
    if sum > 0.0 {
        let inv = 1.0 / sum;
        for v in x.iter_mut() {
            *v *= inv;
        }
    }
}

fn local_rms_norm(x: &mut [f64], weight: &[f64], eps: f64) {
    let n = x.len().min(weight.len());
    if n == 0 {
        return;
    }
    let mut ss = 0.0;
    for i in 0..n {
        ss += x[i] * x[i];
    }
    let inv_rms = 1.0 / (ss / n as f64 + eps).sqrt();
    for i in 0..n {
        x[i] = x[i] * inv_rms * weight[i];
    }
}

fn fnv1a_hash(data: &[u8]) -> u32 {
    let mut hash: u32 = 0x811C_9DC5;
    for &b in data {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn local_embed(text: &str) -> Vec<f64> {
    let mut dims = vec![0.0f64; EMBED_DIM];
    let chars: Vec<char> = text.to_lowercase().chars().collect();
    if chars.len() < NGRAM {
        let h = fnv1a_hash(text.as_bytes());
        dims[(h as usize) % EMBED_DIM] += 1.0;
        return dims;
    }
    for i in 0..=chars.len() - NGRAM {
        let ngram: String = chars[i..i + NGRAM].iter().collect();
        let h = fnv1a_hash(ngram.as_bytes());
        dims[(h as usize) % EMBED_DIM] += 1.0;
    }
    let norm = dims.iter().map(|v| v * v).sum::<f64>().sqrt();
    if norm > 0.0 {
        for v in dims.iter_mut() {
            *v /= norm;
        }
    }
    dims
}

fn local_knn_predict(features: &[f64], labels: &[usize], n: usize, p: usize, query: &[f64], k: usize) -> usize {
    if n == 0 || p == 0 || query.len() < p || features.len() < n * p || labels.len() < n {
        return 0;
    }
    let k = k.max(1).min(n);
    let mut dists: Vec<(f64, usize)> = Vec::with_capacity(n);
    for i in 0..n {
        let row = &features[i * p..(i + 1) * p];
        let mut d = 0.0;
        for j in 0..p {
            let diff = row[j] - query[j];
            d += diff * diff;
        }
        dists.push((d, labels[i]));
    }
    dists.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut votes: Vec<(usize, usize)> = Vec::new();
    for &(_, lab) in dists.iter().take(k) {
        if let Some(entry) = votes.iter_mut().find(|(l, _)| *l == lab) {
            entry.1 += 1;
        } else {
            votes.push((lab, 1));
        }
    }
    votes
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .map(|(l, _)| l)
        .unwrap_or(0)
}

fn cosine(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    if n == 0 {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut na = 0.0;
    let mut nb = 0.0;
    for i in 0..n {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let den = na.sqrt() * nb.sqrt();
    if den == 0.0 {
        0.0
    } else {
        dot / den
    }
}

fn local_vector_search(texts: &[String], query: &str, k: usize) -> Vec<(String, f64)> {
    let q = local_embed(query);
    let mut scored: Vec<(String, f64)> = texts
        .iter()
        .enumerate()
        .map(|(i, t)| (format!("doc_{i}"), cosine(&local_embed(t), &q)))
        .collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    scored.truncate(k.max(1));
    scored
}

/// `Inference.relu` — `{ x: [f64] }` → `{ out: [f64] }`.
pub(super) fn run_relu(document: &Document, label: &str) {
    let mut x = resolve_x(document, &[-1.0, 0.0, 2.0]);
    let args_x = x.clone();
    local_relu(&mut x);
    invoke_dual(
        document,
        label,
        "Inference.relu",
        format!("relu sketch → {}", format_vec(&x, 8)),
        json!({ "x": args_x }),
    );
}

/// `Inference.sigmoid` — `{ x: [f64] }` → `{ out: [f64] }`.
pub(super) fn run_sigmoid(document: &Document, label: &str) {
    let mut x = resolve_x(document, &[0.0, 1.0, -1.0]);
    let args_x = x.clone();
    local_sigmoid(&mut x);
    invoke_dual(
        document,
        label,
        "Inference.sigmoid",
        format!("sigmoid sketch → {}", format_vec(&x, 8)),
        json!({ "x": args_x }),
    );
}

/// `Inference.gelu` — `{ x: [f64] }` → `{ out: [f64] }`.
pub(super) fn run_gelu(document: &Document, label: &str) {
    let mut x = resolve_x(document, &[0.0, 1.0, -1.0]);
    let args_x = x.clone();
    local_gelu(&mut x);
    invoke_dual(
        document,
        label,
        "Inference.gelu",
        format!("gelu sketch → {}", format_vec(&x, 8)),
        json!({ "x": args_x }),
    );
}

/// `Inference.softmax` — `{ x: [f64] }` → `{ out: [f64] }`.
pub(super) fn run_softmax(document: &Document, label: &str) {
    let mut x = resolve_x(document, &[1.0, 2.0, 3.0]);
    let args_x = x.clone();
    local_softmax(&mut x);
    invoke_dual(
        document,
        label,
        "Inference.softmax",
        format!("softmax sketch → {}", format_vec(&x, 8)),
        json!({ "x": args_x }),
    );
}

/// `Inference.rms_norm` — `{ x, weight, eps? }` → `{ out: [f64] }`.
pub(super) fn run_rms_norm(document: &Document, label: &str) {
    let container = selected_container(document);
    let mut x = resolve_x(document, &[3.0, -4.0]);
    let weight = {
        let from_attr = string_attr(container.as_ref(), "data-weight")
            .map(|s| parse_numbers(&s))
            .filter(|v| !v.is_empty());
        from_attr.unwrap_or_else(|| vec![1.0; x.len()])
    };
    let weight = if weight.len() == x.len() {
        weight
    } else if weight.len() > x.len() {
        weight[..x.len()].to_vec()
    } else {
        let mut w = weight;
        w.resize(x.len(), 1.0);
        w
    };
    let eps = numeric_attr(container.as_ref(), "data-eps").unwrap_or(1e-6);
    let args_x = x.clone();
    local_rms_norm(&mut x, &weight, eps);
    invoke_dual(
        document,
        label,
        "Inference.rms_norm",
        format!("rms_norm sketch → {}", format_vec(&x, 8)),
        json!({ "x": args_x, "weight": weight, "eps": eps }),
    );
}

/// `Inference.embed` — string → embedding list.
pub(super) fn run_embed(document: &Document, label: &str) {
    let text = selected_source(document).unwrap_or_else(|| "hello world".into());
    let bounded: String = text.chars().take(4096).collect();
    let vec = local_embed(&bounded);
    let nnz = vec.iter().filter(|&&v| v.abs() > 1e-12).count();
    invoke_dual(
        document,
        label,
        "Inference.embed",
        format!(
            "embed sketch dim={EMBED_DIM} nnz={nnz} head {}",
            format_vec(&vec, 4)
        ),
        serde_json::Value::String(bounded),
    );
}

/// `Inference.run_classifier` — knn/nb/svm over surface features (default knn).
pub(super) fn run_run_classifier(document: &Document, label: &str) {
    let container = selected_container(document);
    let method = string_attr(container.as_ref(), "data-method").unwrap_or_else(|| "knn".into());
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let n = numeric_attr(container.as_ref(), "data-n")
        .map(|v| v.max(1.0) as usize)
        .unwrap_or(4);
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.max(1.0) as usize)
        .unwrap_or(2);
    let k = numeric_attr(container.as_ref(), "data-k")
        .map(|v| v.max(1.0) as usize)
        .unwrap_or(3);
    let (features, labels, query) = if nums.len() >= n * p + n + p {
        let features = nums[..n * p].to_vec();
        let labels: Vec<u64> = nums[n * p..n * p + n].iter().map(|v| *v as u64).collect();
        let query = nums[n * p + n..n * p + n + p].to_vec();
        (features, labels, query)
    } else {
        (
            vec![0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0],
            vec![0, 0, 1, 1],
            vec![0.1, 0.1],
        )
    };
    let labels_usize: Vec<usize> = labels.iter().map(|&l| l as usize).collect();
    let predicted = local_knn_predict(&features, &labels_usize, n.min(4).max(1), p.min(features.len()).max(1).min(8), &query, k);
    let n_eff = (features.len() / p.max(1)).min(labels.len());
    invoke_dual(
        document,
        label,
        "Inference.run_classifier",
        format!("classifier({method}) sketch predicted={predicted} (k={k})"),
        json!({
            "method": method,
            "features": features,
            "labels": labels,
            "n": n_eff as u64,
            "p": p as u64,
            "k": k as u64,
            "query": query,
        }),
    );
}

/// `Inference.vector_search` — corpus lines + query → top-k keys.
pub(super) fn run_vector_search(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let mut lines: Vec<String> = source
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .take(32)
        .collect();
    if lines.is_empty() {
        lines = vec![
            "hello world".into(),
            "machine learning".into(),
            "foo bar baz".into(),
        ];
    }
    let query = string_attr(container.as_ref(), "data-query")
        .or_else(|| lines.first().cloned())
        .unwrap_or_else(|| "hello".into());
    let k = numeric_attr(container.as_ref(), "data-k")
        .map(|v| v.max(1.0) as usize)
        .unwrap_or(2);
    let hits = local_vector_search(&lines, &query, k);
    let sketch = hits
        .iter()
        .map(|(key, sim)| format!("{key}={sim:.3}"))
        .collect::<Vec<_>>()
        .join(", ");
    invoke_dual(
        document,
        label,
        "Inference.vector_search",
        format!("vector_search sketch top-k: {sketch}"),
        json!({ "texts": lines, "query": query, "k": k as u64 }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_inf_wave20_activations_match_known() {
        let mut x = vec![-1.0, 0.0, 2.0];
        local_relu(&mut x);
        assert_eq!(x, vec![0.0, 0.0, 2.0]);

        let mut s = vec![0.0];
        local_sigmoid(&mut s);
        assert!((s[0] - 0.5).abs() < 1e-12);

        let mut g = vec![0.0];
        local_gelu(&mut g);
        assert!(g[0].abs() < 1e-12);

        let mut sm = vec![1.0, 2.0, 3.0];
        local_softmax(&mut sm);
        let sum: f64 = sm.iter().sum();
        assert!((sum - 1.0).abs() < 1e-12);
        assert!(sm[2] > sm[1] && sm[1] > sm[0]);

        let mut rn = vec![3.0, -4.0];
        local_rms_norm(&mut rn, &[1.0, 1.0], 0.0);
        let rms = (12.5_f64).sqrt();
        assert!((rn[0] - 3.0 / rms).abs() < 1e-12);
        assert!((rn[1] + 4.0 / rms).abs() < 1e-12);
    }

    #[test]
    fn local_inf_wave20_embed_and_knn_sane() {
        let v = local_embed("hello world");
        assert_eq!(v.len(), EMBED_DIM);
        let norm: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1e-9 || norm == 0.0);

        let features = [0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0];
        let labels = [0usize, 0, 1, 1];
        let pred = local_knn_predict(&features, &labels, 4, 2, &[0.1, 0.1], 3);
        assert_eq!(pred, 0);

        let texts = vec!["hello world".into(), "machine learning".into()];
        let hits = local_vector_search(&texts, "hello", 1);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, "doc_0");
    }
}
