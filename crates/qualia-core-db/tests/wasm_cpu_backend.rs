use std::path::PathBuf;
use std::sync::Arc;

use qualia_core_db::gguf_bridge::wasm_cpu::CpuWasmEngine;

/// Manual real-weight gate. Run with:
/// cargo test -p qualia-core-db --release --test wasm_cpu_backend -- --ignored --nocapture
#[test]
#[ignore = "reads the 386 MB SmolLM2 fixture and executes a full CPU transformer token"]
fn real_smollm2_cpu_backend_executes_without_a_gpu() {
    let model_path = std::env::var_os("QUALIA_WASM_CPU_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("models/smollm2-360m-instruct-q8_0.gguf")
        });
    let bytes = std::fs::read(&model_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", model_path.display()));
    let mut engine = CpuWasmEngine::new_with_context(Arc::<[u8]>::from(bytes), 1024)
        .expect("prepare CPU-WASM model");
    assert_eq!(engine.vocab_len(), 49_152);
    assert_eq!(engine.max_context(), 1024);
    assert!(engine.working_set_bytes() > 42 * 1024 * 1024);

    let tokens = engine.tokenizer().encode_prompt("Paris is the capital of");
    let current = *tokens.last().expect("prompt token");
    engine.reset();
    let step = engine.run_token(current, 0).expect("CPU-WASM decode token");
    assert!(step.token_id < engine.vocab_len());
    assert!(step.max_logit.is_finite());
    eprintln!(
        "CPU_WASM_OK token={} logit={} working_set_mib={:.1}",
        step.token_id,
        step.max_logit,
        engine.working_set_bytes() as f64 / (1024.0 * 1024.0)
    );
}
