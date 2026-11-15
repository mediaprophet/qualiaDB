//! Cold browser execution metadata; unmeasured transfers remain unavailable.
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Receipt {
    schema: &'static str,
    backend: String,
    vocab_size: u32,
    gpu_top1: bool,
    output_projection: &'static str,
    device_to_host_bytes_per_token: Option<u64>,
    readback_basis: &'static str,
    full_logits_bytes_per_token: u64,
    full_logits_bytes_basis: &'static str,
    cpu_working_set_bytes: u64,
    inference_memory_domain: &'static str,
    sentinel_arena_bound: bool,
    execution_mode: &'static str,
    developer_bypass: bool,
    inference_admitted: bool,
    sentinel_enabled: bool,
    governed_token_acceptance: bool,
}

pub(super) fn build() -> Result<JsValue, JsValue> {
    let backend = super::get_wasm_backend();
    let vocab_size = super::get_resident_tokenizer_vocab();
    let (output_projection, device_to_host_bytes_per_token, readback_basis) = match backend.as_str()
    {
        "webgpu" => ("gpu-gemm-cpu-streaming-argmax", None, "unmeasured"),
        "cpu-wasm" => ("cpu", Some(0), "no-device-transfers"),
        _ => ("unavailable", None, "unavailable"),
    };
    let receipt = Receipt {
        schema: "qualia.browser-execution.v2",
        backend,
        vocab_size,
        gpu_top1: false,
        output_projection,
        device_to_host_bytes_per_token,
        readback_basis,
        full_logits_bytes_per_token: u64::from(vocab_size) * 4,
        full_logits_bytes_basis: "tokenizer-vocabulary-estimate",
        cpu_working_set_bytes: super::cpu::working_set_bytes(),
        inference_memory_domain: "llm-independent",
        sentinel_arena_bound: false,
        execution_mode: super::execution_mode::name(),
        developer_bypass: super::execution_mode::developer(),
        inference_admitted: super::execution_mode::developer(),
        sentinel_enabled: false,
        governed_token_acceptance: false,
    };
    receipt
        .serialize(&serde_wasm_bindgen::Serializer::new().serialize_missing_as_null(true))
        .map_err(|error| JsValue::from_str(&error.to_string()))
}
