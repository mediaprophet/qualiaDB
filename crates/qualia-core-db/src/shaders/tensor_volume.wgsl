// GPU mirror of `tensor_search_into` — 10D distance filter (Tier 2 / U1).
// Faithful port of `Tensor10D::full_distance`: the metric is chosen by the QUERY's
// `v` topology class (0 euclidean, 1 cyclic/toroidal, 2 hyperbolic, else boundary),
// so the GPU path and the CPU fallback (`Q42TensorView::tensor_search_into`) agree for
// ALL v — not only v == 0. Keep this in lockstep with `tensor/mod.rs`.

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

// v == 0: 7-dimensional euclidean over x,y,z,t,alpha,mu,sigma.
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

// v == 1: cyclic / toroidal over x,y,z (modulo-1 wrap).
fn cyclic_distance(a: Tensor10D, b: Tensor10D) -> f32 {
    let ax = abs(a.x - b.x);
    let ay = abs(a.y - b.y);
    let az = abs(a.z - b.z);
    let dx = min(ax, 1.0 - ax);
    let dy = min(ay, 1.0 - ay);
    let dz = min(az, 1.0 - az);
    return sqrt(dx * dx + dy * dy + dz * dz);
}

// v == 2: hyperbolic (exponential hierarchy) over x,y,z. WGSL `log` is natural log.
fn hyperbolic_distance(a: Tensor10D, b: Tensor10D) -> f32 {
    let dx = abs(a.x - b.x);
    let dy = abs(a.y - b.y);
    let dz = abs(a.z - b.z);
    return log(exp(dx) + exp(dy) + exp(dz));
}

// else: boundary clique — 0 if the topology class matches, 1 otherwise.
fn boundary_distance(a: Tensor10D, b: Tensor10D) -> f32 {
    if a.v == b.v {
        return 0.0;
    }
    return 1.0;
}

// Dispatch on the QUERY's topology class (mirrors Tensor10D::full_distance).
fn metric_distance(q: Tensor10D, n: Tensor10D) -> f32 {
    let cls = u32(q.v);
    if cls == 0u {
        return euclidean_distance(q, n);
    } else if cls == 1u {
        return cyclic_distance(q, n);
    } else if cls == 2u {
        return hyperbolic_distance(q, n);
    }
    return boundary_distance(q, n);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= params.node_count {
        return;
    }
    let node = node_at(idx);
    let dist = metric_distance(query, node);
    if dist > params.max_distance {
        return;
    }
    let slot = atomicAdd(&hit_count[0], 1u);
    if slot < params.max_hits {
        hits[slot] = idx;
    }
}