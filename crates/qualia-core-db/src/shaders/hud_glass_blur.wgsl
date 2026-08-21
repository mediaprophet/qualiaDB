// HUD Glassmorphism & Chromatic Dispersion Dual-Pass Blur Compute Kernel
// Evaluates depth-aware frosted glass refraction and RGB dispersion for holographic UI.

struct GlassParams {
    width: u32,
    height: u32,
    blur_radius: f32,
    dispersion: f32, // Chromatic separation in pixels
    refraction_index: f32,
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var<uniform> params: GlassParams;
@group(0) @binding(1) var input_texture: texture_2d<f32>;
@group(0) @binding(2) var output_texture: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(16, 16)
fn hud_glass_blur_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if (x >= params.width || y >= params.height) {
        return;
    }

    let coord = vec2<i32>(i32(x), i32(y));
    let radius = i32(params.blur_radius);
    let disp = i32(params.dispersion);

    var r_sum = 0.0;
    var g_sum = 0.0;
    var b_sum = 0.0;
    var weight_sum = 0.0;

    let sigma = max(f32(radius) * 0.5, 1.0);
    let two_sigma_sq = 2.0 * sigma * sigma;

    for (var dy = -radius; dy <= radius; dy = dy + 1) {
        for (var dx = -radius; dx <= radius; dx = dx + 1) {
            let dist_sq = f32(dx * dx + dy * dy);
            if (dist_sq <= f32(radius * radius)) {
                let w = exp(-dist_sq / two_sigma_sq);
                weight_sum = weight_sum + w;

                let r_coord = clamp(coord + vec2<i32>(dx - disp, dy), vec2<i32>(0, 0), vec2<i32>(i32(params.width - 1u), i32(params.height - 1u)));
                let g_coord = clamp(coord + vec2<i32>(dx, dy), vec2<i32>(0, 0), vec2<i32>(i32(params.width - 1u), i32(params.height - 1u)));
                let b_coord = clamp(coord + vec2<i32>(dx + disp, dy), vec2<i32>(0, 0), vec2<i32>(i32(params.width - 1u), i32(params.height - 1u)));

                r_sum = r_sum + textureLoad(input_texture, r_coord, 0).r * w;
                g_sum = g_sum + textureLoad(input_texture, g_coord, 0).g * w;
                b_sum = b_sum + textureLoad(input_texture, b_coord, 0).b * w;
            }
        }
    }

    let inv_w = 1.0 / max(weight_sum, 1e-4);
    let blurred_color = vec4<f32>(r_sum * inv_w, g_sum * inv_w, b_sum * inv_w, params.opacity);

    textureStore(output_texture, coord, blurred_color);
}
