// Low-cost analytic sky. Camera projection, sun direction/radiance and ambient level all come
// from the scene camera uniform, so the background tracks the same lighting state as the meshes.
struct Camera {
    view_projection: mat4x4<f32>,
    tail: vec4<f32>,
    rest: array<vec4<f32>, 3>,
};
@group(0) @binding(0) var<uniform> camera: Camera;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) ray: vec3<f32>,
};

fn camera_forward() -> vec3<f32> {
    return safe_normalize(camera.rest[1].xyz - camera.rest[0].yzw, vec3<f32>(0.0, 0.0, -1.0));
}

fn camera_right(forward: vec3<f32>) -> vec3<f32> {
    var reference_up = vec3<f32>(0.0, 1.0, 0.0);
    if (abs(forward.y) > 0.999) {
        reference_up = vec3<f32>(0.0, 0.0, 1.0);
    }
    return safe_normalize(cross(forward, reference_up), vec3<f32>(1.0, 0.0, 0.0));
}

fn camera_up(forward: vec3<f32>, right: vec3<f32>) -> vec3<f32> {
    return safe_normalize(cross(right, forward), vec3<f32>(0.0, 1.0, 0.0));
}

fn safe_normalize(value: vec3<f32>, fallback: vec3<f32>) -> vec3<f32> {
    let magnitude_squared = dot(value, value);
    if (magnitude_squared > 1e-10 && magnitude_squared < 1e20) {
        return value * inverseSqrt(magnitude_squared);
    }
    return fallback;
}

@vertex
fn vertex_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    let ndc = vec2<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0);
    let forward = camera_forward();
    let right = camera_right(forward);
    let up = camera_up(forward, right);

    // Recover FOV and aspect from the rows of the view-projection matrix. The row norms cancel
    // camera rotation, so this remains exact for the current perspective camera without another
    // uniform or an inverse matrix.
    let horizontal = vec3<f32>(
        camera.view_projection[0].x,
        camera.view_projection[1].x,
        camera.view_projection[2].x
    );
    let vertical = vec3<f32>(
        camera.view_projection[0].y,
        camera.view_projection[1].y,
        camera.view_projection[2].y
    );
    let vertical_scale = max(length(vertical), 1e-5);
    let aspect = vertical_scale / max(length(horizontal), 1e-5);
    let tan_half_fov = 1.0 / vertical_scale;

    var output: VertexOutput;
    output.position = vec4<f32>(ndc, 1.0, 1.0);
    output.ray = forward
        + right * (ndc.x * aspect * tan_half_fov)
        + up * (ndc.y * tan_half_fov);
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let ray = safe_normalize(input.ray, vec3<f32>(0.0, 0.0, -1.0));
    let sun = safe_normalize(vec3<f32>(camera.rest[1].w, camera.rest[2].xy), vec3<f32>(0.0, 1.0, 0.0));
    let sun_radiance = max(camera.rest[2].z, 0.0);
    let ambient = clamp(camera.rest[2].w, 0.0, 1.0);

    // Authored day/night transition with a warm horizon near the sunset sun elevation.
    let daylight = smoothstep(0.12, 0.38, ambient);
    let dusk = daylight * (1.0 - smoothstep(0.18, 0.62, sun.y));
    let night_zenith = vec3<f32>(0.008, 0.018, 0.055);
    let day_zenith = vec3<f32>(0.055, 0.22, 0.52);
    let dusk_zenith = vec3<f32>(0.12, 0.16, 0.30);
    let night_horizon = vec3<f32>(0.035, 0.060, 0.14);
    let day_horizon = vec3<f32>(0.58, 0.73, 0.88);
    let dusk_horizon = vec3<f32>(0.92, 0.34, 0.14);
    let day_night_zenith = mix(night_zenith, day_zenith, daylight);
    let day_night_horizon = mix(night_horizon, day_horizon, daylight);
    let zenith = mix(day_night_zenith, dusk_zenith, dusk);
    let horizon = mix(day_night_horizon, dusk_horizon, dusk);
    let horizon_weight = smoothstep(-0.16, 0.56, ray.y);
    let below_horizon = 1.0 - smoothstep(-0.52, -0.015, ray.y);
    let sky = mix(horizon, zenith, horizon_weight);
    let ground_haze = mix(vec3<f32>(0.035, 0.040, 0.045), horizon, 0.5);
    var color = mix(sky, ground_haze, below_horizon);

    let sun_cos = dot(ray, sun);
    let disk = smoothstep(cos(0.010), cos(0.004), sun_cos)
        * smoothstep(-0.08, 0.02, sun.y);
    let halo = pow(max(sun_cos, 0.0), 48.0) * 0.055;
    let moon_color = vec3<f32>(0.48, 0.62, 1.0);
    let day_sun_color = vec3<f32>(1.0, 0.84, 0.64);
    let dusk_sun_color = vec3<f32>(1.0, 0.34, 0.12);
    let sun_color = mix(mix(moon_color, day_sun_color, daylight), dusk_sun_color, dusk);
    color += sun_color * (disk * sun_radiance + halo * min(sun_radiance, 1.5));
    return vec4<f32>(color, 1.0);
}
