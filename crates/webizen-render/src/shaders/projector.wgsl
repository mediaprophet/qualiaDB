// PGA Projector Vertex Shader
// Applies Projective Geometric Algebra (PGA) Motor transformations to vertices

struct Motor {
    // Rotor components (scalar + bivectors e12, e31, e23)
    rotor_scalar: f32,
    rotor_e12: f32,
    rotor_e31: f32,
    rotor_e23: f32,
    // Translator components (scalar + vector e1, e2, e3)
    trans_scalar: f32,
    trans_e1: f32,
    trans_e2: f32,
    trans_e3: f32,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world_position: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> view_projection: mat4x4<f32>;

@group(0) @binding(1)
var<storage, read> motors: array<Motor>;

@group(0) @binding(2)
var<uniform> motor_index: u32;

// Apply PGA Motor to a 3D point using quaternion rotation + translation
fn apply_motor(motor: Motor, point: vec3<f32>) -> vec3<f32> {
    // Extract rotation components (quaternion)
    let w = motor.rotor_scalar;
    let x = motor.rotor_e12;
    let y = motor.rotor_e31;
    let z = motor.rotor_e23;
    
    // Quaternion rotation formula
    let px = point.x;
    let py = point.y;
    let pz = point.z;
    
    let rotated_x = (1.0 - 2.0 * (y * y + z * z)) * px + 
                    (2.0 * (x * y - z * w)) * py + 
                    (2.0 * (x * z + y * w)) * pz;
    let rotated_y = (2.0 * (x * y + z * w)) * px + 
                    (1.0 - 2.0 * (x * x + z * z)) * py + 
                    (2.0 * (y * z - x * w)) * pz;
    let rotated_z = (2.0 * (x * z - y * w)) * px + 
                    (2.0 * (y * z + x * w)) * py + 
                    (1.0 - 2.0 * (x * x + y * y)) * pz;
    
    // Apply translation (2x the stored value)
    let translation = vec3<f32>(
        motor.trans_e1 * 2.0,
        motor.trans_e2 * 2.0,
        motor.trans_e3 * 2.0
    );
    
    return vec3<f32>(rotated_x, rotated_y, rotated_z) + translation;
}

@vertex
fn vertex_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Get the motor for this vertex
    let motor = motors[motor_index];
    
    // Apply PGA Motor transformation
    let world_pos = apply_motor(motor, input.position);
    
    // Project to clip space
    output.clip_position = view_projection * vec4<f32>(world_pos, 1.0);
    output.color = input.color;
    output.world_position = world_pos;
    
    return output;
}
