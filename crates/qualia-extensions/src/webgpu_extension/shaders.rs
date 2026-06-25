//! WGSL **kernel specs** mirroring the CPU reference solvers.
//!
//! These are the GPU dispatch kernels for the physics in this module. They are
//! provided as the authoritative description of the per-cell update; the CPU
//! solvers in the sibling files are the verifiable reference (analytic-solution
//! tested) and are what runs when no GPU is present. A GPU run routes through
//! the core engine's shared `wgpu` device rather than a second device here.

/// 2D incompressible Navier–Stokes — advection + viscous diffusion + pressure
/// gradient (the projection/Poisson pass runs as a separate Jacobi kernel).
/// Stencils use the true grid spacing `dx`/`dy` (`1/h²`), unlike the old mock.
pub const NAVIER_STOKES_2D: &str = r#"
struct SimParams {
    dt: f32,
    viscosity: f32,
    dx: f32,
    dy: f32,
    nx: u32,
    ny: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<storage, read>        vel_in:  array<vec2<f32>>;
@group(0) @binding(1) var<storage, read>        p_in:    array<f32>;
@group(0) @binding(2) var<storage, read_write>  vel_out: array<vec2<f32>>;
@group(0) @binding(3) var<uniform>              params:  SimParams;

fn widx(ix: i32, iy: i32) -> u32 {
    let nx = i32(params.nx);
    let ny = i32(params.ny);
    let x = ((ix % nx) + nx) % nx;
    let y = ((iy % ny) + ny) % ny;
    return u32(y * nx + x);
}

@compute @workgroup_size(16, 16, 1)
fn navier_stokes(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.nx || gid.y >= params.ny) { return; }
    let ix = i32(gid.x);
    let iy = i32(gid.y);
    let c = widx(ix, iy);

    let v  = vel_in[c];
    let vE = vel_in[widx(ix + 1, iy)];
    let vW = vel_in[widx(ix - 1, iy)];
    let vN = vel_in[widx(ix, iy + 1)];
    let vS = vel_in[widx(ix, iy - 1)];

    let inv_dx2 = 1.0 / (params.dx * params.dx);
    let inv_dy2 = 1.0 / (params.dy * params.dy);

    let lap = (vE + vW - 2.0 * v) * inv_dx2 + (vN + vS - 2.0 * v) * inv_dy2;

    let dudx = (vE - vW) / (2.0 * params.dx);
    let dudy = (vN - vS) / (2.0 * params.dy);
    let adv  = vec2<f32>(v.x * dudx.x + v.y * dudy.x, v.x * dudx.y + v.y * dudy.y);

    let pE = p_in[widx(ix + 1, iy)];
    let pW = p_in[widx(ix - 1, iy)];
    let pN = p_in[widx(ix, iy + 1)];
    let pS = p_in[widx(ix, iy - 1)];
    let grad_p = vec2<f32>((pE - pW) / (2.0 * params.dx), (pN - pS) / (2.0 * params.dy));

    vel_out[c] = v + params.dt * (-adv + params.viscosity * lap - grad_p);
}
"#;

/// 1D electromagnetics — the Yee FDTD leap-frog (Ey/Hz). One workgroup pass
/// advances Hz, the next advances Ey; the host alternates them per step.
pub const MAXWELL_FDTD_1D: &str = r#"
struct EmParams {
    dt: f32,
    dx: f32,
    eps: f32,
    mu: f32,
    sigma: f32,
    n: u32,
    stage: u32,   // 0 = update Hz, 1 = update Ey
    _pad: u32,
};

@group(0) @binding(0) var<storage, read_write> ey: array<f32>;
@group(0) @binding(1) var<storage, read_write> hz: array<f32>;
@group(0) @binding(2) var<uniform>             p:  EmParams;

@compute @workgroup_size(64, 1, 1)
fn maxwell_fdtd(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= p.n) { return; }
    let n = p.n;
    if (p.stage == 0u) {
        let ip1 = select(i + 1u, 0u, i + 1u >= n);
        hz[i] = hz[i] - (p.dt / (p.mu * p.dx)) * (ey[ip1] - ey[i]);
    } else {
        let im1 = select(i - 1u, n - 1u, i == 0u);
        ey[i] = ey[i] - (p.dt / (p.eps * p.dx)) * (hz[i] - hz[im1])
                      - (p.dt * p.sigma / p.eps) * ey[i];
    }
}
"#;

/// 2D heat diffusion — explicit forward-Euler 5-point Laplacian.
pub const HEAT_DIFFUSION_2D: &str = r#"
struct HeatParams { dt: f32, alpha: f32, dx: f32, dy: f32, nx: u32, ny: u32, _p0: u32, _p1: u32, };
@group(0) @binding(0) var<storage, read>       u_in:  array<f32>;
@group(0) @binding(1) var<storage, read_write> u_out: array<f32>;
@group(0) @binding(2) var<uniform>             params: HeatParams;

fn hidx(ix: i32, iy: i32) -> u32 {
    let nx = i32(params.nx); let ny = i32(params.ny);
    let x = ((ix % nx) + nx) % nx; let y = ((iy % ny) + ny) % ny;
    return u32(y * nx + x);
}

@compute @workgroup_size(16, 16, 1)
fn heat(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.nx || gid.y >= params.ny) { return; }
    let ix = i32(gid.x); let iy = i32(gid.y);
    let c = hidx(ix, iy);
    let lap = (u_in[hidx(ix + 1, iy)] + u_in[hidx(ix - 1, iy)] - 2.0 * u_in[c]) / (params.dx * params.dx)
            + (u_in[hidx(ix, iy + 1)] + u_in[hidx(ix, iy - 1)] - 2.0 * u_in[c]) / (params.dy * params.dy);
    u_out[c] = u_in[c] + params.dt * params.alpha * lap;
}
"#;

/// 2D wave equation — explicit leap-frog (u_old / u / u_new triple buffer).
pub const WAVE_EQUATION_2D: &str = r#"
struct WaveParams { c: f32, dt: f32, dx: f32, dy: f32, nx: u32, ny: u32, _p0: u32, _p1: u32, };
@group(0) @binding(0) var<storage, read>       u_old: array<f32>;
@group(0) @binding(1) var<storage, read>       u_cur: array<f32>;
@group(0) @binding(2) var<storage, read_write> u_new: array<f32>;
@group(0) @binding(3) var<uniform>             params: WaveParams;

fn vwidx(ix: i32, iy: i32) -> u32 {
    let nx = i32(params.nx); let ny = i32(params.ny);
    let x = ((ix % nx) + nx) % nx; let y = ((iy % ny) + ny) % ny;
    return u32(y * nx + x);
}

@compute @workgroup_size(16, 16, 1)
fn wave(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.nx || gid.y >= params.ny) { return; }
    let ix = i32(gid.x); let iy = i32(gid.y);
    let c = vwidx(ix, iy);
    let lap = (u_cur[vwidx(ix + 1, iy)] + u_cur[vwidx(ix - 1, iy)] - 2.0 * u_cur[c]) / (params.dx * params.dx)
            + (u_cur[vwidx(ix, iy + 1)] + u_cur[vwidx(ix, iy - 1)] - 2.0 * u_cur[c]) / (params.dy * params.dy);
    let coef = params.c * params.dt * params.c * params.dt;
    u_new[c] = 2.0 * u_cur[c] - u_old[c] + coef * lap;
}
"#;
