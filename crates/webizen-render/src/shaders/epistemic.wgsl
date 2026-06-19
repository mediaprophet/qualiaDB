// Epistemic Fragment Shader
// Implements epistemic material system with certainty-based visual fidelity

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
    confidence: f32,    // Certainty weight (0.0 = uncertain, 1.0 = certain)
    intensity: f32,     // Semantic intensity
    _pad: vec2<f32>,
}

// Epistemic Level of Detail: Adjust opacity based on certainty
fn epistemic_lod(base_color: vec4<f32>, confidence: f32, intensity: f32) -> vec4<f32> {
    // Uncertainty increases translucency
    let certainty_opacity = mix(0.3, 1.0, confidence);
    
    // Intensity affects color brightness
    let intensity_factor = mix(0.5, 1.0, intensity);
    
    return vec4<f32>(
        base_color.rgb * intensity_factor,
        base_color.a * certainty_opacity
    );
}

@fragment
fn fragment_main(input: FragmentInput) -> FragmentOutput {
    var output: FragmentOutput;
    
    // Apply epistemic material system
    output.color = epistemic_lod(
        input.color,
        epistemic_params.confidence,
        epistemic_params.intensity
    );
    
    return output;
}
