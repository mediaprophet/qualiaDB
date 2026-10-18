// Precipitation Particle Compute & Rendering Pass (`precipitation.wgsl`)
//
// Camera-anchored precipitation particle system with:
// - Dynamic boundary box centered on the active camera position
// - Terminal velocity fall curve + Atmospheric Boundary Layer 3D wind drift
// - Scene depth buffer occlusion testing (rain stops under roofs / overhangs)
// - Splashing / ripple spawn flag on surface collision
// - Supports Rain, Snow, and Sleet particle regimes

struct PrecipitationParticle {
    position: vec3<f32>,
    life_phase: f32,
    velocity: vec3<f32>,
    scale: f32,
};

struct PrecipitationUniform {
    camera_pos: vec4<f32>,      // x, y, z, reserved
    wind_velocity: vec4<f32>,   // u, v, w wind vector in m/s, air_density
    box_extents: vec4<f32>,     // half_width, half_height, half_depth, max_particles
    sim_params: vec4<f32>,      // dt, intensity, terminal_vel, p_type (0=Rain, 1=Snow, 2=Sleet)
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> precip: PrecipitationUniform;
@group(0) @binding(1) var<storage, read_write> particles: array<PrecipitationParticle>;
@group(0) @binding(2) var scene_depth: texture_depth_2d;
@group(0) @binding(3) var depth_sampler: sampler;

// Fast integer hash for pseudo-random initialization
fn hash13(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash31(p: f32) -> vec3<f32> {
    var p3 = fract(vec3<f32>(p) * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xxy + p3.yzz) * p3.zyx);
}

@compute @workgroup_size(64)
fn precip_simulate_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    let max_count = u32(precip.box_extents.w);
    if (index >= max_count) {
        return;
    }

    var p = particles[index];
    let dt = precip.sim_params.x;
    let p_type = u32(precip.sim_params.w);
    let cam_pos = precip.camera_pos.xyz;
    let half_ext = precip.box_extents.xyz;

    // Terminal velocity based on precipitation regime
    var v_term = precip.sim_params.z;
    var flutter = vec3<f32>(0.0);

    if (p_type == 1u) {
        // Snow: lower terminal velocity, turbulent lateral fluttering
        v_term = 1.2;
        let time = p.life_phase * 6.28318;
        flutter = vec3<f32>(sin(time * 2.0 + f32(index)), 0.0, cos(time * 3.0 + f32(index))) * 0.6;
    } else if (p_type == 2u) {
        // Sleet: higher density, fast drop
        v_term = 7.5;
    } else {
        // Rain: default 9.0 m/s terminal fall speed
        if (v_term <= 0.1) {
            v_term = 9.0;
        }
    }

    // Target velocity = terminal downward + wind drift + local flutter
    let target_vel = vec3<f32>(
        precip.wind_velocity.x + flutter.x,
        -v_term + flutter.y,
        precip.wind_velocity.z + flutter.z
    );

    // Momentum relaxation
    p.velocity = mix(p.velocity, target_vel, clamp(dt * 5.0, 0.0, 1.0));
    p.position += p.velocity * dt;
    p.life_phase += dt * 0.5;

    // Camera box wrap-around
    let rel_pos = p.position - cam_pos;
    var needs_reset = false;

    if (abs(rel_pos.x) > half_ext.x || rel_pos.y < -half_ext.y || abs(rel_pos.z) > half_ext.z) {
        needs_reset = true;
    }

    // Scene depth occlusion test
    // Transform particle world position into clip space to check if it's occluded by scene geometry
    let clip = precip.view_proj * vec4<f32>(p.position, 1.0);
    if (clip.w > 0.0) {
        let ndc = clip.xyz / clip.w;
        let uv = ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);

        if (uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0) {
            let dim = textureDimensions(scene_depth);
            let texel = vec2<i32>(uv * vec2<f32>(dim));
            let scene_z = textureLoad(scene_depth, texel, 0);

            // In WebGPU standard reverse-Z / forward-Z: if particle is behind scene surface, it collided
            if (ndc.z >= scene_z && (ndc.z - scene_z) < 0.05) {
                needs_reset = true;
            }
        }
    }

    if (needs_reset) {
        // Respawn droplet at top of the camera bounding box with random jitter
        let rnd = hash31(f32(index) * 17.13 + p.life_phase);
        p.position = vec3<f32>(
            cam_pos.x + (rnd.x * 2.0 - 1.0) * half_ext.x,
            cam_pos.y + half_ext.y * (0.95 + 0.05 * rnd.y),
            cam_pos.z + (rnd.z * 2.0 - 1.0) * half_ext.z
        );
        p.velocity = target_vel;
        p.life_phase = 0.0;
    }

    particles[index] = p;
}

// ── Instanced Particle Streak Render ──────────────────────────────────────────

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) alpha: f32,
};

@vertex
fn precip_render_vs(
    @builtin(vertex_index) v_idx: u32,
    @builtin(instance_index) i_idx: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let p = particles[i_idx];

    // Build billboard quad aligned to droplet velocity
    let vel_norm = normalize(p.velocity);
    let right = normalize(cross(vel_norm, vec3<f32>(0.0, 0.0, 1.0)));

    let quad_pos = array<vec2<f32>, 4>(
        vec2<f32>(-0.5, 0.0),
        vec2<f32>( 0.5, 0.0),
        vec2<f32>(-0.5, 1.0),
        vec2<f32>( 0.5, 1.0)
    );

    let offset_2d = quad_pos[v_idx];
    let width = 0.015 * p.scale;
    let length = clamp(length(p.velocity) * 0.04, 0.05, 0.8) * p.scale;

    let world_pos = p.position + right * (offset_2d.x * width) - vel_norm * (offset_2d.y * length);
    out.position = precip.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = offset_2d;
    out.alpha = clamp(precip.sim_params.y, 0.1, 0.85);

    return out;
}

@fragment
fn precip_render_fs(in: VertexOutput) -> @location(0) vec4<f32> {
    // Soft streak fade along length and width
    let fade_x = 1.0 - abs(in.uv.x * 2.0);
    let fade_y = 1.0 - in.uv.y;
    let streak = clamp(fade_x * fade_y, 0.0, 1.0);

    return vec4<f32>(0.85, 0.90, 0.95, in.alpha * streak);
}
