// Kinematics — one softened inverse-square N-body step (gravity / electrostatics) with
// a symplectic-Euler (Euler–Cromer) update.
//
// This is a CERTIFIED forge kernel: its exact CPU oracle and the naga-validation /
// GPU-certify tests live in `wgsl_forge::physics::kinematics`, which embeds this file
// via `include_str!` so there is a single source of truth.
//
// Flat scalar layout (no `vec3` padding): one particle is 8 contiguous `f32`:
//   [ px, py, pz,  vx, vy, vz,  mass, charge ]
//
// DOUBLE-BUFFERED to be deterministic: forces are read from `state_in` only and the
// result is written to `state_out`, so the answer is independent of the order in which
// invocations run (the previous shader read and wrote the SAME buffer — a data race
// that made the result non-deterministic; fixed here).
//
// `params = [ dt, softening², coupling ]`. The pairwise force on i is
//   F_i = coupling · Σ_{j≠i}  q_i·q_j · (r_ij) / (|r_ij|² + softening²)^{3/2},   r_ij = x_i − x_j
// (Plummer softening removes the r→0 singularity, so there is no data-dependent skip
// branch — the kernel is smooth and exactly reproducible by the CPU oracle.) Then
//   v_i ← v_i + (F_i / m_i)·dt ;   x_i ← x_i + v_i·dt   (uses the NEW velocity).
// `coupling` selects the law: +k·qᵢqⱼ for electrostatics (repulsive like-charges),
// −G for gravity (use mass in the charge slot).

@group(0) @binding(0) var<storage, read> state_in: array<f32>;
@group(0) @binding(1) var<storage, read_write> state_out: array<f32>;
@group(0) @binding(2) var<storage, read> params: array<f32>;

@compute @workgroup_size(64)
fn nbody_step(@builtin(global_invocation_id) gid: vec3<u32>) {
    let stride = 8u;
    let count = arrayLength(&state_in) / stride;
    let i = gid.x;
    if (i >= count) {
        return;
    }
    let dt = params[0];
    let soft = params[1];
    let coupling = params[2];

    let bi = i * stride;
    let pix = state_in[bi + 0u];
    let piy = state_in[bi + 1u];
    let piz = state_in[bi + 2u];
    let qi = state_in[bi + 7u];

    var fx = 0.0;
    var fy = 0.0;
    var fz = 0.0;
    for (var j: u32 = 0u; j < count; j = j + 1u) {
        if (j == i) {
            continue;
        }
        let bj = j * stride;
        let rx = pix - state_in[bj + 0u];
        let ry = piy - state_in[bj + 1u];
        let rz = piz - state_in[bj + 2u];
        let r2 = rx * rx + ry * ry + rz * rz + soft;
        // 1 / (r² + soft)^{3/2}.
        let inv = coupling * qi * state_in[bj + 7u] / (r2 * sqrt(r2));
        fx = fx + rx * inv;
        fy = fy + ry * inv;
        fz = fz + rz * inv;
    }

    let mass = state_in[bi + 6u];
    let inv_m = select(0.0, 1.0 / mass, mass != 0.0);
    let vx = state_in[bi + 3u] + fx * inv_m * dt;
    let vy = state_in[bi + 4u] + fy * inv_m * dt;
    let vz = state_in[bi + 5u] + fz * inv_m * dt;

    state_out[bi + 0u] = pix + vx * dt;
    state_out[bi + 1u] = piy + vy * dt;
    state_out[bi + 2u] = piz + vz * dt;
    state_out[bi + 3u] = vx;
    state_out[bi + 4u] = vy;
    state_out[bi + 5u] = vz;
    state_out[bi + 6u] = mass;
    state_out[bi + 7u] = qi;
}
