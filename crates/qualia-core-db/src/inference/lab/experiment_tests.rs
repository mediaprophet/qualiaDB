use super::*;
use crate::inference::lab::config_space::{Configuration, ConfigurationSpace, ParameterDef};
use crate::inference::runtime::receipt::execution::COUNTER_TIME_TO_FIRST_TOKEN_US;
use crate::inference_bench::BenchConfig;

#[test]
fn quality_score_from_verified() {
    use crate::post_turn_verify::{VerifiedTurn, VerifyCheck};
    let v = VerifiedTurn {
        final_text: "Paris".into(),
        display_html: String::new(),
        cml_turtle: String::new(),
        repaired: false,
        checks: vec![
            VerifyCheck {
                id: "a".into(),
                ok: true,
                detail: "ok".into(),
            },
            VerifyCheck {
                id: "b".into(),
                ok: true,
                detail: "ok".into(),
            },
        ],
        grounding_reason: None,
    };
    let qs = quality_from_verified(&v);
    assert_eq!(qs.pass_rate, 1.0);
    assert_eq!(qs.total_checks, 2);
    assert!(!qs.repaired);
}

#[test]
fn quality_score_with_repair() {
    use crate::post_turn_verify::{VerifiedTurn, VerifyCheck};
    let v = VerifiedTurn {
        final_text: "Paris".into(),
        display_html: String::new(),
        cml_turtle: String::new(),
        repaired: true,
        checks: vec![
            VerifyCheck {
                id: "a".into(),
                ok: true,
                detail: "ok".into(),
            },
            VerifyCheck {
                id: "b".into(),
                ok: false,
                detail: "fail".into(),
            },
        ],
        grounding_reason: Some("capital".into()),
    };
    let qs = quality_from_verified(&v);
    assert_eq!(qs.pass_rate, 0.5);
    assert!(qs.repaired);
    assert!((qs.composite() - 0.45).abs() < 1e-9);
}

#[test]
fn experiment_config_builds_bench_config() {
    let space = ConfigurationSpace::new("test").with("gpu_topk", ParameterDef::Bool);
    let cfg = space.build_from_normalized(&[1.0]);
    let ec = ExperimentConfig {
        space,
        config: cfg,
        model_path: "/dev/null".into(),
        quantization: "Q8_0".into(),
        prompt: "Hello".into(),
        decode_tokens: 16,
        warm_repeats: 1,
        seed: 42,
        hypothesis_id: Some("H-001".into()),
        evaluation_mode: EvaluationMode::MeasuredLiveBackend,
    };
    let bc = ec.to_bench_config();
    assert_eq!(bc.quantization, "Q8_0");
    assert_eq!(bc.decode_tokens, 16);
    assert!(bc.label.contains("test_"));
}

#[test]
fn jsonl_log_roundtrip() {
    let scratch = tempfile::tempdir().unwrap();
    let tmp = scratch.path().join("experiment.jsonl");
    let result = ExperimentResult {
        config_hash: 42,
        hypothesis_id: Some("H-001".into()),
        evaluation_mode: EvaluationMode::MeasuredLiveBackend,
        receipt: None,
        bench: None,
        phase: PhaseSnapshotSerde::default(),
        quality: QualityScore::default(),
        thermal: ThermalSnapshot::default(),
        vram_used: 0,
        config_cbor: vec![0xA1, 0x01],
        timestamp_ms: 12345,
        seed: 99,
        error: Some("test".into()),
        evidence: Default::default(),
    };
    append_experiment_jsonl(&tmp, &result).unwrap();
    let loaded = load_experiment_log(&tmp).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].config_hash, 42);
    assert_eq!(loaded[0].seed, 99);
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn experiment_result_schema3_receipt_roundtrip() {
    let mut receipt = ExecutionReceipt::new(
        BackendKind::WgpuVulkan,
        BackendKind::WgpuVulkan,
        "mod-1",
        "plan-1",
    );
    receipt.counters.decode_steps = 32;
    receipt.mark_measured(COUNTER_DECODE_STEPS);
    receipt.latency.time_to_first_token_us = 45000;
    receipt.mark_measured(COUNTER_TIME_TO_FIRST_TOKEN_US);

    let result = ExperimentResult {
        config_hash: 101,
        hypothesis_id: Some("H-002".into()),
        evaluation_mode: EvaluationMode::MeasuredLiveBackend,
        receipt: Some(receipt),
        bench: None,
        phase: PhaseSnapshotSerde::default(),
        quality: QualityScore::default(),
        thermal: ThermalSnapshot::default(),
        vram_used: 1024 * 1024 * 512,
        config_cbor: vec![],
        timestamp_ms: 1000,
        seed: 7,
        error: None,
        evidence: Default::default(),
    };

    let json = serde_json::to_string(&result).unwrap();
    let de: ExperimentResult = serde_json::from_str(&json).unwrap();
    assert_eq!(de.evaluation_mode, EvaluationMode::MeasuredLiveBackend);
    let rc = de.receipt.expect("receipt preserved");
    assert_eq!(rc.schema_version, 3);
    assert!(rc.is_measured(COUNTER_DECODE_STEPS));
    assert_eq!(rc.counters.decode_steps, 32);
    assert_eq!(rc.latency.time_to_first_token_us, 45000);
}

#[test]
fn test_experiment_apply_toggles_consumes_all_parameters() {
    use super::super::config_space::ParameterValue;
    use crate::inference_bench::{
        kv_pool_budget_mb, prefill_chunk_size_override, prefix_cache_enabled, prompt_precision_mode,
    };

    let mut values = std::collections::BTreeMap::new();
    values.insert(
        "prefix_cache_enabled".to_string(),
        ParameterValue::Bool(false),
    );
    values.insert("kv_pool_budget_mb".to_string(), ParameterValue::Int(512));
    values.insert("prefill_chunk_size".to_string(), ParameterValue::Int(128));
    values.insert(
        "prompt_precision_mode".to_string(),
        ParameterValue::String("strict_evidence".to_string()),
    );

    let cfg = ExperimentConfig {
        space: ConfigurationSpace::new("test"),
        config: Configuration {
            space_name: "test".to_string(),
            values,
        },
        model_path: "test".to_string(),
        quantization: "F16".to_string(),
        prompt: "hello".to_string(),
        decode_tokens: 16,
        warm_repeats: 0,
        seed: 42,
        hypothesis_id: None,
        evaluation_mode: EvaluationMode::MeasuredLiveBackend,
    };

    cfg.apply_toggles();

    assert!(!prefix_cache_enabled());
    assert_eq!(kv_pool_budget_mb(), 512);
    assert_eq!(prefill_chunk_size_override(), 128);
    assert_eq!(prompt_precision_mode(), "strict_evidence");

    // Clean up / reset
    crate::inference_bench::set_prefix_cache_enabled(true);
    crate::inference_bench::set_kv_pool_budget_mb(0);
    crate::inference_bench::set_prefill_chunk_size_override(0);
    crate::inference_bench::set_prompt_precision_mode("");
}
