//! Cooperative-matrix (tensor-core) WGSL emission (plan §18).
//!
//! Emits a single 8x8x8 GEMM tile `C = A * B` using naga's WGSL cooperative
//! matrix extension (`enable wgpu_cooperative_matrix`, `coop_mat8x8<T, role>`,
//! `coopLoad`/`coopMultiplyAdd`/`coopStore`). One subgroup cooperatively computes
//! the tile. Gated on `AdapterConstraints::supports_coopmat`.
//!
//! 8x8 with f32 is the broadly-supported cooperative-matrix configuration (per
//! wgpu's `EXPERIMENTAL_COOPERATIVE_MATRIX` docs); 16x16 f32 is accepted by the
//! validator but not guaranteed on hardware.

/// The fixed tile dimension (rows == columns == K) of the emitted GEMM.
pub const TILE: u32 = 8;

/// Emits the cooperative-matrix 8x8 GEMM tile for the given scalar WGSL type
/// (`"f32"` or `"f16"`). The accumulator is seeded by loading `c` (zero-filled
/// by the caller).
pub fn matmul_tc_wgsl(scalar: &str) -> String {
    format!(
        r#"enable wgpu_cooperative_matrix;

@group(0) @binding(0) var<storage, read> a: array<{scalar}>;
@group(0) @binding(1) var<storage, read> b: array<{scalar}>;
@group(0) @binding(2) var<storage, read_write> c: array<{scalar}>;

@compute @workgroup_size(32)
fn matmul_tc() {{
    // The `T` (row-major) variants match a host row-major matmul reference.
    let a_frag = coopLoadT<coop_mat8x8<{scalar}, A>>(&a[0], {TILE}u);
    let b_frag = coopLoadT<coop_mat8x8<{scalar}, B>>(&b[0], {TILE}u);
    var acc = coopLoadT<coop_mat8x8<{scalar}, C>>(&c[0], {TILE}u);
    acc = coopMultiplyAdd(a_frag, b_frag, acc);
    coopStoreT(acc, &c[0], {TILE}u);
}}"#,
        scalar = scalar,
        TILE = TILE,
    )
}
