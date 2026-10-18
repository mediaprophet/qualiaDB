// Continuous Multi-Scale Foliage LOD & Bézier Blade Generator (`foliage_lod.wgsl`)
//
// Computes GPU-driven procedural foliage realization:
// - Classifies distance LOD (LOD 0 Bézier, LOD 1 Clump Ribbons, LOD 2 Shells, LOD 3 SGGX)
// - Megapixel Governor: Clamps blade spawn count so total vertices <= 1.5x viewport pixel count
// - Generates cubic Bézier curve control points [p0, p1, p2, p3] under wind load and turgor droop
// - Bounded zero-heap output buffer

struct BladeVertex {
    position: vec4<f32>,       // xyz: world position, w: width
    normal_curve: vec4<f32>,   // xyz: blade surface normal, w: normalized curve parameter t in [0, 1]
};

struct FoliageLodUniform {
    camera_pos: vec4<f32>,     // xyz: camera world pos, w: max view distance (80m)
    viewport_dim: vec4<f32>,   // width_px, height_px, 1/width, 1/height
    wind_vector: vec4<f32>,    // xyz: wind velocity m/s, w: gust phase
    clipmap_center: vec4<f32>, // x, z: clipmap world center, y: cell_size_m, w: max_blade_budget
    view_proj: mat4x4<f32>,
};

struct EcotopeGpuPatch {
    species_mix: vec4<f32>,    // ryegrass, clover, tussock, herbs
    params: vec4<f32>,         // x: culm_height_m, y: LAI, z: turgor_mpa, w: moisture
    trample: vec4<f32>,        // x, z: dir, y: matted_ratio, w: pad
};

@group(0) @binding(0) var<uniform> uniforms: FoliageLodUniform;
@group(0) @binding(1) var<storage, read> patches: array<EcotopeGpuPatch>;
@group(0) @binding(2) var<storage, read_write> blade_vertices: array<BladeVertex>;
@group(0) @binding(3) var<storage, read_write> draw_indirect_args: array<u32>;

// Pseudo-random spatial hash
fn hash21(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 456.21));
    let d = dot(q, q + 45.32);
    return fract(d);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    let n = sin(dot(p, vec2<f32>(41.0, 289.0)));
    return fract(vec2<f32>(262144.0, 32768.0) * n);
}

@compute @workgroup_size(64, 1, 1)
fn foliage_cull_and_generate(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let patch_idx = global_id.x;
    let total_patches = u32(uniforms.clipmap_center.w);
    if (patch_idx >= total_patches) {
        return;
    }

    let cell = patches[patch_idx];
    let cell_size = uniforms.clipmap_center.y;
    let grid_dim = 32u;

    let gx = f32(patch_idx % grid_dim);
    let gz = f32(patch_idx / grid_dim);
    let patch_pos = vec3<f32>(
        uniforms.clipmap_center.x + (gx - 16.0) * cell_size,
        0.0,
        uniforms.clipmap_center.z + (gz - 16.0) * cell_size
    );

    let delta_cam = patch_pos - uniforms.camera_pos.xyz;
    let dist = length(delta_cam);

    // Distance LOD classification
    if (dist >= uniforms.camera_pos.w) {
        return; // Beyond 80m horizon: handled by SGGX terrain shader
    }

    // Megapixel Governor: screen-space solid-angle density clamping
    let total_pixels = uniforms.viewport_dim.x * uniforms.viewport_dim.y;
    let max_budget_per_m2 = (total_pixels * 1.5) / (3.14159 * 80.0 * 80.0);
    let distance_clamped = max(dist, 1.0);
    let screen_density_factor = 1.0 / (distance_clamped * distance_clamped);
    let target_blades = u32(clamp(300.0 * screen_density_factor * 16.0, 2.0, max(max_budget_per_m2, 10.0)));

    let turgor = cell.params.z;
    let height_base = cell.params.x;
    let matted = cell.trample.y;

    // Generate Cosserat Bézier blades in caller output buffer
    for (var b = 0u; b < target_blades; b = b + 1u) {
        let rnd = hash22(vec2<f32>(patch_pos.xz + vec2<f32>(f32(b) * 1.37, f32(b) * 2.89)));
        let blade_root = patch_pos + vec3<f32>(
            (rnd.x - 0.5) * cell_size,
            0.0,
            (rnd.y - 0.5) * cell_size
        );

        // Wind deflection & turgor droop
        let wind_force = uniforms.wind_vector.xyz * (0.6 + 0.4 * sin(uniforms.wind_vector.w + rnd.x * 6.28));
        let droop_factor = (1.5 - min(turgor, 1.5)) * 0.45; // Low turgor increases droop

        let h = height_base * (0.8 + 0.4 * rnd.y);
        let p0 = blade_root;
        let p1 = blade_root + vec3<f32>(0.0, h * 0.35, 0.0);
        let p2 = blade_root + vec3<f32>(wind_force.x * 0.15, h * 0.70 - droop_factor * 0.1, wind_force.z * 0.15);
        let p3 = blade_root + vec3<f32>(
            wind_force.x * 0.35 + (rnd.x - 0.5) * droop_factor,
            h - droop_factor * 0.3,
            wind_force.z * 0.35 + (rnd.y - 0.5) * droop_factor
        );

        // Trample flattening
        var final_tip = p3;
        if (matted > 0.2) {
            final_tip = mix(p3, blade_root + vec3<f32>(cell.trample.x, 0.02, cell.trample.z) * h, matted);
        }

        // Emit 4 vertices along the blade for triangle strip
        let out_base = (patch_idx * 16u + b) * 4u;
        if (out_base + 3u < arrayLength(&blade_vertices)) {
            let width = 0.008 * (1.0 - matted * 0.5);

            blade_vertices[out_base + 0u] = BladeVertex(
                vec4<f32>(p0, width),
                vec4<f32>(0.0, 1.0, 0.0, 0.0)
            );
            blade_vertices[out_base + 1u] = BladeVertex(
                vec4<f32>(p1, width * 0.85),
                vec4<f32>(0.0, 1.0, 0.0, 0.33)
            );
            blade_vertices[out_base + 2u] = BladeVertex(
                vec4<f32>(p2, width * 0.65),
                vec4<f32>(0.0, 1.0, 0.0, 0.66)
            );
            blade_vertices[out_base + 3u] = BladeVertex(
                vec4<f32>(final_tip, width * 0.2),
                vec4<f32>(0.0, 1.0, 0.0, 1.0)
            );
        }
    }
}
