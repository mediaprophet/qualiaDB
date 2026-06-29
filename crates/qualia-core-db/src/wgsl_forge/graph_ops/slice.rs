//! Native `Slice` op-node — extract a contiguous sub-range `input[offset .. offset+len]`.
//!
//! The composable primitive for per-head q/K/V extraction in multi-head attention: the projected
//! q/K/V are produced **on-device** (q = x·Wq, etc.), so per-head slicing must run on the GPU, not
//! on the host. With a head-major K/V cache layout, every per-head slice (q_h, Kᵀ_h, V_h, and the
//! output-projection row block Wo_h) is a contiguous range this op extracts.
//!
//! Certified the forge way: exact CPU oracle + A2000 GPU differential (via the executor).

/// Entry-point name of the emitted slice kernel.
pub const SLICE_ENTRY: &str = "slice_main";

/// Emit the slice WGSL kernel at workgroup size `wg`. Binding ABI: `input` (0, storage read),
/// `output` (1, storage read_write), `params` (2, storage read, `[len, offset, _, _]` u32). One
/// invocation per output element.
pub fn slice_wgsl(wg: u32) -> String {
    format!(
        r#"@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<storage, read> params: array<u32>;
@compute @workgroup_size({wg})
fn {entry}(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let i = gid.x;
    let len = params[0];
    if (i >= len) {{ return; }}
    let offset = params[1];
    output[i] = input[offset + i];
}}
"#,
        entry = SLICE_ENTRY,
    )
}

/// Exact CPU oracle for [`slice_wgsl`].
pub fn slice_cpu(input: &[f32], offset: usize, len: usize) -> Vec<f32> {
    input[offset..offset + len].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::validate::validate_wgsl;

    #[test]
    fn slice_wgsl_validates() {
        let report = validate_wgsl(&slice_wgsl(64)).expect("naga validate");
        assert!(report.entry_points.iter().any(|e| e == SLICE_ENTRY));
    }

    #[test]
    fn slice_cpu_extracts_range() {
        let v: Vec<f32> = (0..10).map(|i| i as f32).collect();
        assert_eq!(slice_cpu(&v, 3, 4), vec![3.0, 4.0, 5.0, 6.0]);
        assert_eq!(slice_cpu(&v, 0, 1), vec![0.0]);
        assert_eq!(slice_cpu(&v, 6, 4), vec![6.0, 7.0, 8.0, 9.0]);
    }
}
