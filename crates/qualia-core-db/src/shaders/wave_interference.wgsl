// Optical/Spectral Wavefront Interference & Doppler Shift Compute Kernel
// Evaluates multi-source wave equations, constructive/destructive interference, and Doppler color shift.

struct WaveSource {
    pos_x: f32,
    pos_y: f32,
    vel_x: f32,
    vel_y: f32,
    frequency: f32,
    amplitude: f32,
    phase: f32,
    _pad: f32,
};

struct WaveParams {
    width: u32,
    height: u32,
    source_count: u32,
    time: f32,
    wave_speed: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var<uniform> params: WaveParams;
@group(0) @binding(1) var<storage, read> sources: array<WaveSource>;
@group(0) @binding(2) var output_field: texture_storage_2d<rgba8unorm, write>;

const PI: f32 = 3.14159265359;

@compute @workgroup_size(16, 16)
fn wave_interference_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if (x >= params.width || y >= params.height) {
        return;
    }

    let pos = vec2<f32>(f32(x), f32(y));
    var total_field = 0.0;
    var doppler_shift_sum = 0.0;

    for (var i = 0u; i < params.source_count; i = i + 1u) {
        let src = sources[i];
        let src_pos = vec2<f32>(src.pos_x + src.vel_x * params.time,
                                src.pos_y + src.vel_y * params.time);
        let delta = pos - src_pos;
        let dist = length(delta);

        let v_rel = dot(vec2<f32>(src.vel_x, src.vel_y), normalize(delta + vec2<f32>(1e-4, 1e-4)));
        let doppler_factor = params.wave_speed / max(params.wave_speed - v_rel, 1e-3);
        let f_eff = src.frequency * doppler_factor;

        let k = 2.0 * PI * f_eff / max(params.wave_speed, 1e-3);
        let phase = k * dist - 2.0 * PI * f_eff * params.time + src.phase;
        let attenuation = 1.0 / sqrt(max(dist, 1.0));

        total_field = total_field + src.amplitude * attenuation * cos(phase);
        doppler_shift_sum = doppler_shift_sum + (doppler_factor - 1.0);
    }

    // Color mapping: normalized field amplitude + Doppler blue/red tint
    let norm_amp = clamp(total_field * 0.5 + 0.5, 0.0, 1.0);
    let blue_shift = clamp(doppler_shift_sum * 0.5, 0.0, 0.5);
    let red_shift = clamp(-doppler_shift_sum * 0.5, 0.0, 0.5);

    let color = vec4<f32>(
        clamp(norm_amp + red_shift, 0.0, 1.0),
        norm_amp * 0.8,
        clamp(norm_amp + blue_shift, 0.0, 1.0),
        1.0
    );

    textureStore(output_field, vec2<i32>(i32(x), i32(y)), color);
}
