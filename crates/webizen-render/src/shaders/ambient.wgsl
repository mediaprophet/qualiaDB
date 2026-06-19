// Ambient visualization shader
// Renders 50,000 knowledge particles with GPU-driven animation
// Zero CPU cost - all animation happens in shaders

struct Uniforms {
    time: f32,
    view_width: f32,
    view_height: f32,
    _padding: f32,
};

struct Telemetry {
    memory_pressure: f32,
    network_ripple: f32,
    baking_crystallization: f32,
    logic_flashes: f32,
    llm_heat: f32,
    padding: vec3<f32>,
};

struct ParticleInstance {
    position: vec3<f32>,
    _padding: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<uniform> telemetry: Telemetry;
@group(0) @binding(2) var<storage, read> particles: array<ParticleInstance>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) size: f32,
};

@vertex
fn vertex_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    // Base quad vertices (centered at origin)
    let quad_vertices = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0)
    );

    let base_vertex = quad_vertices[vertex_index];

    // Get particle instance data
    let particle = particles[instance_index];
    let base_pos = particle.position;

    // Time-based animation (all GPU-side)
    let t = uniforms.time;

    // Memory pressure affects particle density (compression toward center)
    let compression = 1.0 - telemetry.memory_pressure * 0.5;
    let pos = base_pos * compression;

    // Network ripple creates wave displacement on X/Z axes
    let ripple_phase = pos.x * 2.0 + pos.z * 2.0;
    let ripple_amp = telemetry.network_ripple * 0.3;
    let ripple = sin(t * 3.0 + ripple_phase) * ripple_amp;

    // Baking crystallization morphs between chaos and order
    let chaos = sin(t * 0.5 + pos.x) * cos(t * 0.3 + pos.y) * sin(t * 0.4 + pos.z);
    let order = floor(pos.x * 2.0) * 0.5 + floor(pos.y * 2.0) * 0.5 + floor(pos.z * 2.0) * 0.5;
    let morph = mix(chaos, order, telemetry.baking_crystallization);

    // LLM heat adds high-frequency vibration
    let heat_jitter = sin(t * 20.0 + pos.x * 10.0) * telemetry.llm_heat * 0.1;

    // Combine all displacements
    let animated_pos = pos + vec3<f32>(ripple + heat_jitter, morph + heat_jitter, ripple + heat_jitter);

    // Project to screen space (simple perspective)
    let fov = 1.0;
    let z_depth = 5.0 + animated_pos.z;
    let scale = fov / max(z_depth, 0.1);

    // Aspect ratio correction
    let aspect = uniforms.view_width / uniforms.view_height;
    let screen_x = animated_pos.x * scale / aspect;
    let screen_y = animated_pos.y * scale;

    // Apply quad vertex offset
    let particle_size = 0.02 * scale * (1.0 + telemetry.llm_heat * 0.5);
    let final_x = screen_x + base_vertex.x * particle_size;
    let final_y = screen_y + base_vertex.y * particle_size;

    var output: VertexOutput;
    output.position = vec4<f32>(final_x, final_y, 0.0, 1.0);

    // Color based on telemetry
    // Base color: cool blue/purple for idle state
    let base_color = vec3<f32>(0.2, 0.4, 0.8);

    // Network ripple adds cyan/white highlights
    let ripple_color = vec3<f32>(0.0, 1.0, 1.0) * telemetry.network_ripple;

    // LLM heat shifts to bright white/spectral hues
    let heat_color = vec3<f32>(1.0, 1.0, 1.0) * telemetry.llm_heat;

    // Logic flashes add brief bright bursts
    let flash = step(0.9, sin(t * 10.0 + instance_index as f32 * 0.1)) * telemetry.logic_flashes;
    let flash_color = vec3<f32>(1.0, 0.9, 0.7) * flash;

    // Combine colors
    let rgb = base_color + ripple_color + heat_color + flash_color;
    output.color = vec4<f32>(rgb, 0.6 + telemetry.llm_heat * 0.4);

    // Size for fragment shader
    output.size = particle_size;

    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Calculate distance from center of particle quad
    let uv = (input.position.xy / vec2<f32>(1920.0, 1080.0)) * 2.0 - 1.0;
    let dist = length(uv);

    // Soft circular falloff for particle glow
    let alpha = smoothstep(1.0, 0.0, dist);

    // Add glow effect (brighter center)
    let glow = 1.0 - dist;
    let glow_intensity = glow * glow;

    // Apply color with glow and alpha
    let final_color = input.color.rgb * (1.0 + glow_intensity * 0.5);
    let final_alpha = input.color.a * alpha;

    return vec4<f32>(final_color, final_alpha);
}
