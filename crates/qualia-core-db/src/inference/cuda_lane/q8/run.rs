use super::kernel::{Q8_0_GEMV_ENTRY, Q8_0_GEMV_ROWS, Q8_0_GEMV_SRC};
use super::oracle::{Q8_0_BLOCK_BYTES, Q8_0_BLOCK_ELEMS};
use crate::inference::cuda_lane::device::{
    ensure_device, ensure_sticky_x, ensure_weight_resident, multi_weight_device,
};
use crate::inference::cuda_lane::weight_cache::weight_fingerprint;

/// Execute one native CUDA Q8_0 GEMV. This public runner is a differential/profiling boundary;
/// prepared whole-model decode uses the same kernel with pre-resident views.
pub fn try_q8_0_cuda_gemv(
    n_in: usize,
    n_out: usize,
    x: &[f32],
    weight_raw: &[u8],
    out: &mut [f32],
) -> bool {
    use crate::wgsl_forge::dispatch::{caps, ensure_cuda_runtime_path};
    use crate::wgsl_forge::execute::{CudaPipeline, QualiaCompute};

    if !crate::inference_modes::prefer_tensor_core_gemm()
        || n_in == 0
        || n_out == 0
        || !n_in.is_multiple_of(Q8_0_BLOCK_ELEMS)
        || x.len() < n_in
        || out.len() < n_out
    {
        return false;
    }
    ensure_cuda_runtime_path();
    if !caps().cuda {
        return false;
    }
    let row_bytes = (n_in / Q8_0_BLOCK_ELEMS) * Q8_0_BLOCK_BYTES;
    let Some(need) = row_bytes.checked_mul(n_out) else {
        return false;
    };
    if weight_raw.len() < need || need > 256 * 1024 * 1024 {
        return false;
    }
    let key = weight_fingerprint(&weight_raw[..need], n_in, n_out);
    let Ok(mut guard) = multi_weight_device().lock() else {
        return false;
    };
    if !ensure_device(&mut guard) {
        return false;
    }
    let dev = guard.as_mut().unwrap();
    if !ensure_weight_resident(dev, key, &weight_raw[..need]) {
        return false;
    }
    let mut view_x = match ensure_sticky_x(dev, &x[..n_in]) {
        Some(view) => view,
        None => return false,
    };
    let mut view_w = *dev.weights.get(&key).unwrap();
    dev.ctx.restore_checkpoint(dev.permanent_end);
    let mut view_y = match dev.ctx.allocate_transient(n_out * 4, 2, 0) {
        Ok(view) => view,
        Err(_) => return false,
    };
    let dims = [n_in as u32, n_out as u32, row_bytes as u32];
    let mut view_dims = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&dims), 3, 0)
    {
        Ok(view) => view,
        Err(_) => return false,
    };
    view_x.binding = 0;
    view_w.binding = 1;
    view_y.binding = 2;
    view_dims.binding = 3;
    let pipeline = match CudaPipeline::compile_cuda_c_source_cached(
        &dev.ctx,
        Q8_0_GEMV_SRC,
        Q8_0_GEMV_ENTRY,
        &[0, 1, 2, 3],
    ) {
        Ok(pipeline) => pipeline,
        Err(error) => {
            log::warn!("cuda_lane|q8_0|compile|{error:?}");
            return false;
        }
    };
    let schedule = crate::wgsl_forge::Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    let blocks = n_out.div_ceil(Q8_0_GEMV_ROWS).max(1);
    if let Err(error) = pipeline.dispatch(
        &[view_x, view_w, view_y, view_dims],
        &schedule,
        blocks * 256,
    ) {
        log::warn!("cuda_lane|q8_0|dispatch|{error:?}");
        return false;
    }
    let values = match dev.ctx.read_buffer_f32(&view_y) {
        Ok(values) => values,
        Err(error) => {
            log::warn!("cuda_lane|q8_0|readback|{error:?}");
            return false;
        }
    };
    out[..n_out].copy_from_slice(&values[..n_out]);
    true
}
