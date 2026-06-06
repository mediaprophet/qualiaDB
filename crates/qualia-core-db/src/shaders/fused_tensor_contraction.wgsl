@group(0) @binding(0) var<storage, read> input_activations: array<f32>;
@group(0) @binding(1) var<storage, read> weights: array<f32>;
@group(0) @binding(2) var<storage, read_write> output_logits: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    if i >= 4096u {
        return;
    }
    
    // Very simplified placeholder for a fused attention + FFN block.
    var sum = 0.0;
    for (var k = 0u; k < 4096u; k = k + 1u) {
        sum = sum + input_activations[k] * weights[i * 4096u + k];
    }
    
    output_logits[i] = max(0.0, sum); // ReLU mock
}
