// Ambient visualization fragment shader
// Creates soft, glowing particles with additive blending
// Zero CPU cost - all effects in shaders

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) size: f32,
};

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
