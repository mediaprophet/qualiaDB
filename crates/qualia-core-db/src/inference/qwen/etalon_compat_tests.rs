//! Actual consumer import/type checks; no full-model qualification.
use crate::inference::{moe, qwen};
use qualia_inference_kernel::{moe as canonical_moe, qwen as canonical_qwen, recurrent};

#[test]
fn canonical_errors_and_configuration_cross_legacy_module_and_parent_paths() {
    let canonical = canonical_moe::routing::MoeError::DimensionMismatch;
    let module: moe::routing::MoeError = canonical;
    let parent: moe::MoeError = module;
    let back: canonical_moe::routing::MoeError = parent;
    assert_eq!(back, canonical);
    assert_eq!(
        parent.to_string(),
        "Dimension mismatch in MoE tensor execution"
    );

    let canonical = canonical_moe::nvfp4::Nvfp4Error::InvalidBlockAlignment;
    let module: moe::nvfp4::Nvfp4Error = canonical;
    let parent: moe::Nvfp4Error = module;
    let back: canonical_moe::nvfp4::Nvfp4Error = parent;
    assert_eq!(back, canonical);
    assert_eq!(
        parent.to_string(),
        "Data length is not aligned to NVFP4 16-element blocks"
    );

    let canonical = recurrent::gated_deltanet::QwenStateError::BufferTooSmall;
    let module: qwen::gated_deltanet::QwenStateError = canonical;
    let parent: qwen::QwenStateError = module;
    let back: recurrent::gated_deltanet::QwenStateError = parent;
    assert_eq!(back, canonical);
    assert_eq!(parent.to_string(), "Output or state buffer is too small");

    let canonical = canonical_qwen::QwenNumericError::InvalidEpsilon;
    let module: qwen::numerics::QwenNumericError = canonical;
    let parent: qwen::QwenNumericError = module;
    let back: canonical_qwen::QwenNumericError = parent;
    assert_eq!(back, canonical);
    let config: moe::routing::MoeRoutingConfig =
        canonical_moe::routing::MoeRoutingConfig::qwen3_6_35b();
    let parent: moe::MoeRoutingConfig = config;
    let back: canonical_moe::routing::MoeRoutingConfig = parent;
    assert_eq!(back.num_routed_experts, 256);
    assert_eq!(back.num_experts_per_tok, 8);
    assert_eq!(moe::MAX_MOE_TOPK, canonical_moe::routing::MAX_MOE_TOPK);
    assert_eq!(
        moe::NVFP4_BLOCK_SIZE,
        canonical_moe::nvfp4::NVFP4_BLOCK_SIZE
    );
    assert_eq!(
        qwen::CAUSAL_CONV_KERNEL,
        recurrent::gated_deltanet::CAUSAL_CONV_KERNEL
    );
}

#[test]
fn legacy_moe_and_recurrent_imports_execute_preserved_vectors_and_partial_refusal() {
    let route: fn(
        &[f32],
        usize,
        bool,
        &mut [u16],
        &mut [f32],
    ) -> Result<usize, canonical_moe::routing::MoeError> = moe::routing::route_topk;
    let mut ids = [99; 3];
    let mut weights = [99.0; 3];
    route(&[4.0, 4.0, 1.0], 2, true, &mut ids, &mut weights).unwrap();
    assert_eq!(ids, [0, 1, 99]);
    assert_eq!(weights, [0.5, 0.5, 99.0]);
    let mut combined = [97.0; 3];
    assert_eq!(
        moe::combine_expert_outputs(&[&[2.0, 4.0], &[]], &[0.5, 0.5], 2, &mut combined),
        Err(canonical_moe::routing::MoeError::DimensionMismatch)
    );
    assert_eq!(combined, [1.0, 2.0, 97.0]);

    let mut decoded = [93.0; 18];
    let result: Result<(), canonical_moe::nvfp4::Nvfp4Error> =
        moe::nvfp4::dequantize_nvfp4_row(&[0x22; 8], &[0x38], 2.0, &mut decoded);
    result.unwrap();
    assert_eq!(&decoded[..16], &[2.0; 16]);
    assert_eq!(&decoded[16..], &[93.0; 2]);
    let mut gemv = [91.0; 2];
    moe::nvfp4_gemv_zero_heap(&[1.0; 16], &[0x22; 8], &[0x38], 2.0, 1, 16, &mut gemv).unwrap();
    assert_eq!(gemv, [32.0, 91.0]);

    let mut history = [1.0, 2.0, 3.0, 89.0];
    let mut out = [87.0; 2];
    let result: Result<(), recurrent::gated_deltanet::QwenStateError> =
        qwen::gated_deltanet::step_causal_conv1d(
            &mut history,
            &[1.0, 2.0, 3.0, 4.0],
            &[4.0],
            &mut out,
        );
    result.unwrap();
    assert_eq!(history, [2.0, 3.0, 4.0, 89.0]);
    assert_eq!(out, [30.0, 87.0]);
    let mut state = [0.0, 0.0, 83.0];
    qwen::step_gated_deltanet(
        &mut state,
        &[1.0],
        &[1.0],
        &[2.0, 4.0],
        &[0.5],
        &[0.25],
        1,
        2,
        &mut out,
    )
    .unwrap();
    assert_eq!(state, [0.5, 1.0, 83.0]);
    assert_eq!(out, [0.5, 1.0]);
}

#[test]
fn legacy_numeric_imports_preserve_error_precedence_and_float_order() {
    let rms: fn(&[f32], &[f32], f32, &mut [f32]) -> Result<(), canonical_qwen::QwenNumericError> =
        qwen::numerics::rms_norm_into;
    let mut out = [79.0; 3];
    assert_eq!(
        rms(&[1.0], &[], f32::NAN, &mut out),
        Err(canonical_qwen::QwenNumericError::BufferTooSmall)
    );
    assert_eq!(out, [79.0; 3]);
    rms(&[3.0, 4.0], &[2.0, 0.5], 3.5, &mut out).unwrap();
    assert_eq!(out, [1.5, 0.5, 79.0]);
    let input = [0.3, 4.0];
    qwen::hyper_connection::group_rms_norm_into(&input, 1, 2, &[0.1, 1.0], 1e-6, &mut out).unwrap();
    let mut inplace = [0.3, 4.0, 73.0];
    qwen::group_rms_norm_in_place(&mut inplace, 1, 2, &[0.1, 1.0], 1e-6).unwrap();
    assert_eq!(out[0].to_bits(), 0x3c2d4ab7);
    assert_eq!(inplace[0].to_bits(), 0x3c2d4ab8);
    assert_eq!(out[2], 79.0);
    assert_eq!(inplace[2], 73.0);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn actual_native_error_conversions_and_distinct_streamed_gates_remain_intact() {
    for error in [
        canonical_qwen::QwenNumericError::BufferTooSmall,
        canonical_qwen::QwenNumericError::InvalidEpsilon,
    ] {
        let ple: qwen::ple_block::PleBlockError = error.into();
        let streamed: qwen::streamed_hyper::StreamedHyperError = error.into();
        assert_eq!(ple, qwen::PleBlockError::Numeric(error));
        assert_eq!(streamed, qwen::StreamedHyperError::Numeric(error));
        assert_eq!(ple.to_string(), error.to_string());
        assert_eq!(streamed.to_string(), error.to_string());
    }
    let mut learned = [71.0; 3];
    let mut simple = learned;
    qwen::mix_hyper_streams(&[2.0, 4.0, 6.0, 8.0], &[0.0; 4], 2, 2, &mut learned).unwrap();
    qwen::mix_streams_into(&[2.0, 4.0, 6.0, 8.0], &[0.0; 4], 2, 2, &mut simple).unwrap();
    assert_eq!(learned, [2.0, 3.0, 71.0]);
    assert_eq!(simple, [0.0, 0.0, 71.0]);
    let mut learned = [1.0, 1.0, 1.0, 1.0, 67.0];
    let mut simple = learned;
    qwen::inject_hyper_update(&mut learned, &[3.0, -1.0], &[0.0; 2], 2, 2).unwrap();
    qwen::inject_stream_update(&mut simple, &[3.0, -1.0], &[0.0; 4], 2, 2).unwrap();
    assert_eq!(learned, [4.0, 0.0, 4.0, 0.0, 67.0]);
    assert_eq!(simple, [1.0, 1.0, 1.0, 1.0, 67.0]);
}
