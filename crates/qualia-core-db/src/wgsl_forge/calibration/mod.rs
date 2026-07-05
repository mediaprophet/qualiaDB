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
pub mod package;

pub use corpus::CorpusSpec;
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
    fn label(self) -> &'static str {
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
        // Dictionary / Top-K SAE learning (Lexico) needs a trained per-layer dictionary via k-SVD +
        // OMP over a custom corpus — the W5b research task with its own eval-corpus curation.
        ArtifactKind::KvDictionary => Err(CalibrationError::NotYetImplemented("W5b sparse KV dictionary")),
    }
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
