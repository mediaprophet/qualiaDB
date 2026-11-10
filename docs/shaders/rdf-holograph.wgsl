// QualiaDB cinema RDF holograph — WebGPU desktop path.
// Bind layout matches the engine Camera/Observer + node SSBO.
// Engine holographic.wgsl stays lockstep; this file is RDF-theatre only.

struct Camera {
    view_projection: mat4x4<f32>,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    tensor_mode: u32,
    _padding0: vec4<f32>,
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

struct HolographicNodeGpu {
    position_scale: vec4<f32>,
    half_extents_primitive: vec4<f32>,
    frequency_amplitude: vec4<f32>,
    manifold_tensor: vec4<f32>,
    connection_endpoint: vec4<f32>,
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

fn hue_rgb(h: f32) -> vec3<f32> {
    let t = fract(h) * 6.0;
    let x = 1.0 - abs(t - 2.0 * floor(t / 2.0) - 1.0);
    if t < 1.0 { return vec3<f32>(1.0, x, 0.0); }
    if t < 2.0 { return vec3<f32>(x, 1.0, 0.0); }
    if t < 3.0 { return vec3<f32>(0.0, 1.0, x); }
    if t < 4.0 { return vec3<f32>(0.0, x, 1.0); }
    if t < 5.0 { return vec3<f32>(x, 0.0, 1.0); }
    return vec3<f32>(1.0, 0.0, x);
}

fn sdf_sphere(p: vec3<f32>, r: f32) -> f32 { return length(p) - r; }

fn sdf_octahedron(p: vec3<f32>, s: f32) -> f32 {
    let a = abs(p);
    return (a.x + a.y + a.z - s) * 0.57735027;
}

fn sdf_diamond(p: vec3<f32>, s: f32) -> f32 {
    let a = abs(p);
    return sdf_octahedron(vec3<f32>(a.x, a.y * 0.70, a.z), s);
}

fn sdf_shard(p: vec3<f32>, s: f32) -> f32 {
    return sdf_octahedron(vec3<f32>(p.x, p.y * 2.15, p.z * 1.65), s);
}

fn sdf_capsule(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>, r: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-5), 0.0, 1.0);
    return length(pa - ba * h) - r;
}

fn rotate_y(p: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec3<f32>(p.x * c - p.z * s, p.y, p.x * s + p.z * c);
}

@vertex
fn vertex_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    let node = nodes[instance_index];
    let time = camera._padding0.x;
    let w_angle = time * 0.055;
    let grow = 0.90 + 0.10 * smoothstep(0.0, 1.15, time - f32(instance_index) * 0.040);
    let breath = 1.0 + 0.05 * sin(time * node.frequency_amplitude.x * 0.55 + node.frequency_amplitude.z);
    let corner = CUBE_VERTICES[CUBE_INDICES[vertex_index]];
    let kind = i32(round(node.manifold_tensor.y));
    let s = max(node.position_scale.w, 0.28) * grow * breath;
    let pad = 0.42;
    var extents: vec3<f32>;
    if kind == 5 {
        extents = vec3<f32>(s * 1.08);
    } else if kind >= 3 {
        extents = (node.half_extents_primitive.xyz + vec3<f32>(pad * 0.55)) * grow * breath;
    } else if kind == 1 {
        extents = vec3<f32>(s * 1.05 + pad, s * 1.50 + pad, s * 1.05 + pad);
    } else if kind == 2 {
        let ss = s * 0.95;
        extents = vec3<f32>(ss + pad, ss / 2.15 + pad, ss / 1.65 + pad);
    } else {
        extents = vec3<f32>(s * 0.90 + pad);
    }
    let rotated = rotate_y(node.position_scale.xyz, w_angle);
    let world_pos = rotated + corner * extents;
    var output: VertexOutput;
    output.clip_position = camera.view_projection * vec4<f32>(world_pos, 1.0);
    output.world_pos = world_pos;
    output.node_index = instance_index;
    return output;
}

struct HoloFragmentOut {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

@fragment
fn fragment_main(input: VertexOutput) -> HoloFragmentOut {
    let node = nodes[input.node_index];
    let time = camera._padding0.x;
    let camera_eye = camera._padding0.yzw;
    let w_angle = time * 0.055;
    let kind = i32(round(node.manifold_tensor.y));
    let prim = i32(round(node.half_extents_primitive.w));
    let grow = 0.90 + 0.10 * smoothstep(0.0, 1.15, time - f32(input.node_index) * 0.040);
    let breath = 1.0 + 0.045 * sin(time * node.frequency_amplitude.x * 0.55 + node.frequency_amplitude.z);
    let scale = max(node.position_scale.w, 0.28) * grow * breath;
    let center = rotate_y(node.position_scale.xyz, w_angle);
    let endp = rotate_y(node.connection_endpoint.xyz, w_angle);

    let rd = normalize(input.world_pos - camera_eye);
    let ro = camera_eye - center;

    let t_hit = length(input.world_pos - camera_eye);
    var t = max(t_hit - 1.25, 0.0);
    var d = 1.0;
    var closest = 8.0;
    var along = 0.0;
    for (var i = 0; i < 48; i = i + 1) {
        let p = ro + rd * t;
        if kind == 5 {
            d = sdf_sphere(p, scale * 0.92);
        } else if kind == 3 || kind == 4 || prim == 4 {
            let rad = select(0.052, 0.085, kind == 3) * (0.75 + 0.35 * node.position_scale.w);
            d = sdf_capsule(p, vec3<f32>(0.0), endp - center, rad);
            let ba = endp - center;
            along = clamp(dot(p, ba) / max(dot(ba, ba), 1e-5), 0.0, 1.0);
        } else if kind == 1 {
            let q = rotate_y(p, time * 0.11 + node.frequency_amplitude.z);
            d = sdf_diamond(q, scale * 1.05);
        } else if kind == 2 {
            d = sdf_shard(p, scale * 0.95);
        } else {
            d = sdf_octahedron(p, scale * 0.90);
        }
        closest = min(closest, d);
        if d < 0.0016 { break; }
        t = t + max(d, 0.003);
        if t > t_hit + 8.0 { break; }
    }

    let void_col = vec3<f32>(0.008, 0.016, 0.038);
    let base = max(node.emissive_color.rgb, vec3<f32>(0.18, 0.45, 0.85));
    let selected_idx = i32(round(observer._padding0.x));
    let selected = f32(i32(input.node_index) == selected_idx);
    let dim = mix(1.0, 0.28, f32(selected_idx >= 0) * (1.0 - selected));
    var out: HoloFragmentOut;
    let hit_pos = camera_eye + rd * t;
    let clip_pos = camera.view_projection * vec4<f32>(hit_pos, 1.0);
    out.depth = clamp(clip_pos.z / max(clip_pos.w, 1e-6), 0.0, 1.0);

    if kind == 5 {
        let well = exp(-max(closest, 0.0) * 2.35) * grow * dim;
        if well < 0.035 { discard; }
        let glow = vec3<f32>(0.08, 0.14, 0.40) * (0.18 + well * 0.55);
        let fog = exp(-0.018 * t);
        out.color = vec4<f32>(mix(void_col, glow, fog), clamp(0.012 + well * 0.055, 0.012, 0.045));
        return out;
    }

    if d > 0.024 {
        var halo = exp(-max(closest, 0.0) * select(8.4, 6.2, kind >= 3)) * grow * dim;
        if kind >= 3 { halo = halo * 0.50; }
        if halo < 0.045 { discard; }
        var glow = base * (0.22 + halo * 0.80) * (1.0 + selected * 0.35);
        if kind == 3 { glow = mix(glow, vec3<f32>(1.0, 0.84, 0.32), 0.62); }
        let fog = exp(-0.028 * t);
        out.color = vec4<f32>(mix(void_col, glow, fog), clamp(0.035 + halo * 0.18, 0.03, 0.16));
        return out;
    }

    var n = normalize(hit_pos - center);
    if kind == 1 || kind == 2 {
        let e = 0.004;
        let p = hit_pos - center;
        let q = rotate_y(p, time * 0.11 + node.frequency_amplitude.z);
        let s0 = select(sdf_shard(p, scale * 0.95), sdf_diamond(q, scale * 1.05), kind == 1);
        let sx = select(sdf_shard(p + vec3<f32>(e, 0.0, 0.0), scale * 0.95), sdf_diamond(q + vec3<f32>(e, 0.0, 0.0), scale * 1.05), kind == 1);
        let sy = select(sdf_shard(p + vec3<f32>(0.0, e, 0.0), scale * 0.95), sdf_diamond(q + vec3<f32>(0.0, e, 0.0), scale * 1.05), kind == 1);
        let sz = select(sdf_shard(p + vec3<f32>(0.0, 0.0, e), scale * 0.95), sdf_diamond(q + vec3<f32>(0.0, 0.0, e), scale * 1.05), kind == 1);
        n = normalize(vec3<f32>(sx - s0, sy - s0, sz - s0));
    }

    let fresnel = pow(1.0 - abs(dot(n, -rd)), 2.6);
    let pulse = 0.78 + 0.22 * sin(time * node.frequency_amplitude.x * 0.7 + node.frequency_amplitude.z);
    let core = base * (0.42 + 0.28 * pulse);
    let prism = hue_rgb(fract(dot(n, vec3<f32>(0.18, 0.62, 0.31)) + time * 0.03 + node.manifold_tensor.x * 0.05));
    var col = mix(core, core * vec3<f32>(0.62, 0.78, 1.12), 0.38) + prism * fresnel * 0.42 + base * fresnel * 0.38;

    if kind >= 3 {
        let travel = smoothstep(0.07, 0.0, abs(fract(along - time * 0.22) - 0.5) - 0.08);
        col = base * (0.42 + travel * 1.15 + fresnel * 0.55);
        if kind == 3 {
            col = mix(col, vec3<f32>(1.0, 0.84, 0.30), 0.58 + travel * 0.28);
        }
    }
    if kind == 1 {
        let facet = pow(max(max(abs(n.x), abs(n.y)), abs(n.z)), 8.0);
        col = mix(vec3<f32>(0.16, 0.28, 0.62), base, 0.42);
        col = col + vec3<f32>(0.86, 0.93, 1.0) * facet * 0.95;
        col = col + prism * fresnel * 0.35;
        col = col + vec3<f32>(0.12, 0.20, 0.48) * (1.0 - fresnel) * 0.40;
    }
    if kind == 0 {
        col = mix(col, base * 0.82, 0.22);
        col = col + vec3<f32>(0.10, 0.42, 0.68) * pow(fresnel, 1.6) * 0.55;
    }
    if kind == 2 {
        col = mix(col, vec3<f32>(0.92, 0.68, 0.22), 0.38);
        col = col + vec3<f32>(1.0, 0.90, 0.52) * fresnel * 0.45;
    }

    col = col * dim;
    col = mix(col, vec3<f32>(1.0, 0.90, 0.48) * 1.15, selected * 0.40);
    col = min(col, vec3<f32>(1.05));
    let fog = exp(-0.022 * t);
    col = mix(void_col, col, fog);
    var alpha = clamp(0.34 + fresnel * 0.28, 0.22, 0.62);
    if kind >= 3 {
        alpha = clamp(0.18 + fresnel * 0.22, 0.12, 0.42);
    }
    let q = clamp(observer.epistemic_q, 0.0, 1.0);
    col = col * (0.92 + 0.08 * q);
    out.color = vec4<f32>(col, alpha);
    return out;
}
