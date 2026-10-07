// Qualia's versioned SDR output transform (PBR Neutral v1 + exactly one sRGB encoding).
// The PBR Neutral coefficients and operation order follow Khronos ToneMapping/PBR_Neutral.
fn pbr_neutral_v1(input_color: vec3<f32>) -> vec3<f32> {
    var color = max(input_color, vec3<f32>(0.0));
    let min_channel = min(color.r, min(color.g, color.b));
    var offset = 0.04;
    if min_channel < 0.08 {
        offset = min_channel - 6.25 * min_channel * min_channel;
    }
    color -= vec3<f32>(offset);

    let peak = max(color.r, max(color.g, color.b));
    let start_compression = 0.8 - 0.04;
    if peak < start_compression {
        return color;
    }

    let d = 1.0 - start_compression;
    let new_peak = 1.0 - d * d / (peak + d - start_compression);
    color *= new_peak / peak;
    let gray_mix = 1.0 - 1.0 / (0.15 * (peak - new_peak) + 1.0);
    return mix(color, vec3<f32>(new_peak), gray_mix);
}

fn linear_to_srgb(linear: vec3<f32>) -> vec3<f32> {
    let low = linear * 12.92;
    let high = 1.055 * pow(linear, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(high, low, linear <= vec3<f32>(0.0031308));
}

fn qualia_sdr_output(
    scene_linear: vec3<f32>,
    exposure: f32,
    white_balance_gains: vec3<f32>,
    surface_is_srgb: f32
) -> vec3<f32> {
    let display_linear = pbr_neutral_v1(scene_linear * exposure * white_balance_gains);
    if surface_is_srgb > 0.5 {
        return display_linear;
    }
    return linear_to_srgb(display_linear);
}
