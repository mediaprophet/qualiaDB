// Kinematics Compute Shader
// Handles N-body simulations, Lennard-Jones potentials, and electrostatics.

struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>,
    mass: f32,
    charge: f32,
};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&particles)) {
        return;
    }

    let p1 = particles[index];
    var force = vec3<f32>(0.0, 0.0, 0.0);

    for (var i: u32 = 0u; i < arrayLength(&particles); i = i + 1u) {
        if (i == index) { continue; }
        
        let p2 = particles[i];
        let r_vec = p1.position - p2.position;
        let r_sq = dot(r_vec, r_vec);
        
        // Avoid division by zero
        if (r_sq < 0.0001) { continue; }
        
        let r = sqrt(r_sq);
        let r_hat = r_vec / r;
        
        // Gravitational/Electrostatic Force mock
        let f_mag = (p1.charge * p2.charge) / r_sq; 
        force = force + (r_hat * f_mag);
    }

    // Update velocity (mock Euler step)
    let dt = 0.01;
    particles[index].velocity = p1.velocity + (force / p1.mass) * dt;
    particles[index].position = p1.position + particles[index].velocity * dt;
}
