//! On-device Q4_K SoA dequant-GEMV kernels — single-matrix GEMV, fused QKV,
//! FFN block (SwiGLU + down-projection), and standalone fused SwiGLU.

use super::device::{ensure_device, ensure_sticky_x, ensure_weight_resident, multi_weight_device};
use super::weight_cache::weight_fingerprint;

/// On-device Q4_K **SoA** dequant-GEMV (type 112 / `.soa.p64`).
///
/// Dequant in CUDA kernel. **Multi-weight residency**: each distinct matrix
/// fingerprint is uploaded once into a permanent slab region; subsequent layers
/// hit device without PCIe re-upload (the novel Qualia CUDA lane).
pub fn try_q4k_soa_gemv(
    n_in: usize,
    n_out: usize,
    x: &[f32],
    weight_raw: &[u8],
    out: &mut [f32],
) -> bool {
    use crate::ggml_quants::{ggml_row_bytes, GGML_TYPE_Q4_K_SOA};
    use crate::wgsl_forge::dispatch::{caps, ensure_cuda_runtime_path};
    use crate::wgsl_forge::emit::cuda_c::{Q4K_SOA_GEMV_ENTRY, Q4K_SOA_GEMV_SRC};
    use crate::wgsl_forge::execute::{CudaPipeline, QualiaCompute};

    if !crate::inference_modes::prefer_tensor_core_gemm() {
        return false;
    }
    ensure_cuda_runtime_path();
    if !caps().cuda {
        return false;
    }
    if n_in == 0 || n_out == 0 || x.len() < n_in || out.len() < n_out {
        return false;
    }
    // Vocab projection is huge — keep on wgpu; CUDA GEMV targets FFN/attn matrices.
    if n_out > 32_768 {
        return false;
    }
    let Some(row_bytes) = ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_in) else {
        return false;
    };
    let need = row_bytes.saturating_mul(n_out);
    if weight_raw.len() < need || need > 48 * 1024 * 1024 {
        return false;
    }
    let key = weight_fingerprint(weight_raw, n_in, n_out);

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
    let view_w = *dev.weights.get(&key).unwrap();
    let view_x = match ensure_sticky_x(dev, &x[..n_in]) {
        Some(v) => v,
        None => return false,
    };
    // Transient y/dims after permanent (+ sticky x).
    dev.ctx.restore_checkpoint(dev.permanent_end);
    let zeros = vec![0.0f32; n_out];
    let view_y = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&zeros), 2, 0)
    {
        Ok(v) => v,
        Err(_) => return false,
    };
    let dims: [u32; 3] = [n_in as u32, n_out as u32, row_bytes as u32];
    let view_dims = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&dims), 3, 0)
    {
        Ok(v) => v,
        Err(_) => return false,
    };

    let buffers = vec![view_x, view_w, view_y, view_dims];
    let pipeline = match CudaPipeline::compile_cuda_c_source_cached(
        &dev.ctx,
        Q4K_SOA_GEMV_SRC,
        Q4K_SOA_GEMV_ENTRY,
        &[0, 1, 2, 3],
    ) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("cuda_lane|q4k_soa|compile|{e:?}");
            return false;
        }
    };
    let schedule = crate::wgsl_forge::Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    // Multi-row kernel: one block owns Q4K_SOA_GEMV_ROWS output rows.
    let rows = crate::wgsl_forge::emit::cuda_c::Q4K_SOA_GEMV_ROWS as usize;
    let n_blocks = n_out.div_ceil(rows).max(1);
    if let Err(e) = pipeline.dispatch(&buffers, &schedule, n_blocks.saturating_mul(256)) {
        log::warn!("cuda_lane|q4k_soa|dispatch|{e:?}");
        return false;
    }
    let y = match dev.ctx.read_buffer_f32(&view_y) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("cuda_lane|q4k_soa|readback|{e:?}");
            return false;
        }
    };
    out[..n_out].copy_from_slice(&y[..n_out]);
    true
}

/// Project Q, K, and V from the same activation with **one sticky x upload** and
/// **one fused QKV kernel** (shared act tile; GQA: K/V only for `row < n_kv`).
pub fn try_q4k_soa_qkv(
    n_in: usize,
    n_q: usize,
    n_kv: usize,
    x: &[f32],
    q_raw: &[u8],
    k_raw: &[u8],
    v_raw: &[u8],
    q_out: &mut [f32],
    k_out: &mut [f32],
    v_out: &mut [f32],
) -> bool {
    use crate::ggml_quants::{ggml_row_bytes, GGML_TYPE_Q4_K_SOA};
    use crate::wgsl_forge::dispatch::{caps, ensure_cuda_runtime_path};
    use crate::wgsl_forge::emit::cuda_c::{Q4K_SOA_GEMV_ROWS, Q4K_SOA_QKV_ENTRY, Q4K_SOA_QKV_SRC};
    use crate::wgsl_forge::execute::{CudaPipeline, QualiaCompute};

    if !crate::inference_modes::prefer_tensor_core_gemm() {
        return false;
    }
    ensure_cuda_runtime_path();
    if !caps().cuda {
        return false;
    }
    if n_in == 0
        || n_q == 0
        || n_kv == 0
        || n_kv > n_q
        || x.len() < n_in
        || q_out.len() < n_q
        || k_out.len() < n_kv
        || v_out.len() < n_kv
        || n_q > 32_768
    {
        return false;
    }
    let Some(row_bytes) = ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_in) else {
        return false;
    };
    let need_q = row_bytes.saturating_mul(n_q);
    let need_kv = row_bytes.saturating_mul(n_kv);
    if q_raw.len() < need_q
        || k_raw.len() < need_kv
        || v_raw.len() < need_kv
        || need_q > 48 * 1024 * 1024
    {
        return false;
    }
    let kq = weight_fingerprint(q_raw, n_in, n_q);
    let kk = weight_fingerprint(k_raw, n_in, n_kv);
    let kv = weight_fingerprint(v_raw, n_in, n_kv);

    let Ok(mut guard) = multi_weight_device().lock() else {
        return false;
    };
    if !ensure_device(&mut guard) {
        return false;
    }
    let dev = guard.as_mut().unwrap();
    if !ensure_weight_resident(dev, kq, &q_raw[..need_q])
        || !ensure_weight_resident(dev, kk, &k_raw[..need_kv])
        || !ensure_weight_resident(dev, kv, &v_raw[..need_kv])
    {
        // Fall back to three separate GEMVs (still sticky-x).
        drop(guard);
        return try_q4k_soa_gemv(n_in, n_q, x, q_raw, q_out)
            && try_q4k_soa_gemv(n_in, n_kv, x, k_raw, k_out)
            && try_q4k_soa_gemv(n_in, n_kv, x, v_raw, v_out);
    }
    let mut view_x = match ensure_sticky_x(dev, &x[..n_in]) {
        Some(v) => v,
        None => return false,
    };
    view_x.binding = 0;
    let mut view_wq = *dev.weights.get(&kq).unwrap();
    let mut view_wk = *dev.weights.get(&kk).unwrap();
    let mut view_wv = *dev.weights.get(&kv).unwrap();
    view_wq.binding = 1;
    view_wk.binding = 2;
    view_wv.binding = 3;

    dev.ctx.restore_checkpoint(dev.permanent_end);
    let zq = vec![0.0f32; n_q];
    let zk = vec![0.0f32; n_kv];
    let zv = vec![0.0f32; n_kv];
    let mut view_yq = match dev.ctx.allocate_and_write(bytemuck::cast_slice(&zq), 4, 0) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let mut view_yk = match dev.ctx.allocate_and_write(bytemuck::cast_slice(&zk), 5, 0) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let mut view_yv = match dev.ctx.allocate_and_write(bytemuck::cast_slice(&zv), 6, 0) {
        Ok(v) => v,
        Err(_) => return false,
    };
    view_yq.binding = 4;
    view_yk.binding = 5;
    view_yv.binding = 6;
    let dims: [u32; 4] = [n_in as u32, n_q as u32, n_kv as u32, row_bytes as u32];
    let view_dims = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&dims), 7, 0)
    {
        Ok(mut v) => {
            v.binding = 7;
            v
        }
        Err(_) => return false,
    };

    let pipe = match CudaPipeline::compile_cuda_c_source_cached(
        &dev.ctx,
        Q4K_SOA_QKV_SRC,
        Q4K_SOA_QKV_ENTRY,
        &[0, 1, 2, 3, 4, 5, 6, 7],
    ) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("cuda_lane|qkv|compile|{e:?}");
            drop(guard);
            return try_q4k_soa_gemv(n_in, n_q, x, q_raw, q_out)
                && try_q4k_soa_gemv(n_in, n_kv, x, k_raw, k_out)
                && try_q4k_soa_gemv(n_in, n_kv, x, v_raw, v_out);
        }
    };
    let schedule = crate::wgsl_forge::Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    let n_blocks = n_q.div_ceil(Q4K_SOA_GEMV_ROWS as usize).max(1);
    if let Err(e) = pipe.dispatch(
        &[
            view_x, view_wq, view_wk, view_wv, view_yq, view_yk, view_yv, view_dims,
        ],
        &schedule,
        n_blocks.saturating_mul(256),
    ) {
        log::warn!("cuda_lane|qkv|dispatch|{e:?}");
        return false;
    }
    let yq = match dev.ctx.read_buffer_f32(&view_yq) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let yk = match dev.ctx.read_buffer_f32(&view_yk) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let yv = match dev.ctx.read_buffer_f32(&view_yv) {
        Ok(v) => v,
        Err(_) => return false,
    };
    q_out[..n_q].copy_from_slice(&yq[..n_q]);
    k_out[..n_kv].copy_from_slice(&yk[..n_kv]);
    v_out[..n_kv].copy_from_slice(&yv[..n_kv]);
    true
}

/// Full pre-norm FFN expansion+down on CUDA with **one host→device x upload** and
/// **one device→host readback** (mid stays on-device between kernels).
///
/// If `residual` is `Some`, `out[i] = residual[i] + down_i` (on-device residual fuse).
/// If `None`, `out` is the down-projection only (caller adds residual on host).
/// Shapes: gate/up are `n_ffn × n_embd`, down is `n_embd × n_ffn`.
pub fn try_q4k_soa_ffn_block(
    n_embd: usize,
    n_ffn: usize,
    x: &[f32],
    gate_raw: &[u8],
    up_raw: &[u8],
    down_raw: &[u8],
    out: &mut [f32],
) -> bool {
    try_q4k_soa_ffn_block_ex(n_embd, n_ffn, x, None, gate_raw, up_raw, down_raw, out)
}

/// Like [`try_q4k_soa_ffn_block`] but fuses residual on device:
/// `out = residual + FFN(x)` with a single host readback.
pub fn try_q4k_soa_ffn_block_residual(
    n_embd: usize,
    n_ffn: usize,
    x_normed: &[f32],
    residual: &[f32],
    gate_raw: &[u8],
    up_raw: &[u8],
    down_raw: &[u8],
    out: &mut [f32],
) -> bool {
    try_q4k_soa_ffn_block_ex(
        n_embd,
        n_ffn,
        x_normed,
        Some(residual),
        gate_raw,
        up_raw,
        down_raw,
        out,
    )
}

fn try_q4k_soa_ffn_block_ex(
    n_embd: usize,
    n_ffn: usize,
    x: &[f32],
    residual: Option<&[f32]>,
    gate_raw: &[u8],
    up_raw: &[u8],
    down_raw: &[u8],
    out: &mut [f32],
) -> bool {
    use crate::ggml_quants::{ggml_row_bytes, GGML_TYPE_Q4_K_SOA};
    use crate::wgsl_forge::dispatch::{caps, ensure_cuda_runtime_path};
    use crate::wgsl_forge::emit::cuda_c::{
        Q4K_SOA_FUSED_SWIGLU_ENTRY, Q4K_SOA_FUSED_SWIGLU_SRC, Q4K_SOA_GEMV_ENTRY, Q4K_SOA_GEMV_SRC,
    };
    use crate::wgsl_forge::execute::{CudaPipeline, QualiaCompute};

    if !crate::inference_modes::prefer_tensor_core_gemm() {
        return false;
    }
    ensure_cuda_runtime_path();
    if !caps().cuda {
        return false;
    }
    if n_embd == 0
        || n_ffn == 0
        || x.len() < n_embd
        || out.len() < n_embd
        || n_ffn > 32_768
        || n_embd > 32_768
    {
        return false;
    }
    if let Some(r) = residual {
        if r.len() < n_embd {
            return false;
        }
    }
    let Some(row_ffn) = ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_embd) else {
        return false;
    };
    let Some(row_down) = ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_ffn) else {
        return false;
    };
    let need_g = row_ffn.saturating_mul(n_ffn);
    let need_d = row_down.saturating_mul(n_embd);
    if gate_raw.len() < need_g
        || up_raw.len() < need_g
        || down_raw.len() < need_d
        || need_g > 48 * 1024 * 1024
        || need_d > 48 * 1024 * 1024
    {
        return false;
    }
    let kg = weight_fingerprint(gate_raw, n_embd, n_ffn);
    let ku = weight_fingerprint(up_raw, n_embd, n_ffn);
    let kd = weight_fingerprint(down_raw, n_ffn, n_embd);

    let Ok(mut guard) = multi_weight_device().lock() else {
        return false;
    };
    if !ensure_device(&mut guard) {
        return false;
    }
    let dev = guard.as_mut().unwrap();
    if !ensure_weight_resident(dev, kg, &gate_raw[..need_g])
        || !ensure_weight_resident(dev, ku, &up_raw[..need_g])
        || !ensure_weight_resident(dev, kd, &down_raw[..need_d])
    {
        return false;
    }
    let mut view_x = match ensure_sticky_x(dev, &x[..n_embd]) {
        Some(v) => v,
        None => return false,
    };
    view_x.binding = 0;

    // One transient arena: mid (n_ffn) + delta (n_embd) + two dims packs.
    dev.ctx.restore_checkpoint(dev.permanent_end);
    let mid_zeros = vec![0.0f32; n_ffn];
    let mut view_mid = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&mid_zeros), 3, 0)
    {
        Ok(v) => v,
        Err(e) => {
            log::warn!("cuda_lane|ffn_block|mid|{e:?}");
            return false;
        }
    };
    let delta_zeros = vec![0.0f32; n_embd];
    let mut view_delta = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&delta_zeros), 2, 0)
    {
        Ok(v) => v,
        Err(_) => return false,
    };
    let dims_s: [u32; 3] = [n_embd as u32, n_ffn as u32, row_ffn as u32];
    let view_dims_s = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&dims_s), 4, 0)
    {
        Ok(v) => v,
        Err(_) => return false,
    };
    let dims_d: [u32; 3] = [n_ffn as u32, n_embd as u32, row_down as u32];
    let view_dims_d = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&dims_d), 3, 0)
    {
        Ok(v) => v,
        Err(_) => return false,
    };

    let mut view_g = *dev.weights.get(&kg).unwrap();
    let mut view_u = *dev.weights.get(&ku).unwrap();
    let mut view_d = *dev.weights.get(&kd).unwrap();
    view_g.binding = 1;
    view_u.binding = 2;
    view_mid.binding = 3;
    // SwiGLU: x=0, Wg=1, Wu=2, mid=3, dims=4
    let mut dims_s_v = view_dims_s;
    dims_s_v.binding = 4;
    let pipe_s = match CudaPipeline::compile_cuda_c_source_cached(
        &dev.ctx,
        Q4K_SOA_FUSED_SWIGLU_SRC,
        Q4K_SOA_FUSED_SWIGLU_ENTRY,
        &[0, 1, 2, 3, 4],
    ) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("cuda_lane|ffn_block|swiglu_compile|{e:?}");
            return false;
        }
    };
    let schedule = crate::wgsl_forge::Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    let rows = crate::wgsl_forge::emit::cuda_c::Q4K_SOA_GEMV_ROWS as usize;
    // SwiGLU is still 1-row-per-block (dual-weight kernel); multi-row is on plain GEMV.
    // Async launch: same-stream ordered; final readback fences (P4 fence budget).
    if let Err(e) = pipe_s.dispatch_async(
        &[view_x, view_g, view_u, view_mid, dims_s_v],
        &schedule,
        n_ffn.saturating_mul(256),
    ) {
        log::warn!("cuda_lane|ffn_block|swiglu_dispatch|{e:?}");
        return false;
    }

    // Down GEMV multi-row: mid stays on device. Optional residual fuse on device.
    view_mid.binding = 0;
    view_d.binding = 1;
    view_delta.binding = 2;
    let mut dims_d_v = view_dims_d;
    dims_d_v.binding = 3;
    let n_down_blocks = n_embd.div_ceil(rows).max(1);
    if let Some(resid) = residual {
        // residual sticky in permanent region for this call
        let view_r = match dev
            .ctx
            .allocate_and_write(bytemuck::cast_slice(&resid[..n_embd]), 4, 0)
        {
            Ok(mut v) => {
                v.binding = 4;
                v
            }
            Err(e) => {
                log::warn!("cuda_lane|ffn_block|resid_upload|{e:?}");
                return false;
            }
        };
        use crate::wgsl_forge::emit::cuda_c::{Q4K_SOA_GEMV_RESID_ENTRY, Q4K_SOA_GEMV_RESID_SRC};
        let pipe_d = match CudaPipeline::compile_cuda_c_source_cached(
            &dev.ctx,
            Q4K_SOA_GEMV_RESID_SRC,
            Q4K_SOA_GEMV_RESID_ENTRY,
            &[0, 1, 2, 3, 4],
        ) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("cuda_lane|ffn_block|down_resid_compile|{e:?}");
                return false;
            }
        };
        // Final op before D2H: fenced QualiaCompute::dispatch (not async) so
        // readback sees completed kernels — implements the trait path fully.
        if let Err(e) = QualiaCompute::dispatch(
            &pipe_d,
            &[view_mid, view_d, view_delta, dims_d_v, view_r],
            &schedule,
            n_down_blocks.saturating_mul(256),
        ) {
            log::warn!("cuda_lane|ffn_block|down_resid_dispatch|{e:?}");
            return false;
        }
    } else {
        let pipe_d = match CudaPipeline::compile_cuda_c_source_cached(
            &dev.ctx,
            Q4K_SOA_GEMV_SRC,
            Q4K_SOA_GEMV_ENTRY,
            &[0, 1, 2, 3],
        ) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("cuda_lane|ffn_block|down_compile|{e:?}");
                return false;
            }
        };
        if let Err(e) = QualiaCompute::dispatch(
            &pipe_d,
            &[view_mid, view_d, view_delta, dims_d_v],
            &schedule,
            n_down_blocks.saturating_mul(256),
        ) {
            log::warn!("cuda_lane|ffn_block|down_dispatch|{e:?}");
            return false;
        }
    }
    let y = match dev.ctx.read_buffer_f32(&view_delta) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("cuda_lane|ffn_block|readback|{e:?}");
            return false;
        }
    };
    out[..n_embd].copy_from_slice(&y[..n_embd]);
    true
}

/// Fused SwiGLU expansion on two sticky Q4_K SoA weights (T-A2):  
/// `out[i] = silu(gate_row_i · x) * (up_row_i · x)`. One kernel, one `x` upload.
pub fn try_q4k_soa_fused_swiglu(
    n_in: usize,
    n_out: usize,
    x: &[f32],
    gate_raw: &[u8],
    up_raw: &[u8],
    out: &mut [f32],
) -> bool {
    use crate::ggml_quants::{ggml_row_bytes, GGML_TYPE_Q4_K_SOA};
    use crate::wgsl_forge::dispatch::{caps, ensure_cuda_runtime_path};
    use crate::wgsl_forge::emit::cuda_c::{Q4K_SOA_FUSED_SWIGLU_ENTRY, Q4K_SOA_FUSED_SWIGLU_SRC};
    use crate::wgsl_forge::execute::{CudaPipeline, QualiaCompute};

    if !crate::inference_modes::prefer_tensor_core_gemm() {
        return false;
    }
    ensure_cuda_runtime_path();
    if !caps().cuda {
        return false;
    }
    if n_in == 0 || n_out == 0 || x.len() < n_in || out.len() < n_out || n_out > 32_768 {
        return false;
    }
    let Some(row_bytes) = ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_in) else {
        return false;
    };
    let need = row_bytes.saturating_mul(n_out);
    if gate_raw.len() < need || up_raw.len() < need || need > 48 * 1024 * 1024 {
        return false;
    }
    let kg = weight_fingerprint(gate_raw, n_in, n_out);
    let ku = weight_fingerprint(up_raw, n_in, n_out);

    let Ok(mut guard) = multi_weight_device().lock() else {
        return false;
    };
    if !ensure_device(&mut guard) {
        return false;
    }
    let dev = guard.as_mut().unwrap();
    if !ensure_weight_resident(dev, kg, &gate_raw[..need])
        || !ensure_weight_resident(dev, ku, &up_raw[..need])
    {
        return false;
    }
    // Permanent weight views store the binding id from first allocate (usually 1).
    // Remap for this launch: x=0, W_gate=1, W_up=2, y=3, dims=4.
    let mut view_g = *dev.weights.get(&kg).unwrap();
    let mut view_u = *dev.weights.get(&ku).unwrap();
    view_g.binding = 1;
    view_u.binding = 2;
    let mut view_x = match ensure_sticky_x(dev, &x[..n_in]) {
        Some(v) => v,
        None => return false,
    };
    view_x.binding = 0;
    dev.ctx.restore_checkpoint(dev.permanent_end);
    let zeros = vec![0.0f32; n_out];
    let view_y = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&zeros), 3, 0)
    {
        Ok(v) => v,
        Err(_) => return false,
    };
    let dims: [u32; 3] = [n_in as u32, n_out as u32, row_bytes as u32];
    let view_dims = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&dims), 4, 0)
    {
        Ok(v) => v,
        Err(_) => return false,
    };
    let buffers = vec![view_x, view_g, view_u, view_y, view_dims];
    let pipeline = match CudaPipeline::compile_cuda_c_source_cached(
        &dev.ctx,
        Q4K_SOA_FUSED_SWIGLU_SRC,
        Q4K_SOA_FUSED_SWIGLU_ENTRY,
        &[0, 1, 2, 3, 4],
    ) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("cuda_lane|fused_swiglu|compile|{e:?}");
            return false;
        }
    };
    let schedule = crate::wgsl_forge::Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    if let Err(e) = pipeline.dispatch(&buffers, &schedule, n_out.saturating_mul(256)) {
        log::warn!("cuda_lane|fused_swiglu|dispatch|{e:?}");
        return false;
    }
    let y = match dev.ctx.read_buffer_f32(&view_y) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("cuda_lane|fused_swiglu|readback|{e:?}");
            return false;
        }
    };
    out[..n_out].copy_from_slice(&y[..n_out]);
    true
}
