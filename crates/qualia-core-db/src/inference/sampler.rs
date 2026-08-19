//! W2 — exact CPU sampling chain for decode.
//!
//! The native decode loop selects tokens by pure greedy argmax, which is prone to the
//! repetition collapse documented across the bench probes. This module adds a full,
//! llama.cpp-compatible sampling chain that runs on the CPU over the full logit vector
//! (read back once per token — acceptable at the W1 one-fence-per-token cost):
//!
//!   repetition/frequency/presence penalties → temperature → top-k → top-p → seeded draw
//!
//! Design invariants:
//! - **Greedy is a hard short-circuit.** `temperature <= 0` returns the argmax BEFORE any
//!   penalty or filter is applied, so greedy decode is bit-identical to the pre-W2 path and
//!   the a1a/a1c/a1d guarantees are untouched.
//! - **Deterministic.** The draw uses a self-contained SplitMix64 PRNG seeded from the config;
//!   the same seed + same logits + same context reproduce the same token, on native and wasm
//!   (no `rand`, no float transcendentals in the RNG, no platform entropy).
//! - **Exact, not top-K-approximated.** The chain runs over the whole vocabulary; top-k / top-p
//!   are applied as masks, never as a lossy pre-reduction on the GPU.
//! - **Pure + zero-GPU.** Everything here is testable without a device or a model.

/// Sampling parameters. `temperature <= 0.0` ⇒ greedy argmax (all other fields ignored).
///
/// The canonical wire form is a CBOR map (the project's CBOR-first payload substrate) — see
/// [`SamplerConfig::to_cbor`] / [`SamplerConfig::from_cbor`]. Transports that are JSON-enveloped
/// (e.g. the MCP boundary) carry it as a hex-encoded CBOR blob, decoded via the existing JSON
/// string helper — no ad-hoc per-field JSON float parsing.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct SamplerConfig {
    /// Softmax temperature. `<= 0.0` selects greedy argmax (the pre-W2 default behaviour).
    pub temperature: f32,
    /// Keep only the `top_k` highest-logit tokens (`0` ⇒ no top-k limit).
    pub top_k: u32,
    /// Nucleus threshold in `(0.0, 1.0]`; keep the smallest set whose cumulative prob ≥ `top_p`
    /// (`>= 1.0` ⇒ no top-p limit).
    pub top_p: f32,
    /// Multiplicative penalty on logits of tokens present in the penalty window (`1.0` ⇒ none).
    pub repeat_penalty: f32,
    /// Additive penalty per prior occurrence in the window (subtracted from the logit).
    pub freq_penalty: f32,
    /// Additive penalty for any presence in the window (subtracted once).
    pub presence_penalty: f32,
    /// Number of most-recent context tokens the penalties consider (`0` ⇒ whole context).
    pub penalty_window: u32,
    /// PRNG seed — fixed seed ⇒ reproducible draws.
    pub seed: u64,
}

impl Default for SamplerConfig {
    fn default() -> Self {
        // Default is GREEDY (temperature 0) so an unconfigured caller gets the pre-W2 behaviour.
        Self {
            temperature: 0.0,
            top_k: 0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            freq_penalty: 0.0,
            presence_penalty: 0.0,
            penalty_window: 64,
            seed: 0,
        }
    }
}

impl SamplerConfig {
    /// True when this config selects the greedy argmax short-circuit.
    #[inline]
    pub fn is_greedy(&self) -> bool {
        !(self.temperature > 0.0)
    }

    /// Encode as a CBOR map (the canonical payload form). Infallible for this fixed scalar schema.
    pub fn to_cbor(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        // ciborium only fails here on an allocator/writer error, which `Vec` never raises.
        let _ = ciborium::into_writer(self, &mut buf);
        buf
    }

    /// Decode from a CBOR map payload. Returns `None` on malformed CBOR or a schema mismatch —
    /// the caller then keeps greedy decode rather than sampling on a bad config.
    pub fn from_cbor(bytes: &[u8]) -> Option<Self> {
        ciborium::from_reader(bytes).ok()
    }

    /// A sensible default for interactive chat / instruct generation: enough randomness to avoid the
    /// greedy repetition-collapse, plus a light repeat penalty, while staying reproducible (fixed
    /// seed). Benchmarks and determinism tests must NOT install this — the unconfigured global stays
    /// greedy, preserving the bit-exact a1a/a6a guarantees.
    pub fn chat_default() -> Self {
        Self {
            temperature: 0.7,
            top_k: 40,
            top_p: 0.95,
            repeat_penalty: 1.1,
            freq_penalty: 0.0,
            presence_penalty: 0.0,
            penalty_window: 64,
            seed: 0,
        }
    }
}

/// Stateful sampler: owns the PRNG stream so successive tokens advance it deterministically.
#[derive(Debug, Clone)]
pub struct SamplerState {
    pub cfg: SamplerConfig,
    rng: u64,
}

/// SplitMix64 — tiny, allocation-free, platform-independent PRNG (Steele et al. 2014).
#[inline]
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl SamplerState {
    pub fn new(cfg: SamplerConfig) -> Self {
        // Avoid the all-zero SplitMix64 fixed point; mix the seed once.
        let mut rng = cfg.seed ^ 0x2545_F491_4F6C_DD1D;
        let _ = splitmix64(&mut rng);
        Self { cfg, rng }
    }

    /// Uniform f32 in `[0, 1)` from the PRNG stream (53-bit mantissa precision).
    #[inline]
    fn next_unit(&mut self) -> f32 {
        let bits = splitmix64(&mut self.rng) >> 11; // 53 bits
        (bits as f64 * (1.0 / (1u64 << 53) as f64)) as f32
    }

    /// Argmax with lowest-token-id tie-break — identical selection rule to the CPU argmax path.
    #[inline]
    fn argmax(logits: &[f32]) -> u32 {
        let mut best_i = 0u32;
        let mut best_v = f32::NEG_INFINITY;
        for (i, &v) in logits.iter().enumerate() {
            if v > best_v {
                best_v = v;
                best_i = i as u32;
            }
        }
        best_i
    }

    /// Sample the next token id. `logits` is the full vocab (mutated in place as a scratch buffer);
    /// `ctx` is the running token history (prompt + generated) for the penalty window.
    ///
    /// Greedy (`temperature <= 0`) returns the argmax with no mutation of selection semantics.
    pub fn sample(&mut self, logits: &mut [f32], ctx: &[u32]) -> u32 {
        if logits.is_empty() {
            return 0;
        }
        if self.cfg.is_greedy() {
            return Self::argmax(logits);
        }
        let vocab = logits.len();

        // 1) Penalties over the penalty window (most-recent N context tokens).
        let (rp, fp, pp) = (
            self.cfg.repeat_penalty,
            self.cfg.freq_penalty,
            self.cfg.presence_penalty,
        );
        if rp != 1.0 || fp != 0.0 || pp != 0.0 {
            let window = if self.cfg.penalty_window == 0 {
                ctx.len()
            } else {
                (self.cfg.penalty_window as usize).min(ctx.len())
            };
            // Count occurrences of each in-window token id (only ids within vocab matter).
            // A small hashmap-free pass: walk the window, apply per occurrence.
            let start = ctx.len() - window;
            // presence needs "seen at least once" — track via first-touch using freq accumulation.
            for &tok in &ctx[start..] {
                let t = tok as usize;
                if t >= vocab {
                    continue;
                }
                let l = logits[t];
                // repeat_penalty: divide positive logits, multiply negative (llama.cpp convention).
                if rp != 1.0 {
                    logits[t] = if l > 0.0 { l / rp } else { l * rp };
                }
                if fp != 0.0 {
                    logits[t] -= fp; // per-occurrence frequency penalty
                }
            }
            if pp != 0.0 {
                // presence: subtract once per DISTINCT in-window id. Second pass with a seen-guard
                // built from the window (bounded, no alloc beyond a small local set is avoided by
                // re-scanning — window is small, and this keeps the module alloc-free).
                for (rel, &tok) in ctx[start..].iter().enumerate() {
                    let t = tok as usize;
                    if t >= vocab {
                        continue;
                    }
                    // apply only on first occurrence within the window
                    let first = ctx[start..start + rel].iter().all(|&p| p != tok);
                    if first {
                        logits[t] -= pp;
                    }
                }
            }
        }

        // 2) Temperature scale.
        let inv_t = 1.0 / self.cfg.temperature;
        for l in logits.iter_mut() {
            *l *= inv_t;
        }

        // 3) Build an index list sorted by logit desc (stable tie-break by id) for top-k / top-p.
        //    Vocab is ~50k; one sort/token is cheap next to a forward pass.
        let mut order: Vec<u32> = (0..vocab as u32).collect();
        order.sort_unstable_by(|&a, &b| {
            let (la, lb) = (logits[a as usize], logits[b as usize]);
            lb.partial_cmp(&la)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.cmp(&b))
        });

        // 4) top-k truncation.
        let mut keep = if self.cfg.top_k > 0 {
            (self.cfg.top_k as usize).min(vocab)
        } else {
            vocab
        };

        // 5) Softmax over the kept prefix (numerically stable), then top-p nucleus truncation.
        let max_l = logits[order[0] as usize];
        let mut probs: Vec<f32> = Vec::with_capacity(keep);
        let mut sum = 0.0f32;
        for &id in &order[..keep] {
            let p = (logits[id as usize] - max_l).exp();
            probs.push(p);
            sum += p;
        }
        if sum <= 0.0 || !sum.is_finite() {
            return order[0]; // degenerate — fall back to the top logit
        }
        if self.cfg.top_p < 1.0 && self.cfg.top_p > 0.0 {
            let mut cum = 0.0f32;
            let mut nucleus = keep;
            for (i, p) in probs.iter().enumerate() {
                cum += p / sum;
                if cum >= self.cfg.top_p {
                    nucleus = i + 1;
                    break;
                }
            }
            keep = nucleus.max(1);
            probs.truncate(keep);
            sum = probs.iter().sum();
        }

        // 6) Seeded categorical draw over the kept nucleus.
        let r = self.next_unit() * sum;
        let mut acc = 0.0f32;
        for (i, p) in probs.iter().enumerate() {
            acc += *p;
            if r < acc {
                return order[i];
            }
        }
        order[keep - 1]
    }

    /// Sample with a DOMINO constrained-decoding mask applied first.
    ///
    /// This is the R9 integration point: the decode loop calls this instead
    /// of [`sample`](Self::sample) when a [`DominoMasker`] is active. The
    /// masker sets disallowed token logits to `-inf` before the sampling
    /// chain runs, so the sampler can only select grammar-valid tokens.
    ///
    /// When the masker is inactive (`!is_active()`), this is equivalent to
    /// calling `sample` directly — the mask is a no-op.
    ///
    /// After sampling, the chosen token's bytes should be fed back into the
    /// masker via [`DominoMasker::feed_token`] so the grammar state advances.
    pub fn sample_constrained(
        &mut self,
        logits: &mut [f32],
        ctx: &[u32],
        masker: &mut crate::inference::speculative_decode::DominoMasker,
    ) -> u32 {
        masker.apply_mask_preserving(logits);
        let token = self.sample(logits, ctx);
        // Feed the token bytes back into the grammar state. The caller is
        // responsible for looking up the token bytes in the vocabulary and
        // calling `feed_token` if they want the grammar to advance. We do
        // NOT do that here because we don't have the vocabulary mapping.
        // The caller should call `masker.feed_token_id(token, &vocab)`
        // after this returns.
        token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logits(vals: &[f32]) -> Vec<f32> {
        vals.to_vec()
    }

    #[test]
    fn t1_greedy_returns_argmax() {
        let cfg = SamplerConfig::default(); // temperature 0 ⇒ greedy
        let mut s = SamplerState::new(cfg);
        let mut l = logits(&[0.1, 0.9, 0.3, 0.9]); // tie between id 1 and 3 → lowest id wins
        assert_eq!(s.sample(&mut l, &[]), 1);
    }

    #[test]
    fn t2_same_seed_same_sequence() {
        let cfg = SamplerConfig {
            temperature: 1.0,
            seed: 42,
            ..Default::default()
        };
        let base = [2.0f32, 1.0, 0.5, 0.2, -1.0, 3.0, 0.0, 1.5];
        let draw = |seed: u64| {
            let mut s = SamplerState::new(SamplerConfig { seed, ..cfg });
            (0..100)
                .map(|_| s.sample(&mut base.to_vec(), &[]))
                .collect::<Vec<_>>()
        };
        assert_eq!(draw(42), draw(42), "same seed must reproduce the sequence");
    }

    #[test]
    fn t3_different_seed_diverges() {
        let cfg = SamplerConfig {
            temperature: 1.5,
            seed: 1,
            ..Default::default()
        };
        let base = [1.0f32, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]; // uniform ⇒ draws depend only on rng
        let seq = |seed: u64| {
            let mut s = SamplerState::new(SamplerConfig { seed, ..cfg });
            (0..100)
                .map(|_| s.sample(&mut base.to_vec(), &[]))
                .collect::<Vec<_>>()
        };
        assert_ne!(
            seq(1),
            seq(999),
            "different seeds should diverge on a uniform dist"
        );
    }

    #[test]
    fn t4_top_k_1_is_argmax_any_temperature() {
        for temp in [0.5f32, 1.0, 2.0, 5.0] {
            let cfg = SamplerConfig {
                temperature: temp,
                top_k: 1,
                seed: 7,
                ..Default::default()
            };
            let mut s = SamplerState::new(cfg);
            let mut l = logits(&[0.2, 0.1, 2.5, 0.4, 2.5]); // argmax id 2 (tie→lowest)
            assert_eq!(
                s.sample(&mut l, &[]),
                2,
                "top_k=1 must equal argmax at T={temp}"
            );
        }
    }

    #[test]
    fn t5_repeat_penalty_suppresses_repeat() {
        // id 0 is the clear argmax; with a strong repeat penalty and it in-context, it must lose.
        let cfg = SamplerConfig {
            temperature: 1.0,
            top_k: 1, // deterministic: pick the post-penalty argmax
            repeat_penalty: 100.0,
            penalty_window: 8,
            seed: 3,
            ..Default::default()
        };
        let mut s = SamplerState::new(cfg);
        let mut l = logits(&[5.0, 4.0, 3.0, 2.0]);
        let picked = s.sample(&mut l, &[0, 0, 0]); // id 0 heavily penalized
        assert_ne!(
            picked, 0,
            "repeated token should be suppressed below the runner-up"
        );
        assert_eq!(picked, 1);
    }

    #[test]
    fn t6_top_p_head_only() {
        // One dominant token; top_p=0.5 must keep only it, so the draw is deterministic.
        let cfg = SamplerConfig {
            temperature: 1.0,
            top_p: 0.5,
            seed: 11,
            ..Default::default()
        };
        let mut s = SamplerState::new(cfg);
        // softmax([10,0,0,0]) ≈ [0.9999,...]; nucleus 0.5 keeps just id 0.
        let l = logits(&[10.0, 0.0, 0.0, 0.0]);
        for _ in 0..20 {
            assert_eq!(s.sample(&mut l.clone(), &[]), 0);
        }
    }

    #[test]
    fn t7_greedy_ignores_penalties() {
        // Greedy short-circuits BEFORE penalties — proves the a1a/a1c/a1d greedy contract holds.
        let cfg = SamplerConfig {
            temperature: 0.0,
            repeat_penalty: 100.0,
            presence_penalty: 100.0,
            seed: 5,
            ..Default::default()
        };
        let mut s = SamplerState::new(cfg);
        let mut l = logits(&[5.0, 4.0, 3.0]);
        assert_eq!(
            s.sample(&mut l, &[0, 0, 0]),
            0,
            "greedy must ignore penalties and return argmax"
        );
    }

    #[test]
    fn t9_cbor_round_trip() {
        let cfg = SamplerConfig {
            temperature: 0.8,
            top_k: 40,
            top_p: 0.95,
            repeat_penalty: 1.1,
            freq_penalty: 0.2,
            presence_penalty: 0.1,
            penalty_window: 128,
            seed: 0xDEAD_BEEF,
        };
        let bytes = cfg.to_cbor();
        let back = SamplerConfig::from_cbor(&bytes).expect("cbor round-trip");
        assert_eq!(back.temperature, cfg.temperature);
        assert_eq!(back.top_k, cfg.top_k);
        assert_eq!(back.top_p, cfg.top_p);
        assert_eq!(back.repeat_penalty, cfg.repeat_penalty);
        assert_eq!(back.freq_penalty, cfg.freq_penalty);
        assert_eq!(back.presence_penalty, cfg.presence_penalty);
        assert_eq!(back.penalty_window, cfg.penalty_window);
        assert_eq!(back.seed, cfg.seed);
        // Malformed CBOR ⇒ None (caller falls back to greedy, never samples on garbage).
        assert!(SamplerConfig::from_cbor(&[0xFF, 0x00, 0x13, 0x37]).is_none());
    }

    #[test]
    fn t8_presence_once_per_distinct() {
        // presence penalty applies once per distinct id regardless of count.
        let cfg = SamplerConfig {
            temperature: 1.0,
            top_k: 1,
            presence_penalty: 1.5,
            penalty_window: 16,
            seed: 2,
            ..Default::default()
        };
        let mut s = SamplerState::new(cfg);
        // id0 logit 5, id1 logit 4. presence -1.5 to id0 (appears 3x, once applied) → 3.5 < 4 → id1.
        let mut l = logits(&[5.0, 4.0, 0.0]);
        assert_eq!(s.sample(&mut l, &[0, 0, 0]), 1);
    }

    #[test]
    fn t10_chat_default_is_non_greedy_reproducible() {
        let cfg = SamplerConfig::chat_default();
        assert!(
            !cfg.is_greedy(),
            "chat default must sample, not fall back to greedy"
        );
        assert!(cfg.temperature > 0.0 && cfg.top_p <= 1.0 && cfg.repeat_penalty > 1.0);
        // Seeded ⇒ reproducible: the same config reproduces the same draw sequence.
        let base = [2.0f32, 1.0, 0.5, 3.0, 0.2, 1.5];
        let run = || {
            let mut s = SamplerState::new(SamplerConfig::chat_default());
            (0..50)
                .map(|_| s.sample(&mut base.to_vec(), &[]))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            run(),
            run(),
            "chat default must be reproducible for a fixed seed"
        );
    }

    // ── R9: DOMINO sampler integration ──────────────────────────────────

    fn domino_vocab() -> Vec<(u32, String)> {
        vec![
            (0, "=".to_string()),
            (1, " ".to_string()),
            (2, "\n".to_string()),
            (3, "module".to_string()),
            (4, "import".to_string()),
            (5, "fn".to_string()),
            (6, "let".to_string()),
            (7, "return".to_string()),
            (8, "x".to_string()),
            (9, "42".to_string()),
            (10, ";".to_string()),
            (11, "{".to_string()),
            (12, "}".to_string()),
            (13, "(".to_string()),
            (14, ")".to_string()),
            (15, "1".to_string()),
            (16, "2".to_string()),
            (17, "test".to_string()),
            (18, "abc".to_string()),
        ]
    }

    #[test]
    fn r9_constrained_sample_inactive_equals_plain() {
        // When the masker is inactive, sample_constrained == sample.
        // Use two separate sampler states with the same seed so the PRNG
        // streams are identical.
        let cfg = SamplerConfig {
            temperature: 1.0,
            seed: 7,
            ..Default::default()
        };
        let vocab = domino_vocab();
        let mut masker = crate::inference::speculative_decode::DominoMasker::new(&vocab);
        // Masker is inactive by default.
        assert!(!masker.is_active());

        let mut s1 = SamplerState::new(cfg);
        let mut s2 = SamplerState::new(cfg);
        let base: Vec<f32> = (0..vocab.len() as i32).map(|i| i as f32 * 0.1).collect();

        let mut l1 = base.clone();
        let mut l2 = base.clone();
        let t1 = s1.sample(&mut l1, &[]);
        let t2 = s2.sample_constrained(&mut l2, &[], &mut masker);
        assert_eq!(t1, t2, "inactive masker must not change sampling");
    }

    #[test]
    fn r9_constrained_sample_only_returns_valid_tokens() {
        // When the masker is active, the sampled token must be in the
        // grammar-valid set. At Start state, only tokens starting with
        // '=', whitespace, or alpha are valid — digits and braces are not.
        let cfg = SamplerConfig {
            temperature: 1.0,
            top_k: 0, // no truncation — let the mask do the work
            top_p: 1.0,
            seed: 99,
            ..Default::default()
        };
        let vocab = domino_vocab();
        let mut masker = crate::inference::speculative_decode::DominoMasker::new(&vocab);
        masker.enable();
        assert!(masker.is_active());

        let mut s = SamplerState::new(cfg);

        // Give digit tokens (15="1", 16="2", 9="42") very high logits.
        // If the mask works, they should be masked to -inf and never sampled.
        let mut logits = vec![0.1f32; vocab.len()];
        logits[15] = 100.0; // "1" — invalid at Start
        logits[16] = 99.0; // "2" — invalid at Start
        logits[9] = 98.0; // "42" — invalid at Start
        logits[8] = 1.0; // "x" — valid at Start

        // Sample many times — should never return a digit token.
        for _ in 0..100 {
            let mut l = logits.clone();
            let token = s.sample_constrained(&mut l, &[], &mut masker);
            // Token 9 ("42"), 15 ("1"), 16 ("2") should never be sampled.
            assert_ne!(token, 9, "digit token '42' should be masked");
            assert_ne!(token, 15, "digit token '1' should be masked");
            assert_ne!(token, 16, "digit token '2' should be masked");
            // Reset grammar state for the next iteration (we're not feeding
            // tokens here, just testing the mask at Start state).
            masker.reset();
        }
    }

    #[test]
    fn r9_constrained_sample_greedy_with_mask() {
        // Greedy + mask: the argmax should be the highest-logit VALID token,
        // not the highest-logit token overall.
        let cfg = SamplerConfig::default(); // greedy (temperature 0)
        let vocab = domino_vocab();
        let mut masker = crate::inference::speculative_decode::DominoMasker::new(&vocab);
        masker.enable();

        let mut s = SamplerState::new(cfg);

        // Token 15 ("1") has the highest logit but is invalid at Start.
        // Token 8 ("x") has a lower logit but is valid.
        let mut logits = vec![0.1f32; vocab.len()];
        logits[15] = 100.0; // "1" — invalid
        logits[8] = 50.0; // "x" — valid

        let token = s.sample_constrained(&mut logits, &[], &mut masker);
        // Greedy should pick the highest VALID token, not the highest overall.
        // "1" is masked to -inf, so "x" (id 8) should win.
        // But we need to check: are there other valid tokens with higher logits?
        // At Start, valid tokens are: 0 ("="), 1 (" "), 2 ("\n"), 3 ("module"),
        // 4 ("import"), 5 ("fn"), 6 ("let"), 7 ("return"), 8 ("x"), 17 ("test"),
        // 18 ("abc"). Token 8 has logit 50.0, all others have 0.1. So 8 wins.
        assert_eq!(token, 8, "greedy + mask should pick highest valid token");
    }

    #[test]
    fn r9_constrained_sample_preserves_logit_values() {
        // The apply_mask_preserving method should keep original logit values
        // for valid tokens, not zero them out.
        let cfg = SamplerConfig {
            temperature: 1.0,
            seed: 3,
            ..Default::default()
        };
        let vocab = domino_vocab();
        let mut masker = crate::inference::speculative_decode::DominoMasker::new(&vocab);
        masker.enable();

        let mut s = SamplerState::new(cfg);
        let mut logits = vec![0.0f32; vocab.len()];
        logits[8] = 5.0; // "x" — valid at Start
        logits[3] = 3.0; // "module" — valid at Start
        logits[15] = 10.0; // "1" — invalid at Start

        // The sampler should be able to distinguish between valid tokens
        // based on their original logit values (not just pick any valid one).
        // With temperature 1.0 and seed 3, token 8 (logit 5.0) should be
        // sampled more often than token 3 (logit 3.0).
        let mut count_8 = 0;
        let mut count_3 = 0;
        for _ in 0..1000 {
            let mut l = logits.clone();
            let token = s.sample_constrained(&mut l, &[], &mut masker);
            if token == 8 {
                count_8 += 1;
            } else if token == 3 {
                count_3 += 1;
            }
            masker.reset();
        }
        // Token 8 has higher logit → should be sampled more often.
        assert!(
            count_8 > count_3,
            "higher-logit valid token should be sampled more often (8: {count_8}, 3: {count_3})"
        );
    }
}
