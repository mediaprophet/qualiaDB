// Epistemic fragment shader — certainty / q-state LOD (pairs with projector pass)

struct FragmentInput {
    @location(0) color: vec4<f32>,
    @location(1) world_position: vec3<f32>,
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
}

@group(0) @binding(3)
var<uniform> epistemic_params: EpistemicParams;

struct EpistemicParams {
    confidence: f32,
    intensity: f32,
    _pad: vec2<f32>,
}

fn epistemic_lod(base_color: vec4<f32>, confidence: f32, intensity: f32) -> vec4<f32> {
    let certainty_opacity = mix(0.3, 1.0, confidence);
    let intensity_factor = mix(0.5, 1.0, intensity);
    return vec4<f32>(
        base_color.rgb * intensity_factor,
        base_color.a * certainty_opacity
    );
}

@fragment
fn fragment_main(input: FragmentInput) -> FragmentOutput {
    var output: FragmentOutput;
    output.color = epistemic_lod(
        input.color,
        epistemic_params.confidence,
        epistemic_params.intensity
    );
    return output;
}