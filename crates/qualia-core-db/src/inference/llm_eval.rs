//! W1 — in-project quality oracle (no external libs): perplexity, KL-divergence, and a coherence
//! ratio, measured by teacher-forcing an eval corpus through the engine and comparing a candidate
//! model's output distribution against a higher-fidelity reference's.
//!
//! **Honesty note.** ΔPPL and KL here are *relative* (candidate vs reference on identical text), so
//! the comparison is self-consistent regardless of who authored the corpus. They measure *engineering
//! fidelity* — does quantization preserve the reference model's behaviour — not whether the model's
//! outputs are true or whether any direction is correct. The "reference" is the highest-fidelity model
//! on disk (Q8_0 unless a real F16 is supplied); it is labelled as such, never silently called "FP16".
//!
//! This module is pure math + thresholds + a corpus loader; the engine forward pass that feeds it
//! lives in the bench harness. Metric paths take slices and return scalars (heap only in the loader).

use std::sync::atomic::{AtomicU64, Ordering};

// ── Quality gate (set by Timothy / Gemini, 2026-06-25) ────────────────────────
/// Max relative perplexity increase vs the reference (soft evidence).
pub const MAX_DELTA_PPL: f64 = 0.05; // ≤ 5%
/// Max average per-token KL-divergence (reference ‖ candidate) over the corpus.
pub const MAX_AVG_KL: f64 = 0.06;
/// Min unique-word ratio on a generation loop (hard gate — eliminates repetition collapse).
pub const MIN_UNIQ_WORD: f64 = 0.90;

/// Numerically-stable log-sum-exp over `logits` (f64 accumulation).
pub fn log_sum_exp(logits: &[f32]) -> f64 {
    if logits.is_empty() {
        return f64::NEG_INFINITY;
    }
    let m = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max) as f64;
    if !m.is_finite() {
        return m;
    }
    let s: f64 = logits.iter().map(|&l| (l as f64 - m).exp()).sum();
    m + s.ln()
}

/// Negative log-likelihood (nats) of `target` under `softmax(logits)` = `logsumexp - logits[target]`.
pub fn token_nll(logits: &[f32], target: usize) -> f64 {
    if target >= logits.len() {
        return f64::INFINITY;
    }
    log_sum_exp(logits) - logits[target] as f64
}

/// Perplexity from a summed NLL (nats) over `n_tokens`: `exp(total_nll / n_tokens)`.
pub fn perplexity(total_nll: f64, n_tokens: usize) -> f64 {
    if n_tokens == 0 {
        return f64::INFINITY;
    }
    (total_nll / n_tokens as f64).exp()
}

/// Relative perplexity increase of `candidate` over `reference`: `(cand - ref) / ref`.
pub fn delta_ppl(reference: f64, candidate: f64) -> f64 {
    if reference <= 0.0 || !reference.is_finite() {
        return f64::INFINITY;
    }
    (candidate - reference) / reference
}

/// Write the numerically-stable log-softmax of `logits` into `out` (same length).
fn log_softmax_into(logits: &[f32], out: &mut [f64]) {
    let lse = log_sum_exp(logits);
    for (o, &l) in out.iter_mut().zip(logits) {
        *o = l as f64 - lse;
    }
}

/// KL-divergence `D(reference ‖ candidate)` between the two softmax distributions, in nats.
/// `= Σ p_ref · (log p_ref − log p_cand)`, computed via log-softmax for stability. Non-negative.
pub fn kl_divergence(ref_logits: &[f32], cand_logits: &[f32], scratch: &mut [f64]) -> f64 {
    let n = ref_logits.len();
    if n == 0 || cand_logits.len() != n || scratch.len() < 2 * n {
        return f64::INFINITY;
    }
    let (lp_ref, lp_cand) = scratch.split_at_mut(n);
    log_softmax_into(ref_logits, lp_ref);
    log_softmax_into(cand_logits, &mut lp_cand[..n]);
    let mut kl = 0.0f64;
    for i in 0..n {
        let p = lp_ref[i].exp();
        if p > 0.0 {
            kl += p * (lp_ref[i] - lp_cand[i]);
        }
    }
    kl.max(0.0) // clamp tiny negative from float error
}

/// Unique-word ratio (coherence proxy): distinct whitespace tokens / total. Repetition collapse → low.
pub fn unique_word_ratio(text: &str) -> f64 {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return 0.0;
    }
    let uniq: std::collections::HashSet<&str> = words.iter().copied().collect();
    uniq.len() as f64 / words.len() as f64
}

/// The three-tier verdict against the gate. `hard_pass` (coherence) must hold; ΔPPL/KL are evidence.
#[derive(Debug, Clone, Copy)]
pub struct QualityVerdict {
    pub delta_ppl: f64,
    pub avg_kl: f64,
    pub uniq_word: f64,
}

impl QualityVerdict {
    /// Hard gate: coherence (no repetition collapse). A change that fails this is rejected outright.
    pub fn hard_pass(&self) -> bool {
        self.uniq_word >= MIN_UNIQ_WORD
    }
    /// Soft gate: ΔPPL and KL within budget.
    pub fn soft_pass(&self) -> bool {
        self.delta_ppl <= MAX_DELTA_PPL && self.avg_kl <= MAX_AVG_KL
    }
    /// Overall accept: both gates hold.
    pub fn accept(&self) -> bool {
        self.hard_pass() && self.soft_pass()
    }
}

/// Load the eval corpus (one passage per line; blank lines dropped). Searches the standard roots so it
/// works from the crate dir or the repo root, mirroring the model/results lookups in the bench.
pub fn load_corpus() -> std::io::Result<Vec<String>> {
    const CANDIDATES: [&str; 3] = [
        "benchmarks/data/eval_corpus.txt",
        "../../benchmarks/data/eval_corpus.txt",
        "../benchmarks/data/eval_corpus.txt",
    ];
    let mut last_err = std::io::Error::new(std::io::ErrorKind::NotFound, "eval_corpus.txt not found");
    for p in CANDIDATES {
        match std::fs::read_to_string(p) {
            Ok(s) => {
                return Ok(s
                    .lines()
                    .map(|l| l.to_string())
                    .filter(|l| !l.trim().is_empty())
                    .collect())
            }
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

// ── Process-wide PPL accumulators (so a teacher-forced forward on the engine thread can report back) ──
static EVAL_NLL_BITS: AtomicU64 = AtomicU64::new(0); // f64 total NLL, bit-encoded
static EVAL_TOKENS: AtomicU64 = AtomicU64::new(0);

/// Reset the teacher-forced PPL accumulators before an eval pass.
pub fn reset_ppl() {
    EVAL_NLL_BITS.store(0, Ordering::Relaxed);
    EVAL_TOKENS.store(0, Ordering::Relaxed);
}

/// Add one position's NLL (nats) + token to the accumulators.
pub fn add_ppl(nll: f64, tokens: u64) {
    // simple non-atomic-RMW-safe accumulate: single eval thread, so load/store is fine.
    let cur = f64::from_bits(EVAL_NLL_BITS.load(Ordering::Relaxed));
    EVAL_NLL_BITS.store((cur + nll).to_bits(), Ordering::Relaxed);
    EVAL_TOKENS.fetch_add(tokens, Ordering::Relaxed);
}

/// Current `(total_nll, token_count)` snapshot.
pub fn ppl_snapshot() -> (f64, u64) {
    (
        f64::from_bits(EVAL_NLL_BITS.load(Ordering::Relaxed)),
        EVAL_TOKENS.load(Ordering::Relaxed),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nll_and_ppl_known_values() {
        // Uniform logits over 4 classes → each prob 0.25 → NLL = ln 4; PPL over n identical = 4.
        let logits = [0.0f32, 0.0, 0.0, 0.0];
        let nll = token_nll(&logits, 2);
        assert!((nll - 4.0f64.ln()).abs() < 1e-9);
        assert!((perplexity(nll * 3.0, 3) - 4.0).abs() < 1e-6);
    }

    #[test]
    fn kl_zero_for_identical_and_positive_otherwise() {
        let a = [2.0f32, 1.0, 0.0, -1.0];
        let b = [0.5f32, 0.5, 0.5, 0.5];
        let mut scratch = vec![0f64; 2 * a.len()];
        assert!(kl_divergence(&a, &a, &mut scratch) < 1e-9);
        assert!(kl_divergence(&a, &b, &mut scratch) > 0.0);
    }

    #[test]
    fn delta_ppl_and_gate() {
        assert!((delta_ppl(10.0, 10.5) - 0.05).abs() < 1e-9);
        let pass = QualityVerdict { delta_ppl: 0.03, avg_kl: 0.04, uniq_word: 0.95 };
        let fail_hard = QualityVerdict { delta_ppl: 0.0, avg_kl: 0.0, uniq_word: 0.10 };
        let fail_soft = QualityVerdict { delta_ppl: 0.20, avg_kl: 0.04, uniq_word: 0.95 };
        assert!(pass.accept());
        assert!(!fail_hard.accept() && !fail_hard.hard_pass());
        assert!(!fail_soft.accept() && fail_soft.hard_pass() && !fail_soft.soft_pass());
    }

    #[test]
    fn unique_word_detects_collapse() {
        assert!(unique_word_ratio("the quick brown fox jumps") > 0.9);
        assert!(unique_word_ratio("the the the the the") < 0.3);
        assert_eq!(unique_word_ratio(""), 0.0);
    }
}
