// Equality sieve over one NQuin u64 field (subject/predicate/object/context/metadata).
// Output: bitmask, one bit per Quin (same contract as sieve.wgsl).
// field_id: 0=subject 1=predicate 2=object 3=context 4=metadata

struct Quin {
    subject_lo: u32, subject_hi: u32,
    predicate_lo: u32, predicate_hi: u32,
    object_lo: u32, object_hi: u32,
    context_lo: u32, context_hi: u32,
    meta_lo: u32, meta_hi: u32,
    parity_lo: u32, parity_hi: u32,
}

struct Params {
    n: u32,
    field_id: u32,
    match_lo: u32,
    match_hi: u32,
}

@group(0) @binding(0) var<storage, read> quins: array<Quin>;
@group(0) @binding(1) var<storage, read_write> out_bitmask: array<atomic<u32>>;
@group(0) @binding(2) var<uniform> params: Params;

fn field_of(q: Quin, field_id: u32) -> vec2<u32> {
    switch field_id {
        case 0u: { return vec2<u32>(q.subject_lo, q.subject_hi); }
        case 1u: { return vec2<u32>(q.predicate_lo, q.predicate_hi); }
        case 2u: { return vec2<u32>(q.object_lo, q.object_hi); }
        case 3u: { return vec2<u32>(q.context_lo, q.context_hi); }
        default: { return vec2<u32>(q.meta_lo, q.meta_hi); }
    }
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= params.n) {
        return;
    }
    let f = field_of(quins[i], params.field_id);
    if (f.x == params.match_lo && f.y == params.match_hi) {
        let bucket = i / 32u;
        let bit = i % 32u;
        atomicOr(&out_bitmask[bucket], 1u << bit);
    }
}
