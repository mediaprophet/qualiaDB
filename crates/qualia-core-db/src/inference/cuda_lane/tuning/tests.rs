use super::q8_config::CudaQ8Profile;
use super::q8_config::CudaQ8Tuning;

fn winning_tuning() -> CudaQ8Tuning {
    CudaQ8Tuning {
        profile: CudaQ8Profile::A2000SmolLm2Q8V1,
        stage_debug: false,
        dp4a_swiglu: true,
        dp4a_swiglu_layers: 31,
        dp4a_qkv: true,
        dp4a_down_projection: true,
        dp4a_o_projection: true,
        dp4a_lm_head: true,
    }
}

#[test]
fn named_profile_fails_closed_on_shape_mismatch() {
    let tuning = CudaQ8Tuning::a2000_smollm2_q8_v1(false);
    assert_eq!(
        tuning.for_model(4096, 32, 8, 128, 32, 128_256).profile,
        CudaQ8Profile::Incumbent
    );
    assert_eq!(tuning.for_model(960, 15, 5, 64, 32, 49_152), tuning);
}

#[test]
fn incumbent_auto_promotes_only_for_certified_adapter_and_shape() {
    let incumbent = CudaQ8Tuning::incumbent();
    let promoted = incumbent.for_model_on_adapter(
        960,
        15,
        5,
        64,
        32,
        49_152,
        0x10de,
        0x2571,
        "NVIDIA RTX A2000 12GB",
    );
    assert_eq!(promoted.profile, CudaQ8Profile::A2000SmolLm2Q8V1);

    let wrong_adapter = incumbent.for_model_on_adapter(
        960,
        15,
        5,
        64,
        32,
        49_152,
        0x10de,
        0x2684,
        "NVIDIA RTX 4090",
    );
    assert_eq!(wrong_adapter.profile, CudaQ8Profile::Incumbent);

    let wrong_shape = incumbent.for_model_on_adapter(
        4096,
        32,
        8,
        128,
        32,
        128_256,
        0x10de,
        0x2571,
        "NVIDIA RTX A2000 12GB",
    );
    assert_eq!(wrong_shape.profile, CudaQ8Profile::Incumbent);
}

#[test]
fn custom_environment_is_never_replaced_by_auto_selection() {
    let custom = CudaQ8Tuning {
        profile: CudaQ8Profile::CustomEnvironment,
        dp4a_qkv: true,
        ..CudaQ8Tuning::incumbent()
    };
    assert_eq!(
        custom.for_model_on_adapter(
            960,
            15,
            5,
            64,
            32,
            49_152,
            0x10de,
            0x2571,
            "NVIDIA RTX A2000 12GB",
        ),
        custom
    );
}

#[test]
fn winning_a2000_schedule_has_exact_graph_node_count() {
    assert_eq!(winning_tuning().q8_graph_nodes(32), 387);
}

#[test]
fn graph_fingerprint_changes_for_every_execution_affecting_field() {
    let baseline = winning_tuning();
    let baseline_key = baseline.graph_fingerprint();
    let variants = [
        CudaQ8Tuning {
            dp4a_swiglu_layers: 30,
            ..baseline
        },
        CudaQ8Tuning {
            dp4a_qkv: false,
            ..baseline
        },
        CudaQ8Tuning {
            dp4a_down_projection: false,
            ..baseline
        },
        CudaQ8Tuning {
            dp4a_o_projection: false,
            ..baseline
        },
        CudaQ8Tuning {
            dp4a_lm_head: false,
            ..baseline
        },
    ];
    for variant in variants {
        assert_ne!(variant.graph_fingerprint(), baseline_key);
    }
}

#[test]
fn swiglu_layer_mask_is_bounded() {
    let tuning = winning_tuning();
    assert!(tuning.dp4a_swiglu_layer(30));
    assert!(!tuning.dp4a_swiglu_layer(31));
}
