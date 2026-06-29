// Molecular-dynamics integrator — velocity-Verlet drift/kick under a constant force
// over one step `dt`, with periodic boundary conditions (PBC).
//
// This is a CERTIFIED forge kernel: its exact CPU oracle and the naga-validation /
// GPU-certify tests live in `wgsl_forge::physics::molecular_dynamics`, which embeds
// this file via `include_str!` so there is a single source of truth.
//
// Flat scalar layout (NO `vec3` padding — std430 pads each `vec3` to 16 B, which the
// earlier struct version got wrong): one molecule is 10 contiguous `f32`:
//   [ px, py, pz,  vx, vy, vz,  fx, fy, fz,  mass ]
// `state` is `count * 10` long; one invocation updates one molecule's own slots only
// (it reads no other molecule), so the in-place read_write buffer is race-free.
//
// `params = [ box_x, box_y, box_z, dt ]`. For a force `f` held constant across the
// step, velocity-Verlet reduces to the exact closed form
//   x(t+dt) = x(t) + v(t)·dt + ½·(f/m)·dt²
//   v(t+dt) = v(t) + (f/m)·dt
// (the previous shader updated position but LEFT VELOCITY UNCHANGED — fixed here).
// Positions are then wrapped into `[0, box)` per axis by `x − box·floor(x/box)`.

@group(0) @binding(0) var<storage, read_write> state: array<f32>;
@group(0) @binding(1) var<storage, read> params: array<f32>;

// Wrap `v` into `[0, b)` for `b > 0` (PBC); a non-positive box leaves `v` untouched.
fn wrap(v: f32, b: f32) -> f32 {
    if (b <= 0.0) {
        return v;
    }
    return v - b * floor(v / b);
}

@compute @workgroup_size(64)
fn md_step(@builtin(global_invocation_id) gid: vec3<u32>) {
    let stride = 10u;
    let count = arrayLength(&state) / stride;
    let i = gid.x;
    if (i >= count) {
        return;
    }
    let base = i * stride;
    let dt = params[3];

    let mass = state[base + 9u];
    let inv_m = select(0.0, 1.0 / mass, mass != 0.0);
    // Acceleration a = f / m.
    let ax = state[base + 6u] * inv_m;
    let ay = state[base + 7u] * inv_m;
    let az = state[base + 8u] * inv_m;

    // Position: x += v·dt + ½·a·dt².
    let half_dt2 = 0.5 * dt * dt;
    var px = state[base + 0u] + state[base + 3u] * dt + ax * half_dt2;
    var py = state[base + 1u] + state[base + 4u] * dt + ay * half_dt2;
    var pz = state[base + 2u] + state[base + 5u] * dt + az * half_dt2;

    // Velocity: v += a·dt.
    state[base + 3u] = state[base + 3u] + ax * dt;
    state[base + 4u] = state[base + 4u] + ay * dt;
    state[base + 5u] = state[base + 5u] + az * dt;

    // Periodic boundary conditions.
    state[base + 0u] = wrap(px, params[0]);
    state[base + 1u] = wrap(py, params[1]);
    state[base + 2u] = wrap(pz, params[2]);
}
