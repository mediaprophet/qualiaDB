//! Bounded adapters for POET's P2 infrastructure inspector panels.

use super::super::args;
use super::super::render;
use crate::inference::gguf_sharder::GgufTokenizer;
use crate::inference::orchestrator::ModelLifecycle;
use crate::q42::p64_weight::{
    has_p64_magic, P64_DEFAULT_PAGE_LOG2, P64_MAGIC, P64_TENSOR_ENTRY_BYTES, P64_VERSION,
    P64_WEIGHT_HEADER_BYTES,
};
use crate::query::mini_parser::{self, compile_ntriples_to_bytecode};
use crate::solvers::linear_algebra::gemm::matmul;
use crate::specialized_libs::linear_algebra::privacy::{
    gaussian_sigma, CompositionMethod, DifferentialPrivacy,
};
use vibe::{Diagnostic, Span, Value};

const MAX_ITEMS: usize = 64;
const SLG_ARENA_BYTES: u64 = 42 * 1024 * 1024;
const QUIN_BYTES: u64 = 48;

pub fn compute(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    match args::rec_str(args_v, "mode") {
        Some("bytecode") => bytecode(args_v, span),
        Some("slg_arena") => slg_arena(args_v, span),
        Some("forge") => forge(args_v, span),
        Some("compute_profile") => render::gpu_backend_info(args_v, span),
        Some("privacy") => privacy(args_v, span),
        Some("model_lifecycle") => lifecycle(args_v, span),
        Some("inference_monitor") => Err(args::bad(
            span,
            "Inference telemetry requires an active LocalLlmAgent decode session on this daemon",
        )),
        Some("gguf_tokenizer") => tokenizer(args_v, span),
        Some("p64") => p64(args_v, span),
        _ => Err(args::bad(
            span,
            "InfraLogic.compute needs a supported `mode`",
        )),
    }
}

fn bytecode(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let source = args::rec_str(args_v, "source").unwrap_or("");
    if source.trim().is_empty() {
        return Err(args::bad(
            span,
            "bytecode inspection needs an N-Triples pattern in `source`",
        ));
    }
    let mut program = [0u8; 1024];
    let len = compile_ntriples_to_bytecode(source.as_bytes(), &mut program)
        .map_err(|error| args::bad(span, format!("bytecode compile failed: {error:?}")))?;
    let mut matches = 0u64;
    let mut jumps = 0u64;
    let mut names = Vec::new();
    let mut i = 0usize;
    while i < len {
        let opcode = program[i];
        let (name, width) = match opcode {
            mini_parser::OP_END => ("OP_END", 1),
            mini_parser::OP_MATCH_SUBJECT => ("OP_MATCH_SUBJECT", 9),
            mini_parser::OP_MATCH_PREDICATE => ("OP_MATCH_PREDICATE", 9),
            mini_parser::OP_MATCH_OBJECT => ("OP_MATCH_OBJECT", 9),
            mini_parser::OP_HALT_IF_FALSE => ("OP_HALT_IF_FALSE", 1),
            mini_parser::OP_EVAL_PERMIT => ("OP_EVAL_PERMIT", 1),
            mini_parser::OP_EVAL_OBLIGATE => ("OP_EVAL_OBLIGATE", 1),
            mini_parser::OP_EVAL_FORBID => ("OP_EVAL_FORBID", 1),
            mini_parser::OP_HALT_VIOLATION => ("OP_HALT_VIOLATION", 1),
            other => {
                names.push(Value::String(format!("unknown:{other:#04x}")));
                i += 1;
                continue;
            }
        };
        if name.starts_with("OP_MATCH") {
            matches += 1;
        }
        if name.contains("HALT") {
            jumps += 1;
        }
        names.push(Value::String(name.into()));
        i += width;
    }
    Ok(args::record([
        ("bytes", Value::U64(len as u64)),
        ("instructions", Value::List(names)),
        ("matches", Value::U64(matches)),
        ("halts", Value::U64(jumps)),
        (
            "stats_only",
            Value::Bool(args::rec_str(args_v, "operation") == Some("stats")),
        ),
    ]))
}

fn slg_arena(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let max_slots = SLG_ARENA_BYTES / QUIN_BYTES;
    let used = args::rec_u64(args_v, "used_slots").unwrap_or(0);
    if used > max_slots {
        return Err(args::bad(
            span,
            format!("used_slots exceeds the 42MB Sentinel ceiling ({max_slots} Quin slots)"),
        ));
    }
    Ok(args::record([
        ("arena_bytes", Value::U64(SLG_ARENA_BYTES)),
        ("quin_bytes", Value::U64(QUIN_BYTES)),
        ("max_slots", Value::U64(max_slots)),
        ("used_slots", Value::U64(used)),
        ("occupancy", Value::F64(used as f64 / max_slots as f64)),
        ("recent_slot_ring", Value::U64(512)),
        ("max_fixpoint_rounds", Value::U64(16)),
    ]))
}

fn forge(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    match args::rec_str(args_v, "operation").unwrap_or("top_k") {
        "top_k" => {
            let mut values = args::rec_f64_list(args_v, "values")
                .ok_or_else(|| args::bad(span, "top_k needs `values`"))?;
            if values.is_empty() || values.len() > MAX_ITEMS {
                return Err(args::bad(span, "values must contain 1..=64 finite numbers"));
            }
            if !values.iter().all(|value| value.is_finite()) {
                return Err(args::bad(span, "values must be finite"));
            }
            let k = args::rec_u64(args_v, "k").unwrap_or(values.len() as u64) as usize;
            values.sort_by(|a, b| b.total_cmp(a));
            values.truncate(k.max(1).min(values.len()));
            Ok(args::record([
                ("k", Value::U64(values.len() as u64)),
                ("values", args::f64_list_value(values)),
            ]))
        }
        "gemm" => {
            let a = args::rec_f64_list(args_v, "a")
                .ok_or_else(|| args::bad(span, "gemm needs matrix `a`"))?;
            let b = args::rec_f64_list(args_v, "b")
                .ok_or_else(|| args::bad(span, "gemm needs matrix `b`"))?;
            let m = args::rec_u64(args_v, "m").unwrap_or(0) as usize;
            let k = args::rec_u64(args_v, "k_dim")
                .or_else(|| args::rec_u64(args_v, "k"))
                .unwrap_or(0) as usize;
            let n = args::rec_u64(args_v, "n").unwrap_or(0) as usize;
            if m == 0 || k == 0 || n == 0 || m * k != a.len() || k * n != b.len() {
                return Err(args::bad(span, "gemm needs m,k,n matching a and b lengths"));
            }
            if m > 32 || n > 32 || k > 32 {
                return Err(args::bad(span, "gemm dimensions exceed 32"));
            }
            let mut c = vec![0.0; m * n];
            matmul(m, k, n, &a, &b, &mut c)
                .map_err(|error| args::bad(span, format!("gemm failed: {error:?}")))?;
            Ok(args::record([
                ("rows", Value::U64(m as u64)),
                ("cols", Value::U64(n as u64)),
                ("data", args::f64_list_value(c)),
            ]))
        }
        "fft" => {
            let samples = args::rec_f64_list(args_v, "samples")
                .ok_or_else(|| args::bad(span, "fft needs `samples`"))?;
            if samples.len() < 2
                || samples.len() > MAX_ITEMS
                || !samples.len().is_power_of_two()
                || !samples.iter().all(|value| value.is_finite())
            {
                return Err(args::bad(
                    span,
                    "fft samples must be 2..=64 finite power-of-two reals",
                ));
            }
            #[cfg(feature = "wgsl-forge")]
            {
                let interleaved: Vec<f32> = samples
                    .iter()
                    .flat_map(|value| [*value as f32, 0.0])
                    .collect();
                let spectrum = crate::wgsl_forge::dispatch::fft_f32(&interleaved)
                    .map_err(|error| args::bad(span, format!("fft failed: {error:?}")))?;
                return Ok(args::record([
                    ("n", Value::U64(samples.len() as u64)),
                    (
                        "spectrum",
                        args::f64_list_value(spectrum.into_iter().map(f64::from)),
                    ),
                ]));
            }
            #[cfg(not(feature = "wgsl-forge"))]
            Err(args::bad(
                span,
                "FFT requires the native WGSL Forge CPU/GPU floor",
            ))
        }
        "roofline" => {
            let flops = need_f64(args_v, "flops", span)?;
            let bytes = need_f64(args_v, "bytes", span)?;
            if bytes <= 0.0 {
                return Err(args::bad(span, "roofline `bytes` must be positive"));
            }
            Ok(args::record([(
                "operational_intensity",
                Value::F64(flops / bytes),
            )]))
        }
        "certify" | "autotune" | "validate" => Err(args::bad(
            span,
            "Kernel certification, auto-tune, and Naga validation require a live WGSL Forge GPU session",
        )),
        other => Err(args::bad(span, format!("unknown forge operation `{other}`"))),
    }
}

fn privacy(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    match args::rec_str(args_v, "operation").unwrap_or("dp_accounting") {
        "dp_laplace" | "dp_gaussian" => {
            let values = args::rec_f64_list(args_v, "plaintext")
                .ok_or_else(|| args::bad(span, "DP release needs `plaintext`"))?;
            if values.is_empty() || values.len() > MAX_ITEMS {
                return Err(args::bad(span, "plaintext must contain 1..=64 values"));
            }
            let sensitivity = need_f64(args_v, "sensitivity", span)?;
            let epsilon = need_f64(args_v, "epsilon", span)?;
            let mut engine = DifferentialPrivacy::with_budget(
                args::rec_f64(args_v, "budget_epsilon").unwrap_or(1.0),
                args::rec_f64(args_v, "budget_delta").unwrap_or(1e-6),
                CompositionMethod::BasicComposition,
            )
            .map_err(|error| args::bad(span, format!("privacy budget: {error:?}")))?;
            let mut out = vec![0.0; values.len()];
            let count = if args::rec_str(args_v, "operation") == Some("dp_gaussian") {
                engine
                    .release_gaussian_into(
                        &values,
                        sensitivity,
                        epsilon,
                        args::rec_f64(args_v, "delta").unwrap_or(1e-5),
                        &mut out,
                    )
                    .map_err(|error| args::bad(span, format!("gaussian release: {error:?}")))?
            } else {
                engine
                    .release_laplace_into(&values, sensitivity, epsilon, &mut out)
                    .map_err(|error| args::bad(span, format!("laplace release: {error:?}")))?
            };
            Ok(args::record([
                ("released", args::f64_list_value(out)),
                ("count", Value::U64(count as u64)),
                (
                    "epsilon_spent",
                    Value::F64(engine.privacy_accountant.total_epsilon_spent),
                ),
            ]))
        }
        "dp_accounting" => {
            let sigma = gaussian_sigma(
                need_f64(args_v, "sensitivity", span)?,
                need_f64(args_v, "epsilon", span)?,
                args::rec_f64(args_v, "delta").unwrap_or(1e-5),
            )
            .map_err(|error| args::bad(span, format!("gaussian_sigma: {error:?}")))?;
            Ok(args::record([("gaussian_sigma", Value::F64(sigma))]))
        }
        "secure_agg" => {
            let participants = args::rec_u64(args_v, "participants").unwrap_or(0) as usize;
            let threshold = args::rec_u64(args_v, "threshold").unwrap_or(0) as usize;
            Ok(args::record([(
                "quorum_met",
                Value::Bool(crate::modalities::carrier::multisig_satisfied(
                    participants,
                    threshold,
                )),
            )]))
        }
        "bfv_encrypt" | "bfv_decrypt" | "bfv_add" | "bfv_mul" | "key_rotate" => Err(args::bad(
            span,
            "BFV ciphertext operations require a generated BfvEngine session; this panel evaluates DP releases, Gaussian sigma, and aggregation quorum without synthesizing ciphertext",
        )),
        other => Err(args::bad(
            span,
            format!("unknown privacy operation `{other}`"),
        )),
    }
}

fn lifecycle(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let current = parse_lifecycle(args::rec_str(args_v, "state").unwrap_or("discovered"), span)?;
    let action = args::rec_str(args_v, "action").unwrap_or("status");
    let next = match (current, action) {
        (_, "status") => current,
        (ModelLifecycle::Discovered, "map") => ModelLifecycle::MappedToDisk,
        (ModelLifecycle::MappedToDisk, "stream") => ModelLifecycle::StreamingVRAM,
        (ModelLifecycle::StreamingVRAM, "activate") => ModelLifecycle::Active,
        (ModelLifecycle::Active, "evict" | "scrub") => ModelLifecycle::Scrubbing,
        (ModelLifecycle::Scrubbing, "complete") => ModelLifecycle::Discovered,
        (state, other) => {
            return Err(args::bad(
                span,
                format!("lifecycle `{state:?}` cannot `{other}`"),
            ))
        }
    };
    Ok(args::record([
        (
            "model",
            Value::String(args::rec_str(args_v, "model").unwrap_or_default().into()),
        ),
        ("state", Value::String(format!("{next:?}"))),
        ("from", Value::String(format!("{current:?}"))),
        ("action", Value::String(action.into())),
    ]))
}

fn tokenizer(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let text = args::rec_str(args_v, "text").unwrap_or("");
    if text.is_empty() {
        return Err(args::bad(span, "tokenizer inspection needs `text`"));
    }
    if text.len() > 1024 {
        return Err(args::bad(span, "tokenizer text exceeds 1024 bytes"));
    }
    let tokenizer = GgufTokenizer::default();
    let ids = tokenizer.encode(text);
    Ok(args::record([
        ("vocab_size", Value::U64(tokenizer.vocab.len() as u64)),
        ("family", Value::String("byte-level-default".into())),
        ("bos_token_id", Value::U64(tokenizer.bos_token_id as u64)),
        ("eos_token_id", Value::U64(tokenizer.eos_token_id as u64)),
        (
            "ids",
            Value::List(ids.into_iter().map(|id| Value::U64(id as u64)).collect()),
        ),
        (
            "note",
            Value::String(
                "No GGUF file was supplied; this is the engine's 256-entry byte-level fallback tokenizer."
                    .into(),
            ),
        ),
    ]))
}

fn p64(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let hex = args::rec_str(args_v, "bytes").unwrap_or("");
    let mut payload = Vec::new();
    if !hex.trim().is_empty() {
        payload = decode_hex(hex, span)?;
    }
    Ok(args::record([
        (
            "magic",
            Value::String(P64_MAGIC.iter().map(|b| format!("{b:02x}")).collect()),
        ),
        ("version", Value::U64(P64_VERSION as u64)),
        ("page_log2", Value::U64(P64_DEFAULT_PAGE_LOG2 as u64)),
        ("header_bytes", Value::U64(P64_WEIGHT_HEADER_BYTES as u64)),
        (
            "tensor_entry_bytes",
            Value::U64(P64_TENSOR_ENTRY_BYTES as u64),
        ),
        ("has_magic", Value::Bool(has_p64_magic(&payload))),
        ("payload_bytes", Value::U64(payload.len() as u64)),
    ]))
}

fn parse_lifecycle(state: &str, span: Span) -> Result<ModelLifecycle, Diagnostic> {
    Ok(match state.to_ascii_lowercase().as_str() {
        "discovered" => ModelLifecycle::Discovered,
        "mappedtodisk" | "mapped_to_disk" => ModelLifecycle::MappedToDisk,
        "streamingvram" | "streaming_vram" => ModelLifecycle::StreamingVRAM,
        "active" => ModelLifecycle::Active,
        "scrubbing" => ModelLifecycle::Scrubbing,
        other => {
            return Err(args::bad(
                span,
                format!("unknown lifecycle state `{other}`"),
            ))
        }
    })
}

fn need_f64(args_v: &Value, key: &str, span: Span) -> Result<f64, Diagnostic> {
    let value =
        args::rec_f64(args_v, key).ok_or_else(|| args::bad(span, format!("needs `{key}`")))?;
    value
        .is_finite()
        .then_some(value)
        .ok_or_else(|| args::bad(span, format!("`{key}` must be finite")))
}

fn decode_hex(hex: &str, span: Span) -> Result<Vec<u8>, Diagnostic> {
    let cleaned: String = hex.chars().filter(|ch| !ch.is_whitespace()).collect();
    if cleaned.len() % 2 != 0 || cleaned.len() > 512 {
        return Err(args::bad(
            span,
            "P64 bytes must be even-length hex up to 256 bytes",
        ));
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&cleaned[index..index + 2], 16)
                .map_err(|_| args::bad(span, "P64 bytes contain non-hex"))
        })
        .collect()
}

#[cfg(test)]
#[path = "infra_workbench_tests.rs"]
mod tests;
