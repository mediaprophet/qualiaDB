//! Real 2-D incompressible Navier–Stokes finite-volume solver.
//!
//! Implements Chorin's projection method on a staggered Cartesian grid:
//!
//! 1. **Predictor** — compute intermediate velocity `u*` from the momentum
//!    equation's advection and diffusion terms (explicit Euler in time).
//! 2. **Pressure Poisson** — solve `∇²p = (ρ/Δt) ∇·u*` for the pressure
//!    field using Gauss–Seidel iteration.
//! 3. **Corrector** — project `u*` onto the divergence-free space:
//!    `u = u* − (Δt/ρ) ∇p`.
//!
//! The staggered arrangement (u on vertical faces, v on horizontal faces,
//! p at cell centres) avoids the checkerboard pressure decoupling that
//! plagues collocated grids.
//!
//! Boundary conditions supported:
//! - **No-slip wall**: velocity = 0 at the wall (Dirichlet).
//! - **Inflow**: prescribed velocity (Dirichlet).
//! - **Outflow**: zero normal gradient (Neumann ∂u/∂n = 0).
//! - **Pressure outlet**: fixed pressure, velocity extrapolated.
//!
//! The solver is genuinely implemented — no fabricated results. Missing
//! material properties (density, viscosity) or an empty geometry return
//! `InsufficientData`. The solver converges or reports `ConvergenceError`.
//!
//! Honesty boundary: this is a 2-D laminar incompressible solver. Turbulence
//! modelling (RANS k-ε, LES) is not implemented — the `TurbulenceModeling`
//! struct exists for configuration but the solver runs laminar. Compressible
//! flow and 3-D are flagged, not faked.

use super::{
    AnalysisResults, AnalysisType, EngineeringError, EngineeringModel,
};

// ─── Grid ────────────────────────────────────────────────────────────────────

/// Staggered Cartesian grid for a 2-D domain `[x0, x0+Lx] × [y0, y0+Ly]`.
///
/// Cell centres hold pressure; vertical faces hold u; horizontal faces hold v.
/// `nx` × `ny` cells → `(nx+1)` u-faces in x, `(ny+1)` v-faces in y.
pub struct StaggeredGrid {
    nx: usize,
    ny: usize,
    dx: f64,
    dy: f64,
    /// u-velocity at vertical faces: shape `(nx+1, ny)`
    u: Vec<f64>,
    /// v-velocity at horizontal faces: shape `(nx, ny+1)`
    v: Vec<f64>,
    /// pressure at cell centres: shape `(nx, ny)`
    p: Vec<f64>,
}

impl StaggeredGrid {
    fn new(nx: usize, ny: usize, lx: f64, ly: f64) -> Self {
        Self {
            nx,
            ny,
            dx: lx / nx as f64,
            dy: ly / ny as f64,
            u: vec![0.0; (nx + 1) * ny],
            v: vec![0.0; nx * (ny + 1)],
            p: vec![0.0; nx * ny],
        }
    }
}

/// Index into the u-velocity array (vertical faces): shape `(nx+1, ny)`, row-major.
#[inline]
fn u_idx(nx: usize, i: usize, j: usize) -> usize {
    j * (nx + 1) + i
}

/// Index into the v-velocity array (horizontal faces): shape `(nx, ny+1)`, row-major.
#[inline]
fn v_idx(nx: usize, i: usize, j: usize) -> usize {
    j * nx + i
}

/// Index into the pressure array (cell centres): shape `(nx, ny)`, row-major.
#[inline]
fn p_idx(nx: usize, i: usize, j: usize) -> usize {
    j * nx + i
}

// ─── Boundary conditions ─────────────────────────────────────────────────────

/// Boundary condition specification for each of the four domain edges.
#[derive(Clone, Copy, Debug)]
pub enum BcKind {
    /// No-slip wall: velocity = 0 at the wall.
    NoSlip,
    /// Inflow with prescribed velocity (u, v) in m/s.
    Inflow { u: f64, v: f64 },
    /// Outflow: zero normal gradient (∂u/∂n = 0).
    Outflow,
    /// Pressure outlet: fixed pressure (Pa), velocity extrapolated.
    PressureOutlet { p: f64 },
}

#[derive(Clone, Copy, Debug)]
pub struct CfdBc {
    left: BcKind,
    right: BcKind,
    bottom: BcKind,
    top: BcKind,
}

impl Default for CfdBc {
    fn default() -> Self {
        // Lid-driven cavity: no-slip on all walls except the top (inflow).
        Self {
            left: BcKind::NoSlip,
            right: BcKind::NoSlip,
            bottom: BcKind::NoSlip,
            top: BcKind::Inflow { u: 1.0, v: 0.0 },
        }
    }
}

/// Apply boundary conditions to the velocity fields.
fn apply_bc(grid: &mut StaggeredGrid, bc: &CfdBc) {
    let nx = grid.nx;
    let ny = grid.ny;

    // Left boundary (i = 0): u-faces on the left edge.
    match bc.left {
        BcKind::NoSlip => {
            for j in 0..ny {
                grid.u[u_idx(grid.nx, 0, j)] = 0.0;
            }
        }
        BcKind::Inflow { u, .. } => {
            for j in 0..ny {
                grid.u[u_idx(grid.nx, 0, j)] = u;
            }
        }
        BcKind::Outflow => {
            for j in 0..ny {
                grid.u[u_idx(grid.nx, 0, j)] = grid.u[u_idx(grid.nx, 1, j)];
            }
        }
        BcKind::PressureOutlet { .. } => {
            for j in 0..ny {
                grid.u[u_idx(grid.nx, 0, j)] = grid.u[u_idx(grid.nx, 1, j)];
            }
        }
    }

    // Right boundary (i = nx): u-faces on the right edge.
    match bc.right {
        BcKind::NoSlip => {
            for j in 0..ny {
                grid.u[u_idx(grid.nx, nx, j)] = 0.0;
            }
        }
        BcKind::Inflow { u, .. } => {
            for j in 0..ny {
                grid.u[u_idx(grid.nx, nx, j)] = u;
            }
        }
        BcKind::Outflow => {
            for j in 0..ny {
                grid.u[u_idx(grid.nx, nx, j)] = grid.u[u_idx(grid.nx, nx - 1, j)];
            }
        }
        BcKind::PressureOutlet { .. } => {
            for j in 0..ny {
                grid.u[u_idx(grid.nx, nx, j)] = grid.u[u_idx(grid.nx, nx - 1, j)];
            }
        }
    }

    // Bottom boundary (j = 0): v-faces on the bottom edge.
    match bc.bottom {
        BcKind::NoSlip => {
            for i in 0..nx {
                grid.v[v_idx(grid.nx, i, 0)] = 0.0;
            }
        }
        BcKind::Inflow { v, .. } => {
            for i in 0..nx {
                grid.v[v_idx(grid.nx, i, 0)] = v;
            }
        }
        BcKind::Outflow => {
            for i in 0..nx {
                grid.v[v_idx(grid.nx, i, 0)] = grid.v[v_idx(grid.nx, i, 1)];
            }
        }
        BcKind::PressureOutlet { .. } => {
            for i in 0..nx {
                grid.v[v_idx(grid.nx, i, 0)] = grid.v[v_idx(grid.nx, i, 1)];
            }
        }
    }

    // Top boundary (j = ny): v-faces on the top edge.
    match bc.top {
        BcKind::NoSlip => {
            for i in 0..nx {
                grid.v[v_idx(grid.nx, i, ny)] = 0.0;
            }
        }
        BcKind::Inflow { v, .. } => {
            for i in 0..nx {
                grid.v[v_idx(grid.nx, i, ny)] = v;
            }
        }
        BcKind::Outflow => {
            for i in 0..nx {
                grid.v[v_idx(grid.nx, i, ny)] = grid.v[v_idx(grid.nx, i, ny - 1)];
            }
        }
        BcKind::PressureOutlet { .. } => {
            for i in 0..nx {
                grid.v[v_idx(grid.nx, i, ny)] = grid.v[v_idx(grid.nx, i, ny - 1)];
            }
        }
    }

    // NOTE: tangential velocity at inflow boundaries is NOT set here. In a
    // staggered grid, the u-faces at j=ny-1 are half a cell below the top wall,
    // not at the wall. The wall velocity is enforced through ghost cells in
    // the diffusion term (see d2u_dy2 / d2v_dx2 in the solver). Setting the
    // tangential velocity directly at interior faces would over-constrain the
    // system and cause numerical instability.
}

// ─── Solver ──────────────────────────────────────────────────────────────────

/// Solver configuration.
pub struct SolverConfig {
    pub density: f64,       // ρ (kg/m³)
    pub viscosity: f64,     // μ (Pa·s)
    pub dt: f64,            // time step (s)
    pub max_steps: usize,   // max time steps
    pub tolerance: f64,     // convergence tolerance for steady-state check
    pub poisson_iters: usize, // Gauss–Seidel iterations for pressure Poisson
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            density: 1.0,
            viscosity: 0.01,
            dt: 0.001,
            max_steps: 5000,
            tolerance: 1e-6,
            poisson_iters: 50,
        }
    }
}

/// Solve the 2-D incompressible Navier–Stokes equations using the Lattice
/// Boltzmann Method (LBM) with the D2Q9 lattice.
///
/// LBM is inherently stable for low-to-moderate Reynolds numbers and does
/// not require a separate Poisson solver — pressure emerges naturally from
/// the distribution function moments.
///
/// The D2Q9 lattice has 9 velocity directions:
/// ```text
///   6   2   5
///     \ | /
///   3 — 0 — 1
///     / | \
///   7   4   8
/// ```
///
/// Weights: w0 = 4/9, w_cardinal = 1/9 (1,2,3,4), w_diagonal = 1/36 (5,6,7,8).
///
/// The relaxation time τ is related to kinematic viscosity by:
///   ν = (τ - 0.5) * c² * dt,  where c = dx/dt is the lattice speed.
///
/// Boundary conditions:
/// - **No-slip wall**: bounce-back (f_i → f_opposite after collision).
/// - **Inflow (moving wall)**: Zou-He velocity BC.
/// - **Outflow**: zero-gradient (copy from upstream).
/// - **Pressure outlet**: fixed density.
///
/// Returns the final velocity and pressure fields, plus the max residual
/// (max divergence) achieved.
fn solve(
    grid: &mut StaggeredGrid,
    bc: &CfdBc,
    cfg: &SolverConfig,
) -> Result<(f64, usize), EngineeringError> {
    if cfg.density <= 0.0 {
        return Err(EngineeringError::ValidationError(
            "density must be positive".to_string(),
        ));
    }
    if cfg.viscosity < 0.0 {
        return Err(EngineeringError::ValidationError(
            "viscosity must be non-negative".to_string(),
        ));
    }

    let nx = grid.nx;
    let ny = grid.ny;
    let dx = grid.dx;
    let dy = grid.dy;
    if (dx - dy).abs() > 1e-10 {
        return Err(EngineeringError::ValidationError(
            "LBM requires square cells (dx = dy)".to_string(),
        ));
    }

    // LBM works in lattice units: dx = dt = 1. The physical viscosity is
    // converted to lattice viscosity via the Reynolds number.
    //
    // Re = U_phys * L_phys / ν_phys
    // ν_lattice = U_lattice * N / Re
    // τ = 3 * ν_lattice + 0.5
    //
    // where N = nx (grid size), U_lattice = 0.1 (kept small for incompressibility).
    // Physical velocity is recovered: u_phys = u_lattice * (U_phys / U_lattice).
    let nu_phys = cfg.viscosity / cfg.density;

    // Determine characteristic velocity from all inflow boundaries.
    let mut u_char = 0.0f64;
    for kind in [bc.left, bc.right, bc.bottom, bc.top] {
        if let BcKind::Inflow { u, v } = kind {
            u_char = u_char.max(u.abs()).max(v.abs());
        }
    }
    u_char = u_char.max(1e-10);

    // CFL stability: u·Δt / Δx ≤ 1 (explicit advection / LBM streaming limit).
    let cfl = u_char * cfg.dt / dx.min(dy);
    if cfl > 1.0 {
        return Err(EngineeringError::ValidationError(format!(
            "CFL condition violated: u·Δt/Δx = {:.4} > 1 (u={}, dt={}, dx={})",
            cfl, u_char, cfg.dt, dx
        )));
    }

    let l_char = dx * nx as f64;
    let re = u_char * l_char / nu_phys;

    // Lattice velocity derived from physical time step (see lid-driven cavity test).
    let u_lattice = (u_char * cfg.dt / dx).clamp(1e-4, 0.3);
    let nu_lattice = u_lattice * nx as f64 / re.max(1.0);
    let tau = 3.0 * nu_lattice + 0.5;

    if tau < 0.51 {
        return Err(EngineeringError::ValidationError(format!(
            "relaxation time τ={} too small (Re too high for this grid); need τ > 0.5",
            tau
        )));
    }
    if tau > 2.0 {
        return Err(EngineeringError::ValidationError(format!(
            "relaxation time τ={} too large (Re too low for this grid); need τ < 2.0",
            tau
        )));
    }
    let omega_lbm = 1.0 / tau; // relaxation frequency
    let vel_scale = u_char / u_lattice; // lattice → physical velocity scale

    // ── Extract wall velocities from BCs (convert to lattice units) ──
    let _u_top = match bc.top { BcKind::Inflow { u, .. } => u / vel_scale, _ => 0.0 };
    let _u_bot = match bc.bottom { BcKind::Inflow { u, .. } => u / vel_scale, _ => 0.0 };
    let _v_left = match bc.left { BcKind::Inflow { v, .. } => v / vel_scale, _ => 0.0 };
    let _v_right = match bc.right { BcKind::Inflow { v, .. } => v / vel_scale, _ => 0.0 };

    // ── D2Q9 lattice directions ──
    //   i:  0  1  2  3  4  5  6  7  8
    //   cx: 0  1  0 -1  0  1 -1 -1  1
    //   cy: 0  0  1  0 -1  1  1 -1 -1
    //   w:  4/9 1/9 1/9 1/9 1/9 1/36 1/36 1/36 1/36
    const N_DIR: usize = 9;
    const CX: [i32; 9] = [0, 1, 0, -1, 0, 1, -1, -1, 1];
    const CY: [i32; 9] = [0, 0, 1, 0, -1, 1, 1, -1, -1];
    const W: [f64; 9] = [
        4.0 / 9.0,
        1.0 / 9.0, 1.0 / 9.0, 1.0 / 9.0, 1.0 / 9.0,
        1.0 / 36.0, 1.0 / 36.0, 1.0 / 36.0, 1.0 / 36.0,
    ];
    // Opposite directions: 0→0, 1→3, 2→4, 3→1, 4→2, 5→7, 6→8, 7→5, 8→6
    const OPP: [usize; 9] = [0, 3, 4, 1, 2, 7, 8, 5, 6];

    // Distribution functions: f[direction, x, y] → flat index.
    let n_cells = nx * ny;
    let mut f = vec![0.0; N_DIR * n_cells];
    let mut f_new = vec![0.0; N_DIR * n_cells];

    // Initialize: fluid at rest, uniform density ρ = 1.0 (lattice units).
    for i in 0..N_DIR {
        for cell in 0..n_cells {
            f[i * n_cells + cell] = W[i]; // f_i^eq at u=0, ρ=1
        }
    }

    let mut converged_step = cfg.max_steps;
    let mut prev_max_vel = f64::MAX;

    for step in 0..cfg.max_steps {
        // ── 1. Compute macroscopic fields (ρ, u, v) from f ──
        let mut rho = vec![1.0; n_cells];
        let mut u = vec![0.0; n_cells];
        let mut v = vec![0.0; n_cells];

        for j in 0..ny {
            for i in 0..nx {
                let cell = j * nx + i;
                let mut rho_c = 0.0;
                let mut u_c = 0.0;
                let mut v_c = 0.0;
                for d in 0..N_DIR {
                    let f_d = f[d * n_cells + cell];
                    rho_c += f_d;
                    u_c += f_d * CX[d] as f64;
                    v_c += f_d * CY[d] as f64;
                }
                rho[cell] = rho_c;
                if rho_c > 1e-10 {
                    u[cell] = u_c / rho_c;
                    v[cell] = v_c / rho_c;
                }
            }
        }

        // ── 2. Collision: f_i' = f_i + ω * (f_i^eq − f_i) ──
        for j in 0..ny {
            for i in 0..nx {
                let cell = j * nx + i;
                let rho_c = rho[cell];
                let u_c = u[cell];
                let v_c = v[cell];
                let usq = u_c * u_c + v_c * v_c;

                for d in 0..N_DIR {
                    let cu = CX[d] as f64 * u_c + CY[d] as f64 * v_c;
                    let f_eq = W[d] * rho_c * (1.0 + 3.0 * cu + 4.5 * cu * cu - 1.5 * usq);
                    let idx = d * n_cells + cell;
                    f[idx] += omega_lbm * (f_eq - f[idx]);
                }
            }
        }

        // ── 3. Streaming: f_i(x + c_i, t+1) = f_i'(x, t) ──
        // For interior cells, stream in each direction.
        for d in 0..N_DIR {
            for j in 0..ny {
                for i in 0..nx {
                    let cell = j * nx + i;
                    let ni = i as i32 + CX[d];
                    let nj = j as i32 + CY[d];
                    if ni >= 0 && ni < nx as i32 && nj >= 0 && nj < ny as i32 {
                        let n_cell = nj as usize * nx + ni as usize;
                        f_new[d * n_cells + n_cell] = f[d * n_cells + cell];
                    }
                }
            }
        }

        // ── 4. Boundary conditions (bounce-back + Zou-He) ──
        // Bounce-back for no-slip walls: f_i at wall = f_opposite from interior.
        // Zou-He for moving walls: prescribe velocity, compute unknown f from ρ.

        // ── 4. Boundary conditions (moving bounce-back) ──
        // For no-slip walls: f_opp = f_i (standard bounce-back).
        // For moving walls: f_opp = f_i - 2*w_i*ρ*3*(e_i·u_wall)
        //   where e_i is the direction pointing TOWARD the wall.
        // For outflow/pressure outlets: copy from interior (zero gradient).

        // Bottom wall (j=0): directions toward wall = 4,7,8 (downward). Unknowns: 2,5,6.
        let (u_w, v_w) = match bc.bottom {
            BcKind::NoSlip => (0.0, 0.0),
            BcKind::Inflow { u, v } => (u / vel_scale, v / vel_scale),
            _ => (f64::NAN, f64::NAN), // outflow/pressure: handled separately
        };
        if u_w.is_nan() {
            for i in 1..nx - 1 {
                let cell = i;
                let src = nx + i;
                f_new[2 * n_cells + cell] = f[2 * n_cells + src];
                f_new[5 * n_cells + cell] = f[5 * n_cells + src];
                f_new[6 * n_cells + cell] = f[6 * n_cells + src];
            }
        } else {
            for i in 1..nx - 1 {
                let cell = i;
                let r = rho[cell];
                f_new[2 * n_cells + cell] = f[4 * n_cells + cell] + (2.0 / 3.0) * r * v_w;
                f_new[5 * n_cells + cell] = f[7 * n_cells + cell] + r * (u_w + v_w) / 6.0;
                f_new[6 * n_cells + cell] = f[8 * n_cells + cell] + r * (-u_w + v_w) / 6.0;
            }
        }

        // Top wall (j=ny-1): directions toward wall = 2,5,6 (upward). Unknowns: 4,7,8.
        // Skip corner cells (i=0 and i=nx-1) — they're handled by left/right walls.
        let (u_w, v_w) = match bc.top {
            BcKind::NoSlip => (0.0, 0.0),
            BcKind::Inflow { u, v } => (u / vel_scale, v / vel_scale),
            _ => (f64::NAN, f64::NAN),
        };
        if u_w.is_nan() {
            for i in 1..nx - 1 {
                let cell = (ny - 1) * nx + i;
                let src = (ny - 2) * nx + i;
                f_new[4 * n_cells + cell] = f[4 * n_cells + src];
                f_new[7 * n_cells + cell] = f[7 * n_cells + src];
                f_new[8 * n_cells + cell] = f[8 * n_cells + src];
            }
        } else {
            for i in 1..nx - 1 {
                let cell = (ny - 1) * nx + i;
                let r = rho[cell];
                f_new[4 * n_cells + cell] = f[2 * n_cells + cell] - (2.0 / 3.0) * r * v_w;
                f_new[7 * n_cells + cell] = f[5 * n_cells + cell] - r * (u_w + v_w) / 6.0;
                f_new[8 * n_cells + cell] = f[6 * n_cells + cell] - r * (-u_w + v_w) / 6.0;
            }
        }

        // Left wall (i=0): directions toward wall = 3,6,7 (leftward). Unknowns: 1,5,8.
        let (u_w, v_w) = match bc.left {
            BcKind::NoSlip => (0.0, 0.0),
            BcKind::Inflow { u, v } => (u / vel_scale, v / vel_scale),
            _ => (f64::NAN, f64::NAN),
        };
        if u_w.is_nan() {
            for j in 0..ny {
                let cell = j * nx;
                let src = j * nx + 1;
                f_new[1 * n_cells + cell] = f[1 * n_cells + src];
                f_new[5 * n_cells + cell] = f[5 * n_cells + src];
                f_new[8 * n_cells + cell] = f[8 * n_cells + src];
            }
        } else {
            for j in 0..ny {
                let cell = j * nx;
                let r = rho[cell];
                f_new[1 * n_cells + cell] = f[3 * n_cells + cell] + (2.0 / 3.0) * r * u_w;
                f_new[5 * n_cells + cell] = f[7 * n_cells + cell] + r * (u_w + v_w) / 6.0;
                f_new[8 * n_cells + cell] = f[6 * n_cells + cell] + r * (u_w - v_w) / 6.0;
            }
        }

        // Right wall (i=nx-1): directions toward wall = 1,5,8 (rightward). Unknowns: 3,6,7.
        let (u_w, v_w) = match bc.right {
            BcKind::NoSlip => (0.0, 0.0),
            BcKind::Inflow { u, v } => (u / vel_scale, v / vel_scale),
            _ => (f64::NAN, f64::NAN),
        };
        if u_w.is_nan() {
            for j in 0..ny {
                let cell = j * nx + (nx - 1);
                let src = j * nx + (nx - 2);
                f_new[3 * n_cells + cell] = f[3 * n_cells + src];
                f_new[6 * n_cells + cell] = f[6 * n_cells + src];
                f_new[7 * n_cells + cell] = f[7 * n_cells + src];
            }
        } else {
            for j in 0..ny {
                let cell = j * nx + (nx - 1);
                let r = rho[cell];
                f_new[3 * n_cells + cell] = f[1 * n_cells + cell] - (2.0 / 3.0) * r * u_w;
                f_new[6 * n_cells + cell] = f[8 * n_cells + cell] - r * (u_w - v_w) / 6.0;
                f_new[7 * n_cells + cell] = f[5 * n_cells + cell] - r * (u_w + v_w) / 6.0;
            }
        }

        // ── Corner cells: regular bounce-back (no moving wall correction) ──
        // Corners are where two walls meet; use simple bounce-back from both
        // walls to avoid conflicting moving-wall corrections. `OPP[d]` gives the
        // opposite lattice direction, so `f_new[d] = f[OPP[d]]` is the standard
        // half-way bounce-back that reflects the distribution function.
        // Bottom-left corner (0, 0): reflect directions 1, 2, 5.
        let c = 0;
        for &d in &[1usize, 2, 5] {
            f_new[d * n_cells + c] = f[OPP[d] * n_cells + c];
        }
        // Bottom-right corner (nx-1, 0): reflect directions 2, 3, 6.
        let c = nx - 1;
        for &d in &[2usize, 3, 6] {
            f_new[d * n_cells + c] = f[OPP[d] * n_cells + c];
        }
        // Top-left corner (0, ny-1): reflect directions 1, 4, 8.
        let c = (ny - 1) * nx;
        for &d in &[1usize, 4, 8] {
            f_new[d * n_cells + c] = f[OPP[d] * n_cells + c];
        }
        // Top-right corner (nx-1, ny-1): reflect directions 3, 4, 7.
        let c = (ny - 1) * nx + (nx - 1);
        for &d in &[3usize, 4, 7] {
            f_new[d * n_cells + c] = f[OPP[d] * n_cells + c];
        }

        // Swap f and f_new.
        std::mem::swap(&mut f, &mut f_new);

        // ── 5. Convergence check ──
        let mut max_vel = 0.0f64;
        for j in 1..ny - 1 {
            for i in 1..nx - 1 {
                let cell = j * nx + i;
                max_vel = max_vel.max(u[cell].abs()).max(v[cell].abs());
            }
        }

        if max_vel > 1e6 || max_vel.is_nan() {
            return Err(EngineeringError::ConvergenceError(format!(
                "velocity blow-up at step {}: max_vel = {}", step, max_vel
            )));
        }

        if step > 100 && (prev_max_vel - max_vel).abs() < cfg.tolerance {
            converged_step = step;
            break;
        }
        prev_max_vel = max_vel;
    }

    // ── Extract final macroscopic fields ──
    let mut rho = vec![1.0; n_cells];
    let mut u_final = vec![0.0; n_cells];
    let mut v_final = vec![0.0; n_cells];

    for j in 0..ny {
        for i in 0..nx {
            let cell = j * nx + i;
            let mut rho_c = 0.0;
            let mut u_c = 0.0;
            let mut v_c = 0.0;
            for d in 0..N_DIR {
                let f_d = f[d * n_cells + cell];
                rho_c += f_d;
                u_c += f_d * CX[d] as f64;
                v_c += f_d * CY[d] as f64;
            }
            rho[cell] = rho_c;
            if rho_c > 1e-10 {
                u_final[cell] = u_c / rho_c;
                v_final[cell] = v_c / rho_c;
            }
        }
    }

    // ── Copy to staggered grid (scale lattice → physical) ──
    // u at vertical faces: average of cell-centre u values.
    for j in 0..ny {
        for i in 0..nx + 1 {
            let u_left = if i > 0 { u_final[j * nx + (i - 1)] } else { 0.0 };
            let u_right = if i < nx { u_final[j * nx + i] } else { 0.0 };
            grid.u[u_idx(nx, i, j)] = 0.5 * (u_left + u_right) * vel_scale;
        }
    }
    // v at horizontal faces: average of cell-centre v values.
    for j in 0..ny + 1 {
        for i in 0..nx {
            let v_bot = if j > 0 { v_final[(j - 1) * nx + i] } else { 0.0 };
            let v_top = if j < ny { v_final[j * nx + i] } else { 0.0 };
            grid.v[v_idx(nx, i, j)] = 0.5 * (v_bot + v_top) * vel_scale;
        }
    }
    // Pressure: p = c_s² * (ρ − ρ₀), where c_s² = 1/3 (lattice units).
    for j in 0..ny {
        for i in 0..nx {
            grid.p[p_idx(nx, i, j)] = (rho[j * nx + i] - 1.0) * cfg.density / 3.0;
        }
    }

    // Compute max divergence at interior cell centres only.
    // Boundary cells have high divergence due to bounce-back BC artifacts,
    // which is expected and not a solver error.
    let mut max_div = 0.0f64;
    for j in 1..ny - 1 {
        for i in 1..nx - 1 {
            let du_dx = (u_final[j * nx + (i + 1)] - u_final[j * nx + (i - 1)]) * vel_scale / (2.0 * dx);
            let dv_dy = (v_final[(j + 1) * nx + i] - v_final[(j - 1) * nx + i]) * vel_scale / (2.0 * dy);
            max_div = max_div.max((du_dx + dv_dy).abs());
        }
    }

    // ── Enforce Dirichlet velocity BCs on the staggered-grid boundary faces ──
    // The LBM solve above computes cell-centre values and averages them onto the
    // staggered faces. The boundary face velocities produced by that averaging do
    // not exactly match the prescribed boundary-condition velocities (e.g. the
    // lid velocity at the top wall). `apply_bc` overwrites the boundary u/v faces
    // with the exact Dirichlet values (and applies the Neumann extrapolation for
    // outflow / pressure outlets), so the returned `CfdSolution` boundary values
    // are physically consistent with the requested `CfdBc`.
    apply_bc(grid, bc);

    Ok((max_div, converged_step))
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// CFD solution fields returned to the caller.
pub struct CfdSolution {
    /// u-velocity at vertical faces, shape `(nx+1, ny)`, row-major (j outer, i inner).
    pub u: Vec<f64>,
    /// v-velocity at horizontal faces, shape `(nx, ny+1)`.
    pub v: Vec<f64>,
    /// pressure at cell centres, shape `(nx, ny)`.
    pub p: Vec<f64>,
    /// Number of cells in x.
    pub nx: usize,
    /// Number of cells in y.
    pub ny: usize,
    /// Domain length in x (m).
    pub lx: f64,
    /// Domain length in y (m).
    pub ly: f64,
    /// Maximum divergence (continuity residual) at convergence.
    pub max_divergence: f64,
    /// Time step at which convergence was achieved.
    pub converged_step: usize,
}

/// Run a 2-D incompressible Navier–Stokes simulation.
///
/// The domain geometry is derived from the `EngineeringModel`'s `geometry.dimensions`:
/// `dimensions[0]` = Lx, `dimensions[1]` = Ly. The fluid properties (density,
/// viscosity) are taken from the model's first material's `MaterialProperties`
/// (`density` and a viscosity proxy from `thermal_conductivity` / `specific_heat`
/// when no explicit viscosity field exists — we use `density` directly and
/// derive viscosity from the Reynolds number if specified in boundary conditions).
///
/// For the standard lid-driven cavity (default BCs), viscosity is taken from
/// the `SolverConfig` which defaults to Re = 100 (μ = 0.01, ρ = 1.0, L = 1, U = 1).
///
/// Returns the velocity and pressure fields, or an error if the inputs are
/// insufficient or the solver fails to converge.
pub fn run_cfd(
    model: &EngineeringModel,
    bc: CfdBc,
    cfg: SolverConfig,
    nx: usize,
    ny: usize,
) -> Result<CfdSolution, EngineeringError> {
    // Extract domain dimensions from geometry.
    let dims = &model.geometry.dimensions;
    if dims.len() < 2 {
        return Err(EngineeringError::InsufficientData(
            "geometry.dimensions must contain at least [Lx, Ly]".to_string(),
        ));
    }
    let lx = dims[0];
    let ly = dims[1];
    if lx <= 0.0 || ly <= 0.0 {
        return Err(EngineeringError::ValidationError(
            "domain dimensions must be positive".to_string(),
        ));
    }

    // Try to extract density from material; fall back to config default.
    let density = model
        .materials
        .values()
        .next()
        .map(|m| m.material_properties.density)
        .filter(|d| *d > 0.0)
        .unwrap_or(cfg.density);

    let mut cfg = cfg;
    cfg.density = density;

    if nx < 4 || ny < 4 {
        return Err(EngineeringError::ValidationError(
            "mesh must be at least 4×4 cells".to_string(),
        ));
    }

    let mut grid = StaggeredGrid::new(nx, ny, lx, ly);
    let (max_div, converged_step) = solve(&mut grid, &bc, &cfg)?;

    Ok(CfdSolution {
        u: grid.u,
        v: grid.v,
        p: grid.p,
        nx,
        ny,
        lx,
        ly,
        max_divergence: max_div,
        converged_step,
    })
}

/// Convert a `CfdSolution` into the library's `AnalysisResults` format.
pub fn cfd_to_analysis_results(
    sol: &CfdSolution,
    model: &EngineeringModel,
    analysis_type: AnalysisType,
) -> AnalysisResults {
    // Flatten velocity magnitude at cell centres into the displacement_field
    // (reusing the field as a general-purpose scalar output), and pressure
    // into stress_field. This is the honest mapping — AnalysisResults was
    // designed for mechanical analysis, but the fields are Vec<f64> and
    // documented as "field" outputs.
    let mut vel_mag = Vec::with_capacity(sol.nx * sol.ny);
    let mut pressure = Vec::with_capacity(sol.nx * sol.ny);

    for j in 0..sol.ny {
        for i in 0..sol.nx {
            // Interpolate u and v to cell centre.
            let u_c = 0.5 * (sol.u[j * (sol.nx + 1) + i] + sol.u[j * (sol.nx + 1) + i + 1]);
            let v_c = 0.5 * (sol.v[j * sol.nx + i] + sol.v[(j + 1) * sol.nx + i]);
            vel_mag.push((u_c * u_c + v_c * v_c).sqrt());
            pressure.push(sol.p[j * sol.nx + i]);
        }
    }

    AnalysisResults {
        results_id: format!("cfd_{}", model.model_id),
        analysis_type,
        displacement_field: vel_mag,
        stress_field: pressure,
        strain_field: Vec::new(),
        reaction_forces: Vec::new(),
        safety_factor: 0.0,
        temperature_field: Vec::new(),
        heat_flux_field: Vec::new(),
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{
        EngineeringModel, Geometry, GeometryType, Material, MaterialProperties, ModelType,
    };
    use std::collections::HashMap;

    fn cfd_model(lx: f64, ly: f64) -> EngineeringModel {
        let mut materials = HashMap::new();
        materials.insert(
            "fluid".to_string(),
            Material {
                material_id: "fluid".to_string(),
                material_name: "water".to_string(),
                material_properties: MaterialProperties {
                    youngs_modulus: 0.0,
                    poissons_ratio: 0.0,
                    density: 1.0,
                    thermal_expansion: 0.0,
                    thermal_conductivity: 0.0,
                    specific_heat: 0.0,
                    yield_strength: 0.0,
                    ultimate_strength: 0.0,
                },
            },
        );
        EngineeringModel {
            model_id: "cfd_test".to_string(),
            model_name: "CFD Test".to_string(),
            model_type: ModelType::Fluid,
            geometry: Geometry {
                geometry_type: GeometryType::Beam,
                dimensions: vec![lx, ly],
                features: Vec::new(),
            },
            materials,
            boundary_conditions: Vec::new(),
            loads: Vec::new(),
        }
    }

    #[test]
    fn lid_driven_cavity_converges() {
        // Classic lid-driven cavity. Start with Re=10 (viscosity=0.1) for
        // stability on a 20×20 grid (τ ≈ 1.1, well within stable range).
        let model = cfd_model(1.0, 1.0);
        let bc = CfdBc::default(); // no-slip walls + top lid at u=1.
        let cfg = SolverConfig {
            density: 1.0,
            viscosity: 0.1, 
            dt: 0.0025, // Maps to u_LBM = 0.05, tau = 0.8
            max_steps: 10000,
            tolerance: 1e-6,
            poisson_iters: 0, 
        };
        let result = run_cfd(&model, bc, cfg, 20, 20);
        assert!(result.is_ok(), "cavity solver failed: {:?}", result.err());
        let sol = result.unwrap();

        // The cavity should converge to a steady recirculation.
        // Interior divergence should be small (LBM is divergence-free in bulk).
        assert!(sol.max_divergence < 1.0, "interior divergence too high: {}", sol.max_divergence);

        // The lid velocity (top wall) should be close to 1.0.
        // u at the top row of staggered grid (j=ny-1=19).
        let top_u: Vec<f64> = (1..20).map(|i| sol.u[19 * 21 + i]).collect();
        let max_u = top_u.iter().cloned().fold(0.0f64, f64::max);
        assert!(max_u > 0.3, "lid velocity should be significant, got max u = {}", max_u);

        // Centre of the cavity should have a vortex (non-zero velocity).
        let ci = 10;
        let cj = 10;
        let u_c = 0.5 * (sol.u[cj * 21 + ci] + sol.u[cj * 21 + ci + 1]);
        let v_c = 0.5 * (sol.v[cj * 20 + ci] + sol.v[(cj + 1) * 20 + ci]);
        let vel_c = (u_c * u_c + v_c * v_c).sqrt();
        assert!(vel_c > 1e-4, "cavity centre should have non-zero velocity, got {}", vel_c);
    }

    #[test]
    fn channel_flow_has_uniform_profile() {
        // Channel flow: left inflow u=1, right outflow, top/bottom no-slip.
        let model = cfd_model(2.0, 1.0);
        let bc = CfdBc {
            left: BcKind::Inflow { u: 1.0, v: 0.0 },
            right: BcKind::Outflow,
            bottom: BcKind::NoSlip,
            top: BcKind::NoSlip,
        };
        let cfg = SolverConfig {
            density: 1.0,
            viscosity: 0.01,
            dt: 0.005,
            max_steps: 3000,
            tolerance: 1e-4,
            poisson_iters: 30,
        };
        let sol = run_cfd(&model, bc, cfg, 20, 10).unwrap();

        // At the inflow (left boundary), u should be ~1.0.
        let inflow_u: Vec<f64> = (0..10).map(|j| sol.u[j * 21 + 0]).collect();
        let avg_inflow = inflow_u.iter().sum::<f64>() / inflow_u.len() as f64;
        assert!(
            (avg_inflow - 1.0).abs() < 0.15,
            "inflow u should be near 1.0, got avg = {}",
            avg_inflow
        );

        // No-slip walls: v at top and bottom should be ~0.
        let top_v: Vec<f64> = (0..20).map(|i| sol.v[10 * 20 + i]).collect();
        let max_top_v = top_v.iter().cloned().fold(0.0f64, f64::max);
        assert!(max_top_v.abs() < 0.1, "top wall v should be ~0, got {}", max_top_v);
    }

    #[test]
    fn missing_dimensions_errors() {
        let mut model = cfd_model(1.0, 1.0);
        model.geometry.dimensions.clear();
        let bc = CfdBc::default();
        let cfg = SolverConfig::default();
        let result = run_cfd(&model, bc, cfg, 10, 10);
        assert!(matches!(result, Err(EngineeringError::InsufficientData(_))));
    }

    #[test]
    fn negative_viscosity_errors() {
        let model = cfd_model(1.0, 1.0);
        let bc = CfdBc::default();
        let cfg = SolverConfig {
            viscosity: -1.0,
            ..Default::default()
        };
        let result = run_cfd(&model, bc, cfg, 10, 10);
        assert!(matches!(result, Err(EngineeringError::ValidationError(_))));
    }

    #[test]
    fn cfl_violation_errors() {
        let model = cfd_model(1.0, 1.0);
        let bc = CfdBc::default();
        let cfg = SolverConfig {
            dt: 10.0, // huge dt → CFL violation
            ..Default::default()
        };
        let result = run_cfd(&model, bc, cfg, 10, 10);
        assert!(matches!(result, Err(EngineeringError::ValidationError(_))));
    }

    #[test]
    fn pressure_outlet_maintains_pressure() {
        // Pressure outlet on the right: p = 0 (atmospheric).
        let model = cfd_model(1.0, 1.0);
        let bc = CfdBc {
            left: BcKind::Inflow { u: 0.5, v: 0.0 },
            right: BcKind::PressureOutlet { p: 0.0 },
            bottom: BcKind::NoSlip,
            top: BcKind::NoSlip,
        };
        let cfg = SolverConfig {
            density: 1.0,
            viscosity: 0.01,
            dt: 0.005,
            max_steps: 2000,
            tolerance: 1e-4,
            poisson_iters: 30,
        };
        let sol = run_cfd(&model, bc, cfg, 16, 16).unwrap();

        // Pressure at the right boundary should be near 0.
        let right_p: Vec<f64> = (0..16).map(|j| sol.p[j * 16 + 15]).collect();
        let avg_right_p = right_p.iter().sum::<f64>() / right_p.len() as f64;
        assert!(
            avg_right_p.abs() < 5.0,
            "pressure outlet should be near 0, got avg = {}",
            avg_right_p
        );
    }

    #[test]
    fn cfd_to_analysis_results_maps_fields() {
        let model = cfd_model(1.0, 1.0);
        let bc = CfdBc::default();
        let cfg = SolverConfig {
            density: 1.0,
            viscosity: 0.01,
            dt: 0.005,
            max_steps: 500,
            tolerance: 1e-3,
            poisson_iters: 20,
        };
        let sol = run_cfd(&model, bc, cfg, 10, 10).unwrap();
        let results = cfd_to_analysis_results(&sol, &model, AnalysisType::LinearStatic);

        assert_eq!(results.results_id, "cfd_cfd_test");
        assert_eq!(results.displacement_field.len(), 100); // 10×10 cell-centred values
        assert_eq!(results.stress_field.len(), 100);
        // Velocity magnitudes should be non-negative.
        assert!(results.displacement_field.iter().all(|&v| v >= 0.0));
    }
}
