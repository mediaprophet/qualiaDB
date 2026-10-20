struct Camera {
    view_projection: mat4x4<f32>,
    tail: vec4<f32>,
    rest: array<vec4<f32>, 3>,
};
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> model: mat4x4<f32>;
struct MeshInstance {
    world_from_local: mat4x4<f32>,
    normal_from_local: mat3x3<f32>,
    semantic_id: vec2<u32>,
    orientation_sign: f32,
    _padding: u32,
};
@group(3) @binding(0) var<storage, read> mesh_instances: array<MeshInstance>;
struct AoUniform {
    viewport: vec4<f32>,
    eye: vec4<f32>,
    camera_forward: vec4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    params: vec4<f32>,
    depth: vec4<f32>,
};
@group(0) @binding(2) var<uniform> ao: AoUniform;
struct Material {
    base_color: vec4<f32>,
    emissive: vec4<f32>,
    factors: vec4<f32>,
    flags: vec4<u32>,
    texture_flags: vec4<u32>,
    texture_flags_extra: vec4<u32>,
    coverage: vec4<u32>,
    sampler_modes: vec4<u32>,
    uv_transform: array<vec4<f32>, 12>,
};
@group(2) @binding(0) var<uniform> material: Material;
@group(2) @binding(1) var base_color_map: texture_2d<f32>;
@group(2) @binding(7) var base_color_sampler: sampler;
@group(2) @binding(8) var normal_sampler: sampler;
@group(2) @binding(9) var metallic_roughness_sampler: sampler;
@group(2) @binding(10) var occlusion_sampler: sampler;
@group(2) @binding(11) var emissive_sampler: sampler;
@group(2) @binding(12) var stylized_ramp_sampler: sampler;

fn material_uv(uv: vec2<f32>, role: u32) -> vec2<f32> {
    let offset_scale = material.uv_transform[role * 2u];
    let rotation = material.uv_transform[role * 2u + 1u];
    let scaled = uv * offset_scale.zw;
    return vec2<f32>(
        rotation.x * scaled.x - rotation.y * scaled.y,
        rotation.y * scaled.x + rotation.x * scaled.y
    ) + offset_scale.xy;
}

fn nearest_base_color_texel(uv: vec2<f32>, rho: f32) -> vec3<i32> {
    let record = material.sampler_modes[0] & 127u;
    var wrapped = uv;
    let wrap_s = (record >> 3u) & 3u;
    let wrap_t = (record >> 5u) & 3u;
    if (wrap_s == 0u) { wrapped.x = clamp(wrapped.x, 0.0, 1.0); }
    if (wrap_s == 1u) {
        let phase = wrapped.x - 2.0 * floor(wrapped.x / 2.0);
        wrapped.x = select(phase, 2.0 - phase, phase > 1.0);
    }
    if (wrap_s == 2u) { wrapped.x = wrapped.x - floor(wrapped.x); }
    if (wrap_t == 0u) { wrapped.y = clamp(wrapped.y, 0.0, 1.0); }
    if (wrap_t == 1u) {
        let phase = wrapped.y - 2.0 * floor(wrapped.y / 2.0);
        wrapped.y = select(phase, 2.0 - phase, phase > 1.0);
    }
    if (wrap_t == 2u) { wrapped.y = wrapped.y - floor(wrapped.y); }
    let dims = textureDimensions(base_color_map, 0);
    var level = 0u;
    if ((record & 7u) == 2u && textureNumLevels(base_color_map) > 1u) {
        let lod = clamp(log2(max(rho, 0.000001)), 0.0, f32(textureNumLevels(base_color_map) - 1u));
        level = u32(floor(lod + 0.5));
    }
    let level_dims = max(dims >> vec2<u32>(level), vec2<u32>(1u));
    let pixel = min(vec2<u32>(floor(wrapped * vec2<f32>(level_dims))), level_dims - vec2<u32>(1u));
    return vec3<i32>(vec2<i32>(pixel), i32(level));
}

fn sample_base_color(uv: vec2<f32>) -> vec4<f32> {
    let sampled = textureSample(base_color_map, base_color_sampler, uv);
    let dims = textureDimensions(base_color_map, 0);
    let dx = dpdx(uv) * vec2<f32>(dims);
    let dy = dpdy(uv) * vec2<f32>(dims);
    let rho = max(length(dx), length(dy));
    let record = material.sampler_modes[0] & 127u;
    let min_filter = record & 7u;
    let min_nearest = min_filter == 0u || min_filter == 2u;
    let mag_nearest = (material.coverage.w & 1u) != 0u;
    let is_nearest = (mag_nearest && rho <= 1.0) || (min_nearest && rho > 1.0);
    if ((material.coverage.y & 1u) != 0u && is_nearest) {
        let p = nearest_base_color_texel(uv, rho);
        return textureLoad(base_color_map, p.xy, p.z);
    }
    return sampled;
}

struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv0: vec2<f32>,
};

struct PgaSkinMotor {
    r: vec4<f32>,
    d: vec4<f32>,
};

fn pga_blade4_to_quat(b: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(b.x, -b.w, -b.z, -b.y);
}

fn pga_quat_mul(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(
        a.x * b.x - a.y * b.y - a.z * b.z - a.w * b.w,
        a.x * b.y + a.y * b.x + a.z * b.w - a.w * b.z,
        a.x * b.z - a.y * b.w + a.z * b.x + a.w * b.y,
        a.x * b.w + a.y * b.z - a.z * b.y + a.w * b.x
    );
}

fn pga_motor_blend_4(
    m0_r: vec4<f32>, m0_d: vec4<f32>,
    m1_r: vec4<f32>, m1_d: vec4<f32>,
    m2_r: vec4<f32>, m2_d: vec4<f32>,
    m3_r: vec4<f32>, m3_d: vec4<f32>,
    weights: vec4<f32>
) -> PgaSkinMotor {
    let s1 = select(1.0, -1.0, dot(m0_r, m1_r) < 0.0);
    let s2 = select(1.0, -1.0, dot(m0_r, m2_r) < 0.0);
    let s3 = select(1.0, -1.0, dot(m0_r, m3_r) < 0.0);
    let r_sum = m0_r * weights.x + m1_r * (weights.y * s1) + m2_r * (weights.z * s2) + m3_r * (weights.w * s3);
    let d_sum = m0_d * weights.x + m1_d * (weights.y * s1) + m2_d * (weights.z * s2) + m3_d * (weights.w * s3);
    let inv_norm = inverseSqrt(max(dot(r_sum, r_sum), 1e-12));
    let r_unit = r_sum * inv_norm;
    let d_scaled = d_sum * inv_norm;
    let d_ortho = d_scaled - r_unit * dot(r_unit, d_scaled);
    return PgaSkinMotor(r_unit, d_ortho);
}

fn pga_motor_transform_vector(m_r: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let qr = pga_blade4_to_quat(m_r);
    let qr_conj = vec4<f32>(qr.x, -qr.y, -qr.z, -qr.w);
    let v_q = vec4<f32>(0.0, v.x, v.y, v.z);
    let v_rot = pga_quat_mul(pga_quat_mul(qr, v_q), qr_conj);
    return vec3<f32>(v_rot.y, v_rot.z, v_rot.w);
}

fn pga_eval_proc_motor_oscillation(
    pos: vec3<f32>,
    axis: vec3<f32>,
    amp_rad: f32,
    freq_hz: f32,
    damping: f32,
    time_s: f32,
    phase_rad: f32
) -> vec3<f32> {
    let omega = 6.283185307 * freq_hz;
    let decay = exp(-damping * max(time_s, 0.0));
    let theta = amp_rad * sin(omega * time_s + phase_rad) * decay;
    let half_theta = theta * 0.5;
    let ax = normalize(select(vec3<f32>(0.0, 0.0, 1.0), axis, dot(axis, axis) > 1e-8));
    let s = sin(half_theta);
    let r = vec4<f32>(cos(half_theta), s * (-ax.z), s * ax.y, s * (-ax.x));
    return pga_motor_transform_vector(r, pos);
}

fn oct_encode(normal: vec3<f32>) -> vec2<f32> {
    let n = normalize(normal);
    let projected = n.xy / (abs(n.x) + abs(n.y) + abs(n.z));
    var folded = projected;
    if (n.z < 0.0) {
        folded = (vec2<f32>(1.0) - abs(projected.yx))
            * select(vec2<f32>(-1.0), vec2<f32>(1.0), projected >= vec2<f32>(0.0));
    }
    return folded * 0.5 + vec2<f32>(0.5);
}

@vertex
fn vertex_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv0: vec2<f32>,
    @builtin(instance_index) instance_index: u32
) -> Output {
    var output: Output;
    let instance_model = model * mesh_instances[instance_index].world_from_local;
    let world_position = (instance_model * vec4<f32>(position, 1.0)).xyz;
    output.position = camera.view_projection * vec4<f32>(world_position, 1.0);
    output.normal = normalize((instance_model * vec4<f32>(normal, 0.0)).xyz);
    output.world_position = world_position;
    output.color = color;
    output.uv0 = uv0;
    return output;
}

// Pack octahedral world normal (RG8) and normalized linear view depth (BA16)
// into one universally renderable RGBA8 target. Code 0 is the clear/background
// sentinel; valid surface depths occupy 1..65535.
@fragment
fn fragment_main(input: Output) -> @location(0) vec4<f32> {
    if (material.coverage.x == 1u) {
        let vertex_alpha = select(1.0, input.color.a, material.flags.y != 0u);
        let base_sample = select(
            1.0,
            sample_base_color(material_uv(input.uv0, 0u)).a,
            material.texture_flags.x != 0u
        );
        let alpha = material.base_color.a * vertex_alpha * base_sample;
        if (alpha < material.factors.w) {
            discard;
        }
    }
    let eye = camera.rest[0].yzw;
    let camera_focus = camera.rest[1].xyz;
    let forward = normalize(camera_focus - eye);
    let view_depth = max(dot(input.world_position - eye, forward), 0.0);
    let normalized_depth = clamp(view_depth / ao.depth.x, 0.0, 1.0);
    let code = 1u + u32(round(normalized_depth * 65534.0));
    let high = (code >> 8u) & 255u;
    let low = code & 255u;
    let encoded_normal = oct_encode(input.normal);
    return vec4<f32>(
        encoded_normal,
        f32(high) * (1.0 / 255.0),
        f32(low) * (1.0 / 255.0)
    );
}
