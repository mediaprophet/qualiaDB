// Deferred Resolve Compute Kernel: Continuous EMF Spectral Integration & Decoupled CMF Projection.
//
// Complies with Update-Frequency Bind Group Architecture:
// - Group 0: Frame-Global Data (Camera, Clustered Grid, Lights, Blue Noise)
// - Group 1: Pass-Specific G-Buffers (c0..c2/Roughness, Normal/Metallic, Depth)
// - Group 2: Output Targets (Resolve Out RGBA16F)
// - Group 3: Reserved for dynamic environment probes / shadow atlases.
//
// Decoupled CMF Architecture:
// Accumulates spectral radiance into a 4-component Hero Wavelength vector inside the
// cluster light loop (4 FMAs per light), maintaining < 32 registers per invocation
// for 100% occupancy on mobile TBDR architectures (Apple G14, Adreno 740, Mali G715).
// Projects into CIE 1931 XYZ tristimulus space only AFTER the light loop completes.

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    viewport_size: vec4<f32>,   // xy = dimensions, zw = 1.0 / dimensions
    temporal_jitter: vec4<f32>, // xy = jitter in NDC, z = frame_index, w = golden_ratio_offset
};

struct Light {
    position: vec4<f32>,     // xyz = world position, w = radius
    direction: vec4<f32>,    // xyz = direction, w = spot inner cosine
    radiance: vec4<f32>,     // xyz = linear RGB / radiant flux, w = illuminance lux
    params: vec4<f32>,       // x = type (0=dir, 1=point, 2=spot), y = cast_shadow, z = spot outer cosine, w = active
};

// Group 0: Frame-Global Data
@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<storage, read> cluster_grid: array<u32>;
@group(0) @binding(2) var<storage, read> light_data: array<Light>;
@group(0) @binding(3) var blue_noise_tex: texture_2d<f32>;
@group(0) @binding(4) var clamp_sampler: sampler;

// Group 1: Pass-Specific G-Buffer Inputs
@group(1) @binding(0) var gbuffer0: texture_2d<f32>; // c0, c1, c2, Roughness
@group(1) @binding(1) var gbuffer1: texture_2d<f32>; // Normal.xy, Metallic, Flags
@group(1) @binding(2) var depth_buffer: texture_depth_2d;

// Group 2: Output Targets
@group(2) @binding(0) var resolve_out: texture_storage_2d<rgba16float, write>;

const PI: f32 = 3.141592653589793;
const INV_PI: f32 = 0.3183098861837907;

fn sign_not_zero(v: f32) -> f32 {
    return select(-1.0, 1.0, v >= 0.0);
}

// Decode octahedral normal in [0.0, 1.0]^2 to unit 3D world normal.
fn decode_oct16(oct: vec2<f32>) -> vec3<f32> {
    let p = oct * 2.0 - vec2<f32>(1.0);
    var n = vec3<f32>(p.x, p.y, 1.0 - abs(p.x) - abs(p.y));
    if (n.z < 0.0) {
        let old_x = n.x;
        n.x = (1.0 - abs(n.y)) * sign_not_zero(old_x);
        n.y = (1.0 - abs(old_x)) * sign_not_zero(n.y);
    }
    return normalize(n);
}

// Reconstruct world position from reversed-Z depth buffer.
fn reconstruct_world_pos(pixel_coord: vec2<i32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel_coord) + vec2<f32>(0.5)) * camera.viewport_size.zw;
    let ndc = vec4<f32>(
        uv.x * 2.0 - 1.0,
        (1.0 - uv.y) * 2.0 - 1.0,
        depth,
        1.0
    );
    let world_h = camera.inv_view_proj * ndc;
    return world_h.xyz / max(world_h.w, 1e-7);
}

// Wyman et al. (2013) Analytic Gaussian Sum Color Matching Functions (CMFs)
fn eval_cmf_x(lambda: vec4<f32>) -> vec4<f32> {
    let x1 = 1.056 * exp(-0.5 * pow((lambda - 599.8) / 43.2, vec4<f32>(2.0)));
    let x2 = 0.362 * exp(-0.5 * pow((lambda - 442.0) / 32.0, vec4<f32>(2.0)));
    let x3 = -0.065 * exp(-0.5 * pow((lambda - 501.1) / 20.4, vec4<f32>(2.0)));
    return max(x1 + x2 + x3, vec4<f32>(0.0));
}

fn eval_cmf_y(lambda: vec4<f32>) -> vec4<f32> {
    let y1 = 0.821 * exp(-0.5 * pow((lambda - 568.8) / 46.9, vec4<f32>(2.0)));
    let y2 = 0.286 * exp(-0.5 * pow((lambda - 530.9) / 16.3, vec4<f32>(2.0)));
    return max(y1 + y2, vec4<f32>(0.0));
}

fn eval_cmf_z(lambda: vec4<f32>) -> vec4<f32> {
    let z1 = 1.217 * exp(-0.5 * pow((lambda - 437.0) / 11.8, vec4<f32>(2.0)));
    let z2 = 0.681 * exp(-0.5 * pow((lambda - 459.0) / 26.0, vec4<f32>(2.0)));
    return max(z1 + z2, vec4<f32>(0.0));
}

// Convert CIE 1931 XYZ to linear sRGB primaries (D65 white point).
fn xyz_to_linear_srgb(xyz: vec3<f32>) -> vec3<f32> {
    let r =  3.2404542 * xyz.x - 1.5371385 * xyz.y - 0.4985314 * xyz.z;
    let g = -0.9692660 * xyz.x + 1.8760108 * xyz.y + 0.0415560 * xyz.z;
    let b =  0.0556434 * xyz.x - 0.2040259 * xyz.y + 1.0572252 * xyz.z;
    return max(vec3<f32>(r, g, b), vec3<f32>(0.0));
}

// Jakob & Hanika (2019) 6-ALU cycle bounded spectral basis evaluation.
fn evaluate_spectral_reflectance(coeffs: vec3<f32>, u: vec4<f32>) -> vec4<f32> {
    let f = fma(u, fma(vec4<f32>(coeffs.x), u, vec4<f32>(coeffs.y)), vec4<f32>(coeffs.z));
    let denom = fma(f, f, vec4<f32>(1.0));
    return fma(f * inverseSqrt(denom), vec4<f32>(0.5), vec4<f32>(0.5));
}

@compute @workgroup_size(8, 8, 1)
fn cs_deferred_resolve(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel_coord = vec2<i32>(global_id.xy);
    let dims = vec2<i32>(camera.viewport_size.xy);

    if (pixel_coord.x >= dims.x || pixel_coord.y >= dims.y) {
        return;
    }

    // 1. Fetch depth and early out for sky/background
    let depth = textureLoad(depth_buffer, pixel_coord, 0);
    if (depth <= 0.0 || depth >= 1.0) {
        // Output neutral background
        textureStore(resolve_out, pixel_coord, vec4<f32>(0.02, 0.02, 0.04, 1.0));
        return;
    }

    // 2. Fetch G-Buffers
    let gb0 = textureLoad(gbuffer0, pixel_coord, 0);
    let gb1 = textureLoad(gbuffer1, pixel_coord, 0);

    let spectral_coeffs = gb0.xyz;
    let roughness = clamp(gb0.w, 0.04, 1.0);
    let normal = decode_oct16(gb1.xy);
    let metallic = gb1.z;

    // 3. World Position & View Direction
    let world_pos = reconstruct_world_pos(pixel_coord, depth);
    let V = normalize(camera.camera_pos.xyz - world_pos);

    // 4. Per-Pixel Blue-Noise Hero Wavelength Generation
    let screen_tile = (pixel_coord % 64);
    let base_noise = textureLoad(blue_noise_tex, screen_tile, 0).r;
    let temporal_noise = fract(base_noise + camera.temporal_jitter.w);

    let lambda_0 = 380.0 + temporal_noise * 400.0;
    let offsets = vec4<f32>(0.0, 0.25, 0.5, 0.75);
    let hero_lambdas = 380.0 + fract(vec4<f32>((lambda_0 - 380.0) / 400.0) + offsets) * 400.0;
    let hero_u = (hero_lambdas - 380.0) / 400.0;

    // 5. Evaluate Material Reflectance across 4 Hero Wavelengths
    let hero_albedo = evaluate_spectral_reflectance(spectral_coeffs, hero_u);

    // 6. Precompute CMF Weights for the 4 Hero Wavelengths (Decoupled from Light Loop)
    let cmf_x = eval_cmf_x(hero_lambdas);
    let cmf_y = eval_cmf_y(hero_lambdas);
    let cmf_z = eval_cmf_z(hero_lambdas);

    // 7. Light Loop — Accumulates Hero Spectral Radiance (4 FMAs per light)
    var hero_radiance = vec4<f32>(0.0);
    let num_lights = arrayLength(&light_data);

    for (var i = 0u; i < num_lights; i = i + 1u) {
        let light = light_data[i];
        if (light.params.w == 0.0) {
            continue;
        }

        var L: vec3<f32>;
        var attenuation = 1.0;

        if (light.params.x == 0.0) {
            // Directional Light
            L = normalize(-light.direction.xyz);
            attenuation = light.radiance.w; // Lux
        } else {
            // Point or Spot Light
            let to_light = light.position.xyz - world_pos;
            let dist_sq = dot(to_light, to_light);
            let dist = sqrt(dist_sq);
            L = to_light / max(dist, 1e-5);

            let radius = light.position.w;
            let d_over_r = dist / max(radius, 1e-4);
            let smooth_att = clamp(1.0 - d_over_r * d_over_r, 0.0, 1.0);
            attenuation = (smooth_att * smooth_att) / max(dist_sq, 0.01) * light.radiance.w;

            if (light.params.x == 2.0) {
                // Spot cone attenuation
                let spot_cos = dot(-L, normalize(light.direction.xyz));
                let inner_cos = light.direction.w;
                let outer_cos = light.params.z;
                let spot_att = clamp((spot_cos - outer_cos) / max(inner_cos - outer_cos, 1e-4), 0.0, 1.0);
                attenuation = attenuation * (spot_att * spot_att);
            }
        }

        let n_dot_l = max(dot(normal, L), 0.0);
        if (n_dot_l <= 0.0) {
            continue;
        }

        let H = normalize(L + V);
        let n_dot_h = max(dot(normal, H), 0.0);
        let n_dot_v = max(dot(normal, V), 1e-5);
        let v_dot_h = max(dot(V, H), 0.0);

        // Diffuse Lambertian in Spectral Space
        let diffuse_spectral = hero_albedo * (INV_PI * (1.0 - metallic));

        // Cook-Torrance Specular BRDF (scalar geometry & distribution)
        let a = roughness * roughness;
        let a2 = a * a;
        let denom_d = fma(n_dot_h * n_dot_h, a2 - 1.0, 1.0);
        let D = a2 / max(PI * denom_d * denom_d, 1e-7);

        let k = (roughness + 1.0) * (roughness + 1.0) * 0.125;
        let G1_l = n_dot_l / fma(n_dot_l, 1.0 - k, k);
        let G1_v = n_dot_v / fma(n_dot_v, 1.0 - k, k);
        let G = G1_l * G1_v;

        // Schlick Fresnel across hero wavelengths
        let f0_hero = mix(vec4<f32>(0.04), hero_albedo, metallic);
        let f90 = 1.0;
        let fresnel_hero = fma(vec4<f32>(f90) - f0_hero, vec4<f32>(pow(1.0 - v_dot_h, 5.0)), f0_hero);

        let spec_scalar = (D * G) / max(4.0 * n_dot_l * n_dot_v, 1e-7);
        let spec_spectral = fresnel_hero * spec_scalar;

        let bsdf_hero = (diffuse_spectral + spec_spectral) * n_dot_l * attenuation;

        // 4 FMAs per light accumulation
        hero_radiance = fma(vec4<f32>(light.radiance.x), bsdf_hero, hero_radiance);
    }

    // 8. Project Accumulated Hero Spectral Radiance into CIE 1931 XYZ
    let X = dot(hero_radiance, cmf_x) * 0.25;
    let Y = dot(hero_radiance, cmf_y) * 0.25;
    let Z = dot(hero_radiance, cmf_z) * 0.25;

    // 9. Convert CIE XYZ to Linear sRGB
    let linear_rgb = xyz_to_linear_srgb(vec3<f32>(X, Y, Z));

    // 10. Store in Output Buffer
    textureStore(resolve_out, pixel_coord, vec4<f32>(linear_rgb, 1.0));
}
