// Reactive mask producer for temporal accumulation.
// Combines surface reactivity weights (e.g. water flag, alpha-tested edges, particles)
// into a normalized scalar map [0.0, 1.0].

struct ReactiveParams {
    weights: vec4<f32>, // x: alpha_edge, y: water, z: particle, w: base
};

@group(0) @binding(0) var<uniform> params: ReactiveParams;

fn fullscreen_position(vertex_index: u32) -> vec4<f32> {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    return vec4<f32>(positions[vertex_index], 0.0, 1.0);
}

@vertex
fn temporal_reactive_vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    return fullscreen_position(vertex_index);
}

@fragment
fn temporal_reactive_fs(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(params.weights.w, 0.0, 0.0, 1.0);
}
