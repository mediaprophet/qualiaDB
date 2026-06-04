// Molecular Dynamics Integrator Compute Shader
// Handles Verlet integrators and Periodic Boundary Conditions (PBCs).

struct Molecule {
    position: vec3<f32>,
    velocity: vec3<f32>,
    force: vec3<f32>,
    mass: f32,
};

struct PBCBounds {
    box_size: vec3<f32>,
};

@group(0) @binding(0) var<storage, read_write> molecules: array<Molecule>;
@group(0) @binding(1) var<uniform> bounds: PBCBounds;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&molecules)) {
        return;
    }

    var m = molecules[index];
    let dt = 0.001;

    // Velocity Verlet integrator step 1: Update position
    m.position = m.position + m.velocity * dt + 0.5 * (m.force / m.mass) * dt * dt;

    // Apply Periodic Boundary Conditions (PBCs)
    // Wrap positions to keep molecules inside the simulated box (prevent edge effects)
    m.position = m.position % bounds.box_size;
    if (m.position.x < 0.0) { m.position.x = m.position.x + bounds.box_size.x; }
    if (m.position.y < 0.0) { m.position.y = m.position.y + bounds.box_size.y; }
    if (m.position.z < 0.0) { m.position.z = m.position.z + bounds.box_size.z; }

    // (In a real MD shader, here we'd calculate new forces, and then Velocity Verlet step 2)

    molecules[index] = m;
}
