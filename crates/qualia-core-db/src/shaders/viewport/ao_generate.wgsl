struct AoUniform {
    viewport: vec4<f32>,
    eye: vec4<f32>,
    camera_forward: vec4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    params: vec4<f32>,
    depth: vec4<f32>,
};
@group(0) @binding(0) var scene_surface: texture_2d<f32>;
@group(0) @binding(1) var<uniform> ao: AoUniform;

struct FullscreenOutput { @builtin(position) position: vec4<f32> };
@vertex
fn vertex_main(@builtin(vertex_index) index: u32) -> FullscreenOutput {
    var output: FullscreenOutput;
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    output.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    return output;
}

fn decode_depth_code(surface: vec4<f32>) -> u32 {
    let high = u32(round(surface.b * 255.0));
    let low = u32(round(surface.a * 255.0));
    return (high << 8u) | low;
}

fn decode_normal(surface: vec4<f32>) -> vec3<f32> {
    let encoded = surface.rg * 2.0 - vec2<f32>(1.0);
    var normal = vec3<f32>(encoded, 1.0 - abs(encoded.x) - abs(encoded.y));
    let fold = clamp(-normal.z, 0.0, 1.0);
    normal.x += select(-fold, fold, normal.x >= 0.0);
    normal.y += select(-fold, fold, normal.y >= 0.0);
    return normalize(normal);
}

fn decode_view_depth(surface: vec4<f32>) -> f32 {
    let code = decode_depth_code(surface);
    return f32(code - 1u) * ao.depth.y;
}

fn reconstruct_world(pixel: vec2<i32>, view_depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + vec2<f32>(0.5)) * ao.viewport.zw;
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let view_x = ndc.x * ao.depth.w * ao.depth.z * view_depth;
    let view_y = ndc.y * ao.depth.z * view_depth;
    return ao.eye.xyz
        + ao.camera_forward.xyz * view_depth
        + ao.camera_right.xyz * view_x
        + ao.camera_up.xyz * view_y;
}

@fragment
fn fragment_main(input: FullscreenOutput) -> @location(0) f32 {
    let pixel = vec2<i32>(input.position.xy);
    let surface = textureLoad(scene_surface, pixel, 0);
    let depth_code = decode_depth_code(surface);
    if (depth_code == 0u) { return 1.0; }

    let view_depth = decode_view_depth(surface);
    let normal = decode_normal(surface);
    let center = reconstruct_world(pixel, view_depth);
    let view_distance = max(view_depth, 0.05);
    let pixel_radius = clamp(ao.params.x * 2.41421356 * ao.viewport.y / (2.0 * view_distance), 1.0, 14.0);
    let directions4 = array<vec2<f32>, 4>(
        vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(-1.0, 0.0), vec2<f32>(0.0, -1.0)
    );
    let directions8 = array<vec2<f32>, 8>(
        vec2<f32>(1.0, 0.0), vec2<f32>(0.70710678, 0.70710678),
        vec2<f32>(0.0, 1.0), vec2<f32>(-0.70710678, 0.70710678),
        vec2<f32>(-1.0, 0.0), vec2<f32>(-0.70710678, -0.70710678),
        vec2<f32>(0.0, -1.0), vec2<f32>(0.70710678, -0.70710678)
    );
    let directions12 = array<vec2<f32>, 12>(
        vec2<f32>(1.0, 0.0), vec2<f32>(0.8660254, 0.5),
        vec2<f32>(0.5, 0.8660254), vec2<f32>(0.0, 1.0),
        vec2<f32>(-0.5, 0.8660254), vec2<f32>(-0.8660254, 0.5),
        vec2<f32>(-1.0, 0.0), vec2<f32>(-0.8660254, -0.5),
        vec2<f32>(-0.5, -0.8660254), vec2<f32>(0.0, -1.0),
        vec2<f32>(0.5, -0.8660254), vec2<f32>(0.8660254, -0.5)
    );
    var occlusion = 0.0;
    var samples = 0.0;
    let sample_limit = u32(ao.eye.w);
    for (var i = 0u; i < sample_limit; i = i + 1u) {
        var direction = directions12[i];
        if (sample_limit == 4u) {
            direction = directions4[i];
        } else if (sample_limit == 8u) {
            direction = directions8[i];
        }
        let radial_scale = select(0.72, 1.0, (i & 1u) != 0u);
        let offset = vec2<i32>(round(direction * pixel_radius * radial_scale));
        let sample_pixel = pixel + offset;
        if (any(sample_pixel < vec2<i32>(0)) || any(sample_pixel >= vec2<i32>(ao.viewport.xy))) { continue; }
        let sample_surface = textureLoad(scene_surface, sample_pixel, 0);
        if (decode_depth_code(sample_surface) == 0u) { continue; }
        let sample_depth = decode_view_depth(sample_surface);
        let sample_world = reconstruct_world(sample_pixel, sample_depth);
        let delta = sample_world - center;
        let distance = length(delta);
        if (distance > 1e-5 && distance < ao.params.x) {
            let elevation = dot(normal, delta);
            let horizon = smoothstep(ao.params.z, ao.params.z + 0.04, elevation);
            occlusion += horizon * (1.0 - distance / ao.params.x);
            samples += 1.0;
        }
    }
    let visibility = 1.0 - ao.params.y * occlusion / max(samples, 1.0);
    return clamp(visibility, 0.0, 1.0);
}
