//! Hermetic CLI-path smoke tests for the `qualia-cli shader` subcommands.
//!
//! `shader.rs` is a module of the `qualia-cli` *binary* crate (there is no library
//! target), so its `run` / `ShaderAction` symbols are not reachable from an
//! integration test. The argument parsing those subcommands rely on is covered by
//! `clap`'s derive layer; what is worth pinning here is the **library code path** the
//! `generate` and `validate` arms drive — `generate_builtin(..) -> GeneratedShader`
//! followed by `validate_wgsl(&source)` — which must succeed for the WGSL target with
//! no GPU, no network, and no filesystem.
//!
//! These mirror exactly the `ShaderAction::Generate` and `ShaderAction::Validate`
//! arms in `crates/qualia-cli/src/shader.rs` for `TargetBackend::Wgsl`.

use qualia_core_db::wgsl_forge::{
    generate_builtin, validate_wgsl, BuiltinKernel, Schedule, TargetBackend,
};

/// `shader generate <kernel> --target wgsl` path: every built-in emits deterministic
/// WGSL on the default schedule with no adapter.
#[test]
fn generate_subcommand_path_emits_wgsl_for_all_builtins_without_gpu() {
    for builtin in BuiltinKernel::ALL {
        let generated = generate_builtin(builtin, Schedule::default(), TargetBackend::Wgsl)
            .unwrap_or_else(|e| panic!("generate {} failed: {e}", builtin.name()));
        assert_eq!(generated.kernel_id, builtin.name());
        assert!(
            !generated.source.is_empty(),
            "{} produced empty WGSL",
            builtin.name()
        );
        assert!(
            !generated.source_hash.is_empty(),
            "{} produced no source hash",
            builtin.name()
        );
    }
}

/// `shader validate <kernel> --target wgsl` path: the generated WGSL passes the same
/// Naga parse + semantic validation the CLI runs, headless.
#[test]
fn validate_subcommand_path_naga_validates_generated_wgsl_without_gpu() {
    // affine-f32 is the CLI default kernel; exercise that exact default path.
    let generated =
        generate_builtin(BuiltinKernel::AffineF32, Schedule::default(), TargetBackend::Wgsl)
            .expect("generate affine-f32");
    let report = validate_wgsl(&generated.source).expect("naga validate affine-f32");
    assert!(report.naga_validated);
    assert!(report.binding_count >= 1, "expected at least one binding");
    assert!(
        report.entry_points.iter().any(|e| e == "affine_f32"),
        "expected the affine_f32 entry point, got {:?}",
        report.entry_points
    );
    // Hermetic: validation is in-process Naga, no native tool invoked.
    assert!(report.native_tool_validated.is_none());
}

/// The generate -> validate pipeline holds for every GPU-oracle-less *and* -ful
/// built-in on the WGSL target, confirming the headless CLI path is sound across the
/// kernel set (not just the default).
#[test]
fn generate_then_validate_pipeline_holds_for_all_builtins_without_gpu() {
    for builtin in BuiltinKernel::ALL {
        let generated = generate_builtin(builtin, Schedule::default(), TargetBackend::Wgsl)
            .unwrap_or_else(|e| panic!("generate {} failed: {e}", builtin.name()));
        let report = validate_wgsl(&generated.source)
            .unwrap_or_else(|e| panic!("validate {} failed: {e}", builtin.name()));
        assert!(report.naga_validated, "{} not naga-validated", builtin.name());
    }
}
