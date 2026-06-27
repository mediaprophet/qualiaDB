// Subgroup (wave) reduction variant of coop_gemv.
//
// Concatenated after fused_transformer.wgsl (with a leading `enable subgroups;`) into ONE module, so
// it reuses `coop_row_dot`, the dequant functions, the bindings, and the workgroup vars verbatim —
// the ONLY difference vs `coop_gemv` is the final reduction. The 8-step barrier-synced shared-memory
// tree is replaced by one `subgroupAdd` per subgroup (a barrier-free in-register wave reduce) plus a
// tiny cross-subgroup combine. Built only when the adapter advertises the SUBGROUP feature
// (init.rs); the shared-memory `coop_gemv` remains the universal fallback (browser/WebGPU, or any
// adapter without subgroups).
@compute @workgroup_size(256)
fn coop_gemv_sg(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(subgroup_size) sg_size: u32,
    @builtin(subgroup_invocation_id) sg_lane: u32,
) {
    // row / m are uniform across the workgroup (from workgroup_id) → the early returns and the
    // barriers inside coop_row_dot / below are in uniform control flow.
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

    // All 256 threads reconverge here (the dequant branch condition is uniform), so the subgroup op
    // below runs in uniform control flow as required.
    let acc = coop_row_dot(row, t, in_base);

    // Wave reduce: each subgroup sums its own lanes in registers (no barrier). Lane 0 publishes the
    // subgroup partial to shared memory; thread 0 sums the ≤32 subgroup partials. Subgroups are
    // assigned contiguously over the 1D workgroup, so subgroup index == t / sg_size and lane-0 slots
    // (t = k·sg_size → k) are distinct.
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
