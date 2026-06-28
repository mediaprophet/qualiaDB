//! Cooperative-matrix (tensor-core) WGSL emission (plan §18).
//!
//! Emits a single 8x8x8 GEMM tile `C = A * B` using naga's WGSL cooperative
//! matrix extension (`enable wgpu_cooperative_matrix`, `coop_mat8x8<T, role>`,
//! `coopLoadT`/`coopMultiplyAdd`/`coopStoreT`). One subgroup cooperatively
//! computes the tile. Gated on `AdapterConstraints::supports_coopmat`.
//!
//! The inputs are f16 and the accumulator is f32 — the canonical tensor-core
//! configuration. (f32 inputs validate but the f32x f32 *multiply* is not a
//! supported hardware config: it loads/stores fine but returns zero.) 8x8 is the
//! broadly-supported tile size; 16x16 f32 returns inf on this hardware.

/// The fixed tile dimension (rows == columns == K) of the emitted GEMM.
pub const TILE: u32 = 8;

/// Emits the cooperative-matrix 8x8 GEMM tile: f16 A and B inputs multiplied with
/// an f32 accumulator (seeded by loading `c`, zero-filled by the caller).
pub fn matmul_tc_wgsl() -> String {
    format!(
        r#"enable f16;
enable wgpu_cooperative_matrix;

@group(0) @binding(0) var<storage, read> a: array<f16>;
@group(0) @binding(1) var<storage, read> b: array<f16>;
@group(0) @binding(2) var<storage, read_write> c: array<f32>;

@compute @workgroup_size(32)
fn matmul_tc() {{
    // The `T` (row-major) variants match a host row-major matmul reference.
    let a_frag = coopLoadT<coop_mat8x8<f16, A>>(&a[0], {TILE}u);
    let b_frag = coopLoadT<coop_mat8x8<f16, B>>(&b[0], {TILE}u);
    let acc_in = coopLoadT<coop_mat8x8<f32, C>>(&c[0], {TILE}u);
    let acc = coopMultiplyAdd(a_frag, b_frag, acc_in);
    coopStoreT(acc, &c[0], {TILE}u);
}}"#,
        TILE = TILE,
    )
}
