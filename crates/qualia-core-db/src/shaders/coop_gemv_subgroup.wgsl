// Subgroup (wave) reduction variants of coop_gemv / residual.
//
// Concatenated after fused_transformer.wgsl (with a leading `enable subgroups;`) into ONE module, so
// it reuses `coop_row_dot`, the dequant functions, the bindings, and the workgroup vars verbatim —
// the ONLY difference vs shared-memory entries is the final reduction. Built only when the adapter
// advertises SUBGROUP (init.rs).
@compute @workgroup_size(256)
fn coop_gemv_sg(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(subgroup_size) sg_size: u32,
    @builtin(subgroup_invocation_id) sg_lane: u32,
) {
    let m = wg_id.y;
    let batch = max(params.n_batch, 1u);
    if m >= batch {
        return;
    }
    let row = wg_id.x;
    if row >= params.n_out {
        return;
    }
    let t = lid.x;
    let in_stride = select(params.n_in, params.in_row_stride, params.in_row_stride > 0u);
    let out_stride = select(params.n_out, params.out_row_stride, params.out_row_stride > 0u);
    let in_base = m * in_stride;
    let out_base = m * out_stride;

    let acc = coop_row_dot(row, t, in_base);
    let sg_sum = subgroupAdd(acc);
    if sg_lane == 0u {
        coop_partial[t / sg_size] = sg_sum;
    }
    workgroupBarrier();
    if t == 0u {
        let n_sg = (COOP_WG + sg_size - 1u) / sg_size;
        var total = 0.0;
        for (var s = 0u; s < n_sg; s = s + 1u) {
            total = total + coop_partial[s];
        }
        output[out_base + row] = total;
    }
}

// Residual + subgroup reduce: output = residual + W·x (O/down path in resident mega-pass).
@compute @workgroup_size(256)
fn coop_gemv_residual_sg(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(subgroup_size) sg_size: u32,
    @builtin(subgroup_invocation_id) sg_lane: u32,
) {
    let m = wg_id.y;
    let batch = max(params.n_batch, 1u);
    if m >= batch {
        return;
    }
    let row = wg_id.x;
    if row >= params.n_out {
        return;
    }
    let t = lid.x;
    let in_stride = select(params.n_in, params.in_row_stride, params.in_row_stride > 0u);
    let out_stride = select(params.n_out, params.out_row_stride, params.out_row_stride > 0u);
    let in_base = m * in_stride;
    let out_base = m * out_stride;

    let acc = coop_row_dot(row, t, in_base);
    let sg_sum = subgroupAdd(acc);
    if sg_lane == 0u {
        coop_partial[t / sg_size] = sg_sum;
    }
    workgroupBarrier();
    if t == 0u {
        let n_sg = (COOP_WG + sg_size - 1u) / sg_size;
        var total = 0.0;
        for (var s = 0u; s < n_sg; s = s + 1u) {
            total = total + coop_partial[s];
        }
        output[out_base + row] = residual[out_base + row] + total;
    }
}
