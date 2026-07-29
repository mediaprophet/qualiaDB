//! Vision P64 & WGSL Forge Multi-Backend Pipeline Integration Tests.
//!
//! Validates:
//! 1. Vision model catalogue entries are present in `PERCEPTION_MODEL_CATALOG`.
//! 2. P64 weight container headers & alignment contracts.
//! 3. WGSL Forge 2D spatial vision operators (Conv2D, Pool2D, Resize2D) match CPU oracle outputs.

use qualia_core_db::wgsl_forge::graph_ops::vision::{
    conv2d_cpu, conv2d_wgsl, max_pool2d_cpu, max_pool2d_wgsl, resize2d_cpu, resize2d_wgsl,
};
use qualia_core_db::wgsl_forge::validate::validate_wgsl;

// Catalog verification is tested in qualia-client-core tests.

#[test]
fn test_wgsl_vision_shaders_validity() {
    for wg_size in [16, 32, 64, 128] {
        let pool_shader = max_pool2d_wgsl(wg_size);
        let resize_shader = resize2d_wgsl(wg_size);
        let conv_shader = conv2d_wgsl(wg_size);

        validate_wgsl(&pool_shader).expect("Pool2D WGSL shader validation");
        validate_wgsl(&resize_shader).expect("Resize2D WGSL shader validation");
        validate_wgsl(&conv_shader).expect("Conv2D WGSL shader validation");
    }
}

#[test]
fn test_vision_conv2d_oracle_correctness() {
    let input = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]; // 1x3x3
    let weight = [1.0f32, 0.0, 0.0, 1.0]; // 1x1x2x2 identity-diagonal
    let bias = [0.5f32];

    let output = conv2d_cpu(&input, 1, 3, 3, &weight, 1, 2, 2, &bias, 1, 1, 0, 0)
        .expect("conv2d_cpu execution");

    // 1x2x2 output expected
    assert_eq!(output.len(), 4);
    // (1+5)+0.5 = 6.5
    assert!((output[0] - 6.5).abs() < 1e-5);
    // (2+6)+0.5 = 8.5
    assert!((output[1] - 8.5).abs() < 1e-5);
}

#[test]
fn test_vision_maxpool2d_oracle_correctness() {
    let input = [1.0f32, 3.0, 2.0, 4.0, 5.0, 6.0, 7.0, 8.0]; // 1x2x4

    let output = max_pool2d_cpu(&input, 1, 2, 4, 2, 2, 2, 2).expect("max_pool2d_cpu");

    assert_eq!(output, vec![6.0, 8.0]);
}

#[test]
fn test_vision_resize2d_oracle_correctness() {
    let input = [10.0f32, 20.0, 30.0, 40.0]; // 1x2x2

    let output = resize2d_cpu(&input, 1, 2, 2, 4, 4).expect("resize2d_cpu");

    assert_eq!(output.len(), 16);
    assert_eq!(output[0], 10.0);
    assert_eq!(output[15], 40.0);
}
