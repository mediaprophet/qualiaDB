// Triangle-mesh renderer — imported OBJ / STL / GLB surfaces (Phase 1.2,
// RENDERER_IMPLEMENTATION_PLAN.md). Flat-shaded from screen-space derivatives, so no per-vertex
// normal buffer is needed for this first cut. Shares the orbit camera with the projector/ambient.

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

@group(0) @binding(0) var<uniform> camera: Camera;
// Per-artefact model transform (Phase 2): the kinematic-joint pose, identity when not animating.
@group(1) @binding(0) var<uniform> model: mat4x4<f32>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vertex_main(
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>
) -> VertexOutput {
    var output: VertexOutput;
    let world = (model * vec4<f32>(position, 1.0)).xyz;
    output.world_pos = world;
    output.color = color;
    output.clip_position = camera.view_projection * vec4<f32>(world, 1.0);
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Flat per-face normal from the derivative of world position across the triangle.
    let n = normalize(cross(dpdx(input.world_pos), dpdy(input.world_pos)));

    // Configurable directional key/sun light:
    // _padding1.w = sun_dir_x, _padding2.x = sun_dir_y, _padding2.y = sun_dir_z
    var sun_dir = vec3<f32>(camera._padding1.w, camera._padding2.x, camera._padding2.y);
    if (dot(sun_dir, sun_dir) < 0.01) {
        sun_dir = vec3<f32>(0.45, 0.8, 0.55);
    }
    let key = normalize(sun_dir);

    // _padding2.z = sun_intensity (defaults to 1.0), _padding2.w = ambient_intensity (defaults to 0.25)
    let sun_int = select(1.0, camera._padding2.z, camera._padding2.z > 0.0);
    let amb_int = select(0.25, camera._padding2.w, camera._padding2.w > 0.0);

    let diffuse = clamp(dot(n, key), 0.0, 1.0) * sun_int;

    // Rim / fresnel term so silhouettes read cleanly
    let facing = clamp(abs(n.z), 0.0, 1.0);
    let rim = pow(1.0 - facing, 2.0);
    let base = input.color.rgb;
    let col = base * (amb_int + diffuse) + vec3<f32>(0.10, 0.14, 0.22) * rim;
    return vec4<f32>(col, input.color.a);
}
