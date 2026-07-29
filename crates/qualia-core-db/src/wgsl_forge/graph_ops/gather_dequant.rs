//! Native `GatherDequant` op-node — on-the-fly **ternary weight dequant** producing an f32
//! tensor that a downstream `MatMul` consumes (the `{GatherDequant → MatMul}` split, plan
//! §2 / P4b). This is the LLM weight-decompression primitive: BitNet-style 2-bit ternary
//! codes unpacked to `{0, +1, −1}` and scaled per row.
//!
//! # Packing (matches the certified `ternary-gemv` BuiltinKernel)
//!
//! A weight row of `cols` codes is 2-bit-packed, **16 codes per `u32`**, low-to-high lanes;
//! code `0→0.0, 1→+1.0, 2→-1.0, 3→0.0`. `k_words = ceil(cols/16)` words per row, rows laid
//! out contiguously. `out[row, col] = scale[row] · ternary(code[row, col])`.
//!
//! # Why the packed words bind as `array<u32>`
//!
//! The graph executor uploads externals as an f32 storage buffer. The packed code-words are
//! carried as `f32::from_bits(word)` so the **bytes** are the exact `u32`, and the WGSL binds
//! that buffer as `array<u32>` (a byte reinterpret — no f32 *load*). This sidesteps GPU NaN
//! canonicalization, which would mangle code-words that happen to be f32 NaN bit patterns.
//! The CPU oracle recovers the word with `f32::to_bits` (a pure bit reinterpret), so the two
//! agree exactly.
//!
//! Certified the forge way: an exact CPU oracle, naga validation, and an A2000 GPU
//! differential test (here, and end-to-end as `{GatherDequant → MatMul}` in the executor).

use crate::wgsl_forge::ForgeError;

/// Entry-point name of the emitted ternary gather-dequant kernel.
pub const GATHER_DEQUANT_ENTRY: &str = "gather_dequant_main";

/// Decode one 2-bit ternary `code` to its value (`0→0, 1→+1, 2→-1, 3→0`).
#[inline]
fn ternary(code: u32) -> f32 {
    match code {
        1 => 1.0,
        2 => -1.0,
        _ => 0.0,
    }
}

/// Emit the WGSL module for ternary `GatherDequant` at workgroup size `wg`. One invocation
/// per output element `o = row*cols + col`. Binding ABI: `packed` (0, storage read,
/// **`array<u32>`** — code-words), `scale` (1, storage read, per-row f32), `output` (2,
/// storage read_write, `[rows*cols]` f32), `params` (3, storage read,
/// `[rows, cols, k_words, _]` u32).
pub fn gather_dequant_ternary_wgsl(wg: u32) -> String {
    format!(
        r#"@group(0) @binding(0) var<storage, read> packed: array<u32>;
@group(0) @binding(1) var<storage, read> scale: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<storage, read> params: array<u32>;
@compute @workgroup_size({wg})
fn {entry}(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let o = gid.x;
    let rows = params[0];
    let cols = params[1];
    let k_words = params[2];
    if (o >= rows * cols) {{ return; }}
    let row = o / cols;
    let col = o % cols;
    let word = packed[row * k_words + (col / 16u)];
    // Extract the 2-bit ternary code for this lane (low-to-high): 0->0, 1->+1, 2->-1.
    let code = (word >> ((col % 16u) * 2u)) & 3u;
    var tern = 0.0;
    if (code == 1u) {{
        tern = 1.0;
    }} else if (code == 2u) {{
        tern = -1.0;
    }}
    output[o] = scale[row] * tern;
}}
"#,
        entry = GATHER_DEQUANT_ENTRY,
    )
}

/// Exact CPU oracle for [`gather_dequant_ternary_wgsl`]: dequantize the `rows × cols` ternary
/// weight matrix. `packed` carries the code-words as `f32::from_bits(word)` (same as the GPU
/// external); `scale` is per-row. Returns `rows*cols` row-major f32.
pub fn gather_dequant_ternary_cpu(
    packed: &[f32],
    scale: &[f32],
    rows: usize,
    cols: usize,
) -> Vec<f32> {
    let k_words = cols.div_ceil(16);
    let mut out = vec![0.0f32; rows * cols];
    for row in 0..rows {
        let s = scale.get(row).copied().unwrap_or(0.0);
        for col in 0..cols {
            let word = packed
                .get(row * k_words + (col / 16))
                .map(|f| f.to_bits())
                .unwrap_or(0);
            let code = (word >> ((col % 16) * 2)) & 3;
            out[row * cols + col] = s * ternary(code);
        }
    }
    out
}

/// Pack a row-major `rows × cols` ternary weight matrix (`values ∈ {-1, 0, +1}`) into the
/// code-word layout the kernel/oracle read, returned as `f32::from_bits(word)` per word
/// (`rows · ceil(cols/16)` elements). Inverse of the unpack; the test fixture + a real
/// encoder both use it. Non-`{-1,0,1}` values map to code `0` (zero).
pub fn pack_ternary_as_words(values: &[f32], rows: usize, cols: usize) -> Vec<f32> {
    let k_words = cols.div_ceil(16);
    let mut words = vec![0u32; rows * k_words];
    for row in 0..rows {
        for col in 0..cols {
            let code: u32 = match values.get(row * cols + col).copied().unwrap_or(0.0) {
                v if v > 0.5 => 1,
                v if v < -0.5 => 2,
                _ => 0,
            };
            words[row * k_words + (col / 16)] |= code << ((col % 16) * 2);
        }
    }
    words.into_iter().map(f32::from_bits).collect()
}

/// Run the ternary gather-dequant on the GPU (standalone) and read back the `rows*cols`
/// result — the host-side single-kernel path mirroring [`reduce_gpu`](super::reduce::reduce_gpu).
pub fn gather_dequant_ternary_gpu(
    packed: &[f32],
    scale: &[f32],
    rows: usize,
    cols: usize,
) -> Result<Vec<f32>, ForgeError> {
    use crate::wgsl_forge::execute::{
        BindingUsage, QualiaCompute, WgpuComputeContext, WgpuPipeline,
    };
    use crate::wgsl_forge::Schedule;

    if rows == 0 || cols == 0 {
        return Err(ForgeError::GpuValidation(
            "gather_dequant_ternary_gpu: zero rows/cols".to_string(),
        ));
    }
    let n = rows * cols;
    let k_words = cols.div_ceil(16);
    let wg: u32 = 64;
    let src = gather_dequant_ternary_wgsl(wg);
    let mut ctx = WgpuComputeContext::new((n * 4).max(4 << 20))?;
    let view_packed = ctx.allocate_and_write(
        bytemuck::cast_slice(packed),
        0,
        0,
        BindingUsage::StorageRead,
    )?;
    let view_scale =
        ctx.allocate_and_write(bytemuck::cast_slice(scale), 1, 0, BindingUsage::StorageRead)?;
    let zeros = vec![0.0f32; n];
    let view_out = ctx.allocate_and_write(
        bytemuck::cast_slice(&zeros),
        2,
        0,
        BindingUsage::StorageReadWrite,
    )?;
    let params: [u32; 4] = [rows as u32, cols as u32, k_words as u32, 0];
    let view_params = ctx.allocate_and_write(
        bytemuck::cast_slice(&params),
        3,
        0,
        BindingUsage::StorageRead,
    )?;
    let pipeline = WgpuPipeline::compile(&ctx, &src, GATHER_DEQUANT_ENTRY)?;
    let schedule = Schedule {
        workgroup_size: wg,
        ..Default::default()
    };
    pipeline.dispatch(
        &[view_packed, view_scale, view_out, view_params],
        &schedule,
        n,
    )?;
    let mut out = ctx.read_buffer_f32(&view_out)?;
    out.truncate(n);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::validate::validate_wgsl;

    #[test]
    fn gather_dequant_wgsl_validates() {
        let report = validate_wgsl(&gather_dequant_ternary_wgsl(64)).expect("validate");
        assert!(report
            .entry_points
            .iter()
            .any(|e| e == GATHER_DEQUANT_ENTRY));
        assert_eq!(report.binding_count, 4);
    }

    #[test]
    fn pack_unpack_roundtrips_cpu() {
        // 2 rows × 20 cols (so k_words = 2 per row), values in {-1,0,1}, scale per row.
        let (rows, cols) = (2usize, 20usize);
        let vals: Vec<f32> = (0..rows * cols)
            .map(|i| match i % 3 {
                0 => 1.0,
                1 => -1.0,
                _ => 0.0,
            })
            .collect();
        let packed = pack_ternary_as_words(&vals, rows, cols);
        assert_eq!(packed.len(), rows * cols.div_ceil(16));
        let scale = vec![2.0f32, 0.5];
        let got = gather_dequant_ternary_cpu(&packed, &scale, rows, cols);
        // Expected: scale[row] * ternary(value).
        for row in 0..rows {
            for col in 0..cols {
                let want = scale[row] * vals[row * cols + col];
                assert_eq!(got[row * cols + col], want, "[{row},{col}]");
            }
        }
    }

    #[test]
    #[serial_test::serial(gpu)]
    fn gather_dequant_gpu_matches_oracle() {
        if !crate::wgsl_forge::test_gpu_available() {
            return;
        }
        let (rows, cols) = (4usize, 40usize);
        let vals: Vec<f32> = (0..rows * cols)
            .map(|i| match (i * 7) % 3 {
                0 => 1.0,
                1 => -1.0,
                _ => 0.0,
            })
            .collect();
        let packed = pack_ternary_as_words(&vals, rows, cols);
        let scale: Vec<f32> = (0..rows).map(|r| 0.5 + r as f32 * 0.25).collect();
        let gpu = gather_dequant_ternary_gpu(&packed, &scale, rows, cols).expect("gpu");
        let cpu = gather_dequant_ternary_cpu(&packed, &scale, rows, cols);
        assert_eq!(gpu, cpu);
    }
}
