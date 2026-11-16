//! True algebraic residual `||b − A x||` and scale-aware admission.
//!
//! `DdgReport.residual` remains iterate change `maxᵢ |x⁺ᵢ − xᵢ|`. This module
//! never renames that field. The algebraic quantity is always labelled
//! `algebraic_residual` and carries a declared [`ResidualNorm`].

use super::conventions::{
    boundary_from_fixed, conventions_from_fixed, BoundaryConditionKind, HeatPoissonSolver,
    HeatRouteStabilityDomain, HeatTimeIntegrator, NullspacePolicy, OperatorConventions,
    ResidualNorm, ResidualRequest, SolverLimits, ABSOLUTE_DENSE_VERTEX_CAP,
    EXPLICIT_EULER_CFL_LIMIT, HEAT_GEODESICS_EXPLICIT_STEPS,
};
use super::super::ddg_operators::{
    cotangent_laplacian, solve_poisson_jacobi, DdgError, DdgReport, SparseEntry,
};
use super::super::geometry_workspace::production_budget::{
    BudgetError, BufferClass, CapacityPlan, ProductionBudget,
};
use super::super::primitives::Point3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DdgProductionError {
    InvalidInput(&'static str),
    UnsupportedSize { vertices: usize, limit: usize },
    IllConditioned { estimate: f64, limit: f64 },
    SingularNullspace,
    UnstableHeatRoute { cfl: f64, limit: f64 },
    OutputTooSmall { required: usize },
    DimensionMismatch,
    Budget(BudgetError),
    Ddg(DdgError),
}
impl From<BudgetError> for DdgProductionError {
    fn from(e: BudgetError) -> Self { Self::Budget(e) }
}
impl From<DdgError> for DdgProductionError {
    fn from(e: DdgError) -> Self { Self::Ddg(e) }
}
impl core::fmt::Display for DdgProductionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidInput(field) => write!(f, "ddg production invalid input: {field}"),
            Self::UnsupportedSize { vertices, limit } => {
                write!(f, "ddg production size {vertices} exceeds dense cap {limit}")
            }
            Self::IllConditioned { estimate, limit } => {
                write!(f, "ddg production condition {estimate} exceeds {limit}")
            }
            Self::SingularNullspace => f.write_str("ddg production Neumann load is not orthogonal to 1"),
            Self::UnstableHeatRoute { cfl, limit } => {
                write!(f, "ddg production explicit heat CFL {cfl} exceeds {limit}")
            }
            Self::OutputTooSmall { required } => write!(f, "ddg production output too small, need {required}"),
            Self::DimensionMismatch => f.write_str("ddg production dimension mismatch"),
            Self::Budget(e) => write!(f, "ddg production budget: {e}"),
            Self::Ddg(e) => write!(f, "ddg production operator: {e:?}"),
        }
    }
}
impl std::error::Error for DdgProductionError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConvergenceStatus { Converged, IterationLimited, NotConverged, Rejected }

/// `||b − A x||` under a declared norm, reduced over free DOFs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlgebraicResidual {
    pub value: f64,
    pub norm: ResidualNorm,
    pub rhs_norm: f64,
    pub solution_norm: f64,
    pub relative: f64,
    pub free_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleAwareVerdict {
    pub status: ConvergenceStatus,
    pub absolute_tolerance: f64,
    pub relative_tolerance: f64,
    pub reference_scale: f64,
    pub scale: f64,
}

/// Instrumented Jacobi report. `jacobi.residual` is still iterate change.
#[derive(Clone, Debug, PartialEq)]
pub struct InstrumentedSolveReport {
    pub jacobi: DdgReport,
    pub algebraic_residual: AlgebraicResidual,
    pub scale_aware: ScaleAwareVerdict,
    pub condition_estimate: f64,
    pub conventions: OperatorConventions,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssembledOperator {
    pub nnz: usize,
    pub vertex_count: usize,
    pub condition_estimate: f64,
    pub conventions: OperatorConventions,
}

pub struct OperatorScratch<'a> {
    pub laplacian: &'a mut [SparseEntry],
    pub mass: &'a mut [f64],
    pub residual: &'a mut [f64],
}

fn charge(budget: &mut ProductionBudget<'_>, factors: &[u64]) -> Result<(), DdgProductionError> {
    let mut work = 1u64;
    for &factor in factors {
        work = work.checked_mul(factor).ok_or(DdgProductionError::InvalidInput("work_overflow"))?;
    }
    budget.checkpoint(work)?;
    Ok(())
}
fn as_u64(n: usize) -> Result<u64, DdgProductionError> {
    u64::try_from(n).map_err(|_| DdgProductionError::InvalidInput("work_overflow"))
}
fn require_len<T>(buf: &[T], required: usize) -> Result<(), DdgProductionError> {
    if buf.len() < required { Err(DdgProductionError::OutputTooSmall { required }) } else { Ok(()) }
}
fn finite(value: f64, field: &'static str) -> Result<f64, DdgProductionError> {
    if value.is_finite() { Ok(value) } else { Err(DdgProductionError::InvalidInput(field)) }
}

/// Live caller buffers. Internal dense assembly of `cotangent_laplacian` is
/// not counted; [`admit_operator_size`] rejects before that allocation.
pub fn operator_capacity_plan(
    vertex_count: usize, triangle_count: usize, laplacian_capacity: usize,
) -> Result<CapacityPlan, DdgProductionError> {
    let mut plan = CapacityPlan::default();
    plan.add_slice::<Point3>(BufferClass::Input, vertex_count)?;
    plan.add_slice::<[u32; 3]>(BufferClass::Input, triangle_count)?;
    plan.add_slice::<bool>(BufferClass::Input, vertex_count)?;
    plan.add_slice::<f64>(BufferClass::Input, vertex_count)?;
    plan.add_slice::<SparseEntry>(BufferClass::Scratch, laplacian_capacity)?;
    plan.add_slice::<f64>(BufferClass::Scratch, vertex_count)?;
    plan.add_slice::<f64>(BufferClass::Scratch, vertex_count)?;
    plan.add_slice::<f64>(BufferClass::Output, vertex_count)?;
    Ok(plan)
}

pub fn admit_operator_size(
    vertex_count: usize, triangle_count: usize, limits: SolverLimits,
) -> Result<(), DdgProductionError> {
    if vertex_count == 0 {
        return Err(DdgProductionError::InvalidInput("empty_vertices"));
    }
    let vcap = limits.effective_vertex_cap();
    let tcap = limits.effective_triangle_cap();
    if vertex_count > vcap {
        return Err(DdgProductionError::UnsupportedSize { vertices: vertex_count, limit: vcap });
    }
    if triangle_count > tcap {
        return Err(DdgProductionError::UnsupportedSize { vertices: triangle_count, limit: tcap });
    }
    vertex_count.checked_mul(vertex_count)
        .and_then(|cells| cells.checked_mul(core::mem::size_of::<f64>()))
        .ok_or(DdgProductionError::InvalidInput("dense_overflow"))?;
    if vertex_count > ABSOLUTE_DENSE_VERTEX_CAP {
        return Err(DdgProductionError::UnsupportedSize {
            vertices: vertex_count, limit: ABSOLUTE_DENSE_VERTEX_CAP,
        });
    }
    Ok(())
}

fn validate_entries(vertex_count: usize, laplacian: &[SparseEntry]) -> Result<(), DdgProductionError> {
    for e in laplacian {
        if e.row as usize >= vertex_count || e.col as usize >= vertex_count || !e.value.is_finite() {
            return Err(DdgProductionError::InvalidInput("laplacian_entry"));
        }
    }
    Ok(())
}

/// `y ← A x` into caller `y`. COO products are serial f64.
pub fn apply_operator(
    vertex_count: usize, laplacian: &[SparseEntry], x: &[f64], y: &mut [f64],
    budget: &mut ProductionBudget<'_>,
) -> Result<(), DdgProductionError> {
    require_len(x, vertex_count)?;
    require_len(y, vertex_count)?;
    validate_entries(vertex_count, laplacian)?;
    charge(budget, &[as_u64(vertex_count)?.saturating_add(as_u64(laplacian.len())?), 2])?;
    if x[..vertex_count].iter().any(|v| !v.is_finite()) {
        return Err(DdgProductionError::InvalidInput("x_nonfinite"));
    }
    y[..vertex_count].fill(0.0);
    for e in laplacian {
        y[e.row as usize] += e.value * x[e.col as usize];
    }
    if y[..vertex_count].iter().any(|v| !v.is_finite()) {
        return Err(DdgProductionError::InvalidInput("ax_overflow"));
    }
    Ok(())
}

fn reduce_norm(values: &[f64], mask: &[bool], norm: ResidualNorm) -> (f64, usize) {
    let mut free = 0usize;
    match norm {
        ResidualNorm::EuclideanTwo => {
            let mut sum = 0.0;
            for (i, &v) in values.iter().enumerate() {
                if mask[i] { continue; }
                sum += v * v;
                free += 1;
            }
            (sum.sqrt(), free)
        }
        ResidualNorm::Infinity => {
            let mut max: f64 = 0.0;
            for (i, &v) in values.iter().enumerate() {
                if mask[i] { continue; }
                max = max.max(v.abs());
                free += 1;
            }
            (max, free)
        }
    }
}

/// `r = b − A x` on free DOFs. Dirichlet rows are zeroed, not residualized.
pub fn algebraic_residual(
    vertex_count: usize, laplacian: &[SparseEntry], rhs: &[f64], values: &[f64],
    fixed: &[bool], residual: &mut [f64], request: ResidualRequest,
    budget: &mut ProductionBudget<'_>,
) -> Result<AlgebraicResidual, DdgProductionError> {
    if rhs.len() < vertex_count || values.len() < vertex_count || fixed.len() < vertex_count {
        return Err(DdgProductionError::DimensionMismatch);
    }
    require_len(residual, vertex_count)?;
    if !request.absolute_tolerance.is_finite() || request.absolute_tolerance < 0.0
        || !request.relative_tolerance.is_finite() || request.relative_tolerance < 0.0
        || !request.reference_scale.is_finite() || request.reference_scale < 0.0
    {
        return Err(DdgProductionError::InvalidInput("residual_request"));
    }
    apply_operator(vertex_count, laplacian, values, residual, budget)?;
    charge(budget, &[as_u64(vertex_count)?, 2])?;
    for i in 0..vertex_count {
        if !rhs[i].is_finite() {
            return Err(DdgProductionError::InvalidInput("rhs_nonfinite"));
        }
        residual[i] = if fixed[i] { 0.0 } else { rhs[i] - residual[i] };
        if !residual[i].is_finite() {
            return Err(DdgProductionError::InvalidInput("residual_overflow"));
        }
    }
    let (value, free_count) = reduce_norm(&residual[..vertex_count], &fixed[..vertex_count], request.norm);
    let (rhs_norm, _) = reduce_norm(&rhs[..vertex_count], &fixed[..vertex_count], request.norm);
    let solution_norm = match request.norm {
        ResidualNorm::EuclideanTwo => values[..vertex_count].iter().map(|v| v * v).sum::<f64>().sqrt(),
        ResidualNorm::Infinity => values[..vertex_count].iter().fold(0.0_f64, |a, v| a.max(v.abs())),
    };
    finite(value, "residual_norm")?;
    finite(rhs_norm, "rhs_norm")?;
    finite(solution_norm, "solution_norm")?;
    let scale = rhs_norm.max(request.reference_scale);
    let relative = if scale > 0.0 { value / scale } else { value };
    Ok(AlgebraicResidual {
        value, norm: request.norm, rhs_norm, solution_norm,
        relative: finite(relative, "relative_residual")?, free_count,
    })
}

pub fn scale_aware_verdict(
    residual: &AlgebraicResidual, request: ResidualRequest, iteration_limited: bool,
) -> ScaleAwareVerdict {
    let scale = residual.rhs_norm.max(request.reference_scale);
    let converged = residual.value <= request.absolute_tolerance
        || residual.relative <= request.relative_tolerance;
    let status = if converged {
        ConvergenceStatus::Converged
    } else if iteration_limited {
        ConvergenceStatus::IterationLimited
    } else {
        ConvergenceStatus::NotConverged
    };
    ScaleAwareVerdict {
        status, absolute_tolerance: request.absolute_tolerance,
        relative_tolerance: request.relative_tolerance,
        reference_scale: request.reference_scale, scale,
    }
}

/// Diagonal ratio `max |L_ii| / min_{free, L_ii ≠ 0} |L_ii|`.
pub fn condition_estimate(
    vertex_count: usize, laplacian: &[SparseEntry], fixed: &[bool],
    budget: &mut ProductionBudget<'_>,
) -> Result<f64, DdgProductionError> {
    if fixed.len() < vertex_count { return Err(DdgProductionError::DimensionMismatch); }
    validate_entries(vertex_count, laplacian)?;
    charge(budget, &[as_u64(vertex_count)?.saturating_add(as_u64(laplacian.len())?)])?;
    let mut max_abs = 0.0_f64;
    let mut min_abs = f64::INFINITY;
    let mut saw_free_diag = false;
    for e in laplacian {
        if e.row != e.col { continue; }
        let i = e.row as usize;
        let mag = e.value.abs();
        max_abs = max_abs.max(mag);
        if !fixed[i] {
            if mag == 0.0 {
                return Err(DdgProductionError::IllConditioned { estimate: f64::INFINITY, limit: 0.0 });
            }
            min_abs = min_abs.min(mag);
            saw_free_diag = true;
        }
    }
    if !saw_free_diag { return Ok(1.0); }
    finite(max_abs / min_abs, "condition_estimate")
}

fn neumann_load_consistent(
    vertex_count: usize, rhs: &[f64], conventions: &OperatorConventions, scale: f64,
) -> Result<(), DdgProductionError> {
    match conventions.nullspace {
        NullspacePolicy::TrivialAfterDirichlet => Ok(()),
        NullspacePolicy::ConstantOnConnectedNeumann { pin } if pin.is_some() => Ok(()),
        NullspacePolicy::ConstantOnConnectedNeumann { .. } => {
            let sum: f64 = rhs[..vertex_count].iter().sum();
            let tol = (scale.max(1.0) * vertex_count as f64) * 1.0e-12;
            if sum.abs() > tol { Err(DdgProductionError::SingularNullspace) } else { Ok(()) }
        }
    }
}

fn pin_matches_fixed(conventions: &OperatorConventions, fixed: &[bool]) -> Result<(), DdgProductionError> {
    if let NullspacePolicy::ConstantOnConnectedNeumann { pin: Some(index) } = conventions.nullspace {
        let i = index as usize;
        if i >= fixed.len() || !fixed[i] {
            return Err(DdgProductionError::InvalidInput("nullspace_pin"));
        }
    }
    Ok(())
}

pub fn assemble_cotangent_admitted(
    vertices: &[Point3], triangles: &[[u32; 3]], scratch: OperatorScratch<'_>,
    conventions: OperatorConventions, limits: SolverLimits, budget: &mut ProductionBudget<'_>,
) -> Result<AssembledOperator, DdgProductionError> {
    let n = vertices.len();
    admit_operator_size(n, triangles.len(), limits)?;
    let required = triangles.len().checked_mul(9).ok_or(DdgProductionError::InvalidInput("nnz_overflow"))?;
    if required > limits.max_nnz {
        return Err(DdgProductionError::UnsupportedSize { vertices: required, limit: limits.max_nnz });
    }
    require_len(scratch.laplacian, required)?;
    require_len(scratch.mass, n)?;
    require_len(scratch.residual, n)?;
    charge(budget, &[as_u64(n)?, as_u64(n)?, as_u64(triangles.len().saturating_add(1))?])?;
    let nnz = cotangent_laplacian(vertices, triangles, scratch.laplacian, scratch.mass)?;
    if nnz > limits.max_nnz {
        return Err(DdgProductionError::UnsupportedSize { vertices: nnz, limit: limits.max_nnz });
    }
    for &mass in &scratch.mass[..n] {
        if !mass.is_finite() || mass < limits.min_positive_mass {
            return Err(DdgProductionError::IllConditioned {
                estimate: if mass == 0.0 { f64::INFINITY } else { mass.abs() },
                limit: limits.min_positive_mass,
            });
        }
    }
    scratch.residual[..n].fill(0.0);
    let mut max_abs = 0.0_f64;
    let mut min_abs = f64::INFINITY;
    for e in &scratch.laplacian[..nnz] {
        if e.row != e.col { continue; }
        let mag = e.value.abs();
        if mag == 0.0 { continue; }
        max_abs = max_abs.max(mag);
        min_abs = min_abs.min(mag);
    }
    let estimate = if min_abs.is_finite() && min_abs > 0.0 { max_abs / min_abs } else { f64::INFINITY };
    if !estimate.is_finite() || estimate > limits.max_condition_estimate {
        return Err(DdgProductionError::IllConditioned { estimate, limit: limits.max_condition_estimate });
    }
    Ok(AssembledOperator { nnz, vertex_count: n, condition_estimate: estimate, conventions })
}

pub fn solve_poisson_instrumented(
    vertex_count: usize, laplacian: &[SparseEntry], rhs: &[f64], fixed: &[bool],
    values: &mut [f64], residual: &mut [f64], iterations: u32, request: ResidualRequest,
    conventions: OperatorConventions, limits: SolverLimits, budget: &mut ProductionBudget<'_>,
) -> Result<InstrumentedSolveReport, DdgProductionError> {
    if rhs.len() < vertex_count || fixed.len() < vertex_count || values.len() < vertex_count {
        return Err(DdgProductionError::DimensionMismatch);
    }
    admit_operator_size(vertex_count, 0, limits)?;
    if laplacian.len() > limits.max_nnz {
        return Err(DdgProductionError::UnsupportedSize { vertices: laplacian.len(), limit: limits.max_nnz });
    }
    let jacobi_cap = limits.effective_jacobi_cap();
    if iterations > jacobi_cap {
        return Err(DdgProductionError::UnsupportedSize { vertices: iterations as usize, limit: jacobi_cap as usize });
    }
    pin_matches_fixed(&conventions, fixed)?;
    match (conventions.boundary, boundary_from_fixed(&fixed[..vertex_count])) {
        (BoundaryConditionKind::NeumannNatural, BoundaryConditionKind::NeumannNatural) => {}
        (
            BoundaryConditionKind::Dirichlet { pinned_count: declared },
            BoundaryConditionKind::Dirichlet { pinned_count: actual },
        ) if declared == actual => {}
        _ => return Err(DdgProductionError::InvalidInput("boundary_mismatch")),
    }
    let rhs_scale = match request.norm {
        ResidualNorm::EuclideanTwo => rhs[..vertex_count].iter().map(|v| v * v).sum::<f64>().sqrt(),
        ResidualNorm::Infinity => rhs[..vertex_count].iter().fold(0.0_f64, |a, v| a.max(v.abs())),
    };
    neumann_load_consistent(vertex_count, rhs, &conventions, rhs_scale.max(request.reference_scale))?;
    let condition = condition_estimate(vertex_count, laplacian, fixed, budget)?;
    if !condition.is_finite() || condition > limits.max_condition_estimate {
        return Err(DdgProductionError::IllConditioned { estimate: condition, limit: limits.max_condition_estimate });
    }
    charge(budget, &[
        u64::from(iterations.max(1)), as_u64(vertex_count)?, as_u64(laplacian.len().saturating_add(1))?,
    ])?;
    let jacobi = solve_poisson_jacobi(
        vertex_count, laplacian, rhs, fixed, values, iterations, request.absolute_tolerance,
    )?;
    let algebraic = algebraic_residual(
        vertex_count, laplacian, rhs, values, fixed, residual, request, budget,
    )?;
    let iteration_limited = jacobi.iterations >= iterations && algebraic.value > request.absolute_tolerance;
    Ok(InstrumentedSolveReport {
        jacobi, algebraic_residual: algebraic,
        scale_aware: scale_aware_verdict(&algebraic, request, iteration_limited),
        condition_estimate: condition, conventions,
    })
}

pub fn heat_route_stability(
    vertex_count: usize, laplacian: &[SparseEntry], mass: &[f64], dt: f64,
    explicit_steps: u32, jacobi_iterations: u32, limits: SolverLimits,
    budget: &mut ProductionBudget<'_>,
) -> Result<HeatRouteStabilityDomain, DdgProductionError> {
    admit_operator_size(vertex_count, 0, limits)?;
    require_len(mass, vertex_count)?;
    if !(dt.is_finite() && dt > 0.0) {
        return Err(DdgProductionError::InvalidInput("dt"));
    }
    if explicit_steps == 0 || explicit_steps > limits.effective_heat_step_cap() {
        return Err(DdgProductionError::UnsupportedSize {
            vertices: explicit_steps as usize, limit: limits.effective_heat_step_cap() as usize,
        });
    }
    if jacobi_iterations > limits.effective_jacobi_cap() {
        return Err(DdgProductionError::UnsupportedSize {
            vertices: jacobi_iterations as usize, limit: limits.effective_jacobi_cap() as usize,
        });
    }
    validate_entries(vertex_count, laplacian)?;
    charge(budget, &[as_u64(vertex_count)?.saturating_add(as_u64(laplacian.len())?)])?;
    let mut max_ratio = 0.0_f64;
    for i in 0..vertex_count {
        let m = mass[i];
        if !m.is_finite() || m < limits.min_positive_mass {
            return Err(DdgProductionError::IllConditioned {
                estimate: if m == 0.0 { f64::INFINITY } else { m.abs() },
                limit: limits.min_positive_mass,
            });
        }
        let mut diag = 0.0;
        for e in laplacian {
            if e.row as usize == i && e.col as usize == i { diag = e.value; break; }
        }
        let ratio = diag.abs() / m;
        if !ratio.is_finite() {
            return Err(DdgProductionError::IllConditioned { estimate: ratio, limit: limits.max_condition_estimate });
        }
        max_ratio = max_ratio.max(ratio);
    }
    let cfl = dt * max_ratio;
    if cfl > EXPLICIT_EULER_CFL_LIMIT {
        return Err(DdgProductionError::UnstableHeatRoute { cfl, limit: EXPLICIT_EULER_CFL_LIMIT });
    }
    Ok(HeatRouteStabilityDomain {
        integrator: HeatTimeIntegrator::ExplicitEuler,
        poisson: HeatPoissonSolver::BoundedJacobi,
        explicit_steps, dt, max_generalized_diagonal: max_ratio, cfl,
        cfl_limit: EXPLICIT_EULER_CFL_LIMIT, jacobi_iterations,
        numerically_stable: true,
        within_declared_route: explicit_steps <= HEAT_GEODESICS_EXPLICIT_STEPS,
    })
}

pub fn default_conventions_for_fixed(fixed: &[bool]) -> OperatorConventions {
    conventions_from_fixed(fixed)
}
