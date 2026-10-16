// Holographic 3D Engine for Semantic Graphs — Procedural Volumetric Light Field.
//
// Architected per `docs/Holographic 3D Engine for Semantic Graphs.md`:
// 1. Proxy Bounding Volumes: Instanced unit cuboids act as "projector lenses".
//    The GPU processes the proxy instantly with zero CPU triangle meshing.
// 2. SVG-like Procedural Shading: Resolution-independent mathematical functions
//    calculate continuous contours, telemetry fringes, and density.
// 3. Volumetric Raymarching via Signed Distance Fields (SDFs):
//    Evaluates smooth-min organic primitives (spheres, capsules, rounded boxes,
//    metaball harmonics) directly along camera rays.
// 4. Wave Interference for N3Logic Reasoning:
//    Constructive interference generates glowing focal cores and relational
//    bridges where assertions align (high epistemic q). Destructive interference
//    creates ethereal voids and transparency.
// 5. EMF Frequency Emission:
//    Direct mapping of semantic field frequency and phase into the CIE 1931
//    XYZ -> linear sRGB spectrum via spectral.wgsl.
//
// @group(0) Camera + Observer standpoint · @group(1) Holographic node storage

struct Camera {
    view_projection: mat4x4<f32>,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    tensor_mode: u32,
    _padding0: vec4<f32>, // frame time, eye xyz
    _padding1: vec4<f32>,
    _padding2: vec4<f32>,
};

struct ObserverStandpoint {
    standpoint_hash: vec2<u32>,
    session_nonce: vec2<u32>,
    epistemic_q: f32,
    t_slice: f32,
    t_window: f32,
    deontic_lane: u32,
    standpoint_class: u32,
    fabric_gate: u32,
    _padding0: vec2<f32>,
    _padding1: vec4<f32>,
    _padding2: vec4<f32>,
    _padding3: vec4<f32>,
    _padding4: vec4<f32>,
    _padding5: vec4<f32>,
};

// 96-byte std140/std430 aligned GPU representation of a semantic holographic entity.
struct HolographicNodeGpu {
    // World space position xyz, bounding sphere / proxy scale w
    position_scale: vec4<f32>,
    // Half extents xyz, primitive type w (0: Sphere, 1: RoundedBox, 2: Capsule, 3: Metaball, 4: InterferenceBridge)
    half_extents_primitive: vec4<f32>,
    // Base frequency x, amplitude y, phase z, manifold curvature w
    frequency_amplitude: vec4<f32>,
    // 10D Manifold parameters: epistemic_q x, topological_spin y, temporal_decay z, density_threshold w
    manifold_tensor: vec4<f32>,
    // Target endpoint xyz (for relational bridges), coherence weight w
    connection_endpoint: vec4<f32>,
    // Emissive tint rgba
    emissive_color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<uniform> observer: ObserverStandpoint;
@group(1) @binding(0) var<storage, read> nodes: array<HolographicNodeGpu>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) @interpolate(flat) node_index: u32,
};

const CUBE_VERTICES: array<vec3<f32>, 8> = array<vec3<f32>, 8>(
    vec3<f32>(-1.0, -1.0, -1.0),
    vec3<f32>( 1.0, -1.0, -1.0),
    vec3<f32>( 1.0,  1.0, -1.0),
    vec3<f32>(-1.0,  1.0, -1.0),
    vec3<f32>(-1.0, -1.0,  1.0),
    vec3<f32>( 1.0, -1.0,  1.0),
    vec3<f32>( 1.0,  1.0,  1.0),
    vec3<f32>(-1.0,  1.0,  1.0)
);

const CUBE_INDICES: array<u32, 36> = array<u32, 36>(
    0u, 1u, 2u,  0u, 2u, 3u,
    4u, 6u, 5u,  4u, 7u, 6u,
    4u, 0u, 3u,  4u, 3u, 7u,
    1u, 5u, 6u,  1u, 6u, 2u,
    3u, 2u, 6u,  3u, 6u, 7u,
    4u, 5u, 1u,  4u, 1u, 0u
);

const TWO_PI: f32 = 6.283185307;

@vertex
fn vertex_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    let node = nodes[instance_index];
    let corner = CUBE_VERTICES[CUBE_INDICES[vertex_index]];
    let proxy_extents = node.half_extents_primitive.xyz * 1.35 + vec3<f32>(0.08);
    let world_pos = node.position_scale.xyz + corner * proxy_extents;

    var output: VertexOutput;
    output.clip_position = camera.view_projection * vec4<f32>(world_pos, 1.0);
    output.world_pos = world_pos;
    output.node_index = instance_index;
    return output;
}

// ── Ray-Box Slab Intersection ───────────────────────────────────────────────

fn ray_box_intersect(ro: vec3<f32>, rd: vec3<f32>, half_size: vec3<f32>) -> vec2<f32> {
    let inv_d = 1.0 / rd;
    let t0 = (-half_size - ro) * inv_d;
    let t1 = (half_size - ro) * inv_d;
    let tmin = min(t0, t1);
    let tmax = max(t0, t1);
    let t_near = max(max(tmin.x, tmin.y), tmin.z);
    let t_far = min(min(tmax.x, tmax.y), tmax.z);
    return vec2<f32>(t_near, t_far);
}

// ── Signed Distance Field Primitives (sdf_ops.rs GPU analogues) ─────────────

fn sdf_sphere(p: vec3<f32>, r: f32) -> f32 {
    return length(p) - r;
}

fn sdf_rounded_box(p: vec3<f32>, half_extents: vec3<f32>, r: f32) -> f32 {
    let q = abs(p) - (half_extents - vec3<f32>(r));
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0) - r;
}

fn sdf_capsule(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>, r: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-6), 0.0, 1.0);
    return length(pa - ba * h) - r;
}

// Polynomial smooth-min (C¹ continuous blending from sdf_ops::sdf_smooth_union)
fn sdf_smooth_union(d1: f32, d2: f32, k: f32) -> f32 {
    if (k <= 0.0) {
        return min(d1, d2);
    }
    let h = clamp(0.5 + 0.5 * (d2 - d1) / k, 0.0, 1.0);
    return mix(d2, d1, h) - k * h * (1.0 - h);
}

// Evaluates the semantic SDF inside the local node coordinate frame.
fn evaluate_node_sdf(local_p: vec3<f32>, node: HolographicNodeGpu, time: f32) -> f32 {
    let prim_type = u32(round(node.half_extents_primitive.w));
    let half_dim = node.half_extents_primitive.xyz;
    let r = min(half_dim.x, min(half_dim.y, half_dim.z)) * 0.75;

    switch (prim_type) {
        case 0u: {
            // Sphere primitive with harmonic pulsation
            let pulse = 0.04 * sin(node.frequency_amplitude.x * time + node.frequency_amplitude.z);
            return sdf_sphere(local_p, r + pulse);
        }
        case 1u: {
            // Rounded box primitive (corridor structures, campsite platforms)
            return sdf_rounded_box(local_p, half_dim, r * 0.4);
        }
        case 2u: {
            // Segment capsule (fauna bones, tree trunks, guide poles)
            let half_h = half_dim.y;
            let a = vec3<f32>(0.0, -half_h, 0.0);
            let b = vec3<f32>(0.0,  half_h, 0.0);
            return sdf_capsule(local_p, a, b, r * 0.5);
        }
        case 3u: {
            // Organic metaball field (foliage canopies, flora organ clusters)
            let spin = node.manifold_tensor.y * time * 0.8;
            let c0 = vec3<f32>(sin(spin) * r * 0.35, 0.0, cos(spin) * r * 0.35);
            let c1 = vec3<f32>(-c0.x, r * 0.25, -c0.z);
            let c2 = vec3<f32>(0.0, -r * 0.2, 0.0);
            let d0 = sdf_sphere(local_p - c0, r * 0.65);
            let d1 = sdf_sphere(local_p - c1, r * 0.55);
            let d2 = sdf_sphere(local_p - c2, r * 0.70);
            let blend_k = r * 0.6;
            return sdf_smooth_union(sdf_smooth_union(d0, d1, blend_k), d2, blend_k);
        }
        default: {
            // Relational interference bridge (waveguide linking this node to target endpoint)
            let local_target = node.connection_endpoint.xyz - node.position_scale.xyz;
            let bridge = sdf_capsule(local_p, vec3<f32>(0.0), local_target, r * 0.35);
            let core = sdf_sphere(local_p, r * 0.5);
            return sdf_smooth_union(core, bridge, r * 0.4);
        }
    }
}

// ── Surface Normal Calculation via Tetrahedron Gradients ────────────────────

fn evaluate_normal(local_p: vec3<f32>, node: HolographicNodeGpu, time: f32) -> vec3<f32> {
    let eps = 0.0035;
    let k = vec2<f32>(1.0, -1.0);
    let n = k.xyy * evaluate_node_sdf(local_p + k.xyy * eps, node, time)
          + k.yyx * evaluate_node_sdf(local_p + k.yyx * eps, node, time)
          + k.yxy * evaluate_node_sdf(local_p + k.yxy * eps, node, time)
          + k.xxx * evaluate_node_sdf(local_p + k.xxx * eps, node, time);
    return normalize(n);
}

// ── Wave Interference & Holographic Emission ────────────────────────────────

fn evaluate_interference_field(
    local_p: vec3<f32>,
    node: HolographicNodeGpu,
    time: f32
) -> f32 {
    let freq = node.frequency_amplitude.x;
    let phase = node.frequency_amplitude.z;
    let k_vec1 = vec3<f32>(freq * 1.8, freq * 2.4, freq * 1.5);
    let wave1 = cos(dot(k_vec1, local_p) - freq * time * 2.0 + phase);

    // Secondary wave driven by epistemic certainty q
    let q = clamp(node.manifold_tensor.x, 0.0, 1.0);
    let k_vec2 = vec3<f32>(-freq * 1.5, freq * 2.1, -freq * 1.9);
    let wave2 = cos(dot(k_vec2, local_p) - freq * time * 3.14 + phase * 1.618);

    // Coherence superposition: constructive alignment where assertions agree
    let superposed = (wave1 + q * wave2) / (1.0 + q);
    return superposed;
}

// Maps phase (radians) to σ in [0, 1) for spectral coloring.
fn phase_to_sigma(phase: f32) -> f32 {
    return fract(phase / TWO_PI);
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let node = nodes[input.node_index];
    let time = camera._padding0.x;
    let camera_eye = camera._padding0.yzw;

    // Ray setup in local node coordinates
    let ro_world = camera_eye;
    let rd_world = normalize(input.world_pos - ro_world);
    let local_ro = ro_world - node.position_scale.xyz;
    let local_rd = rd_world;

    let proxy_extents = node.half_extents_primitive.xyz * 1.35 + vec3<f32>(0.08);
    let hit_bounds = ray_box_intersect(local_ro, local_rd, proxy_extents);
    let t_near = max(hit_bounds.x, 0.0);
    let t_far = hit_bounds.y;

    if (t_far <= t_near) {
        discard;
    }

    // Sphere-tracing ray march through the bounding volume
    var t = t_near;
    var hit_dist = 1e6;
    var hit_p = vec3<f32>(0.0);
    var hit = false;
    let max_steps = 42u;
    let threshold = 0.003;

    for (var i = 0u; i < max_steps; i = i + 1u) {
        let p = local_ro + local_rd * t;
        let d = evaluate_node_sdf(p, node, time);
        if (d < threshold) {
            hit = true;
            hit_p = p;
            hit_dist = d;
            break;
        }
        t = t + max(d * 0.85, 0.008);
        if (t > t_far) {
            break;
        }
    }

    if (!hit) {
        discard;
    }

    // Surface normal & Fresnel rim glow
    let normal = evaluate_normal(hit_p, node, time);
    let view_dir = -local_rd;
    let ndotv = abs(dot(normal, view_dir));
    let curvature = clamp(node.frequency_amplitude.w, 0.1, 4.0);
    let fresnel = pow(1.0 - ndotv, 2.2) * curvature;

    // N3Logic Wave interference pattern
    let interference = evaluate_interference_field(hit_p, node, time);
    let constructive = max(0.0, interference);

    // SVG-like procedural telemetry scanlines & latitude rings
    let scanline = 0.85 + 0.15 * sin(hit_p.y * 55.0 - time * 6.0);
    let latitude_ring = smoothstep(0.04, 0.0, abs(fract(hit_p.y * 4.0) - 0.5));

    // EMF spectral emission frequency
    let sigma_val = phase_to_sigma(node.frequency_amplitude.z + constructive * 0.25 + time * 0.05);
    let spectral_rgb = sigma_to_linear_rgb(sigma_val);

    // Epistemic certainty q controls emission sharpness and transparency
    let q = clamp(node.manifold_tensor.x, 0.0, 1.0);
    let certainty_gain = mix(0.45, 1.35, q);
    let amplitude = node.frequency_amplitude.y;

    // Direct EMF radiant color output (HDR)
    let base_rgb = mix(node.emissive_color.rgb, spectral_rgb, 0.65);
    let emission_hdr = base_rgb * amplitude * (0.6 + fresnel * 1.5 + latitude_ring * 0.8)
                     * (1.0 + constructive * 0.7) * scanline * certainty_gain;

    let alpha = clamp(0.35 + fresnel * 0.65 + latitude_ring * 0.35 + constructive * 0.25, 0.15, 1.0);

    return vec4<f32>(emission_hdr, alpha * node.emissive_color.a);
}
