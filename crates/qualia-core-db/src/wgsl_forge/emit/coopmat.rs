//! Cooperative-matrix (tensor-core) WGSL emission (plan §18).
//!
//! Emits a single 8x8x8 GEMM tile `C = A * B` using naga's WGSL cooperative
//! matrix extension (`enable wgpu_cooperative_matrix`, `coop_mat8x8<T, role>`,
//! `coopLoadT`/`coopMultiplyAdd`/`coopStoreT`). One subgroup cooperatively
//! computes the tile. Gated on [`crate::wgsl_forge::AdapterConstraints::supports_coopmat`].
//!
//! ## Why all-f32, and why the multiply is *execution*-blocked on wgpu 29.0.3
//!
//! This emitter is all-f32 8x8x8 because that is the only configuration wgpu/naga
//! 29.0.3 even claims to support: `wgpu-types-29.0.3/features.rs:1375` states
//! "The implementation currently only supports 8x8 **f32** matrices", and on Vulkan
//! it gates the feature on `vkGetPhysicalDeviceCooperativeMatrixPropertiesKHR`
//! matching 8x8x8 f32. (Mixed-precision f16-in/f32-acc MulAdd — the canonical
//! reduced-precision tensor-core config — was added to wgpu *after* the 29 line,
//! gfx-rs/wgpu#9629, MSL-first; an earlier f16 revision of this file requested that
//! unimplemented config.) The participation set is correct at `@workgroup_size(32)`:
//! naga declares the coop-matrix SPIR-V type with `Scope::Subgroup`
//! (`naga-29.0.3/back/spv/writer.rs`: `get_index_constant(spirv::Scope::Subgroup)`),
//! so on NVIDIA one 32-lane warp is exactly one subgroup — verified empirically,
//! `@workgroup_size(8,8,1)` gives the identical result.
//!
//! Even so, the all-f32 `coopMultiplyAdd` returns **all-zeros** when executed on the
//! 29.0.3 Vulkan path (the `coopLoadT`/`coopStoreT` round-trip works — only the
//! multiply fails). This matches gfx-rs/wgpu#9729/#9741: coopmat emits Device-scope
//! SPIR-V memory ops that are invalid/no-op'd unless `vulkanMemoryModelDeviceScope`
//! is auto-enabled at device creation — a fix that landed on git `main` **after**
//! 29.0.3. 29.0.3 is the newest published wgpu (crates.io), so there is no released
//! fix; the WGSL multiply will start working when a wgpu release carries #9741 (or
//! by pinning wgpu to a git commit — a core-dependency decision left to the human).
//! naga's own coopmat test is a WGSL→SPIR-V *translation* test, not a GPU-execution
//! test, so its passing never implied the multiply runs.
//!
//! The kernel below is therefore correct and naga-validated, and `evaluate_matmul_tc`
//! is kept ready to assert it the moment the upstream fix ships. Until then the
//! genuine tensor-core multiply is delivered + hardware-verified via the CUDA WMMA
//! path (`emit::cuda_c::WMMA_GEMM_16X16_SRC`, `oracle::evaluate_matmul_tc_cuda`),
//! which uses NVIDIA's mature `nvcuda::wmma` API and is unaffected by this wgpu bug.

/// The fixed tile dimension (rows == columns == K) of the emitted GEMM.
pub const TILE: u32 = 8;

/// Emits the cooperative-matrix 8x8 GEMM tile `C = A * B`, all-f32 (the only
/// configuration wgpu/naga 29 implements). Row-major loads/stores (`coopLoadT`/
/// `coopStoreT`, stride = TILE) reproduce a standard row-major reference
/// `c[i][j] = sum_k a[i][k] * b[k][j]`, so it verifies against
/// [`crate::wgsl_forge::oracle::matmul_cpu`]. The accumulator is seeded by loading
/// `c`, which the caller zero-fills.
pub fn matmul_tc_wgsl() -> String {
    format!(
        r#"enable wgpu_cooperative_matrix;

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
