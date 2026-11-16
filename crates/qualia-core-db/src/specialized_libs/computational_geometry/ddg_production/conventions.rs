//! Typed operator conventions for the existing cotangent / heat route.
//!
//! These records declare units, sign, boundary conditions, mass lumping,
//! nullspace treatment, transfer-contract notes, and the explicit heat-route
//! stability domain. They describe `ddg_operators` / `heat_geodesics`; they
//! do not change those modules.

use super::super::primitives::Point3;
use super::super::production_contracts::{EntityDomain, FieldSemantics, TransferRule};

/// Vector norm used for `algebraic_residual`. Always declared, never implied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResidualNorm {
    /// Euclidean 2-norm √(Σ rᵢ²) over free degrees of freedom.
    EuclideanTwo,
    /// Infinity-norm max |rᵢ| over free degrees of freedom.
    Infinity,
}

/// Physical / discrete unit of a DDG quantity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuantityUnit {
    LengthMetres,
    AreaMetres2,
    /// Cotangent weights and assembled stiffness entries are dimensionless.
    DimensionlessStiffness,
    /// Generalized heat operator M⁻¹L has SI units 1/length²; dt supplies time.
    InverseLength2,
    CallerDeclaredPotential,
    CallerDeclaredLoad,
}

/// Units attached to the cotangent operator and lumped mass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OperatorUnits {
    pub length_metres: f64,
    pub potential: QuantityUnit,
    pub stiffness: QuantityUnit,
    pub mass: QuantityUnit,
    pub load: QuantityUnit,
}

impl OperatorUnits {
    pub const fn metres() -> Self {
        Self {
            length_metres: 1.0,
            potential: QuantityUnit::CallerDeclaredPotential,
            stiffness: QuantityUnit::DimensionlessStiffness,
            mass: QuantityUnit::AreaMetres2,
            load: QuantityUnit::CallerDeclaredLoad,
        }
    }
}

/// Sign convention of the assembled cotangent matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaplacianSign {
    /// `L_ii = Σⱼ wᵢⱼ`, `L_ij = −wᵢⱼ`, `w = ½(cot α + cot β)`.
    /// This is the SPD graph Laplacian `L = −Δ` with zero row-sum.
    /// Heat step in code: `u ← u − dt M⁻¹ L u` (explicit Euler).
    NegativeLaplacePositiveStiffness,
}

/// Boundary condition actually presented to Jacobi / residual reduction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryConditionKind {
    /// No pinned vertices. `L` has a constant kernel on a connected mesh.
    NeumannNatural,
    /// `pinned_count` Dirichlet vertices held in `fixed`.
    Dirichlet { pinned_count: usize },
}

/// Mass matrix used by heat and by load/potential transfer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MassLumping {
    /// `cotangent_laplacian` writes barycentric area/3 per incident triangle.
    BarycentricAreaThird,
}

/// How the constant kernel is treated. No coarsening / AMG hierarchy is supplied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NullspacePolicy {
    /// Connected Neumann problem: `L 1 = 0`. Optional pin must also be `fixed`.
    ConstantOnConnectedNeumann { pin: Option<u32> },
    /// At least one Dirichlet pin: trivial kernel on free DOFs.
    TrivialAfterDirichlet,
}

/// Transfer-contract notes for potentials vs mass-weighted loads.
/// Coarse-to-fine injection is intentionally unsupported (no coarsening engine).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransferContractNotes {
    pub domain: EntityDomain,
    pub potential_semantics: FieldSemantics,
    pub load_semantics: FieldSemantics,
    pub potential_transfer: TransferRule,
    pub load_transfer: TransferRule,
    pub mass_lumped_loads: bool,
    pub coarsening_supported: bool,
}

impl TransferContractNotes {
    pub const fn nodal_potential_and_lumped_load() -> Self {
        Self {
            domain: EntityDomain::Vertex,
            potential_semantics: FieldSemantics::Intensive,
            load_semantics: FieldSemantics::Extensive,
            potential_transfer: TransferRule::Interpolation,
            load_transfer: TransferRule::ConservativeIntegration,
            mass_lumped_loads: true,
            coarsening_supported: false,
        }
    }
}

/// Complete declared convention record for a DDG operator application.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OperatorConventions {
    pub units: OperatorUnits,
    pub sign: LaplacianSign,
    pub boundary: BoundaryConditionKind,
    pub mass: MassLumping,
    pub nullspace: NullspacePolicy,
    pub transfer: TransferContractNotes,
}

impl OperatorConventions {
    pub fn cotangent(boundary: BoundaryConditionKind, nullspace: NullspacePolicy) -> Self {
        Self {
            units: OperatorUnits::metres(),
            sign: LaplacianSign::NegativeLaplacePositiveStiffness,
            boundary,
            mass: MassLumping::BarycentricAreaThird,
            nullspace,
            transfer: TransferContractNotes::nodal_potential_and_lumped_load(),
        }
    }
}

pub fn boundary_from_fixed(fixed: &[bool]) -> BoundaryConditionKind {
    let pinned = fixed.iter().filter(|&&flag| flag).count();
    if pinned == 0 {
        BoundaryConditionKind::NeumannNatural
    } else {
        BoundaryConditionKind::Dirichlet {
            pinned_count: pinned,
        }
    }
}

pub fn nullspace_from_boundary(boundary: BoundaryConditionKind) -> NullspacePolicy {
    match boundary {
        BoundaryConditionKind::NeumannNatural => {
            NullspacePolicy::ConstantOnConnectedNeumann { pin: None }
        }
        BoundaryConditionKind::Dirichlet { .. } => NullspacePolicy::TrivialAfterDirichlet,
    }
}

pub fn conventions_from_fixed(fixed: &[bool]) -> OperatorConventions {
    let boundary = boundary_from_fixed(fixed);
    OperatorConventions::cotangent(boundary, nullspace_from_boundary(boundary))
}

/// Time integrator actually used by `heat_step` / `heat_geodesics`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeatTimeIntegrator {
    ExplicitEuler,
}

/// Poisson solver actually used by `heat_geodesics` after the heat steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeatPoissonSolver {
    BoundedJacobi,
}

/// Explicit Euler is stable for `dt · λ_max(M⁻¹L) ≤ 2`.
/// `λ_max` is bounded by `max_i L_ii / M_ii` via Gershgorin on the lumped system.
pub const EXPLICIT_EULER_CFL_LIMIT: f64 = 2.0;
/// `heat_geodesics` always takes eight explicit heat steps.
pub const HEAT_GEODESICS_EXPLICIT_STEPS: u32 = 8;

/// Declared stability domain of the existing heat geodesic route.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeatRouteStabilityDomain {
    pub integrator: HeatTimeIntegrator,
    pub poisson: HeatPoissonSolver,
    pub explicit_steps: u32,
    pub dt: f64,
    /// maxᵢ Lᵢᵢ / Mᵢᵢ over vertices with positive mass.
    pub max_generalized_diagonal: f64,
    /// `dt * max_generalized_diagonal`. Compare with [`EXPLICIT_EULER_CFL_LIMIT`].
    pub cfl: f64,
    pub cfl_limit: f64,
    pub jacobi_iterations: u32,
    pub numerically_stable: bool,
    /// Steps do not exceed the existing heat-geodesics explicit route.
    pub within_declared_route: bool,
}

/// Hard cap on vertices for any path that reuses dense cotangent assembly.
pub const ABSOLUTE_DENSE_VERTEX_CAP: usize = 1024;
pub const ABSOLUTE_TRIANGLE_CAP: usize = 4096;
pub const ABSOLUTE_JACOBI_ITERATION_CAP: u32 = 4096;

/// Caller-tunable limits, always clamped by the absolute dense-assembly caps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolverLimits {
    pub max_vertices: usize,
    pub max_triangles: usize,
    pub max_nnz: usize,
    pub max_condition_estimate: f64,
    pub min_positive_mass: f64,
    pub max_explicit_heat_steps: u32,
    pub max_jacobi_iterations: u32,
}

impl Default for SolverLimits {
    fn default() -> Self {
        Self {
            max_vertices: 512,
            max_triangles: 2048,
            max_nnz: 512 * 18,
            max_condition_estimate: 1.0e10,
            min_positive_mass: 1.0e-18,
            max_explicit_heat_steps: HEAT_GEODESICS_EXPLICIT_STEPS,
            max_jacobi_iterations: 1024,
        }
    }
}

impl SolverLimits {
    pub fn effective_vertex_cap(self) -> usize {
        self.max_vertices.min(ABSOLUTE_DENSE_VERTEX_CAP)
    }

    pub fn effective_triangle_cap(self) -> usize {
        self.max_triangles.min(ABSOLUTE_TRIANGLE_CAP)
    }

    pub fn effective_jacobi_cap(self) -> u32 {
        self.max_jacobi_iterations.min(ABSOLUTE_JACOBI_ITERATION_CAP)
    }

    pub fn effective_heat_step_cap(self) -> u32 {
        self.max_explicit_heat_steps
            .min(HEAT_GEODESICS_EXPLICIT_STEPS)
    }
}

/// Scale-aware residual tolerances. Norm is explicit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResidualRequest {
    pub norm: ResidualNorm,
    pub absolute_tolerance: f64,
    pub relative_tolerance: f64,
    pub reference_scale: f64,
}

impl ResidualRequest {
    pub const fn two_norm() -> Self {
        Self {
            norm: ResidualNorm::EuclideanTwo,
            absolute_tolerance: 1.0e-10,
            relative_tolerance: 1.0e-8,
            reference_scale: 1.0,
        }
    }

    pub const fn inf_norm() -> Self {
        Self {
            norm: ResidualNorm::Infinity,
            absolute_tolerance: 1.0e-10,
            relative_tolerance: 1.0e-8,
            reference_scale: 1.0,
        }
    }
}

/// `heat_geodesics` diffusion time `dt = t_factor · h̄²`.
pub fn heat_geodesics_dt(mean_edge_length: f64, t_factor: f64) -> Option<f64> {
    if !(mean_edge_length.is_finite() && mean_edge_length > 0.0)
        || !(t_factor.is_finite() && t_factor > 0.0)
    {
        return None;
    }
    let dt = t_factor * mean_edge_length * mean_edge_length;
    dt.is_finite().then_some(dt)
}

pub fn recommended_explicit_dt(max_generalized_diagonal: f64) -> Option<f64> {
    if !(max_generalized_diagonal.is_finite() && max_generalized_diagonal > 0.0) {
        return None;
    }
    let dt = EXPLICIT_EULER_CFL_LIMIT / max_generalized_diagonal;
    dt.is_finite().then_some(dt)
}

/// Analytic unit square, two triangles. All four vertices are boundary.
pub fn fill_unit_square(vertices: &mut [Point3], triangles: &mut [[u32; 3]]) -> Option<(usize, usize)> {
    if vertices.len() < 4 || triangles.len() < 2 {
        return None;
    }
    vertices[0] = Point3::new(0.0, 0.0, 0.0);
    vertices[1] = Point3::new(1.0, 0.0, 0.0);
    vertices[2] = Point3::new(1.0, 1.0, 0.0);
    vertices[3] = Point3::new(0.0, 1.0, 0.0);
    triangles[0] = [0, 1, 2];
    triangles[1] = [0, 2, 3];
    Some((4, 2))
}

/// Planar `(nx+1)×(ny+1)` grid. Interior vertices exist when `nx, ny ≥ 2`.
pub fn fill_planar_grid(
    nx: usize,
    ny: usize,
    vertices: &mut [Point3],
    triangles: &mut [[u32; 3]],
) -> Option<(usize, usize)> {
    if nx == 0 || ny == 0 {
        return None;
    }
    let vx = nx.checked_add(1)?;
    let vy = ny.checked_add(1)?;
    let n = vx.checked_mul(vy)?;
    let t = nx.checked_mul(ny).and_then(|c| c.checked_mul(2))?;
    if vertices.len() < n || triangles.len() < t {
        return None;
    }
    for j in 0..vy {
        for i in 0..vx {
            vertices[j * vx + i] = Point3::new(i as f64 / nx as f64, j as f64 / ny as f64, 0.0);
        }
    }
    let mut k = 0usize;
    for j in 0..ny {
        for i in 0..nx {
            let a = (j * vx + i) as u32;
            let c = ((j + 1) * vx + i) as u32;
            triangles[k] = [a, a + 1, c + 1];
            triangles[k + 1] = [a, c + 1, c];
            k += 2;
        }
    }
    Some((n, t))
}
