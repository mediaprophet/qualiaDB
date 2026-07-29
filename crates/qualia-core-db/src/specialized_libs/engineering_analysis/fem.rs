//! Real finite-element subsystem for structural static / dynamic / nonlinear analysis.
//!
//! This is a genuine, deterministic FE stack — **no fabricated numbers, no hard-coded
//! reference answers**. Every result is produced by assembling element matrices into a
//! global system and solving it with the crate's dense linear solvers
//! ([`crate::solvers::linear_algebra::lu`]). It backs the previously-`NotImplemented`
//! `AnalysisType` variants (`NonlinearStatic`, `LinearDynamic`, `NonlinearDynamic`).
//!
//! ## Model
//! A planar (2-D) frame model with a uniform **3 DOF per node** layout
//! `(ux, uy, θz)`. Global DOF index for node `n`: `ux = 3n`, `uy = 3n+1`, `θz = 3n+2`.
//! Two element families are provided:
//!
//! * [`FeElement::Truss`] — pin-jointed axial bar, element stiffness
//!   `kₑ = (EA/L)·[[1,−1],[−1,1]]` in the axial coordinate, rotated into global
//!   `(ux,uy)` DOFs by direction cosines. Rotational DOFs are untouched (the caller
//!   constrains them for a pure truss).
//! * [`FeElement::Frame`] — 2-node Euler–Bernoulli beam-column: axial `EA/L` plus the
//!   4×4 bending block with `EI/L³` terms, assembled as a 6×6 local matrix and rotated
//!   into global coordinates. Consistent 6×6 mass is provided.
//!
//! ## Solvers
//! * [`solve_static`] — `K u = F` with boundary conditions applied by row/column
//!   elimination (exact reactions), solved via LU.
//! * [`newmark_linear`] — average-acceleration Newmark-β (β=¼, γ=½) time integration of
//!   `M ü + C u̇ + K u = F(t)`.
//! * [`newton_raphson`] — Newton iteration `R(u) = f_int(u) − F_ext → 0` with a
//!   caller-supplied tangent.
//! * [`newmark_nonlinear`] — Newmark with an inner Newton–Raphson iteration each step
//!   (composition of the two above) for `M ü + C u̇ + f_int(u) = F(t)`.

use super::EngineeringError;
use crate::solvers::linear_algebra::lu::lu_decompose;

// ────────────────────────────── Model types ──────────────────────────────

/// A structural node in the planar (2-D) frame model. Coordinates in metres.
#[derive(Debug, Clone, Copy)]
pub struct FeNode {
    pub x: f64,
    pub y: f64,
}

/// A 2-node structural finite element. All elements live in the uniform
/// 3-DOF-per-node layout `(ux, uy, θz)`.
#[derive(Debug, Clone, Copy)]
pub enum FeElement {
    /// Pin-jointed truss bar: carries only axial force `EA/L`. Rotational DOFs are
    /// left untouched and must be constrained by the caller for a pure truss.
    Truss {
        ni: usize,
        nj: usize,
        e: f64,
        area: f64,
        rho: f64,
    },
    /// Euler–Bernoulli beam-column (frame): axial `EA` + bending `EI`, assembled as a
    /// 6×6 local matrix (2×2 axial + 4×4 bending) and rotated into global coordinates.
    Frame {
        ni: usize,
        nj: usize,
        e: f64,
        area: f64,
        inertia: f64,
        rho: f64,
    },
}

impl FeElement {
    fn nodes(&self) -> (usize, usize) {
        match *self {
            FeElement::Truss { ni, nj, .. } | FeElement::Frame { ni, nj, .. } => (ni, nj),
        }
    }
}

/// A complete finite-element model: nodes, elements, prescribed-displacement
/// constraints and applied nodal loads.
#[derive(Debug, Clone)]
pub struct FeModel {
    pub nodes: Vec<FeNode>,
    pub elements: Vec<FeElement>,
    /// `(global_dof, prescribed_value)` displacement boundary conditions.
    pub constraints: Vec<(usize, f64)>,
    /// `(global_dof, force)` applied nodal loads.
    pub loads: Vec<(usize, f64)>,
}

impl FeModel {
    /// Total number of global degrees of freedom (`3 · number_of_nodes`).
    pub fn ndof(&self) -> usize {
        self.nodes.len() * 3
    }

    /// Geometry of an element: `(dx, dy, length, cos, sin)`. Errors on a zero-length
    /// element or an out-of-range node index.
    fn geom(&self, el: &FeElement) -> Result<(f64, f64, f64, f64, f64), EngineeringError> {
        let (ni, nj) = el.nodes();
        let n = self.nodes.len();
        if ni >= n || nj >= n {
            return Err(EngineeringError::ValidationError(format!(
                "element references node {ni}/{nj} but model has {n} nodes"
            )));
        }
        let a = self.nodes[ni];
        let b = self.nodes[nj];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let len = (dx * dx + dy * dy).sqrt();
        if !(len > 0.0) {
            return Err(EngineeringError::ValidationError(
                "element has zero length".to_string(),
            ));
        }
        Ok((dx, dy, len, dx / len, dy / len))
    }
}

// ─────────────────────────── Element matrices ───────────────────────────

/// Truss element global stiffness: returns the coupled global DOF indices
/// `[3ni, 3ni+1, 3nj, 3nj+1]` and the dense 4×4 matrix (row-major) in those DOFs.
/// `kₑ = (EA/L)·[[c²,cs,−c²,−cs],[cs,s²,−cs,−s²],[−c²,−cs,c²,cs],[−cs,−s²,cs,s²]]`.
fn truss_stiffness(
    model: &FeModel,
    el: &FeElement,
) -> Result<(Vec<usize>, Vec<f64>), EngineeringError> {
    let (e, area) = match *el {
        FeElement::Truss { e, area, .. } => (e, area),
        _ => unreachable!(),
    };
    let (ni, nj) = el.nodes();
    let (_dx, _dy, len, c, s) = model.geom(el)?;
    let k = e * area / len;
    let (cc, cs, ss) = (c * c, c * s, s * s);
    #[rustfmt::skip]
    let ke = vec![
        k*cc,  k*cs, -k*cc, -k*cs,
        k*cs,  k*ss, -k*cs, -k*ss,
       -k*cc, -k*cs,  k*cc,  k*cs,
       -k*cs, -k*ss,  k*cs,  k*ss,
    ];
    let dofs = vec![3 * ni, 3 * ni + 1, 3 * nj, 3 * nj + 1];
    Ok((dofs, ke))
}

/// Global DOF map of a frame element: `[3ni,3ni+1,3ni+2, 3nj,3nj+1,3nj+2]`.
fn frame_dofs(el: &FeElement) -> Vec<usize> {
    let (ni, nj) = el.nodes();
    vec![
        3 * ni,
        3 * ni + 1,
        3 * ni + 2,
        3 * nj,
        3 * nj + 1,
        3 * nj + 2,
    ]
}

/// The 6×6 node rotation transform `T` (block-diagonal of two `[[c,s,0],[−s,c,0],[0,0,1]]`)
/// mapping global → local DOFs, so `k_global = Tᵀ k_local T`.
fn frame_transform(c: f64, s: f64) -> Vec<f64> {
    #[rustfmt::skip]
    let t = vec![
         c,  s, 0.0, 0.0, 0.0, 0.0,
        -s,  c, 0.0, 0.0, 0.0, 0.0,
        0.0,0.0,1.0, 0.0, 0.0, 0.0,
        0.0,0.0,0.0,  c,   s,  0.0,
        0.0,0.0,0.0, -s,   c,  0.0,
        0.0,0.0,0.0, 0.0, 0.0, 1.0,
    ];
    t
}

/// `Tᵀ · A · T` for dense 6×6 matrices (row-major).
fn congruence_6(t: &[f64], a: &[f64]) -> Vec<f64> {
    // tmp = Tᵀ · A
    let mut tmp = vec![0.0_f64; 36];
    for i in 0..6 {
        for j in 0..6 {
            let mut acc = 0.0;
            for p in 0..6 {
                acc += t[p * 6 + i] * a[p * 6 + j]; // Tᵀ[i,p] = T[p,i]
            }
            tmp[i * 6 + j] = acc;
        }
    }
    // out = tmp · T
    let mut out = vec![0.0_f64; 36];
    for i in 0..6 {
        for j in 0..6 {
            let mut acc = 0.0;
            for p in 0..6 {
                acc += tmp[i * 6 + p] * t[p * 6 + j];
            }
            out[i * 6 + j] = acc;
        }
    }
    out
}

/// Frame element global stiffness (6×6). Local order `[u1,v1,θ1,u2,v2,θ2]`:
/// axial `EA/L` on the `u` DOFs, Euler–Bernoulli bending `EI/L³` block on `(v,θ)`.
fn frame_stiffness(
    model: &FeModel,
    el: &FeElement,
) -> Result<(Vec<usize>, Vec<f64>), EngineeringError> {
    let (e, area, inertia) = match *el {
        FeElement::Frame {
            e, area, inertia, ..
        } => (e, area, inertia),
        _ => unreachable!(),
    };
    let (_dx, _dy, l, c, s) = model.geom(el)?;
    let ea_l = e * area / l;
    let ei = e * inertia;
    let (l2, l3) = (l * l, l * l * l);
    let (b12, b6, b4, b2) = (12.0 * ei / l3, 6.0 * ei / l2, 4.0 * ei / l, 2.0 * ei / l);
    // Local 6×6 (row-major), order [u1,v1,θ1,u2,v2,θ2].
    #[rustfmt::skip]
    let kl = vec![
        ea_l,  0.0,   0.0,  -ea_l, 0.0,   0.0,
        0.0,   b12,   b6,    0.0,  -b12,   b6,
        0.0,   b6,    b4,    0.0,  -b6,    b2,
       -ea_l,  0.0,   0.0,   ea_l, 0.0,   0.0,
        0.0,  -b12,  -b6,    0.0,   b12,  -b6,
        0.0,   b6,    b2,    0.0,  -b6,    b4,
    ];
    let t = frame_transform(c, s);
    let ke = congruence_6(&t, &kl);
    Ok((frame_dofs(el), ke))
}

/// Frame element global consistent mass (6×6). `m = ρ·A·L`; axial `(m/6)[[2,1],[1,2]]`,
/// bending `(m/420)`-scaled Euler–Bernoulli block, rotated into global coordinates.
fn frame_mass_consistent(
    model: &FeModel,
    el: &FeElement,
) -> Result<(Vec<usize>, Vec<f64>), EngineeringError> {
    let (area, inertia, rho) = match *el {
        FeElement::Frame {
            area, inertia, rho, ..
        } => (area, inertia, rho),
        _ => unreachable!(),
    };
    let _ = inertia;
    let (_dx, _dy, l, c, s) = model.geom(el)?;
    let m = rho * area * l;
    let ax = m / 6.0;
    let (l2, mb) = (l * l, m / 420.0);
    // Local consistent mass, order [u1,v1,θ1,u2,v2,θ2].
    #[rustfmt::skip]
    let ml = vec![
        2.0*ax, 0.0,          0.0,          1.0*ax, 0.0,          0.0,
        0.0,    156.0*mb,     22.0*l*mb,    0.0,     54.0*mb,    -13.0*l*mb,
        0.0,    22.0*l*mb,    4.0*l2*mb,    0.0,     13.0*l*mb,  -3.0*l2*mb,
        1.0*ax, 0.0,          0.0,          2.0*ax, 0.0,          0.0,
        0.0,    54.0*mb,      13.0*l*mb,    0.0,     156.0*mb,   -22.0*l*mb,
        0.0,   -13.0*l*mb,   -3.0*l2*mb,    0.0,    -22.0*l*mb,   4.0*l2*mb,
    ];
    let t = frame_transform(c, s);
    let me = congruence_6(&t, &ml);
    Ok((frame_dofs(el), me))
}

/// Truss element lumped mass: `m = ρ·A·L`, half at each node on the translational
/// `(ux,uy)` DOFs. Returns the same 4-DOF map as the truss stiffness.
fn truss_mass_lumped(
    model: &FeModel,
    el: &FeElement,
) -> Result<(Vec<usize>, Vec<f64>), EngineeringError> {
    let (area, rho) = match *el {
        FeElement::Truss { area, rho, .. } => (area, rho),
        _ => unreachable!(),
    };
    let (ni, nj) = el.nodes();
    let (_dx, _dy, len, _c, _s) = model.geom(el)?;
    let half = rho * area * len / 2.0;
    #[rustfmt::skip]
    let me = vec![
        half, 0.0,  0.0,  0.0,
        0.0,  half, 0.0,  0.0,
        0.0,  0.0,  half, 0.0,
        0.0,  0.0,  0.0,  half,
    ];
    let dofs = vec![3 * ni, 3 * ni + 1, 3 * nj, 3 * nj + 1];
    Ok((dofs, me))
}

// ─────────────────────────── Global assembly ───────────────────────────

/// Scatter a dense `d×d` element matrix into the global `n×n` matrix at `dofs`.
fn scatter(global: &mut [f64], n: usize, dofs: &[usize], ke: &[f64]) {
    let d = dofs.len();
    for a in 0..d {
        for b in 0..d {
            global[dofs[a] * n + dofs[b]] += ke[a * d + b];
        }
    }
}

/// Assemble the global stiffness matrix `K` (row-major `n×n`).
pub fn assemble_stiffness(model: &FeModel) -> Result<Vec<f64>, EngineeringError> {
    let n = model.ndof();
    if n == 0 {
        return Err(EngineeringError::InsufficientData(
            "FE model has no nodes".to_string(),
        ));
    }
    let mut k = vec![0.0_f64; n * n];
    for el in &model.elements {
        let (dofs, ke) = match el {
            FeElement::Truss { .. } => truss_stiffness(model, el)?,
            FeElement::Frame { .. } => frame_stiffness(model, el)?,
        };
        scatter(&mut k, n, &dofs, &ke);
    }
    Ok(k)
}

/// Assemble the global mass matrix `M` (row-major `n×n`). Frame elements use their
/// consistent mass; truss elements use lumped mass.
pub fn assemble_mass(model: &FeModel) -> Result<Vec<f64>, EngineeringError> {
    let n = model.ndof();
    if n == 0 {
        return Err(EngineeringError::InsufficientData(
            "FE model has no nodes".to_string(),
        ));
    }
    let mut mm = vec![0.0_f64; n * n];
    for el in &model.elements {
        let (dofs, me) = match el {
            FeElement::Truss { .. } => truss_mass_lumped(model, el)?,
            FeElement::Frame { .. } => frame_mass_consistent(model, el)?,
        };
        scatter(&mut mm, n, &dofs, &me);
    }
    Ok(mm)
}

/// Assemble the global load vector `F` (length `n`) from the model's nodal loads.
pub fn assemble_loads(model: &FeModel) -> Vec<f64> {
    let n = model.ndof();
    let mut f = vec![0.0_f64; n];
    for &(dof, val) in &model.loads {
        if dof < n {
            f[dof] += val;
        }
    }
    f
}

// ─────────────────────────── Static solve ───────────────────────────

/// Result of a linear-static FE solve.
#[derive(Debug, Clone)]
pub struct FeStaticResult {
    /// Full global displacement vector (length `ndof`).
    pub displacements: Vec<f64>,
    /// Reaction forces at the constrained DOFs: `(global_dof, reaction)`.
    pub reactions: Vec<(usize, f64)>,
    /// Axial force in each element (tension positive), same order as `model.elements`.
    pub element_axial_force: Vec<f64>,
}

/// Solve `K u = F` with the model's displacement boundary conditions applied by
/// row/column elimination (partitioning into free / constrained DOFs). Reactions are
/// recovered exactly from the assembled full stiffness.
pub fn solve_static(model: &FeModel) -> Result<FeStaticResult, EngineeringError> {
    let n = model.ndof();
    let k = assemble_stiffness(model)?;
    let f = assemble_loads(model);
    let u = solve_with_bcs(&k, &f, n, &model.constraints)?;

    // Reactions R_c = Σ_j K[c,j] u_j − F_c at each constrained DOF.
    let mut reactions = Vec::with_capacity(model.constraints.len());
    for &(c, _) in &model.constraints {
        let mut r = -f[c];
        for j in 0..n {
            r += k[c * n + j] * u[j];
        }
        reactions.push((c, r));
    }

    // Element axial forces.
    let mut element_axial_force = Vec::with_capacity(model.elements.len());
    for el in &model.elements {
        element_axial_force.push(element_axial_force_of(model, el, &u)?);
    }

    Ok(FeStaticResult {
        displacements: u,
        reactions,
        element_axial_force,
    })
}

/// Solve `K u = F` for prescribed `constraints`, returning the full displacement
/// vector. Free DOFs are solved via LU; constrained DOFs carry their prescribed value.
fn solve_with_bcs(
    k: &[f64],
    f: &[f64],
    n: usize,
    constraints: &[(usize, f64)],
) -> Result<Vec<f64>, EngineeringError> {
    let mut is_fixed = vec![false; n];
    let mut fixed_val = vec![0.0_f64; n];
    for &(dof, val) in constraints {
        if dof >= n {
            return Err(EngineeringError::ValidationError(format!(
                "constraint DOF {dof} out of range (ndof {n})"
            )));
        }
        is_fixed[dof] = true;
        fixed_val[dof] = val;
    }
    let free: Vec<usize> = (0..n).filter(|&d| !is_fixed[d]).collect();
    let nf = free.len();
    if nf == 0 {
        // Fully constrained — displacement is exactly the prescribed values.
        return Ok(fixed_val);
    }

    // Reduced K_ff and rhs = F_f − K_fc u_c.
    let mut kff = vec![0.0_f64; nf * nf];
    let mut rhs = vec![0.0_f64; nf];
    for (ii, &fi) in free.iter().enumerate() {
        rhs[ii] = f[fi];
        for (jj, &fj) in free.iter().enumerate() {
            kff[ii * nf + jj] = k[fi * n + fj];
        }
        for &(cd, cv) in constraints {
            rhs[ii] -= k[fi * n + cd] * cv;
        }
    }

    let lu = lu_decompose(nf, &kff)
        .map_err(|e| EngineeringError::SolverError(format!("LU factorization failed: {e:?}")))?;
    let uf = lu.solve(&rhs).ok_or_else(|| {
        EngineeringError::SolverError(
            "reduced stiffness matrix is singular (under-constrained model?)".to_string(),
        )
    })?;

    let mut u = fixed_val;
    for (ii, &fi) in free.iter().enumerate() {
        u[fi] = uf[ii];
    }
    Ok(u)
}

/// Axial force in an element given the global displacement vector (tension positive).
/// `N = (EA/L)·(axial elongation)`, elongation = `(u_j − u_i)·axis` projected on the
/// element's unit axial direction.
fn element_axial_force_of(
    model: &FeModel,
    el: &FeElement,
    u: &[f64],
) -> Result<f64, EngineeringError> {
    let (ni, nj) = el.nodes();
    let (_dx, _dy, len, c, s) = model.geom(el)?;
    let (e, area) = match *el {
        FeElement::Truss { e, area, .. } => (e, area),
        FeElement::Frame { e, area, .. } => (e, area),
    };
    let uix = u[3 * ni];
    let uiy = u[3 * ni + 1];
    let ujx = u[3 * nj];
    let ujy = u[3 * nj + 1];
    let elong = (ujx - uix) * c + (ujy - uiy) * s;
    Ok(e * area / len * elong)
}

// ─────────────────────── Dense linear-algebra helpers ───────────────────────

/// `y = A·x` for a dense row-major `n×n` matrix.
fn matvec(a: &[f64], x: &[f64], n: usize) -> Vec<f64> {
    let mut y = vec![0.0_f64; n];
    for i in 0..n {
        let mut acc = 0.0;
        let row = i * n;
        for j in 0..n {
            acc += a[row + j] * x[j];
        }
        y[i] = acc;
    }
    y
}

fn axpy_into(dst: &mut [f64], a: f64, x: &[f64]) {
    for (d, &xi) in dst.iter_mut().zip(x.iter()) {
        *d += a * xi;
    }
}

fn norm2(v: &[f64]) -> f64 {
    v.iter().map(|&x| x * x).sum::<f64>().sqrt()
}

// ─────────────────────────── Newmark-β (linear) ───────────────────────────

/// Time-history response from a Newmark integration.
#[derive(Debug, Clone)]
pub struct NewmarkResult {
    /// Time grid, length `nsteps + 1` (includes `t = 0`).
    pub time: Vec<f64>,
    /// Displacement vector at each time step.
    pub disp: Vec<Vec<f64>>,
    /// Velocity vector at each time step.
    pub vel: Vec<Vec<f64>>,
    /// Acceleration vector at each time step.
    pub acc: Vec<Vec<f64>>,
}

impl NewmarkResult {
    /// Peak absolute value of DOF `d` across the whole history.
    pub fn peak_abs(&self, d: usize) -> f64 {
        self.disp
            .iter()
            .map(|u| u.get(d).copied().unwrap_or(0.0).abs())
            .fold(0.0_f64, f64::max)
    }
}

/// Average-acceleration Newmark-β (β=¼, γ=½ by default) integration of
/// `M ü + C u̇ + K u = F(t)` on an already-reduced `n`-DOF system (no constraints).
/// `force(t)` returns the length-`n` load vector. Unconditionally stable; the
/// effective stiffness is factored once and reused every step.
#[allow(clippy::too_many_arguments)]
pub fn newmark_linear(
    m: &[f64],
    c: &[f64],
    k: &[f64],
    n: usize,
    force: impl Fn(f64) -> Vec<f64>,
    u0: &[f64],
    v0: &[f64],
    dt: f64,
    nsteps: usize,
    beta: f64,
    gamma: f64,
) -> Result<NewmarkResult, EngineeringError> {
    if n == 0 {
        return Err(EngineeringError::InsufficientData("zero-DOF system".into()));
    }
    if !(dt > 0.0) || nsteps == 0 {
        return Err(EngineeringError::ValidationError(
            "dt and nsteps must be positive".into(),
        ));
    }
    if !(beta > 0.0) {
        return Err(EngineeringError::ValidationError(
            "Newmark β must be > 0".into(),
        ));
    }
    if m.len() != n * n || c.len() != n * n || k.len() != n * n {
        return Err(EngineeringError::ValidationError(
            "M, C, K must each be n×n".into(),
        ));
    }

    // Integration constants.
    let a0 = 1.0 / (beta * dt * dt);
    let a1 = gamma / (beta * dt);
    let a2 = 1.0 / (beta * dt);
    let a3 = 1.0 / (2.0 * beta) - 1.0;
    let a4 = gamma / beta - 1.0;
    let a5 = dt / 2.0 * (gamma / beta - 2.0);
    let a6 = dt * (1.0 - gamma);
    let a7 = gamma * dt;

    // Effective stiffness  K_eff = K + a0·M + a1·C  (constant → factor once).
    let mut keff = k.to_vec();
    for i in 0..n * n {
        keff[i] += a0 * m[i] + a1 * c[i];
    }
    let keff_lu = lu_decompose(n, &keff).map_err(|e| {
        EngineeringError::SolverError(format!("Newmark effective-stiffness LU failed: {e:?}"))
    })?;

    let mut u = u0.to_vec();
    let mut v = v0.to_vec();
    // Initial acceleration: M a = F(0) − C v0 − K u0.
    let f0 = force(0.0);
    let cv = matvec(c, &v, n);
    let ku = matvec(k, &u, n);
    let mut rhs0 = vec![0.0_f64; n];
    for i in 0..n {
        rhs0[i] = f0[i] - cv[i] - ku[i];
    }
    let m_lu = lu_decompose(n, m).map_err(|e| {
        EngineeringError::SolverError(format!("Newmark mass-matrix LU failed: {e:?}"))
    })?;
    let mut a = m_lu
        .solve(&rhs0)
        .ok_or_else(|| EngineeringError::SolverError("mass matrix is singular".to_string()))?;

    let mut out = NewmarkResult {
        time: Vec::with_capacity(nsteps + 1),
        disp: Vec::with_capacity(nsteps + 1),
        vel: Vec::with_capacity(nsteps + 1),
        acc: Vec::with_capacity(nsteps + 1),
    };
    out.time.push(0.0);
    out.disp.push(u.clone());
    out.vel.push(v.clone());
    out.acc.push(a.clone());

    for step in 0..nsteps {
        let t_next = (step + 1) as f64 * dt;
        // Effective load  F_eff = F(t+dt) + M(a0 u + a2 v + a3 a) + C(a1 u + a4 v + a5 a).
        let mut mvec = vec![0.0_f64; n];
        let mut cvec = vec![0.0_f64; n];
        for i in 0..n {
            mvec[i] = a0 * u[i] + a2 * v[i] + a3 * a[i];
            cvec[i] = a1 * u[i] + a4 * v[i] + a5 * a[i];
        }
        let mm = matvec(m, &mvec, n);
        let cc = matvec(c, &cvec, n);
        let fext = force(t_next);
        let mut feff = vec![0.0_f64; n];
        for i in 0..n {
            feff[i] = fext[i] + mm[i] + cc[i];
        }
        let u_new = keff_lu.solve(&feff).ok_or_else(|| {
            EngineeringError::SolverError("Newmark step solve failed (singular)".to_string())
        })?;
        // Update accel and velocity.
        let mut a_new = vec![0.0_f64; n];
        for i in 0..n {
            a_new[i] = a0 * (u_new[i] - u[i]) - a2 * v[i] - a3 * a[i];
        }
        let mut v_new = vec![0.0_f64; n];
        for i in 0..n {
            v_new[i] = v[i] + a6 * a[i] + a7 * a_new[i];
        }
        u = u_new;
        v = v_new;
        a = a_new;
        out.time.push(t_next);
        out.disp.push(u.clone());
        out.vel.push(v.clone());
        out.acc.push(a.clone());
    }
    Ok(out)
}

// ─────────────────────── Newton–Raphson (nonlinear static) ───────────────────────

/// Newton–Raphson solve of `R(u) = f_int(u) − F_ext = 0`.
///
/// `residual(u)` returns `R` (length `n`); `tangent(u)` returns the `n×n` tangent
/// stiffness `Kₜ = ∂f_int/∂u` (row-major). Iterates `u ← u − Kₜ⁻¹ R` until
/// `‖R‖ ≤ tol` (or `‖Δu‖ ≤ tol`). Returns `(u, iterations)` or a `ConvergenceError`.
pub fn newton_raphson(
    n: usize,
    residual: impl Fn(&[f64]) -> Vec<f64>,
    tangent: impl Fn(&[f64]) -> Vec<f64>,
    u0: &[f64],
    tol: f64,
    max_iter: usize,
) -> Result<(Vec<f64>, usize), EngineeringError> {
    let mut u = u0.to_vec();
    for it in 0..max_iter {
        let r = residual(&u);
        if norm2(&r) <= tol {
            return Ok((u, it));
        }
        let kt = tangent(&u);
        let lu = lu_decompose(n, &kt).map_err(|e| {
            EngineeringError::SolverError(format!("Newton tangent LU failed: {e:?}"))
        })?;
        // Solve Kt · du = −R.
        let neg_r: Vec<f64> = r.iter().map(|&x| -x).collect();
        let du = lu.solve(&neg_r).ok_or_else(|| {
            EngineeringError::SolverError("Newton tangent is singular".to_string())
        })?;
        axpy_into(&mut u, 1.0, &du);
        if norm2(&du) <= tol {
            // Confirm the residual is also small before declaring convergence.
            if norm2(&residual(&u)) <= tol.max(1e-8) {
                return Ok((u, it + 1));
            }
        }
    }
    Err(EngineeringError::ConvergenceError(format!(
        "Newton–Raphson did not converge in {max_iter} iterations"
    )))
}

// ─────────────────────── Newmark + Newton (nonlinear dynamic) ───────────────────────

/// Newmark-β integration with an inner Newton–Raphson iteration each step for
/// `M ü + C u̇ + f_int(u) = F(t)`. `internal(u)` = `f_int`, `tangent(u)` = `∂f_int/∂u`.
/// The step residual is `G(u) = M a(u) + C v(u) + f_int(u) − F(t+dt)` where `a,v`
/// follow the Newmark relations; the effective tangent is `Kₜ + a0·M + a1·C`.
#[allow(clippy::too_many_arguments)]
pub fn newmark_nonlinear(
    m: &[f64],
    c: &[f64],
    n: usize,
    internal: impl Fn(&[f64]) -> Vec<f64>,
    tangent: impl Fn(&[f64]) -> Vec<f64>,
    force: impl Fn(f64) -> Vec<f64>,
    u0: &[f64],
    v0: &[f64],
    dt: f64,
    nsteps: usize,
    beta: f64,
    gamma: f64,
    tol: f64,
    max_iter: usize,
) -> Result<NewmarkResult, EngineeringError> {
    if n == 0 {
        return Err(EngineeringError::InsufficientData("zero-DOF system".into()));
    }
    if !(dt > 0.0) || nsteps == 0 || !(beta > 0.0) {
        return Err(EngineeringError::ValidationError(
            "dt, nsteps, β must be positive".into(),
        ));
    }

    let a0 = 1.0 / (beta * dt * dt);
    let a1 = gamma / (beta * dt);
    let a2 = 1.0 / (beta * dt);
    let a3 = 1.0 / (2.0 * beta) - 1.0;
    let a6 = dt * (1.0 - gamma);
    let a7 = gamma * dt;

    let mut u = u0.to_vec();
    let mut v = v0.to_vec();
    // Initial acceleration: M a = F(0) − C v0 − f_int(u0).
    let f0 = force(0.0);
    let cv = matvec(c, &v, n);
    let fi = internal(&u);
    let rhs0: Vec<f64> = (0..n).map(|i| f0[i] - cv[i] - fi[i]).collect();
    let m_lu = lu_decompose(n, m)
        .map_err(|e| EngineeringError::SolverError(format!("mass LU failed: {e:?}")))?;
    let mut a = m_lu
        .solve(&rhs0)
        .ok_or_else(|| EngineeringError::SolverError("mass matrix singular".into()))?;

    let mut out = NewmarkResult {
        time: vec![0.0],
        disp: vec![u.clone()],
        vel: vec![v.clone()],
        acc: vec![a.clone()],
    };

    for step in 0..nsteps {
        let t_next = (step + 1) as f64 * dt;
        let fext = force(t_next);
        // Predictor: trial u = u_n (start Newton from previous displacement).
        let u_n = u.clone();
        let v_n = v.clone();
        let a_n = a.clone();
        let mut u_trial = u_n.clone();

        let mut converged = false;
        for _ in 0..max_iter {
            // a(u) = a0(u − u_n) − a2 v_n − a3 a_n
            // v(u) = v_n + a6 a_n + a7 a(u)
            let a_u: Vec<f64> = (0..n)
                .map(|i| a0 * (u_trial[i] - u_n[i]) - a2 * v_n[i] - a3 * a_n[i])
                .collect();
            let v_u: Vec<f64> = (0..n).map(|i| v_n[i] + a6 * a_n[i] + a7 * a_u[i]).collect();
            let ma = matvec(m, &a_u, n);
            let cvv = matvec(c, &v_u, n);
            let fi_u = internal(&u_trial);
            let g: Vec<f64> = (0..n).map(|i| ma[i] + cvv[i] + fi_u[i] - fext[i]).collect();
            if norm2(&g) <= tol {
                converged = true;
                break;
            }
            // Effective tangent: Kt + a0 M + a1 C.
            let kt = tangent(&u_trial);
            let mut keff = kt;
            for i in 0..n * n {
                keff[i] += a0 * m[i] + a1 * c[i];
            }
            let lu = lu_decompose(n, &keff).map_err(|e| {
                EngineeringError::SolverError(format!("nonlinear-dynamic tangent LU failed: {e:?}"))
            })?;
            let neg_g: Vec<f64> = g.iter().map(|&x| -x).collect();
            let du = lu
                .solve(&neg_g)
                .ok_or_else(|| EngineeringError::SolverError("tangent singular".into()))?;
            axpy_into(&mut u_trial, 1.0, &du);
            if norm2(&du) <= tol {
                converged = true;
                break;
            }
        }
        if !converged {
            return Err(EngineeringError::ConvergenceError(format!(
                "nonlinear-dynamic Newton did not converge at t = {t_next}"
            )));
        }
        // Commit the step.
        let a_new: Vec<f64> = (0..n)
            .map(|i| a0 * (u_trial[i] - u_n[i]) - a2 * v_n[i] - a3 * a_n[i])
            .collect();
        let v_new: Vec<f64> = (0..n)
            .map(|i| v_n[i] + a6 * a_n[i] + a7 * a_new[i])
            .collect();
        u = u_trial;
        v = v_new;
        a = a_new;
        out.time.push(t_next);
        out.disp.push(u.clone());
        out.vel.push(v.clone());
        out.acc.push(a.clone());
    }
    Ok(out)
}

// ─────────────────── Geometrically-nonlinear axial bar (facade) ───────────────────

/// A single-DOF geometrically-nonlinear axial bar (fixed–free), Green–Lagrange strain.
///
/// For a straight prismatic bar of length `L`, section `EA`, with the fixed end at
/// `x = 0` and axial tip displacement `u`, the Green–Lagrange axial strain is
/// `ε = u/L + ½(u/L)²`. Strain energy `U = ½·EA·L·ε²`, so the internal tip force and
/// its tangent are
///
/// * `f_int(u) = EA·ε·(1 + u/L)`
/// * `Kₜ(u) = (EA/L)·[(1 + u/L)² + ε]`
///
/// These reduce to the linear bar (`f = EA·u/L`) for small `u` and stiffen
/// geometrically for large `u`. This is the concrete nonlinear model wired to the
/// facade `NonlinearStatic` / `NonlinearDynamic` variants.
#[derive(Debug, Clone, Copy)]
pub struct GeoNonlinearBar {
    pub ea: f64,
    pub length: f64,
}

impl GeoNonlinearBar {
    /// Green–Lagrange axial strain at tip displacement `u`.
    pub fn strain(&self, u: f64) -> f64 {
        let r = u / self.length;
        r + 0.5 * r * r
    }
    /// Internal tip force `f_int(u)`.
    pub fn internal_force(&self, u: f64) -> f64 {
        self.ea * self.strain(u) * (1.0 + u / self.length)
    }
    /// Tangent stiffness `Kₜ(u)`.
    pub fn tangent(&self, u: f64) -> f64 {
        let r = u / self.length;
        self.ea / self.length * ((1.0 + r) * (1.0 + r) + self.strain(u))
    }
    /// Solve `f_int(u) = f_ext` for the tip displacement by Newton–Raphson.
    pub fn solve_static(
        &self,
        f_ext: f64,
        tol: f64,
        max_iter: usize,
    ) -> Result<f64, EngineeringError> {
        let ea = self.ea;
        let l = self.length;
        let bar = *self;
        let (u, _it) = newton_raphson(
            1,
            |u| vec![bar.internal_force(u[0]) - f_ext],
            |u| vec![bar.tangent(u[0])],
            &[f_ext * l / ea], // linear guess
            tol,
            max_iter,
        )?;
        Ok(u[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const E_STEEL: f64 = 200.0e9; // Pa

    #[test]
    fn axial_bar_single_element_displacement() {
        // Single 2-D truss bar along x: L = 2 m, A = 0.01 m², E = 200 GPa.
        // Node 0 fixed (ux,uy,rz), node 1 rollers on uy,rz, axial load F = 50 kN.
        // Closed form: δ = F·L / (A·E).
        let f = 50.0e3;
        let (l, area) = (2.0, 0.01);
        let model = FeModel {
            nodes: vec![FeNode { x: 0.0, y: 0.0 }, FeNode { x: l, y: 0.0 }],
            elements: vec![FeElement::Truss {
                ni: 0,
                nj: 1,
                e: E_STEEL,
                area,
                rho: 7850.0,
            }],
            // Fix node0 fully; constrain node1 transverse+rotation so only ux is free.
            constraints: vec![(0, 0.0), (1, 0.0), (2, 0.0), (4, 0.0), (5, 0.0)],
            loads: vec![(3, f)], // ux at node1
        };
        let res = solve_static(&model).unwrap();
        let expected = f * l / (area * E_STEEL);
        let got = res.displacements[3];
        assert!(
            (got - expected).abs() / expected < 1e-9,
            "axial δ = {got} (expected {expected})"
        );
        // Reaction at the fixed ux DOF balances the applied load.
        let rux = res.reactions.iter().find(|(d, _)| *d == 0).unwrap().1;
        assert!(
            (rux + f).abs() / f < 1e-9,
            "reaction ux = {rux} (expected {})",
            -f
        );
        // Element axial force equals the applied tension.
        assert!(
            (res.element_axial_force[0] - f).abs() / f < 1e-9,
            "N = {}",
            res.element_axial_force[0]
        );
    }

    #[test]
    fn two_bar_truss_hand_computed_joint_displacement() {
        // Two collinear bars in series along x, each L = 1 m, A = 0.01, E = 200 GPa.
        // n0 fixed — n1 (free) — n2 (free), axial load F at n2.
        // Series stiffness: each k = EA/L. n2 disp = F/k + F/k = 2F/k (both carry F);
        // n1 disp = F/k. Hand: k = EA/L = 200e9·0.01/1 = 2e9 N/m; F = 20 kN.
        // u1 = F/k = 1e-5 m; u2 = 2F/k = 2e-5 m.
        let f = 20.0e3;
        let (l, area) = (1.0, 0.01);
        let k = E_STEEL * area / l;
        let model = FeModel {
            nodes: vec![
                FeNode { x: 0.0, y: 0.0 },
                FeNode { x: l, y: 0.0 },
                FeNode { x: 2.0 * l, y: 0.0 },
            ],
            elements: vec![
                FeElement::Truss {
                    ni: 0,
                    nj: 1,
                    e: E_STEEL,
                    area,
                    rho: 7850.0,
                },
                FeElement::Truss {
                    ni: 1,
                    nj: 2,
                    e: E_STEEL,
                    area,
                    rho: 7850.0,
                },
            ],
            // Fix n0 fully; constrain transverse + rotation of n1,n2 (pure axial chain).
            constraints: vec![
                (0, 0.0),
                (1, 0.0),
                (2, 0.0),
                (4, 0.0),
                (5, 0.0),
                (7, 0.0),
                (8, 0.0),
            ],
            loads: vec![(6, f)], // ux at n2
        };
        let res = solve_static(&model).unwrap();
        let u1 = res.displacements[3];
        let u2 = res.displacements[6];
        assert!(
            (u1 - f / k).abs() / (f / k) < 1e-9,
            "u1 = {u1} (expected {})",
            f / k
        );
        assert!((u2 - 2.0 * f / k).abs() / (2.0 * f / k) < 1e-9, "u2 = {u2}");
    }

    #[test]
    fn cantilever_tip_deflection_matches_euler_bernoulli() {
        // Cantilever, L = 3 m, E = 200 GPa, I = 8e-6 m⁴, tip point load P = 5 kN.
        // Euler–Bernoulli closed form: δ_tip = P·L³ / (3·E·I).
        // A single 2-node beam element gives the EXACT cubic tip solution for a tip load.
        let (l, inertia, area) = (3.0, 8.0e-6, 0.01);
        let p = 5.0e3;
        let model = FeModel {
            nodes: vec![FeNode { x: 0.0, y: 0.0 }, FeNode { x: l, y: 0.0 }],
            elements: vec![FeElement::Frame {
                ni: 0,
                nj: 1,
                e: E_STEEL,
                area,
                inertia,
                rho: 7850.0,
            }],
            // Fully fix the root node (ux,uy,rz).
            constraints: vec![(0, 0.0), (1, 0.0), (2, 0.0)],
            loads: vec![(4, -p)], // downward transverse load (uy) at the tip
        };
        let res = solve_static(&model).unwrap();
        let tip = res.displacements[4]; // uy at node1
        let expected = -p * l * l * l / (3.0 * E_STEEL * inertia);
        assert!(
            (tip - expected).abs() / expected.abs() < 1e-9,
            "tip δ = {tip} (expected {expected})"
        );
        // Root vertical reaction balances the applied load.
        let ruy = res.reactions.iter().find(|(d, _)| *d == 1).unwrap().1;
        assert!(
            (ruy - p).abs() / p < 1e-9,
            "root reaction = {ruy} (expected {p})"
        );
        // Root moment reaction magnitude = P·L (cantilever). Sign follows the standard
        // Euler–Bernoulli DOF convention: hand-deriving the reduced 2-DOF system gives
        // R_θ1 = −6EI/L²·v2 + 2EI/L·θ2 = +P·L for a downward tip load.
        let rrz = res.reactions.iter().find(|(d, _)| *d == 2).unwrap().1;
        assert!(
            (rrz - p * l).abs() / (p * l) < 1e-9,
            "root moment = {rrz} (expected {})",
            p * l
        );
    }

    #[test]
    fn cantilever_two_elements_also_exact() {
        // Refining to two beam elements must not change the (exact) tip deflection.
        let (l, inertia, area) = (3.0, 8.0e-6, 0.01);
        let p = 5.0e3;
        let model = FeModel {
            nodes: vec![
                FeNode { x: 0.0, y: 0.0 },
                FeNode { x: l / 2.0, y: 0.0 },
                FeNode { x: l, y: 0.0 },
            ],
            elements: vec![
                FeElement::Frame {
                    ni: 0,
                    nj: 1,
                    e: E_STEEL,
                    area,
                    inertia,
                    rho: 7850.0,
                },
                FeElement::Frame {
                    ni: 1,
                    nj: 2,
                    e: E_STEEL,
                    area,
                    inertia,
                    rho: 7850.0,
                },
            ],
            constraints: vec![(0, 0.0), (1, 0.0), (2, 0.0)],
            loads: vec![(7, -p)], // uy at node2 (tip)
        };
        let res = solve_static(&model).unwrap();
        let tip = res.displacements[7];
        let expected = -p * l * l * l / (3.0 * E_STEEL * inertia);
        assert!(
            (tip - expected).abs() / expected.abs() < 1e-8,
            "two-element tip δ = {tip} (expected {expected})"
        );
    }

    #[test]
    fn newmark_sdof_undamped_tracks_cosine() {
        // M ü + K u = 0, u(0)=u0, u̇(0)=0 ⇒ u(t) = u0·cos(ωt), ω = √(K/M).
        let (mass, stiff, u0): (f64, f64, f64) = (2.0, 200.0, 0.05);
        let omega = (stiff / mass).sqrt();
        let period = 2.0 * std::f64::consts::PI / omega;
        let dt = period / 400.0;
        let nsteps = 400; // exactly one period
        let res = newmark_linear(
            &[mass],
            &[0.0],
            &[stiff],
            1,
            |_t| vec![0.0],
            &[u0],
            &[0.0],
            dt,
            nsteps,
            0.25,
            0.5,
        )
        .unwrap();
        // Track the analytic cosine at every step.
        let mut max_err = 0.0_f64;
        for (i, u) in res.disp.iter().enumerate() {
            let t = i as f64 * dt;
            let analytic = u0 * (omega * t).cos();
            max_err = max_err.max((u[0] - analytic).abs());
        }
        assert!(
            max_err < 1e-3 * u0,
            "max deviation from u0·cos(ωt) = {max_err} (u0 = {u0})"
        );
        // Undamped energy stays bounded: 0 ≤ E ≤ E0 (never grows).
        let e0 = 0.5 * stiff * u0 * u0;
        for (u, v) in res.disp.iter().zip(res.vel.iter()) {
            let e = 0.5 * mass * v[0] * v[0] + 0.5 * stiff * u[0] * u[0];
            assert!(e <= e0 * (1.0 + 1e-6), "energy grew: {e} > {e0}");
        }
        // After one full period, it returns to (u0, 0).
        assert!((res.disp[nsteps][0] - u0).abs() < 1e-3 * u0);
        assert!(res.vel[nsteps][0].abs() < 1e-2 * omega * u0);
    }

    #[test]
    fn newton_raphson_cubic_spring_hand_solved_root() {
        // Nonlinear spring F = k·u + k3·u³. With k = 100, k3 = 100, F = 200,
        // the equilibrium is u = 1 exactly (100·1 + 100·1³ = 200).
        let (k, k3, f) = (100.0, 100.0, 200.0);
        let (u, iters) = newton_raphson(
            1,
            |u| vec![k * u[0] + k3 * u[0].powi(3) - f],
            |u| vec![k + 3.0 * k3 * u[0] * u[0]],
            &[0.0],
            1e-12,
            50,
        )
        .unwrap();
        assert!((u[0] - 1.0).abs() < 1e-10, "u = {} (expected 1.0)", u[0]);
        assert!(iters < 20, "took {iters} iterations");

        // A second point: F = k·2 + k3·8 = 200 + 800 = 1000 ⇒ u = 2.
        let (u2, _) = newton_raphson(
            1,
            |u| vec![k * u[0] + k3 * u[0].powi(3) - 1000.0],
            |u| vec![k + 3.0 * k3 * u[0] * u[0]],
            &[0.0],
            1e-12,
            50,
        )
        .unwrap();
        assert!((u2[0] - 2.0).abs() < 1e-10, "u2 = {}", u2[0]);
    }

    #[test]
    fn geometric_nonlinear_bar_stiffens_and_recovers_linear() {
        // Small load ⇒ ~linear (δ ≈ FL/EA); large load ⇒ geometric stiffening (δ < linear).
        let ea = 1.0e6;
        let bar = GeoNonlinearBar { ea, length: 1.0 };
        // Small load: 100 N ⇒ linear δ = 1e-4. Nonlinear correction is tiny.
        let small = bar.solve_static(100.0, 1e-12, 100).unwrap();
        let lin_small = 100.0 / ea;
        assert!(
            (small - lin_small).abs() / lin_small < 1e-3,
            "small u = {small}"
        );
        // Large load: nonlinear tip disp is strictly less than the linear estimate
        // (Green strain stiffens in tension) and satisfies f_int(u) = F exactly.
        let big = bar.solve_static(3.0e5, 1e-10, 100).unwrap();
        let lin_big = 3.0e5 / ea;
        assert!(
            big < lin_big,
            "expected stiffening: u = {big}, linear = {lin_big}"
        );
        assert!(
            (bar.internal_force(big) - 3.0e5).abs() < 1e-3,
            "residual not satisfied: f_int = {}",
            bar.internal_force(big)
        );
    }

    #[test]
    fn newmark_nonlinear_duffing_energy_bounded() {
        // Undamped Duffing oscillator: M ü + k u + k3 u³ = 0, u(0)=u0, u̇(0)=0.
        // Total energy E = ½M v² + ½k u² + ¼k3 u⁴ is conserved. Newmark (avg-accel)
        // is not exactly energy-conserving but must stay bounded and close over time.
        let (mass, k, k3, u0): (f64, f64, f64, f64) = (1.0, 100.0, 500.0, 0.3);
        let e0 = 0.5 * k * u0 * u0 + 0.25 * k3 * u0.powi(4);
        // Small linear-regime period for step selection.
        let omega = (k / mass).sqrt();
        let dt = (2.0 * std::f64::consts::PI / omega) / 500.0;
        let res = newmark_nonlinear(
            &[mass],
            &[0.0],
            1,
            |u| vec![k * u[0] + k3 * u[0].powi(3)],
            |u| vec![k + 3.0 * k3 * u[0] * u[0]],
            |_t| vec![0.0],
            &[u0],
            &[0.0],
            dt,
            2000,
            0.25,
            0.5,
            1e-12,
            50,
        )
        .unwrap();
        let mut max_e = 0.0_f64;
        let mut min_e = f64::INFINITY;
        for (u, v) in res.disp.iter().zip(res.vel.iter()) {
            let e = 0.5 * mass * v[0] * v[0] + 0.5 * k * u[0] * u[0] + 0.25 * k3 * u[0].powi(4);
            max_e = max_e.max(e);
            min_e = min_e.min(e);
        }
        // Energy drift stays under 1% of E0 across 2000 steps (4 linear periods).
        assert!(
            (max_e - e0).abs() / e0 < 1e-2 && (e0 - min_e).abs() / e0 < 1e-2,
            "energy drift: E0 = {e0}, min = {min_e}, max = {max_e}"
        );
        // Amplitude never exceeds the initial (undamped, energy-bounded).
        let peak = res.peak_abs(0);
        assert!(peak <= u0 * (1.0 + 1e-3), "amplitude grew: {peak} > {u0}");
    }
}
