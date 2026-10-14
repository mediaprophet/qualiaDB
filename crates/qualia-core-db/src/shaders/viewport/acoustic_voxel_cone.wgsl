// Acoustic Voxel Cone Tracing WGSL Compute Kernel
//
// Performs continuous acoustic ray/cone marching through a 32x32x32 toroidal
// acoustic absorption volume (rgba8uint).
//
// Frequency bands:
//   R: Low-frequency absorption coefficient (200 Hz anchor)
//   G: Mid-frequency absorption coefficient (1000 Hz anchor)
//   B: High-frequency absorption coefficient (8000 Hz anchor)
//   A: Density / acoustic impedance boundary flag
//
// Step size scaling:
//   dt = max(voxel_size, t * tan(theta / 2))
//
// Volumetric accumulation:
//   T_k = T_{k-1} * (1.0 - alpha_k * dt)
//
// Early termination:
//   Terminates when T < 0.01 across all 3 bands.

struct AcousticConeUniform {
    voxel_size: f32,
    grid_dim: u32,
    cone_angle: f32,
    audibility_threshold: f32,
    max_distance: f32,
    step_limit: u32,
    _pad0: u32,
    _pad1: u32,
};

struct AcousticRayQuery {
    origin: vec3<f32>,
    cone_angle: f32,
    direction: vec3<f32>,
    max_dist: f32,
};

struct AcousticRayResult {
    transmittance: vec3<f32>, // Low (200Hz), Mid (1000Hz), High (8000Hz)
    occlusion: f32,           // 1.0 - average transmittance
    accumulated_distance: f32,
    step_count: u32,
    early_terminated: u32,
    _pad: u32,
};

@group(0) @binding(0) var<uniform> config: AcousticConeUniform;
@group(0) @binding(1) var acoustic_volume: texture_storage_3d<rgba8uint, read>;
@group(0) @binding(2) var<storage, read> ray_queries: array<AcousticRayQuery>;
@group(0) @binding(3) var<storage, read_write> ray_results: array<AcousticRayResult>;

fn sample_toroidal_volume(world_pos: vec3<f32>) -> vec4<f32> {
    let v_float = world_pos / config.voxel_size;
    let v_int = vec3<i32>(floor(v_float));
    let fract_v = fract(v_float);

    var samples: array<vec4<f32>, 8>;
    var idx = 0u;

    // Fetch the 8 cube corners
    for (var z = 0; z <= 1; z++) {
        for (var y = 0; y <= 1; y++) {
            for (var x = 0; x <= 1; x++) {
                let offset = vec3<i32>(x, y, z);
                let coord = v_int + offset;

                // Toroidal modulo: ((coord % 32) + 32) % 32
                let wrapped_coord = vec3<u32>(((coord % 32) + 32) % 32);

                // Fetch and unpack rgba8uint to float [0, 1]
                let raw_val = textureLoad(acoustic_volume, wrapped_coord);
                samples[idx] = vec4<f32>(raw_val) / 255.0;
                idx++;
            }
        }
    }

    // Manual trilinear mix using fract_v
    let mix_x0 = mix(samples[0], samples[1], fract_v.x);
    let mix_x1 = mix(samples[2], samples[3], fract_v.x);
    let mix_x2 = mix(samples[4], samples[5], fract_v.x);
    let mix_x3 = mix(samples[6], samples[7], fract_v.x);
    let mix_y0 = mix(mix_x0, mix_x1, fract_v.y);
    let mix_y1 = mix(mix_x2, mix_x3, fract_v.y);
    return mix(mix_y0, mix_y1, fract_v.z);
}

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let ray_idx = global_id.x;
    if (ray_idx >= arrayLength(&ray_queries)) {
        return;
    }

    let query = ray_queries[ray_idx];
    let dir = normalize(query.direction);
    let angle = select(config.cone_angle, query.cone_angle, query.cone_angle > 0.0);
    let tan_half_theta = tan(angle * 0.5);

    let max_d = select(config.max_distance, query.max_dist, query.max_dist > 0.0);
    let audibility_thresh = select(0.01, config.audibility_threshold, config.audibility_threshold > 0.0);
    let max_steps = select(128u, config.step_limit, config.step_limit > 0u);

    var t = config.voxel_size * 0.5;
    var transmittance = vec3<f32>(1.0, 1.0, 1.0);
    var early_term = 0u;
    var steps = 0u;

    while (t < max_d && steps < max_steps) {
        // Delta t: max(voxel_size, t * tan(theta / 2))
        let delta_t = max(config.voxel_size, t * tan_half_theta);
        let sample_pos = query.origin + dir * t;

        // 8-tap trilinear toroidal fetch
        let absorption = sample_toroidal_volume(sample_pos);
        let alpha = clamp(absorption.rgb, vec3<f32>(0.0), vec3<f32>(1.0));

        // Transmittance compositing across 3 frequency bands:
        // T_k = T_{k-1} * (1.0 - alpha_k * delta_t)
        let decay = clamp(vec3<f32>(1.0) - alpha * delta_t, vec3<f32>(0.0), vec3<f32>(1.0));
        transmittance = transmittance * decay;
        steps++;

        // Early termination when transmittance drops below audibility threshold on all 3 bands
        if (transmittance.x < audibility_thresh &&
            transmittance.y < audibility_thresh &&
            transmittance.z < audibility_thresh) {
            early_term = 1u;
            break;
        }

        t += delta_t;
    }

    let avg_t = (transmittance.x + transmittance.y + transmittance.z) / 3.0;
    let occ = clamp(1.0 - avg_t, 0.0, 1.0);

    ray_results[ray_idx] = AcousticRayResult(
        transmittance,
        occ,
        t,
        steps,
        early_term,
        0u
    );
}
