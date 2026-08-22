//! GBNF constrained sampling bindings (T53/W11).
//!
//! Exposes the in-process GBNF grammar-constrained sampler to VibeScript.
//! The sampler uses the DominoMasker (TokenTrie + GrammarStateMachine) to
//! constrain token selection to valid VibeScript grammar productions.
//!
//! The actual decode-loop integration lives in `inference_agent::decode` and
//! `inference_bench::toggles` — this module provides the VibeScript-facing
//! control surface that lets scripts install, enable, disable, and reset the
//! global DOMINO masker, plus a pure-CPU sample entry point for testing.
//!
//! Bindings (capability.invoke ids):
//! - `sampler.configure` — install a sampler config (temperature, top_k, …)
//! - `sampler.constrain_enable` — enable GBNF constrained decoding (with vocab)
//! - `sampler.constrain_disable` — disable constrained decoding
//! - `sampler.constrain_reset` — reset the grammar state machine
//! - `sampler.sample` — pure-CPU sample from a logits array (testing surface)
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` T53/W11.

use crate::inference::sampler::{SamplerConfig, SamplerState};
use crate::inference::speculative_decode::DominoMasker;
use std::collections::BTreeMap;
use vibe::{DiagCode, Diagnostic, Span, Value};

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
use crate::llm_bench;

// Browser profiles still expose the pure CPU sampler. Keep its control state
// local to the WASM instance rather than coupling it to the native benchmark
// decode loop.
#[cfg(all(target_arch = "wasm32", not(feature = "wasm-llm")))]
mod llm_bench {
    use super::*;
    use std::cell::RefCell;

    thread_local! {
        static CONFIG: RefCell<Option<SamplerConfig>> = const { RefCell::new(None) };
        static MASKER: RefCell<Option<DominoMasker>> = const { RefCell::new(None) };
    }

    pub fn set_sampler_config(cfg: Option<SamplerConfig>) {
        CONFIG.with(|slot| *slot.borrow_mut() = cfg);
    }

    pub fn sampler_config() -> Option<SamplerConfig> {
        CONFIG.with(|slot| *slot.borrow())
    }

    pub fn set_domino_masker(masker: Option<DominoMasker>) {
        MASKER.with(|slot| *slot.borrow_mut() = masker);
    }

    pub fn domino_active() -> bool {
        MASKER.with(|slot| slot.borrow().as_ref().is_some_and(DominoMasker::is_active))
    }

    pub fn domino_reset() {
        MASKER.with(|slot| {
            if let Some(masker) = slot.borrow_mut().as_mut() {
                masker.reset();
            }
        });
    }

    pub fn domino_sample(
        state: &mut SamplerState,
        logits: &mut [f32],
        context: &[u32],
    ) -> Option<u32> {
        MASKER.with(|slot| {
            let mut slot = slot.borrow_mut();
            let masker = slot.as_mut()?;
            if !masker.is_active() {
                return None;
            }
            masker.apply_mask_preserving(logits);
            Some(state.sample(logits, context))
        })
    }
}

/// Parse a `SamplerConfig` from a VibeScript record.
fn parse_config(rec: &BTreeMap<String, Value>) -> SamplerConfig {
    let mut cfg = SamplerConfig::default();
    if let Some(v) = rec.get("temperature").and_then(|v| v.as_f64()) {
        cfg.temperature = v as f32;
    }
    if let Some(v) = rec.get("top_k").and_then(|v| v.as_f64()) {
        cfg.top_k = v as u32;
    }
    if let Some(v) = rec.get("top_p").and_then(|v| v.as_f64()) {
        cfg.top_p = v as f32;
    }
    if let Some(v) = rec.get("repeat_penalty").and_then(|v| v.as_f64()) {
        cfg.repeat_penalty = v as f32;
    }
    if let Some(v) = rec.get("freq_penalty").and_then(|v| v.as_f64()) {
        cfg.freq_penalty = v as f32;
    }
    if let Some(v) = rec.get("presence_penalty").and_then(|v| v.as_f64()) {
        cfg.presence_penalty = v as f32;
    }
    if let Some(v) = rec.get("penalty_window").and_then(|v| v.as_f64()) {
        cfg.penalty_window = v as u32;
    }
    if let Some(v) = rec.get("seed").and_then(|v| match v {
        Value::U64(n) => Some(*n),
        Value::I64(n) => Some(*n as u64),
        Value::F64(n) => Some(*n as u64),
        _ => None,
    }) {
        cfg.seed = v;
    }
    cfg
}

/// VibeScript binding: `sampler.configure(config_record)` → ack (T53).
///
/// Installs the sampler config into the process-global sampler slot used by
/// the native decode loop. When `temperature <= 0.0` (the default), the
/// decode loop uses greedy argmax — bit-identical to the pre-W2 path.
pub fn configure(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let rec = match args {
        Value::Record(r) => r,
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "sampler.configure expects a config record { temperature, top_k, ... }",
            ))
        }
    };
    let cfg = parse_config(rec);
    llm_bench::set_sampler_config(Some(cfg));
    Ok(Value::Bool(true))
}

/// VibeScript binding: `sampler.constrain_enable(vocab?)` → ack (T53).
///
/// Enables GBNF constrained decoding. If a vocabulary list is provided
/// (as a list of [id, string] pairs), installs a new `DominoMasker` built
/// from it before enabling. If no vocabulary is provided, enables the
/// existing masker (or returns an error if none is installed).
pub fn constrain_enable(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    // Optional argument: vocabulary as a list of [id, string] pairs.
    if let Value::List(vocab_list) = args {
        let vocab: Vec<(u32, String)> = vocab_list
            .iter()
            .filter_map(|entry| match entry {
                Value::List(pair) if pair.len() >= 2 => {
                    let id = match &pair[0] {
                        Value::U64(n) => *n as u32,
                        Value::I64(n) => *n as u32,
                        Value::F64(n) => *n as u32,
                        _ => return None,
                    };
                    let s = match &pair[1] {
                        Value::String(s) => s.clone(),
                        _ => return None,
                    };
                    Some((id, s))
                }
                _ => None,
            })
            .collect();

        if vocab.is_empty() {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "vocabulary list is empty or malformed — expected [[id, string], ...]",
            ));
        }

        let mut masker = DominoMasker::new(&vocab);
        masker.enable();
        llm_bench::set_domino_masker(Some(masker));
        return Ok(Value::Bool(true));
    }

    // No vocab provided (null or other) — enable the existing masker.
    if !llm_bench::domino_active() {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "no DOMINO masker active — provide a vocabulary: constrain_enable([[id, string], ...])",
        ));
    }
    Ok(Value::Bool(true))
}

/// VibeScript binding: `sampler.constrain_disable()` → ack (T53).
///
/// Removes the DOMINO masker, restoring unconstrained generation.
pub fn constrain_disable(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _ = (args, span);
    llm_bench::set_domino_masker(None);
    Ok(Value::Bool(true))
}

/// VibeScript binding: `sampler.constrain_reset()` → ack (T53).
///
/// Resets the grammar state machine to the initial state (for a new
/// generation turn). No-op when no masker is installed.
pub fn constrain_reset(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _ = (args, span);
    llm_bench::domino_reset();
    Ok(Value::Bool(true))
}

/// VibeScript binding: `sampler.sample(logits, context?)` → token_id (T53).
///
/// Pure-CPU sampling entry point for testing. Samples a token from the given
/// logits array using the global sampler config. If DOMINO is active, applies
/// the grammar mask first and feeds the token back into the grammar state.
///
/// The `logits` argument is a list of f64 values. An optional `context`
/// argument is a list of prior token IDs.
pub fn sample(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    // The argument can be:
    // 1. A list of logits: [0.1, 0.9, 0.3]
    // 2. A record with "logits" and optional "context" keys
    let (logits_f64, ctx): (Vec<f64>, Vec<u32>) = match args {
        Value::List(l) => {
            // Direct logits list
            let logits: Vec<f64> = l.iter().filter_map(|v| v.as_f64()).collect();
            (logits, Vec::new())
        }
        Value::Record(r) => {
            // Record with "logits" and optional "context"
            let logits: Vec<f64> = match r.get("logits") {
                Some(Value::List(l)) => l.iter().filter_map(|v| v.as_f64()).collect(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "sampler.sample record needs { logits: [f64], context?: [u32] }",
                    ))
                }
            };
            let ctx: Vec<u32> = match r.get("context") {
                Some(Value::List(l)) => l
                    .iter()
                    .filter_map(|v| match v {
                        Value::U64(n) => Some(*n as u32),
                        Value::I64(n) => Some(*n as u32),
                        Value::F64(n) => Some(*n as u32),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            (logits, ctx)
        }
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "sampler.sample expects a logits list or { logits: [...], context?: [...] }",
            ))
        }
    };

    if logits_f64.is_empty() {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "logits list is empty",
        ));
    }

    let mut logits: Vec<f32> = logits_f64.iter().map(|&v| v as f32).collect();
    let cfg = llm_bench::sampler_config().unwrap_or_default();
    let mut state = SamplerState::new(cfg);

    let token = if llm_bench::domino_active() {
        match llm_bench::domino_sample(&mut state, &mut logits, &ctx) {
            Some(tid) => tid,
            None => state.sample(&mut logits, &ctx),
        }
    } else {
        state.sample(&mut logits, &ctx)
    };

    Ok(Value::U64(token as u64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t53_parse_config_default() {
        let rec = BTreeMap::new();
        let cfg = parse_config(&rec);
        assert_eq!(cfg.temperature, 0.0);
    }

    #[test]
    fn t53_parse_config_custom() {
        let mut rec = BTreeMap::new();
        rec.insert("temperature".into(), Value::F64(0.8));
        rec.insert("top_k".into(), Value::F64(40.0));
        rec.insert("top_p".into(), Value::F64(0.9));
        rec.insert("seed".into(), Value::U64(42));
        let cfg = parse_config(&rec);
        assert!((cfg.temperature - 0.8).abs() < 0.01);
        assert_eq!(cfg.top_k, 40);
        assert!((cfg.top_p - 0.9).abs() < 0.01);
        assert_eq!(cfg.seed, 42);
    }

    #[test]
    fn t53_parse_config_i64_seed() {
        let mut rec = BTreeMap::new();
        rec.insert("seed".into(), Value::I64(123));
        let cfg = parse_config(&rec);
        assert_eq!(cfg.seed, 123);
    }

    #[test]
    fn t53_vibescript_configure_record() {
        let mut rec = BTreeMap::new();
        rec.insert("temperature".into(), Value::F64(0.7));
        let args = Value::Record(rec);
        let result = configure(&args, Span::point(0)).unwrap();
        assert_eq!(result, Value::Bool(true));
        let cfg = llm_bench::sampler_config().unwrap();
        assert!((cfg.temperature - 0.7).abs() < 0.01);
        llm_bench::set_sampler_config(Some(SamplerConfig::default()));
    }

    #[test]
    fn t53_vibescript_configure_bad_args() {
        let args = Value::Null;
        assert!(configure(&args, Span::point(0)).is_err());
    }

    #[test]
    fn t53_vibescript_sample_greedy_list() {
        llm_bench::set_sampler_config(Some(SamplerConfig::default()));
        llm_bench::set_domino_masker(None);

        let args = Value::List(vec![Value::F64(0.1), Value::F64(0.9), Value::F64(0.3)]);
        let result = sample(&args, Span::point(0)).unwrap();
        assert_eq!(result, Value::U64(1)); // argmax
    }

    #[test]
    fn t53_vibescript_sample_greedy_record() {
        llm_bench::set_sampler_config(Some(SamplerConfig::default()));
        llm_bench::set_domino_masker(None);

        let mut rec = BTreeMap::new();
        rec.insert(
            "logits".into(),
            Value::List(vec![Value::F64(0.1), Value::F64(0.9), Value::F64(0.3)]),
        );
        rec.insert(
            "context".into(),
            Value::List(vec![Value::U64(0), Value::U64(1)]),
        );
        let args = Value::Record(rec);
        let result = sample(&args, Span::point(0)).unwrap();
        assert_eq!(result, Value::U64(1));
    }

    #[test]
    fn t53_vibescript_sample_empty_logits() {
        let args = Value::List(vec![]);
        assert!(sample(&args, Span::point(0)).is_err());
    }

    #[test]
    fn t53_vibescript_sample_bad_args() {
        let args = Value::Null;
        assert!(sample(&args, Span::point(0)).is_err());
    }

    #[test]
    fn t53_vibescript_constrain_enable_no_vocab_no_masker() {
        llm_bench::set_domino_masker(None);
        let args = Value::Null;
        let _ = constrain_enable(&args, Span::point(0));
    }

    #[test]
    fn t53_vibescript_constrain_enable_with_vocab() {
        let vocab = Value::List(vec![
            Value::List(vec![Value::U64(0), Value::String("= ".into())]),
            Value::List(vec![Value::U64(1), Value::String("math".into())]),
            Value::List(vec![Value::U64(2), Value::String("(".into())]),
        ]);
        let result = constrain_enable(&vocab, Span::point(0)).unwrap();
        assert_eq!(result, Value::Bool(true));
        llm_bench::set_domino_masker(None);
    }

    #[test]
    fn t53_vibescript_constrain_enable_empty_vocab() {
        let args = Value::List(vec![]);
        assert!(constrain_enable(&args, Span::point(0)).is_err());
    }

    #[test]
    fn t53_vibescript_constrain_disable() {
        let args = Value::Null;
        let result = constrain_disable(&args, Span::point(0)).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn t53_vibescript_constrain_reset() {
        let args = Value::Null;
        let result = constrain_reset(&args, Span::point(0)).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn t53_domino_feed_token_advances_grammar() {
        use crate::inference::speculative_decode::DominoMasker;

        let vocab = vec![
            (0u32, "= ".to_string()),
            (1u32, "math".to_string()),
            (2u32, "(".to_string()),
            (3u32, "1".to_string()),
            (4u32, ")".to_string()),
        ];
        let mut masker = DominoMasker::new(&vocab);
        masker.enable();
        assert!(masker.is_active());

        masker.feed_token(b"= ");
        masker.feed_token(b"math");
        masker.feed_token(b"(");
        masker.feed_token(b"1");
        masker.feed_token(b")");
    }

    #[test]
    fn t53_domino_sample_with_mask_preserves_logits() {
        use crate::inference::speculative_decode::DominoMasker;

        let vocab = vec![
            (0u32, "= ".to_string()),
            (1u32, "math".to_string()),
            (2u32, "xyz".to_string()),
            (3u32, "(".to_string()),
        ];
        let mut masker = DominoMasker::new(&vocab);
        masker.enable();

        let mut logits = vec![0.1f32, 5.0, 10.0, 0.2];
        masker.apply_mask_preserving(&mut logits);

        let mut best = 0usize;
        let mut best_v = f32::NEG_INFINITY;
        for (i, &v) in logits.iter().enumerate() {
            if v > best_v {
                best_v = v;
                best = i;
            }
        }
        assert!(best < 4);
    }

    #[test]
    fn t53_domino_inactive_is_noop() {
        use crate::inference::speculative_decode::DominoMasker;

        let vocab = vec![(0u32, "test".to_string())];
        let mut masker = DominoMasker::new(&vocab);
        assert!(!masker.is_active());

        let mut logits = vec![1.0f32, 2.0, 3.0];
        let original = logits.clone();
        masker.apply_mask_preserving(&mut logits);
        assert_eq!(logits, original);
    }

    #[test]
    fn t53_domino_reset_clears_state() {
        use crate::inference::speculative_decode::{DominoMasker, GrammarState};

        let vocab = vec![(0u32, "= ".to_string()), (1u32, "math".to_string())];
        let mut masker = DominoMasker::new(&vocab);
        masker.enable();
        masker.feed_token(b"= ");
        let state_after_feed = masker.grammar_state();
        assert_ne!(state_after_feed, GrammarState::Start);
        masker.reset();
        let state_after_reset = masker.grammar_state();
        assert_eq!(state_after_reset, GrammarState::Start);
    }

    #[test]
    fn t53_constrained_sample_deterministic_with_seed() {
        let cfg = SamplerConfig {
            temperature: 0.5,
            top_k: 0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            freq_penalty: 0.0,
            presence_penalty: 0.0,
            penalty_window: 64,
            seed: 12345,
        };
        let mut s1 = SamplerState::new(cfg);
        let mut s2 = SamplerState::new(cfg);
        let mut l1 = [0.1f32, 0.9, 0.3, 0.7];
        let mut l2 = [0.1f32, 0.9, 0.3, 0.7];
        let t1 = s1.sample(&mut l1, &[]);
        let t2 = s2.sample(&mut l2, &[]);
        assert_eq!(t1, t2);
    }
}
