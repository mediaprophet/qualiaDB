#![cfg(feature = "inference-model")]

use etalon_model::inference::{ftw_config::FtwConfig, ARCH_QWEN2, ARCH_QWEN35_MOE};

const HYBRID: &str = r#"{"model_type":"qwen3_5_moe","text_config":{
        "model_type":"qwen3_5_moe_text","num_hidden_layers":40,"hidden_size":2048,
        "num_attention_heads":16,"num_key_value_heads":2,"head_dim":256,
        "linear_conv_kernel_dim":4,"linear_key_head_dim":128,"linear_value_head_dim":128,
        "linear_num_key_heads":16,"linear_num_value_heads":32,"full_attention_interval":4,
        "num_experts":256,"num_experts_per_tok":8,"rope_parameters":{"rope_theta":10000000}}}"#;

#[test]
fn ftw_qwen35_metadata_preserves_hybrid_geometry() {
    let c: FtwConfig = serde_json::from_str(HYBRID).unwrap();
    let h = c.hyperparams(40, 2048, true).unwrap();
    assert_eq!(h.architecture, ARCH_QWEN35_MOE);
    assert_eq!(
        (h.ssm_conv_kernel, h.ssm_state_size, h.ssm_group_count),
        (4, 128, 16)
    );
    assert_eq!(
        (
            h.ssm_time_step_rank,
            h.ssm_inner_size,
            h.full_attention_interval
        ),
        (32, 4096, 4)
    );
    assert_eq!((h.head_dim, h.rope_freq_base), (256, 10_000_000.0));
    assert!(h.decode_supported_with_override(false).is_err());
}

#[test]
fn ftw_hybrid_rejects_incomplete_inconsistent_and_unknown_metadata() {
    for bad in [
        HYBRID.replace("\"linear_key_head_dim\":128,", ""),
        HYBRID.replace("\"hidden_size\":2048", "\"hidden_size\":1024"),
        HYBRID.replace(
            "\"full_attention_interval\":4",
            "\"full_attention_interval\":0",
        ),
        HYBRID.replace("\"num_experts\":256", "\"num_experts\":512"),
        HYBRID.replace("\"num_experts_per_tok\":8", "\"num_experts_per_tok\":10"),
        HYBRID.replace(
            "\"full_attention_interval\":4",
            "\"layer_types\":[\"full_attention\"],\"full_attention_interval\":4",
        ),
        HYBRID.replace("qwen3_5_moe_text", "unrecognized"),
        HYBRID.replace(
            "\"linear_value_head_dim\":128",
            "\"linear_value_head_dim\":64",
        ),
        HYBRID.replace(
            "\"text_config\":",
            "\"linear_num_value_heads\":16,\"text_config\":",
        ),
        HYBRID.replace(
            "\"text_config\":",
            "\"rope_parameters\":{\"rope_theta\":1000},\"text_config\":",
        ),
        HYBRID.replace("\"text_config\":", "\"num_experts\":128,\"text_config\":"),
    ] {
        let c: FtwConfig = serde_json::from_str(&bad).unwrap();
        assert!(c.hyperparams(40, 2048, true).is_err());
    }
    assert!(serde_json::from_str::<FtwConfig>("{\"hidden_size\":4294967296}").is_err());
}

#[test]
fn ftw_dense_defaults_remain_compatible_but_cannot_hide_hybrid_tensors() {
    let c: FtwConfig = serde_json::from_str(r#"{"model_type":"qwen2","hidden_size":64,"num_attention_heads":8,"num_key_value_heads":2}"#).unwrap();
    let h = c.hyperparams(2, 64, false).unwrap();
    assert_eq!(
        (h.architecture, h.n_layer, h.head_dim, h.ssm_state_size),
        (ARCH_QWEN2, 2, 64, 0)
    );
    assert!(c.hyperparams(2, 64, true).is_err());
}
