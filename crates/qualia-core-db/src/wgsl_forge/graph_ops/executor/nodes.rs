//! Per-node preparation: allocate a node's buffers (read / read_write slab split), compile its
//! kernel, build its bind group, and package it as a recordable [`GraphPass`]. No GPU work is
//! submitted here — the whole graph is recorded into one encoder and submitted once by the
//! [`driver`](super::driver). [`prepare_node`] dispatches per op-class.

use crate::wgsl_forge::execute::{BindingUsage, BufferView, GraphPass, WgpuComputeContext};
use crate::wgsl_forge::graph_ops::{
    broadcast, elementwise, gather_dequant, reduce, slice, stencil, vision,
};
use crate::wgsl_forge::ir::graph::{DType, EwKind, GraphNode, OpNode};
use crate::wgsl_forge::ir::BuiltinKernel;
use crate::wgsl_forge::{ForgeError, Schedule};

/// Resolve the f32 element count of a tensor view.
fn elems(v: &BufferView) -> usize {
    v.length_bytes / 4
}

/// Allocate a zeroed `n`-element f32 output buffer at `binding` (read_write slab).
fn alloc_out(
    ctx: &mut WgpuComputeContext,
    n: usize,
    binding: u32,
) -> Result<BufferView, ForgeError> {
    let zeros = vec![0.0f32; n.max(1)];
    ctx.allocate_and_write(
        bytemuck::cast_slice(&zeros),
        binding,
        0,
        BindingUsage::StorageReadWrite,
    )
}

/// Allocate a `u32` params buffer at `binding` with the given `usage`.
fn alloc_params(
    ctx: &mut WgpuComputeContext,
    vals: &[u32],
    binding: u32,
    usage: BindingUsage,
) -> Result<BufferView, ForgeError> {
    ctx.allocate_and_write(bytemuck::cast_slice(vals), binding, 0, usage)
}

/// Re-bind a (`Copy`) view to a specific binding slot, keeping its slab/offset/length.
fn at(mut v: BufferView, binding: u32) -> BufferView {
    v.binding = binding;
    v
}

/// Compile a node's kernel, build its bind group, and package it as a [`GraphPass`] to be
/// recorded later (no dispatch here — the whole graph is submitted in one encoder). `bindings`
/// is the node's full binding list (inputs at their slots + the read_write `out` view);
/// `out` is that read_write output. Allocates a fresh read-slab buffer for the GPU→GPU
/// hand-off copy and returns it as the node's output `BufferView` for downstream consumers.
fn record_kernel(
    ctx: &mut WgpuComputeContext,
    source: &str,
    entry: &str,
    bindings: &[BufferView],
    out: BufferView,
    sched: Schedule,
    element_count: usize,
) -> Result<(GraphPass, BufferView), ForgeError> {
    // Cached compile: a held ForgeGraphExecutor re-running the same graph (one decode block
    // per token) compiles each node's shader once total, then pays only bind-group + dispatch.
    let pipeline = ctx.compile_pipeline_cached(source, entry)?;
    let bind_group = ctx.create_compute_bind_group(&pipeline, bindings);
    let workgroups = sched.dispatch_workgroups(element_count);
    // Device-side hand-off: a fresh read-slab buffer the consumer binds read-only. The copy
    // (`out` read_write slab → `read_copy` read slab) is recorded into the shared encoder by
    // `submit_graph`, so it is ordered after this node's pass and before any consumer.
    let read_copy = ctx.allocate_transient(out.length_bytes, 0, 0, BindingUsage::StorageRead)?;
    let pass = GraphPass {
        pipeline,
        bind_group,
        workgroups,
        copy: Some((out, read_copy)),
    };
    Ok((pass, read_copy))
}

/// Prepare one node into a recordable [`GraphPass`], returning it plus the node's output as a
/// READ-slab [`BufferView`] (the hand-off copy, ready to feed the next node). Allocates +
/// uploads the node's buffers and compiles its pipeline, but submits no GPU work — the whole
/// graph is recorded into a single encoder and submitted once (see
/// [`ForgeGraphExecutor::run`](super::ForgeGraphExecutor::run)).
pub(super) fn prepare_node(
    ctx: &mut WgpuComputeContext,
    node: &GraphNode,
    ins: &[BufferView],
) -> Result<(GraphPass, BufferView), ForgeError> {
    match node.op {
        OpNode::Reduce { op, .. } => {
            const WG: u32 = 256;
            let n = elems(&ins[0]);
            let out = alloc_out(ctx, 1, 1)?;
            let params = alloc_params(ctx, &[n as u32, 0, 0, 0], 2, BindingUsage::StorageRead)?;
            let src = reduce::reduce_wgsl(op, WG);
            let sched = Schedule {
                workgroup_size: WG,
                ..Default::default()
            };
            // element_count == WG → one workgroup (the reduce is single-workgroup).
            record_kernel(
                ctx,
                &src,
                reduce::REDUCE_ENTRY,
                &[at(ins[0], 0), out, params],
                out,
                sched,
                WG as usize,
            )
        }
        OpNode::Broadcast { .. } => {
            const WG: u32 = 64;
            let in_len = elems(&ins[0]);
            let out_len = node.out.shape.elements() as usize;
            let out = alloc_out(ctx, out_len, 1)?;
            let params = alloc_params(
                ctx,
                &[in_len as u32, out_len as u32, 0, 0],
                2,
                BindingUsage::StorageRead,
            )?;
            let src = broadcast::broadcast_wgsl(WG);
            let sched = Schedule {
                workgroup_size: WG,
                ..Default::default()
            };
            record_kernel(
                ctx,
                &src,
                broadcast::BROADCAST_ENTRY,
                &[at(ins[0], 0), out, params],
                out,
                sched,
                out_len,
            )
        }
        OpNode::Elementwise { f } => prepare_elementwise(ctx, f, node.n_in, ins),
        OpNode::GatherDequant { scheme, .. } => {
            if scheme != DType::Ternary {
                return Err(ForgeError::Emission(format!(
                    "executor GatherDequant: scheme {scheme:?} unsupported (Ternary only this phase)"
                )));
            }
            const WG: u32 = 64;
            let rows = node.out.shape.dims[0];
            let cols = node.out.shape.dims[1];
            let k_words = cols.div_ceil(16);
            let out_elems = (rows as usize) * (cols as usize);
            let out = alloc_out(ctx, out_elems, 2)?;
            let params =
                alloc_params(ctx, &[rows, cols, k_words, 0], 3, BindingUsage::StorageRead)?;
            let src = gather_dequant::gather_dequant_ternary_wgsl(WG);
            let sched = Schedule {
                workgroup_size: WG,
                ..Default::default()
            };
            // packed (0), scale (1), output (2, read_write), params (3).
            record_kernel(
                ctx,
                &src,
                gather_dequant::GATHER_DEQUANT_ENTRY,
                &[at(ins[0], 0), at(ins[1], 1), out, params],
                out,
                sched,
                out_elems,
            )
        }
        OpNode::MatMul {
            m,
            n,
            k,
            tc,
            trans_b,
        } => {
            // `trans_b` consumes B as `[n,k]` row-major (the native GGUF/p64 weight layout
            // `[out,in]`), computing `C[m,n] = A[m,k] · Bᵀ` with no transpose copy — this is what
            // lets the forge feed on p64 projection weights directly. `tc=true` requests tensor
            // cores: the portable wgpu coopmat tiled GEMM, taken only when the adapter advertises
            // coopmat AND the runtime probe confirms it computes (dormant on wgpu 29.0.3 / #9741,
            // falling to the certified plain GEMM floor). The CUDA WMMA path is reached host-side
            // via `dispatch::gemm_f32_tc` / the CudaCLowerer (P5), not this wgpu executor.
            if trans_b {
                prepare_matmul_trans_b(ctx, m, n, k, ins)
            } else if tc
                && m % 8 == 0
                && n % 8 == 0
                && k % 8 == 0
                && crate::wgsl_forge::dispatch::caps().coopmat
                && crate::wgsl_forge::dispatch::coopmat_usable()
            {
                prepare_matmul_coopmat(ctx, m, n, k, ins)
            } else {
                prepare_matmul_plain(ctx, m, n, k, ins)
            }
        }
        OpNode::Slice { offset, len } => {
            const WG: u32 = 64;
            let out_len = len as usize;
            let out = alloc_out(ctx, out_len, 1)?;
            let params = alloc_params(ctx, &[len, offset, 0, 0], 2, BindingUsage::StorageRead)?;
            let src = slice::slice_wgsl(WG);
            let sched = Schedule {
                workgroup_size: WG,
                ..Default::default()
            };
            // input (0), output (1, read_write), params (2).
            record_kernel(
                ctx,
                &src,
                slice::SLICE_ENTRY,
                &[at(ins[0], 0), out, params],
                out,
                sched,
                out_len,
            )
        }
        OpNode::Rope {
            head_dim,
            pos,
            mode,
            base_bits,
        } => {
            const WG: u32 = 64;
            let n = elems(&ins[0]);
            let out = alloc_out(ctx, n, 1)?;
            // params = [n, head_dim, pos, mode, theta_base_bits] (the real RoPE block). `pos` lives
            // in the buffer, not the source, so the pipeline cache stays warm across tokens.
            let params = alloc_params(
                ctx,
                &[n as u32, head_dim, pos, mode, base_bits],
                2,
                BindingUsage::StorageRead,
            )?;
            let src = stencil::rope_wgsl(WG)?;
            let sched = Schedule {
                workgroup_size: WG,
                ..Default::default()
            };
            record_kernel(
                ctx,
                &src,
                stencil::STENCIL_ENTRY,
                &[at(ins[0], 0), out, params],
                out,
                sched,
                n,
            )
        }
        OpNode::Pool2d {
            c,
            h,
            w,
            kh,
            kw,
            stride_h,
            stride_w,
        } => {
            const WG: u32 = 64;
            let ho = (h - kh) / stride_h + 1;
            let wo = (w - kw) / stride_w + 1;
            let out_elems = (c * ho * wo) as usize;
            let out = alloc_out(ctx, out_elems, 1)?;
            let params = alloc_params(
                ctx,
                &[c, h, w, kh, kw, stride_h, stride_w, ho, wo, 0, 0, 0],
                2,
                BindingUsage::StorageRead,
            )?;
            let src = vision::max_pool2d_wgsl(WG);
            let sched = Schedule {
                workgroup_size: WG,
                ..Default::default()
            };
            record_kernel(
                ctx,
                &src,
                vision::POOL2D_ENTRY,
                &[at(ins[0], 0), out, params],
                out,
                sched,
                out_elems,
            )
        }
        OpNode::Resize2d {
            c,
            h_in,
            w_in,
            h_out,
            w_out,
        } => {
            const WG: u32 = 64;
            let out_elems = (c * h_out * w_out) as usize;
            let out = alloc_out(ctx, out_elems, 1)?;
            let params = alloc_params(
                ctx,
                &[c, h_in, w_in, h_out, w_out, 0, 0, 0],
                2,
                BindingUsage::StorageRead,
            )?;
            let src = vision::resize2d_wgsl(WG);
            let sched = Schedule {
                workgroup_size: WG,
                ..Default::default()
            };
            record_kernel(
                ctx,
                &src,
                vision::RESIZE2D_ENTRY,
                &[at(ins[0], 0), out, params],
                out,
                sched,
                out_elems,
            )
        }
        OpNode::Conv2d {
            c_in,
            c_out,
            h,
            w,
            kh,
            kw,
            stride_h,
            stride_w,
            pad_h,
            pad_w,
        } => {
            const WG: u32 = 64;
            let ho = (h + 2 * pad_h - kh) / stride_h + 1;
            let wo = (w + 2 * pad_w - kw) / stride_w + 1;
            let out_elems = (c_out * ho * wo) as usize;
            // Bindings: input(0), weight(1), bias(2), output(3), params(4)
            let out = alloc_out(ctx, out_elems, 3)?;
            let params = alloc_params(
                ctx,
                &[
                    c_in, c_out, h, w, kh, kw, stride_h, stride_w, pad_h, pad_w, ho, wo,
                ],
                4,
                BindingUsage::StorageRead,
            )?;
            let src = vision::conv2d_wgsl(WG);
            let sched = Schedule {
                workgroup_size: WG,
                ..Default::default()
            };
            record_kernel(
                ctx,
                &src,
                vision::CONV2D_ENTRY,
                &[at(ins[0], 0), at(ins[1], 1), at(ins[2], 2), out, params],
                out,
                sched,
                out_elems,
            )
        }
        other => Err(ForgeError::Emission(format!(
            "executor: op {other:?} not supported"
        ))),
    }
}

/// Plain f32 GEMM node (the always-correct floor): the certified [`BuiltinKernel::Gemm`]
/// WGSL kernel, `C[m×n]=A·B` into the read_write slab. Used for `MatMul.tc=false` and as
/// the fallback when the tensor-core path is unavailable.
fn prepare_matmul_plain(
    ctx: &mut WgpuComputeContext,
    m: u32,
    n: u32,
    k: u32,
    ins: &[BufferView],
) -> Result<(GraphPass, BufferView), ForgeError> {
    const WG: u32 = 64;
    let out_elems = (m as usize) * (n as usize);
    let out = alloc_out(ctx, out_elems, 2)?;
    // GEMM params is a 16-byte UNIFORM block [m, n, k, _pad].
    let params = alloc_params(ctx, &[m, n, k, 0], 3, BindingUsage::Uniform)?;
    let spec = BuiltinKernel::Gemm.spec();
    let sched = Schedule {
        workgroup_size: WG,
        ..Default::default()
    };
    let module = crate::wgsl_forge::emit::emit_wgsl(&spec, sched)?;
    record_kernel(
        ctx,
        &module.source,
        &spec.entry_point,
        &[at(ins[0], 0), at(ins[1], 1), out, params],
        out,
        sched,
        out_elems,
    )
}

/// WGSL for `C[m,n] = A[m,k] · Bᵀ` with **B stored `[n,k]` row-major** (the native GGUF/p64 weight
/// layout `[out,in]`). One invocation per output element. Binding ABI: `a`(0), `b`(1), `c`(2,
/// read_write), `dims`(3, read storage `[m,n,k,_]`).
const GEMM_TRANS_B_ENTRY: &str = "gemm_trans_b_main";
fn gemm_trans_b_wgsl(wg: u32) -> String {
    format!(
        r#"@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> c: array<f32>;
@group(0) @binding(3) var<storage, read> dims: array<u32>;
@compute @workgroup_size({wg})
fn {entry}(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let idx = gid.x;
    let m = dims[0]; let n = dims[1]; let k = dims[2];
    if (idx >= m * n) {{ return; }}
    let row = idx / n; let col = idx % n;
    var acc = 0.0;
    for (var kk = 0u; kk < k; kk = kk + 1u) {{
        acc = acc + a[row * k + kk] * b[col * k + kk];
    }}
    c[idx] = acc;
}}
"#,
        entry = GEMM_TRANS_B_ENTRY,
    )
}

/// Transposed-B GEMM node: `C[m,n] = A[m,k] · Bᵀ`, B bound as `[n,k]` row-major. Lets the forge
/// consume native `[out,in]` GGUF/p64 projection weights without a host transpose.
fn prepare_matmul_trans_b(
    ctx: &mut WgpuComputeContext,
    m: u32,
    n: u32,
    k: u32,
    ins: &[BufferView],
) -> Result<(GraphPass, BufferView), ForgeError> {
    const WG: u32 = 64;
    let out_elems = (m as usize) * (n as usize);
    let out = alloc_out(ctx, out_elems, 2)?;
    let dims = alloc_params(ctx, &[m, n, k, 0], 3, BindingUsage::StorageRead)?;
    let src = gemm_trans_b_wgsl(WG);
    let sched = Schedule {
        workgroup_size: WG,
        ..Default::default()
    };
    record_kernel(
        ctx,
        &src,
        GEMM_TRANS_B_ENTRY,
        &[at(ins[0], 0), at(ins[1], 1), out, dims],
        out,
        sched,
        out_elems,
    )
}

/// Tensor-core f32 GEMM node via the tiled cooperative-matrix kernel
/// ([`matmul_tc_wgsl_tiled`](crate::wgsl_forge::emit::matmul_tc_wgsl_tiled)) — the portable
/// wgpu tensor-core path, kept device-side in the slab model like the plain path. One
/// workgroup (== one subgroup, `@workgroup_size(32)`) per 8×8 output tile. Callers gate this
/// on [`coopmat_usable`](crate::wgsl_forge::dispatch::coopmat_usable), so it runs only where
/// the coopmat multiply actually computes (dormant on wgpu 29.0.3 / #9741).
fn prepare_matmul_coopmat(
    ctx: &mut WgpuComputeContext,
    m: u32,
    n: u32,
    k: u32,
    ins: &[BufferView],
) -> Result<(GraphPass, BufferView), ForgeError> {
    use crate::wgsl_forge::emit::{matmul_tc_wgsl_tiled, MATMUL_TC_TILED_ENTRY};
    let out_elems = (m as usize) * (n as usize);
    // c is the read_write slab output AND the zero-seeded accumulator (the kernel loads it).
    let out = alloc_out(ctx, out_elems, 2)?;
    // dims = [m, n, k] as a u32 storage buffer (binding 3, read slab).
    let dims = alloc_params(ctx, &[m, n, k, 0], 3, BindingUsage::StorageRead)?;
    let src = matmul_tc_wgsl_tiled();
    let num_tiles = ((m / 8) * (n / 8)) as usize;
    let sched = Schedule {
        workgroup_size: 32,
        ..Default::default()
    };
    record_kernel(
        ctx,
        &src,
        MATMUL_TC_TILED_ENTRY,
        &[at(ins[0], 0), at(ins[1], 1), out, dims],
        out,
        sched,
        num_tiles * 32,
    )
}

fn prepare_elementwise(
    ctx: &mut WgpuComputeContext,
    f: EwKind,
    n_in: u8,
    ins: &[BufferView],
) -> Result<(GraphPass, BufferView), ForgeError> {
    const WG: u32 = 64;
    let n = elems(&ins[0]);
    let src = elementwise::elementwise_wgsl(f, WG)?;
    let sched = Schedule {
        workgroup_size: WG,
        ..Default::default()
    };
    match n_in {
        1 => {
            let out = alloc_out(ctx, n, 1)?;
            let params = alloc_params(ctx, &[n as u32, 0, 0, 0], 2, BindingUsage::StorageRead)?;
            record_kernel(
                ctx,
                &src,
                elementwise::EWISE_ENTRY,
                &[at(ins[0], 0), out, params],
                out,
                sched,
                n,
            )
        }
        2 => {
            let out = alloc_out(ctx, n, 2)?;
            let params = alloc_params(ctx, &[n as u32, 0, 0, 0], 3, BindingUsage::StorageRead)?;
            record_kernel(
                ctx,
                &src,
                elementwise::EWISE_ENTRY,
                &[at(ins[0], 0), at(ins[1], 1), out, params],
                out,
                sched,
                n,
            )
        }
        3 => {
            let out = alloc_out(ctx, n, 3)?;
            let params = alloc_params(ctx, &[n as u32, 0, 0, 0], 4, BindingUsage::StorageRead)?;
            record_kernel(
                ctx,
                &src,
                elementwise::EWISE_ENTRY,
                &[at(ins[0], 0), at(ins[1], 1), at(ins[2], 2), out, params],
                out,
                sched,
                n,
            )
        }
        other => Err(ForgeError::Emission(format!(
            "elementwise arity {other} unsupported"
        ))),
    }
}
