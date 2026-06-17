// GPU mirror of `tensor_search_into` — 10D euclidean distance filter (Tier 2 / U1).

struct Tensor10D {
    q: f32,
    v: f32,
    w: f32,
    x: f32,
    y: f32,
    z: f32,
    t: f32,
    alpha: f32,
    mu: f32,
    sigma: f32,
}

struct VolumeParams {
    node_count: u32,
    max_distance: f32,
    stride_floats: u32,
    max_hits: u32,
}

@group(0) @binding(0) var<uniform> query: Tensor10D;
@group(0) @binding(1) var<storage, read> nodes: array<f32>;
@group(0) @binding(2) var<uniform> params: VolumeParams;
@group(0) @binding(3) var<storage, read_write> hits: array<u32>;
@group(0) @binding(4) var<storage, read_write> hit_count: array<atomic<u32>>;

fn node_at(idx: u32) -> Tensor10D {
    let base = idx * params.stride_floats;
    return Tensor10D(
        nodes[base + 0u],
        nodes[base + 1u],
        nodes[base + 2u],
        nodes[base + 3u],
        nodes[base + 4u],
        nodes[base + 5u],
        nodes[base + 6u],
        nodes[base + 7u],
        nodes[base + 8u],
        nodes[base + 9u],
    );
}

fn euclidean_distance(a: Tensor10D, b: Tensor10D) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    let dt = a.t - b.t;
    let da = a.alpha - b.alpha;
    let dm = a.mu - b.mu;
    let ds = a.sigma - b.sigma;
    return sqrt(dx * dx + dy * dy + dz * dz + dt * dt + da * da + dm * dm + ds * ds);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= params.node_count {
        return;
    }
    let node = node_at(idx);
    let dist = euclidean_distance(query, node);
    if dist > params.max_distance {
        return;
    }
    let slot = atomicAdd(&hit_count[0], 1u);
    if slot < params.max_hits {
        hits[slot] = idx;
    }
}