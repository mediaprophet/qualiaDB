// Analytical Mass-Spring-Damper Multi-Instance Grid Compute Kernel
// Evaluates parallel closed-form spring dynamics across thousands of vertices/particles.

struct SpringParticle {
    pos_x: f32,
    pos_y: f32,
    pos_z: f32,
    vel_x: f32,
    vel_y: f32,
    vel_z: f32,
    target_x: f32,
    target_y: f32,
    target_z: f32,
    stiffness: f32,
    damping: f32,
    mass: f32,
};

struct SpringParams {
    count: u32,
    dt: f32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var<uniform> params: SpringParams;
@group(0) @binding(1) var<storage, read> particles_in: array<SpringParticle>;
@group(0) @binding(2) var<storage, read_write> particles_out: array<SpringParticle>;

fn step_spring_1d(pos: f32, vel: f32, target: f32, k: f32, c: f32, m: f32, dt: f32) -> vec2<f32> {
    let x0 = pos - target;
    let v0 = vel;
    let mass_safe = max(m, 1e-4);
    let k_safe = max(k, 1e-4);

    let omega_n = sqrt(k_safe / mass_safe);
    let zeta = c / (2.0 * sqrt(mass_safe * k_safe));

    if (abs(zeta - 1.0) < 1e-3) {
        // Critical damping
        let decay = exp(-omega_n * dt);
        let c1 = x0;
        let c2 = v0 + omega_n * x0;
        let x = (c1 + c2 * dt) * decay;
        let v = (c2 - omega_n * (c1 + c2 * dt)) * decay;
        return vec2<f32>(target + x, v);
    } else if (zeta < 1.0) {
        // Under-damped oscillation
        let omega_d = omega_n * sqrt(1.0 - zeta * zeta);
        let decay = exp(-zeta * omega_n * dt);
        let cos_t = cos(omega_d * dt);
        let sin_t = sin(omega_d * dt);
        let c1 = x0;
        let c2 = (v0 + zeta * omega_n * x0) / omega_d;
        let x = decay * (c1 * cos_t + c2 * sin_t);
        let v = decay * ((-zeta * omega_n * c1 + omega_d * c2) * cos_t - (omega_d * c1 + zeta * omega_n * c2) * sin_t);
        return vec2<f32>(target + x, v);
    } else {
        // Over-damped
        let gamma = omega_n * sqrt(zeta * zeta - 1.0);
        let r1 = -zeta * omega_n + gamma;
        let r2 = -zeta * omega_n - gamma;
        let c2 = (v0 - r1 * x0) / (r2 - r1);
        let c1 = x0 - c2;
        let x = c1 * exp(r1 * dt) + c2 * exp(r2 * dt);
        let v = c1 * r1 * exp(r1 * dt) + c2 * r2 * exp(r2 * dt);
        return vec2<f32>(target + x, v);
    }
}

@compute @workgroup_size(64)
fn spring_grid_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.count) {
        return;
    }

    let p = particles_in[idx];
    let res_x = step_spring_1d(p.pos_x, p.vel_x, p.target_x, p.stiffness, p.damping, p.mass, params.dt);
    let res_y = step_spring_1d(p.pos_y, p.vel_y, p.target_y, p.stiffness, p.damping, p.mass, params.dt);
    let res_z = step_spring_1d(p.pos_z, p.vel_z, p.target_z, p.stiffness, p.damping, p.mass, params.dt);

    var out_p = p;
    out_p.pos_x = res_x.x;
    out_p.vel_x = res_x.y;
    out_p.pos_y = res_y.x;
    out_p.vel_y = res_y.y;
    out_p.pos_z = res_z.x;
    out_p.vel_z = res_z.y;

    particles_out[idx] = out_p;
}
