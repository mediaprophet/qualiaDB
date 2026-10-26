//! Cooperative-matrix 8x8 f32 GEMM for the selected Etalon/Naga profile.
//!
//! Generated source uses the extension declared by the pinned Etalon compiler.
//! Syntax/type validation does not establish hardware execution: the existing
//! adapter checks and multiply probe still govern runtime admission. The f32
//! tile here is this emitter's profile, not a general limit on the developed fork.
/// Target Etalon / WGPU API version.
pub const WGPU_API_VERSION: &str = "30.0.0-etalon";

/// Cooperative matrix blocked flag: `false` on Etalon because
/// `etalon-hal` enables `vulkan_memory_model_device_scope`.
pub const WGPU_COOPMAT_BLOCKED: bool = false;

/// The fixed tile dimension (rows == columns == K) of the emitted GEMM.
pub const TILE: u32 = 8;

/// Emits the cooperative-matrix 8x8 GEMM tile `C = A * B`, all-f32 (the only
/// configuration selected by this emitter). Row-major loads/stores (`coopLoadT`/
/// `coopStoreT`, stride = TILE) reproduce a standard row-major reference
/// `c[i][j] = sum_k a[i][k] * b[k][j]`, so it verifies against
/// [`crate::wgsl_forge::oracle::matmul_cpu`]. The accumulator is seeded by loading
/// `c`, which the caller zero-fills.
pub fn matmul_tc_wgsl() -> String {
    format!(
        r#"enable etalon_cooperative_matrix;

@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> c: array<f32>;

@compute @workgroup_size(32)
fn matmul_tc() {{
    // `T` (row-major) loads + a row-major store match a host row-major matmul.
    let a_frag = coopLoadT<coop_mat8x8<f32, A>>(&a[0], {TILE}u);
    let b_frag = coopLoadT<coop_mat8x8<f32, B>>(&b[0], {TILE}u);
    let acc_in = coopLoadT<coop_mat8x8<f32, C>>(&c[0], {TILE}u);
    let acc = coopMultiplyAdd(a_frag, b_frag, acc_in);
    coopStoreT(acc, &c[0], {TILE}u);
}}"#,
        TILE = TILE,
    )
}

/// Entry point of the tiled cooperative-matrix GEMM ([`matmul_tc_wgsl_tiled`]).
pub const MATMUL_TC_TILED_ENTRY: &str = "matmul_tc_tiled";

/// Emits a **tiled** cooperative-matrix GEMM `C[m×n] = A[m×k]·B[k×n]` (row-major,
/// all-f32) that loops the proven single-8×8×8-tile primitive over arbitrary `m`, `n`,
/// `k` (each a multiple of [`TILE`]). One workgroup (== one subgroup == one warp on
/// NVIDIA, `@workgroup_size(32)`) computes one 8×8 output tile, accumulating across the
/// K dimension in a cooperative-matrix register fragment — exactly the structure of the
/// CUDA WMMA tiled kernel ([`crate::wgsl_forge::emit::cuda_c::WMMA_GEMM_TILED_SRC`]), so
/// the two backends mirror each other for the same DAG node.
///
/// Bindings: `a`(0, read) `b`(1, read) `c`(2, read_write, zero-seeded) and
/// `dims`(3, read) = `[m, n, k]`. Dispatch one workgroup per output tile:
/// `num_tiles = (m/8)·(n/8)`, each picked by `@builtin(workgroup_id).x`. Row-major
/// `coopLoadT`/`coopStoreT` with the runtime leading dimensions (`k` for A, `n` for B/C)
/// reproduce the standard reference, so it verifies against
/// [`crate::wgsl_forge::oracle::matmul_cpu`].
///
/// Actual execution requires native adapter advertisement and a passing multiply probe.
pub fn matmul_tc_wgsl_tiled() -> String {
    format!(
        r#"enable etalon_cooperative_matrix;

@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> c: array<f32>;
@group(0) @binding(3) var<storage, read> dims: array<u32>;

@compute @workgroup_size(32)
fn {ENTRY}(@builtin(workgroup_id) wid: vec3<u32>) {{
    let m = dims[0];
    let n = dims[1];
    let k = dims[2];
    let tiles_n = n / {TILE}u;
    let num_tiles = (m / {TILE}u) * tiles_n;
    let tile = wid.x;
    if (tile >= num_tiles) {{ return; }}
    let tile_row = tile / tiles_n;
    let tile_col = tile % tiles_n;
    // Output-tile base offset in the row-major C[m×n].
    let c_off = (tile_row * {TILE}u) * n + tile_col * {TILE}u;
    var acc = coopLoadT<coop_mat8x8<f32, C>>(&c[c_off], n);
    // Accumulate the 8-wide K tiles: A row-major leading dim k, B leading dim n.
    for (var kt: u32 = 0u; kt < k; kt = kt + {TILE}u) {{
        let a_off = (tile_row * {TILE}u) * k + kt;
        let b_off = kt * n + tile_col * {TILE}u;
        let a_frag = coopLoadT<coop_mat8x8<f32, A>>(&a[a_off], k);
        let b_frag = coopLoadT<coop_mat8x8<f32, B>>(&b[b_off], n);
        acc = coopMultiplyAdd(a_frag, b_frag, acc);
    }}
    coopStoreT(acc, &c[c_off], n);
}}"#,
        ENTRY = MATMUL_TC_TILED_ENTRY,
        TILE = TILE,
    )
}
