// Dual Q4_K_SOA GEMV: one workgroup per output row, two weight matrices (K+V)
// → two outputs. Cuts 2 dispatches → 1 in resident decode.
//
// Single-row: barrier-free global act (lane t only). Full-act LDS A/B lost occupancy.
// Dispatch: (n_out, batch, 1) workgroups of 256.
// Bindings:
//   0 input, 1 W_a, 2 params, 3 out_a, 4 W_b, 5 out_b

struct GemmParams {
    n_in: u32,
    n_out: u32,
    weight_ggml_type: u32,
    weight_row_elems: u32,
    weight_byte_len: u32,
    n_batch: u32,
    in_row_stride: u32,
    out_row_stride: u32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> weight_a: array<u32>;
@group(0) @binding(2) var<uniform> params: GemmParams;
@group(0) @binding(3) var<storage, read_write> out_a: array<f32>;
@group(0) @binding(4) var<storage, read> weight_b: array<u32>;
@group(0) @binding(5) var<storage, read_write> out_b: array<f32>;

const BLOCK_Q4K_SOA_BYTES: u32 = 160u;
const BLOCK_Q4K_ELEMS: u32 = 256u;
const GGML_TYPE_Q4_K_SOA: u32 = 112u;
const COOP_WG: u32 = 256u;

// Reduce scratch only (act stays global — each lane owns one column).
var<workgroup> partial_a: array<f32, 256>;
var<workgroup> partial_b: array<f32, 256>;

fn f16_to_f32(bits: u32) -> f32 {
    let s = (bits >> 15u) & 1u;
    var e = (bits >> 10u) & 0x1Fu;
    let f = bits & 0x3FFu;
    if e == 0u {
        if f == 0u { return select(0.0, -0.0, s == 1u); }
        e = 1u;
        var v = f32(f) / 1024.0;
        v *= exp2(-14.0);
        return select(v, -v, s == 1u);
    }
    if e == 31u { return select(1e30, -1e30, s == 1u); }
    var v = 1.0 + f32(f) / 1024.0;
    v *= exp2(f32(i32(e) - 15));
    return select(v, -v, s == 1u);
}

fn weight_row_bytes() -> u32 {
    return (params.weight_row_elems / BLOCK_Q4K_ELEMS) * BLOCK_Q4K_SOA_BYTES;
}

@compute @workgroup_size(256)
fn coop_gemv_dual(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(subgroup_size) sg_size: u32,
    @builtin(subgroup_invocation_id) sg_lane: u32,
) {
    let m = wg_id.y;
    let batch = max(params.n_batch, 1u);
    if m >= batch { return; }
    let row = wg_id.x;
    if row >= params.n_out { return; }
    let t = lid.x;
    let in_stride = select(params.n_in, params.in_row_stride, params.in_row_stride > 0u);
    let out_stride = select(params.n_out, params.out_row_stride, params.out_row_stride > 0u);
    let in_base = m * in_stride;
    let out_base = m * out_stride;

    var acc_a = 0.0;
    var acc_b = 0.0;
    let n_blocks = params.n_in / BLOCK_Q4K_ELEMS;
    let row_base = row * weight_row_bytes();
    let sub = t / 32u;
    let group = t / 64u;
    let local = t % 64u;
    let scale_pair = sub >> 1u;
    let scale_hi = (sub & 1u) == 1u;
    let q_off = group * 32u;

    // Barrier-free FMA: each lane only needs its own activation element.
    for (var b = 0u; b < n_blocks; b = b + 1u) {
        let x = input[in_base + b * BLOCK_Q4K_ELEMS + t];
        let block_base = row_base + b * BLOCK_Q4K_SOA_BYTES;
        let d_word_a = weight_a[(block_base + 128u) / 4u + scale_pair];
        let m_word_a = weight_a[(block_base + 144u) / 4u + scale_pair];
        let d_word_b = weight_b[(block_base + 128u) / 4u + scale_pair];
        let m_word_b = weight_b[(block_base + 144u) / 4u + scale_pair];
        let dsuba = f16_to_f32(select(d_word_a & 0xFFFFu, d_word_a >> 16u, scale_hi));
        let msuba = f16_to_f32(select(m_word_a & 0xFFFFu, m_word_a >> 16u, scale_hi));
        let dsubb = f16_to_f32(select(d_word_b & 0xFFFFu, d_word_b >> 16u, scale_hi));
        let msubb = f16_to_f32(select(m_word_b & 0xFFFFu, m_word_b >> 16u, scale_hi));
        var niba: u32;
        var nibb: u32;
        if local < 32u {
            let ba = block_base + q_off + local;
            niba = (weight_a[ba >> 2u] >> ((ba & 3u) * 8u)) & 0xFu;
            nibb = (weight_b[ba >> 2u] >> ((ba & 3u) * 8u)) & 0xFu;
        } else {
            let ba = block_base + q_off + (local - 32u);
            niba = ((weight_a[ba >> 2u] >> ((ba & 3u) * 8u)) & 0xFFu) >> 4u;
            nibb = ((weight_b[ba >> 2u] >> ((ba & 3u) * 8u)) & 0xFFu) >> 4u;
        }
        acc_a = acc_a + (dsuba * f32(niba) - msuba) * x;
        acc_b = acc_b + (dsubb * f32(nibb) - msubb) * x;
    }

    // Subgroup reduce (matches coop_gemv_sg / fused FFN) — far fewer barriers than
    // a 256-wide shared-memory tree.
    let sa = subgroupAdd(acc_a);
    let sb = subgroupAdd(acc_b);
    if sg_lane == 0u {
        partial_a[t / sg_size] = sa;
        partial_b[t / sg_size] = sb;
    }
    workgroupBarrier();
    if t == 0u {
        let n_sg = (COOP_WG + sg_size - 1u) / sg_size;
        var ta = 0.0;
        var tb = 0.0;
        for (var s = 0u; s < n_sg; s = s + 1u) {
            ta = ta + partial_a[s];
            tb = tb + partial_b[s];
        }
        out_a[out_base + row] = ta;
        out_b[out_base + row] = tb;
    }
}

// Multi-row dual: one WG owns DUAL_ROWS consecutive K/V rows; act tile shared.
// Dispatch: (ceil(n_out / DUAL_ROWS), batch, 1). Same bindings as coop_gemv_dual.
// 3B GQA: kv_dim=1024 → 256 WGs vs 1024 (4× fewer act reloads for K+V).
const DUAL_ROWS: u32 = 4u;

@compute @workgroup_size(256)
fn coop_gemv_dual_mr(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(subgroup_size) sg_size: u32,
    @builtin(subgroup_invocation_id) sg_lane: u32,
) {
    let m = wg_id.y;
    let batch = max(params.n_batch, 1u);
    if m >= batch { return; }
    let row0 = wg_id.x * DUAL_ROWS;
    if row0 >= params.n_out { return; }
    let t = lid.x;
    let in_stride = select(params.n_in, params.in_row_stride, params.in_row_stride > 0u);
    let out_stride = select(params.n_out, params.out_row_stride, params.out_row_stride > 0u);
    let in_base = m * in_stride;
    let out_base = m * out_stride;

    var acc_a0 = 0.0; var acc_a1 = 0.0; var acc_a2 = 0.0; var acc_a3 = 0.0;
    var acc_b0 = 0.0; var acc_b1 = 0.0; var acc_b2 = 0.0; var acc_b3 = 0.0;
    let n_blocks = params.n_in / BLOCK_Q4K_ELEMS;
    let rb = weight_row_bytes();
    let sub = t / 32u;
    let group = t / 64u;
    let local = t % 64u;
    let scale_pair = sub >> 1u;
    let scale_hi = (sub & 1u) == 1u;
    let q_off = group * 32u;

    // Barrier-free: lane t reuses one global act element across DUAL_ROWS weight rows.
    for (var b = 0u; b < n_blocks; b = b + 1u) {
        let x = input[in_base + b * BLOCK_Q4K_ELEMS + t];
        let col_base = b * BLOCK_Q4K_SOA_BYTES;
        for (var r = 0u; r < DUAL_ROWS; r = r + 1u) {
            let row = row0 + r;
            if row >= params.n_out { continue; }
            let block_base = row * rb + col_base;
            let d_word_a = weight_a[(block_base + 128u) / 4u + scale_pair];
            let m_word_a = weight_a[(block_base + 144u) / 4u + scale_pair];
            let d_word_b = weight_b[(block_base + 128u) / 4u + scale_pair];
            let m_word_b = weight_b[(block_base + 144u) / 4u + scale_pair];
            let dsuba = f16_to_f32(select(d_word_a & 0xFFFFu, d_word_a >> 16u, scale_hi));
            let msuba = f16_to_f32(select(m_word_a & 0xFFFFu, m_word_a >> 16u, scale_hi));
            let dsubb = f16_to_f32(select(d_word_b & 0xFFFFu, d_word_b >> 16u, scale_hi));
            let msubb = f16_to_f32(select(m_word_b & 0xFFFFu, m_word_b >> 16u, scale_hi));
            var niba: u32;
            var nibb: u32;
            if local < 32u {
                let ba = block_base + q_off + local;
                niba = (weight_a[ba >> 2u] >> ((ba & 3u) * 8u)) & 0xFu;
                nibb = (weight_b[ba >> 2u] >> ((ba & 3u) * 8u)) & 0xFu;
            } else {
                let ba = block_base + q_off + (local - 32u);
                niba = ((weight_a[ba >> 2u] >> ((ba & 3u) * 8u)) & 0xFFu) >> 4u;
                nibb = ((weight_b[ba >> 2u] >> ((ba & 3u) * 8u)) & 0xFFu) >> 4u;
            }
            let va = (dsuba * f32(niba) - msuba) * x;
            let vb = (dsubb * f32(nibb) - msubb) * x;
            if r == 0u { acc_a0 = acc_a0 + va; acc_b0 = acc_b0 + vb; }
            else if r == 1u { acc_a1 = acc_a1 + va; acc_b1 = acc_b1 + vb; }
            else if r == 2u { acc_a2 = acc_a2 + va; acc_b2 = acc_b2 + vb; }
            else { acc_a3 = acc_a3 + va; acc_b3 = acc_b3 + vb; }
        }
    }

    // Per-row subgroup reduce + write.
    var aa: array<f32, 4>;
    var bb: array<f32, 4>;
    aa[0] = acc_a0; aa[1] = acc_a1; aa[2] = acc_a2; aa[3] = acc_a3;
    bb[0] = acc_b0; bb[1] = acc_b1; bb[2] = acc_b2; bb[3] = acc_b3;
    for (var r = 0u; r < DUAL_ROWS; r = r + 1u) {
        let row = row0 + r;
        let row_ok = row < params.n_out;
        let sa = subgroupAdd(aa[r]);
        let sb = subgroupAdd(bb[r]);
        if sg_lane == 0u {
            partial_a[t / sg_size] = sa;
            partial_b[t / sg_size] = sb;
        }
        workgroupBarrier();
        if t == 0u && row_ok {
            let n_sg = (COOP_WG + sg_size - 1u) / sg_size;
            var ta = 0.0;
            var tb = 0.0;
            for (var s = 0u; s < n_sg; s = s + 1u) {
                ta = ta + partial_a[s];
                tb = tb + partial_b[s];
            }
            out_a[out_base + row] = ta;
            out_b[out_base + row] = tb;
        }
        workgroupBarrier();
    }
}
