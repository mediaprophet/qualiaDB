// Ambient visualization — GPU-driven particle field (U2 Viewport)
// Full 48-byte SystemTelemetry uniform; zero per-frame CPU particle work.

struct Uniforms {
    time: f32,
    view_width: f32,
    view_height: f32,
    _padding: f32,
};

// Matches portal_telemetry::SystemTelemetry (12 × f32, WGSL-aligned).
struct Telemetry {
    memory_pressure: f32,
    network_ripple: f32,
    baking_crystallization: f32,
    logic_flashes: f32,
    llm_heat: f32,
    quantum_activity: f32,
    spectral_shift: f32,
    temporal_pulse: f32,
    epistemic_density: f32,
    manifold_pressure: f32,
    _pad0: f32,
    _pad1: f32,
};

struct ParticleInstance {
    position: vec3<f32>,
    epistemic_q: f32,
};

// Matches portal_telemetry::CameraUniform (128 B). Binding 0 field is `view_projection`
// for `projector.wgsl`; ambient reads the full block at binding 3.
struct Camera {
    view_projection: mat4x4<f32>,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    tensor_mode: u32,
    _padding: array<f32, 12>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<uniform> telemetry: Telemetry;
@group(0) @binding(2) var<storage, read> particles: array<ParticleInstance>;
@group(0) @binding(3) var<uniform> camera: Camera;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local_uv: vec2<f32>,
    @location(2) epistemic_q: f32,
};

@vertex
fn vertex_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    let quad_vertices = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0)
    );

    let base_vertex = quad_vertices[vertex_index];
    let particle = particles[instance_index];
    let base_pos = particle.position;
    let t = uniforms.time;

    let compression = 1.0 - telemetry.memory_pressure * 0.5;
    let pos = base_pos * compression;

    let ripple_phase = pos.x * 2.0 + pos.z * 2.0;
    let ripple = sin(t * 3.0 + ripple_phase) * telemetry.network_ripple * 0.3;

    let chaos = sin(t * 0.5 + pos.x) * cos(t * 0.3 + pos.y) * sin(t * 0.4 + pos.z);
    let order = floor(pos.x * 2.0) * 0.5 + floor(pos.y * 2.0) * 0.5 + floor(pos.z * 2.0) * 0.5;
    let morph = mix(chaos, order, telemetry.baking_crystallization);

    let heat_jitter = sin(t * 20.0 + pos.x * 10.0) * telemetry.llm_heat * 0.1;
    let quantum_flicker = sin(t * 7.0 + f32(instance_index) * 0.05) * telemetry.quantum_activity * 0.08;
    let temporal_wave = sin(t * 1.5 + length(pos) * 2.0) * telemetry.temporal_pulse * 0.12;

    let animated_pos = pos + vec3<f32>(
        ripple + heat_jitter + quantum_flicker,
        morph + heat_jitter + temporal_wave,
        ripple + heat_jitter
    );

    var output: VertexOutput;
    if (camera.tensor_mode != 0u) {
        let clip = camera.view_projection * vec4<f32>(animated_pos, 1.0);
        let inv_w = 1.0 / max(abs(clip.w), 1e-4);
        let ndc = clip.xyz * inv_w;
        let particle_size = 0.018 * (1.0 + telemetry.llm_heat * 0.5) / max(abs(clip.w), 0.35);
        output.position = vec4<f32>(
            ndc.x + base_vertex.x * particle_size,
            ndc.y + base_vertex.y * particle_size,
            ndc.z,
            1.0
        );
    } else {
        let fov = 1.0;
        let z_depth = 5.0 + animated_pos.z;
        let scale = fov / max(z_depth, 0.1);
        let aspect = uniforms.view_width / max(uniforms.view_height, 1.0);
        let screen_x = animated_pos.x * scale / aspect;
        let screen_y = animated_pos.y * scale;
        let particle_size = 0.02 * scale * (1.0 + telemetry.llm_heat * 0.5);
        let final_x = screen_x + base_vertex.x * particle_size;
        let final_y = screen_y + base_vertex.y * particle_size;
        output.position = vec4<f32>(final_x, final_y, 0.0, 1.0);
    }
    output.local_uv = base_vertex;

    let sigma = fract(telemetry.spectral_shift + f32(instance_index) * 0.0017);
    let linear_spectral = sigma_to_linear_rgb(sigma);
    let ripple_energy = vec3<f32>(0.02, 0.08, 0.08) * telemetry.network_ripple;
    let heat_energy = vec3<f32>(0.12, 0.10, 0.08) * telemetry.llm_heat;
    let flash = step(0.9, sin(t * 10.0 + f32(instance_index) * 0.1)) * telemetry.logic_flashes;
    let flash_energy = vec3<f32>(0.18, 0.16, 0.12) * flash;
    let density_gain = 0.35 + telemetry.epistemic_density * 0.25;

    let rgb = linear_spectral * density_gain + ripple_energy + heat_energy + flash_energy;
    output.color = vec4<f32>(rgb, 0.6 + telemetry.llm_heat * 0.4);
    output.epistemic_q = particle.epistemic_q;

    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let dist = length(input.local_uv);
    let alpha = smoothstep(1.0, 0.0, dist);
    let glow = 1.0 - dist;
    let glow_intensity = glow * glow;
    let collapsed = step(input.epistemic_q, 0.001);
    let sandbox = 1.0 - collapsed;
    let certainty_opacity = mix(0.45, 1.0, collapsed);
    let sandbox_pulse = 0.85 + 0.15 * sin(input.epistemic_q * 12.0);
    let epistemic_alpha = mix(certainty_opacity * sandbox_pulse, certainty_opacity, collapsed);
    let ring_boost = select(0.0, 0.35 * smoothstep(0.7, 1.0, dist), sandbox > 0.5);
    // HDR epistemic density + llm_heat drive bloom extraction.
    let hdr_gain = 1.0 + telemetry.epistemic_density * 0.65 + telemetry.llm_heat * 0.45;
    let final_color = input.color.rgb * hdr_gain * (1.0 + glow_intensity * 0.5 + ring_boost);
    let final_alpha = input.color.a * alpha * epistemic_alpha;
    return vec4<f32>(final_color, final_alpha);
}