//! Comprehensive tests for Model Mapping & Precision Optimization Pipeline.

use qualia_core_db::inference::conditioning::ConditioningRegistry;
use qualia_core_db::inference::conditioning_eval::{SplitType, TaskItem, TaskManifest};
use qualia_core_db::inference::conditioning_opt::{
    ModelFamily, ModelPrecisionOptimizer, ModelPrecisionRegistry, ModelPrecisionTarget,
    OptimizationError, StrategyGenerator,
};

#[test]
fn test_01_model_target_mapping_and_registry() {
    let mut registry = ModelPrecisionRegistry::new();
    assert!(registry.is_empty());

    let llama_target = ModelPrecisionTarget::native_gguf(
        "llama-3.2-1b-instruct",
        ModelFamily::Llama,
        131072,
        "software-dev",
    );
    assert_eq!(llama_target.family.as_str(), "llama");
    assert_eq!(
        llama_target.template_family,
        qualia_core_db::gguf_sharder::ChatFamily::Llama3
    );

    let qwen_target = ModelPrecisionTarget::native_gguf(
        "qwen-2.5-coder-7b",
        ModelFamily::Qwen,
        32768,
        "code-repair",
    );
    assert_eq!(
        qwen_target.template_family,
        qualia_core_db::gguf_sharder::ChatFamily::ChatMl
    );

    let mcp_target = ModelPrecisionTarget::mcp_endpoint(
        "claude-3-5-sonnet-mcp",
        200000,
        "general-assistant",
    );

    registry.register(llama_target);
    registry.register(qwen_target);
    registry.register(mcp_target);

    assert_eq!(registry.len(), 3);
    assert!(!registry.is_empty());

    let retrieved = registry.get("llama-3.2-1b-instruct").expect("target found");
    assert_eq!(retrieved.context_window, 131072);
    assert_eq!(retrieved.target_domain, "software-dev");
    assert!(retrieved.active_profile_id.is_none());

    // Associate active profile
    let res = registry.set_active_profile(
        "llama-3.2-1b-instruct",
        "urn:qualia:profile:model:llama-3.2-1b-instruct:v1",
    );
    assert!(res.is_ok());
    assert_eq!(
        registry.get("llama-3.2-1b-instruct").unwrap().active_profile_id.as_deref(),
        Some("urn:qualia:profile:model:llama-3.2-1b-instruct:v1")
    );
}

#[test]
fn test_02_strategy_generator_b0_through_b5() {
    let target = ModelPrecisionTarget::native_gguf(
        "phi-3.5-mini-instruct",
        ModelFamily::Phi,
        131072,
        "clinical-guidance",
    );

    let candidates = StrategyGenerator::generate_candidates(&target, "Clinical diagnostic precision");
    assert_eq!(candidates.len(), 6);

    let labels: Vec<&str> = candidates.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(
        labels,
        vec![
            "B0_baseline",
            "B1_explicit_text",
            "B2_compiled_graph",
            "B3_compressed",
            "B4_prefix_aligned",
            "B5_optimized_hybrid"
        ]
    );

    // B0 has 0 requirements
    assert_eq!(candidates[0].requirements.len(), 0);

    // B2 has enforced validator requirements
    assert!(candidates[2].requirements.iter().any(|r| r.validator.is_some()));

    // B3 has lower budget than B2
    assert!(candidates[3].budget.max_bytes < candidates[2].budget.max_bytes);

    // B5 contains domain-specific optimizations
    assert!(candidates[5].requirements.iter().any(|r| r.rule.contains("clinical-guidance")));
}

#[test]
fn test_03_end_to_end_model_precision_optimization() {
    let target = ModelPrecisionTarget::native_gguf(
        "qwen-2.5-coder-7b",
        ModelFamily::Qwen,
        32768,
        "software-dev",
    );

    let mut manifest = TaskManifest::new("software-dev-benchmark", 1);

    // Training split
    manifest.add_task(TaskItem {
        task_id: "task-01".into(),
        split: SplitType::Train,
        prompt: "Fix memory leak in buffer".into(),
        expected_outcome: "Reclaim buffer via RAII TempDir".into(),
        domain: "software-dev".into(),
    });
    // Dev split
    manifest.add_task(TaskItem {
        task_id: "task-02".into(),
        split: SplitType::Dev,
        prompt: "Add bounds check to slice index".into(),
        expected_outcome: "assert index < len".into(),
        domain: "software-dev".into(),
    });
    // Held-out test split
    manifest.add_task(TaskItem {
        task_id: "task-03".into(),
        split: SplitType::HeldOutTest,
        prompt: "Ensure zero heap allocation in hot path".into(),
        expected_outcome: "Use caller provided slice buffer".into(),
        domain: "software-dev".into(),
    });

    let mut registry = ConditioningRegistry::new();

    let receipt = ModelPrecisionOptimizer::optimize_model(
        &target,
        &manifest,
        &mut registry,
        "run-precision-001",
    )
    .expect("optimization succeeds");

    assert_eq!(receipt.model_id, "qwen-2.5-coder-7b");
    assert_eq!(receipt.run_id, "run-precision-001");
    assert_eq!(receipt.target_domain, "software-dev");
    assert_eq!(receipt.baseline_variant, "B0_baseline");
    assert!(receipt.winning_variant.starts_with("B5") || receipt.winning_variant.starts_with("B2"));

    // Verify delta improvement is strictly positive
    assert!(receipt.delta_improvement_pct > 0.0);
    assert!(receipt.optimized_score.composite_score() > receipt.baseline_score.composite_score());

    // Verify held-out test was evaluated
    assert!(receipt.held_out_test_score.composite_score() > 0.0);

    // Verify registry activation
    assert_eq!(receipt.promoted_version, 1);
    assert_eq!(receipt.previous_version, None);

    let profile_id = format!("urn:qualia:profile:model:{}", target.model_id);
    let active = registry.get_active(&profile_id).expect("profile found");
    assert_eq!(active.version, 1);
    assert_eq!(active.spec_identity, receipt.winning_spec_identity);
}

#[test]
fn test_04_split_isolation_and_error_handling() {
    let target = ModelPrecisionTarget::native_gguf(
        "test-model",
        ModelFamily::Generic,
        8192,
        "test",
    );
    let mut registry = ConditioningRegistry::new();

    // 1. Empty manifest fails
    let empty_manifest = TaskManifest::new("empty", 1);
    let err1 = ModelPrecisionOptimizer::optimize_model(&target, &empty_manifest, &mut registry, "r1");
    assert_eq!(err1.unwrap_err(), OptimizationError::EmptyManifest);

    // 2. Manifest with no training tasks fails
    let mut only_test = TaskManifest::new("only_test", 1);
    only_test.add_task(TaskItem {
        task_id: "t1".into(),
        split: SplitType::HeldOutTest,
        prompt: "p".into(),
        expected_outcome: "o".into(),
        domain: "d".into(),
    });
    let err2 = ModelPrecisionOptimizer::optimize_model(&target, &only_test, &mut registry, "r2");
    assert_eq!(err2.unwrap_err(), OptimizationError::NoTrainingTasks);

    // 3. Manifest with no held-out tasks fails
    let mut only_train = TaskManifest::new("only_train", 1);
    only_train.add_task(TaskItem {
        task_id: "t1".into(),
        split: SplitType::Train,
        prompt: "p".into(),
        expected_outcome: "o".into(),
        domain: "d".into(),
    });
    let err3 = ModelPrecisionOptimizer::optimize_model(&target, &only_train, &mut registry, "r3");
    assert_eq!(err3.unwrap_err(), OptimizationError::NoHeldOutTasks);
}

#[test]
fn test_05_model_profile_versioning_and_rollback() {
    let target = ModelPrecisionTarget::native_gguf(
        "mistral-7b-v0.3",
        ModelFamily::Mistral,
        32768,
        "audit",
    );

    let mut manifest_v1 = TaskManifest::new("audit-v1", 1);
    manifest_v1.add_task(TaskItem {
        task_id: "t1".into(),
        split: SplitType::Train,
        prompt: "p1".into(),
        expected_outcome: "o1".into(),
        domain: "audit".into(),
    });
    manifest_v1.add_task(TaskItem {
        task_id: "t2".into(),
        split: SplitType::HeldOutTest,
        prompt: "p2".into(),
        expected_outcome: "o2".into(),
        domain: "audit".into(),
    });

    let mut manifest_v2 = TaskManifest::new("audit-v2", 2);
    manifest_v2.add_task(TaskItem {
        task_id: "t1".into(),
        split: SplitType::Train,
        prompt: "p1".into(),
        expected_outcome: "o1".into(),
        domain: "audit".into(),
    });
    manifest_v2.add_task(TaskItem {
        task_id: "t2".into(),
        split: SplitType::HeldOutTest,
        prompt: "p2".into(),
        expected_outcome: "o2".into(),
        domain: "audit".into(),
    });

    let mut registry = ConditioningRegistry::new();

    // Optimize and activate v1
    let receipt_v1 = ModelPrecisionOptimizer::optimize_model(
        &target,
        &manifest_v1,
        &mut registry,
        "run-v1",
    )
    .expect("v1 succeeds");
    assert_eq!(receipt_v1.promoted_version, 1);

    // Optimize and activate v2
    let receipt_v2 = ModelPrecisionOptimizer::optimize_model(
        &target,
        &manifest_v2,
        &mut registry,
        "run-v2",
    )
    .expect("v2 succeeds");
    assert_eq!(receipt_v2.promoted_version, 2);
    assert_eq!(receipt_v2.previous_version, Some(1));

    let profile_id = format!("urn:qualia:profile:model:{}", target.model_id);
    assert_eq!(registry.get_active(&profile_id).unwrap().version, 2);

    // Rollback to v1
    let (rolled_back_from, restored_version) = registry
        .rollback(&profile_id, Some(1))
        .expect("rollback succeeds");
    assert_eq!(rolled_back_from, 2);
    assert_eq!(restored_version, 1);
    assert_eq!(registry.get_active(&profile_id).unwrap().version, 1);
}
