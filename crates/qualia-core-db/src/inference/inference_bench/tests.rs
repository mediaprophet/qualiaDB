use super::*;

/// W8 — the GEMM backend selector. On wgpu 29.0.3 `coopmat_gemm_usable()` is `false` (the #9741
/// probe fails / the seam is disarmed by default), so the selector never returns `Coopmat` here —
/// it falls to `CoopGemv`/`Naive` by the coop-GEMV toggle. The coopmat arm self-activates when the
/// probe passes (a wgpu release / soft-fork). Also checks the 8×8×8 tile-fit gate.
#[test]
fn gemm_backend_selection_falls_back_without_coopmat() {
    set_coopmat_gemm(false); // disarmed → never Coopmat regardless of dims
    assert!(!coopmat_gemm_usable());
    set_coop_gemv(true);
    assert_eq!(select_gemm_backend(16, 16, 16), GemmBackend::CoopGemv);
    assert_eq!(select_gemm_backend(1, 960, 320), GemmBackend::CoopGemv); // m=1 decode GEMV
    set_coop_gemv(false);
    assert_eq!(select_gemm_backend(16, 16, 16), GemmBackend::Naive);
    // Arming the seam does not force Coopmat on 29.0.3 — the hardware probe still gates it.
    set_coopmat_gemm(true);
    assert!(
        !coopmat_gemm_usable() || select_gemm_backend(16, 16, 16) == GemmBackend::Coopmat,
        "if usable, an 8-mult matmul selects Coopmat; else it must fall back"
    );
    // Non-8-multiple dims never take the coopmat tile even if usable.
    set_coop_gemv(true);
    assert_ne!(select_gemm_backend(1, 7, 13), GemmBackend::Coopmat);
    // Restore defaults.
    set_coopmat_gemm(false);
    set_coop_gemv(true);
}
