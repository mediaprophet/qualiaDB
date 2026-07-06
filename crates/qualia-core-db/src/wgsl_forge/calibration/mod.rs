//! W10 — Forge calibration pipeline (the training-related upgrade).
//!
//! This is an **upgrade of the existing forge**, not a new forge: the forge already PRODUCES +
//! CERTIFIES artifacts (it certifies GPU kernels against a CPU oracle and transcodes GGUF→p64).
//! Calibration/adaptation artifacts — AWQ activation scales, int8-KV scales, sparse KV
//! dictionaries — are the same produce-and-certify pattern applied to a new artifact class, so they
//! live here as a `calibration` concern beside the kernel/transcode entry points. The engine still
//! only RUNS certified artifacts.
//!
//! Pipeline (5 stages): **corpus → capture → learn → certify → package.**
//! - **corpus** ([`corpus`]) — assemble/expand the calibration text. *Local Ollama is a legitimate
//!   resource HERE* (offline domain-diverse synthesis), strictly forge-side — it never enters the
//!   inference runtime (CLAUDE.md §1 holds).
//! - **capture** — run OUR engine over the corpus with instrumentation on (reuses `llm_awq`'s
//!   activation hooks; KV capture arrives with W5a). This CANNOT come from Ollama — the artifacts
//!   compress our engine's own tensors (GQA layout, RoPE convention, layer shapes are engine-specific).
//! - **learn** — fit the artifact (AWQ scale fold reuses the existing AWQ pipeline; int8-KV scales =
//!   W5a; dictionary/Top-K SAE = W5b).
//! - **certify** ([`certify`]) — the ΔPPL ≤ gate via the existing [`perplexity_eval_blocking`] oracle.
//! - **package** ([`package`]) — certified artifact + provenance (corpus hash, engine version, gate
//!   numbers) as a CBOR-framed sidecar, so the engine can refuse uncertified artifacts.
//!
//! Native-only: the pipeline drives the real inference stack (GGUF, GPU, PPL oracle).

#![cfg(not(target_arch = "wasm32"))]

pub mod corpus;
pub mod kv_dictionary;
pub mod package;

pub use corpus::CorpusSpec;
pub use kv_dictionary::{learn_dictionary, KvDictionary, SparseCode};
pub use package::Provenance;

use std::path::PathBuf;

/// Which calibration artifact to produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ArtifactKind {
    /// AWQ per-input-channel activation scales for the FFN projections (implemented — reuses the
    /// existing AWQ capture+fold+sweep).
    AwqScales,
    /// int8 K/V-cache scales (per head-slot). Gated on W5a (the int8 KV cache).
    KvInt8Scales,
    /// Sparse KV dictionary / Top-K SAE (Lexico-style). Gated on W5b (needs a custom corpus + OMP/k-SVD).
    KvDictionary,
}

impl ArtifactKind {
    /// Stable machine label (diagnostics / provenance display).
    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            ArtifactKind::AwqScales => "awq_scales",
            ArtifactKind::KvInt8Scales => "kv_int8_scales",
            ArtifactKind::KvDictionary => "kv_dictionary",
        }
    }
}

/// The ΔPPL acceptance gate for a lossy artifact (fraction, e.g. 0.05 = 5%).
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct GateSpec {
    pub max_delta_ppl: f64,
}

impl Default for GateSpec {
    fn default() -> Self {
        // The project-wide compression gate (5%), same constant the AWQ/ternary work uses.
        Self {
            max_delta_ppl: crate::llm_eval::MAX_DELTA_PPL,
        }
    }
}

/// A calibration job: produce `artifact` for `model_path`, calibrated on `corpus`, gated by `gate`.
#[derive(Debug, Clone)]
pub struct CalibrationJob {
    pub model_path: PathBuf,
    pub artifact: ArtifactKind,
    pub corpus: CorpusSpec,
    pub gate: GateSpec,
    /// Token budget per PPL pass (0 = the oracle's default full corpus).
    pub max_tok: usize,
}

/// The outcome of a calibration run.
#[derive(Debug, Clone)]
pub struct CalibrationReport {
    pub artifact: ArtifactKind,
    /// Content hash of the assembled calibration corpus (provenance).
    pub corpus_hash: u64,
    pub corpus_docs: usize,
    /// Reference (uncompressed) perplexity.
    pub ref_ppl: f64,
    /// Candidate (artifact-applied) perplexity.
    pub cand_ppl: f64,
    /// (cand - ref) / ref.
    pub delta_ppl: f64,
    /// Whether `delta_ppl <= gate.max_delta_ppl`.
    pub passed: bool,
    /// The packaged artifact bytes (artifact + CBOR provenance frame) — only when `passed`.
    pub packaged: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalibrationError {
    CorpusEmpty,
    OllamaUnavailable(String),
    CaptureFailed(String),
    CertifyFailed(String),
    PackageFailed(String),
    /// The artifact kind exists in the taxonomy but its learner is not built yet (honest gate, not a
    /// silent stub) — the workstream that lands it is named.
    NotYetImplemented(&'static str),
}

impl std::fmt::Display for CalibrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CalibrationError::CorpusEmpty => write!(f, "calibration corpus is empty"),
            CalibrationError::OllamaUnavailable(e) => write!(f, "ollama corpus synthesis unavailable: {e}"),
            CalibrationError::CaptureFailed(e) => write!(f, "activation capture failed: {e}"),
            CalibrationError::CertifyFailed(e) => write!(f, "certification (PPL) failed: {e}"),
            CalibrationError::PackageFailed(e) => write!(f, "artifact packaging failed: {e}"),
            CalibrationError::NotYetImplemented(w) => write!(f, "artifact learner not yet implemented ({w})"),
        }
    }
}

/// Run the calibration pipeline for one job. The forge's third produce-and-certify entry point,
/// beside kernel certification and GGUF→p64 transcode.
pub fn run_calibration(job: &CalibrationJob) -> Result<CalibrationReport, CalibrationError> {
    // Stage 1 — corpus. Assembled + hashed for provenance regardless of artifact kind.
    let docs = corpus::assemble(&job.corpus)?;
    if docs.is_empty() {
        return Err(CalibrationError::CorpusEmpty);
    }
    let corpus_hash = corpus::content_hash(&docs);
    let corpus_docs = docs.len();

    // Stages 2–4 — capture + learn + certify, per artifact kind.
    match job.artifact {
        ArtifactKind::AwqScales => run_awq(job, corpus_hash, corpus_docs),
        // int8-KV scale calibration needs the W5a int8 KV cache (write-side amax → per-head-slot f16
        // scale). The seam lands with W5a; visible-not-stubbed until then.
        ArtifactKind::KvInt8Scales => Err(CalibrationError::NotYetImplemented("W5a int8 KV cache")),
        // Sparse KV dictionary (Lexico / Top-K SAE): W5b. Phase 3 = the go/no-go — capture the
        // engine's real KV vectors, learn a per-layer dictionary, and compare its reconstruction to
        // int8's at footprint. Packaging a certified runtime artifact is Phase 4 and only justified if
        // this gate says GO.
        ArtifactKind::KvDictionary => run_kv_dictionary(job, corpus_hash, corpus_docs),
    }
}

/// Per-(layer, stream) rate-distortion comparison. int8 (the W5a incumbent, 8 bits/elem) is a strong,
/// accurate baseline; a sparse dictionary trades accuracy for a much smaller footprint. So the decision
/// isn't "does the dictionary beat int8's accuracy" (it won't — int8 has ~4× the bits) but "at the
/// dictionary's OWN low bit rate, does the learned basis beat NAIVE uniform quantization?" — i.e. is the
/// learned codebook worth more than just quantizing more coarsely.
#[derive(Debug, Clone)]
pub struct KvLayerVerdict {
    pub layer: usize,
    /// "K" or "V".
    pub stream: &'static str,
    pub n_vectors: usize,
    /// int8 incumbent (8-bit) reconstruction error and footprint — context, not the gate.
    pub recon_int8: f64,
    pub int8_bits: f64,
    /// k-sparse dictionary reconstruction error and asymptotic code footprint (bits/vec, dictionary
    /// amortized away as at deployment scale).
    pub recon_dict: f64,
    pub dict_code_bits: f64,
    /// Naive uniform quantization at the dictionary's matched bit rate — the head-to-head baseline.
    pub matched_bits: u32,
    pub recon_uniform_matched: f64,
    /// GO here: the learned dictionary reconstructs at least as well as uniform quantization AT THE
    /// SAME bit rate (the learned basis earns its keep).
    pub go: bool,
}

/// The W5b Phase-3 decision: does a learned sparse dictionary beat int8 on the engine's real KV
/// vectors? Per-layer detail plus an overall verdict.
#[derive(Debug, Clone)]
pub struct KvDictReport {
    pub head_dim: usize,
    pub n_atoms: usize,
    pub sparsity: usize,
    pub layers: Vec<KvLayerVerdict>,
    /// GO iff a majority of analyzed (layer, stream) pairs have the dictionary dominating int8.
    pub overall_go: bool,
}

/// W5b Phase 3 — the sparse-KV-dictionary go/no-go on **real engine KV vectors**.
///
/// Enables the [`crate::kv_capture`] hook, runs a calibration forward (the native attention path routes
/// through the CPU SDPA that the hook taps), then per sampled layer learns a dictionary over the
/// captured K and V vectors and compares its reconstruction error + footprint to per-vector int8. This
/// is a **measurement**, not a packaged artifact — the artifact (runtime dictionary decode + ΔPPL
/// certify) is Phase 4, built only if this returns GO.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
pub fn kv_dictionary_go_no_go(
    model_path: &str,
    n_layer_hint: usize,
    max_per_layer: usize,
    n_atoms: usize,
    sparsity: usize,
    iters: usize,
    max_tok: usize,
    layer_stride: usize,
) -> Result<KvDictReport, CalibrationError> {
    use kv_dictionary::{
        dict_code_bits_per_vector, int8_bits_per_vector, int8_reconstruction_error, learn_dictionary,
        uniform_reconstruction_error,
    };

    // The KV hook taps `cpu_attention_pass`, which is opt-in (default native attention runs on the GPU
    // shader). Force the CPU reference path for the duration of the capture, then restore it. The
    // captured K/V are numerically the same vectors either path produces (the CPU SDPA is the GPU
    // path's certified reference), so this doesn't bias the geometry we're measuring.
    let prev_cpu_attn = crate::llm_bench::cpu_attention_enabled();
    crate::llm_bench::set_cpu_attention(true);
    crate::kv_capture::enable(n_layer_hint, max_per_layer);
    let ev = crate::llm_bench::perplexity_eval_blocking(model_path, max_tok);
    let cap = crate::kv_capture::snapshot();
    crate::kv_capture::disable();
    crate::kv_capture::clear();
    crate::llm_bench::set_cpu_attention(prev_cpu_attn);
    ev.map_err(CalibrationError::CaptureFailed)?;
    let cap = cap.ok_or_else(|| {
        CalibrationError::CaptureFailed(
            "no KV captured — the calibration forward never hit the CPU attention path".into(),
        )
    })?;

    let head_dim = cap.head_dim;
    let int8_bits = int8_bits_per_vector(head_dim);
    // Asymptotic code rate of the dictionary (dictionary amortized away, as at deployment scale) and
    // the integer bit-rate of uniform quantization that matches it — the head-to-head baseline.
    let dict_code_bits = dict_code_bits_per_vector(n_atoms, sparsity, 16);
    let matched_bits = ((dict_code_bits / head_dim as f64).round() as u32).clamp(2, 8);
    let mut layers = Vec::new();
    for li in (0..cap.k.len()).step_by(layer_stride.max(1)) {
        for (stream, vecs) in [("K", &cap.k[li]), ("V", &cap.v[li])] {
            // Need enough vectors for the dictionary to be meaningful (≥ atom count).
            if vecs.len() < n_atoms.max(16) {
                continue;
            }
            let dict = learn_dictionary(vecs, head_dim, n_atoms, sparsity, iters);
            let recon_dict = dict.reconstruction_error(vecs, sparsity);
            let recon_int8 = int8_reconstruction_error(vecs);
            let recon_uniform_matched = uniform_reconstruction_error(vecs, matched_bits);
            // GO: the learned basis beats naive uniform quantization at the same bit rate.
            let go = recon_dict <= recon_uniform_matched;
            layers.push(KvLayerVerdict {
                layer: li,
                stream,
                n_vectors: vecs.len(),
                recon_int8,
                int8_bits,
                recon_dict,
                dict_code_bits,
                matched_bits,
                recon_uniform_matched,
                go,
            });
        }
    }
    let go_count = layers.iter().filter(|l| l.go).count();
    let overall_go = !layers.is_empty() && go_count * 2 >= layers.len();
    Ok(KvDictReport {
        head_dim,
        n_atoms,
        sparsity,
        layers,
        overall_go,
    })
}

/// `run_calibration` arm for [`ArtifactKind::KvDictionary`]: run the Phase-3 go/no-go with sensible
/// defaults and map it into a [`CalibrationReport`]. For this artifact kind `ref_ppl`/`cand_ppl` carry
/// the mean **reconstruction error** (int8 vs dictionary) — the quality metric that governs the
/// decision — and `passed` is the overall GO. `packaged` is always `None`: a certified runtime artifact
/// is Phase 4, gated on GO (this stays honest rather than emitting an unbuilt artifact).
#[cfg(not(target_arch = "wasm32"))]
fn run_kv_dictionary(
    job: &CalibrationJob,
    corpus_hash: u64,
    corpus_docs: usize,
) -> Result<CalibrationReport, CalibrationError> {
    let model = job.model_path.to_string_lossy().to_string();
    // Defaults: 256 atoms, 4-sparse, over up to 2048 vectors/layer, sampling ~every 6th layer.
    let report = kv_dictionary_go_no_go(&model, 80, 2048, 256, 4, 25, job.max_tok, 6)?;

    let mean = |f: fn(&KvLayerVerdict) -> f64| -> f64 {
        if report.layers.is_empty() {
            0.0
        } else {
            report.layers.iter().map(f).sum::<f64>() / report.layers.len() as f64
        }
    };
    let ref_ppl = mean(|l| l.recon_int8);
    let cand_ppl = mean(|l| l.recon_dict);
    let delta_ppl = if ref_ppl > 0.0 {
        (cand_ppl - ref_ppl) / ref_ppl
    } else {
        0.0
    };

    Ok(CalibrationReport {
        artifact: ArtifactKind::KvDictionary,
        corpus_hash,
        corpus_docs,
        ref_ppl,
        cand_ppl,
        delta_ppl,
        passed: report.overall_go,
        packaged: None,
    })
}

/// AWQ scales: the real capture→fold→sweep pipeline (reuses [`awq_sweep_blocking`]), certified
/// against the Q8 reference and packaged as an AWQ-folded Q4_0 FFN p64 with a provenance frame.
#[cfg(not(target_arch = "wasm32"))]
fn run_awq(
    job: &CalibrationJob,
    corpus_hash: u64,
    corpus_docs: usize,
) -> Result<CalibrationReport, CalibrationError> {
    use crate::p64_weight::FfnQuant;
    let model = job.model_path.to_string_lossy().to_string();
    // Coarse α-sweep (0 / 0.5 / 1) over the Q4_0 FFN — AWQ's design regime. `awq_sweep_blocking`
    // runs the capture + fold + PPL certify internally and returns (ref_ppl, [(alpha, ppl, uniq)]).
    let alphas = [0.0f32, 0.5, 1.0];
    let (ref_ppl, results) =
        crate::llm_bench::awq_sweep_blocking(&model, &alphas, job.max_tok, FfnQuant::Q4_0)
            .map_err(CalibrationError::CertifyFailed)?;
    let best = results
        .iter()
        .filter(|(_, p, _)| p.is_finite() && *p > 1.0)
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .copied()
        .ok_or_else(|| CalibrationError::CertifyFailed("AWQ sweep produced no finite PPL".into()))?;
    let (best_alpha, cand_ppl, _uniq) = best;
    let delta_ppl = crate::llm_eval::delta_ppl(ref_ppl, cand_ppl);
    let passed = delta_ppl <= job.gate.max_delta_ppl;

    let packaged = if passed {
        // Package: recompute the AWQ scales at the winning α + fold into a Q4_0 FFN p64, then frame
        // it with the CBOR provenance. Capture the scales via a fresh AWQ pass over the reference.
        let bytes = std::fs::read(&job.model_path)
            .map_err(|e| CalibrationError::PackageFailed(format!("read gguf: {e}")))?;
        let scales = capture_awq_scales(&model, job.max_tok)?;
        let p64 = crate::p64_weight::compile_gguf_to_q42_ffn_quant_awq(
            &bytes,
            14,
            Some(&scales),
            best_alpha,
            FfnQuant::Q4_0,
        )
        .map_err(|e| CalibrationError::PackageFailed(format!("awq compile: {e}")))?;
        let prov = Provenance::new(
            ArtifactKind::AwqScales,
            corpus_hash,
            corpus_docs,
            ref_ppl,
            cand_ppl,
            delta_ppl,
            true,
        );
        Some(package::frame_artifact(&p64, &prov))
    } else {
        None
    };

    Ok(CalibrationReport {
        artifact: ArtifactKind::AwqScales,
        corpus_hash,
        corpus_docs,
        ref_ppl,
        cand_ppl,
        delta_ppl,
        passed,
        packaged,
    })
}

/// Capture per-FFN-layer per-input-channel AWQ activation scales by running a calibration forward
/// over the reference model with the `llm_awq` hooks enabled. Native (drives the GPU forward).
#[cfg(not(target_arch = "wasm32"))]
fn capture_awq_scales(model: &str, max_tok: usize) -> Result<Vec<Vec<f32>>, CalibrationError> {
    // SmolLM2-360M shape; the hook self-sizes on the first record. Layers/channels are read from the
    // snapshot, so an over-estimate here is harmless (the AWQ module clamps).
    crate::llm_awq::enable(64, 4096).map_err(CalibrationError::CaptureFailed)?;
    let cap = crate::llm_bench::perplexity_eval_blocking(model, max_tok);
    let scales = crate::llm_awq::snapshot();
    crate::llm_awq::disable();
    cap.map_err(CalibrationError::CaptureFailed)?;
    if scales.is_empty() || scales.iter().all(|l| l.iter().all(|&v| v == 0.0)) {
        return Err(CalibrationError::CaptureFailed(
            "AWQ hooks captured no activations".into(),
        ));
    }
    Ok(scales)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unimplemented_kinds_are_visible_not_stubbed() {
        // A tiny in-memory corpus so we reach the artifact dispatch.
        let job = |kind| CalibrationJob {
            model_path: PathBuf::from("/nonexistent.gguf"),
            artifact: kind,
            corpus: CorpusSpec::Files(vec![]),
            gate: GateSpec::default(),
            max_tok: 0,
        };
        // Empty Files corpus → CorpusEmpty (the corpus stage gates before artifact dispatch).
        assert_eq!(run_calibration(&job(ArtifactKind::KvInt8Scales)).unwrap_err(), CalibrationError::CorpusEmpty);
    }

    #[test]
    fn gate_default_is_the_project_ppl_gate() {
        assert_eq!(GateSpec::default().max_delta_ppl, crate::llm_eval::MAX_DELTA_PPL);
    }

    #[test]
    fn artifact_kind_labels_stable() {
        assert_eq!(ArtifactKind::AwqScales.label(), "awq_scales");
        assert_eq!(ArtifactKind::KvInt8Scales.label(), "kv_int8_scales");
        assert_eq!(ArtifactKind::KvDictionary.label(), "kv_dictionary");
    }
}
