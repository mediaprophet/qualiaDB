use super::*;
use super::super::ddg_operators::{heat_step, SparseEntry};
use super::super::geometry_workspace::production_budget::{CapacityPlan, ProductionBudget};
use super::super::geometry_workspace::Cancellation;
use super::super::primitives::Point3;

fn budget<'a>(cancel: &'a Cancellation, work: u64) -> ProductionBudget<'a> {
    ProductionBudget::admit(CapacityPlan::default(), usize::MAX, work, cancel).unwrap()
}

fn spd2() -> [SparseEntry; 4] {
    [
        SparseEntry {
            row: 0,
            col: 0,
            value: 4.0,
        },
        SparseEntry {
            row: 0,
            col: 1,
            value: -1.0,
        },
        SparseEntry {
            row: 1,
            col: 0,
            value: -1.0,
        },
        SparseEntry {
            row: 1,
            col: 1,
            value: 1.0,
        },
    ]
}

fn definite_conventions() -> OperatorConventions {
    OperatorConventions::cotangent(
        BoundaryConditionKind::NeumannNatural,
        NullspacePolicy::TrivialAfterDirichlet,
    )
}

#[test]
fn declared_cotangent_conventions_match_assembly() {
    let cancel = Cancellation::new();
    let mut budget = budget(&cancel, u64::MAX);
    let mut vertices = [Point3::default(); 4];
    let mut triangles = [[0; 3]; 2];
    let (n, t) = fill_unit_square(&mut vertices, &mut triangles).expect("square");
    let mut laplacian = [SparseEntry {
        row: 0,
        col: 0,
        value: 0.0,
    }; 18];
    let mut mass = [0.0; 4];
    let mut residual = [0.0; 4];
    let assembled = assemble_cotangent_admitted(
        &vertices[..n],
        &triangles[..t],
        OperatorScratch {
            laplacian: &mut laplacian,
            mass: &mut mass,
            residual: &mut residual,
        },
        definite_conventions(),
        SolverLimits::default(),
        &mut budget,
    )
    .unwrap();
    assert_eq!(
        assembled.conventions.sign,
        LaplacianSign::NegativeLaplacePositiveStiffness
    );
    assert_eq!(assembled.conventions.mass, MassLumping::BarycentricAreaThird);
    assert!(!assembled.conventions.transfer.coarsening_supported);
    for r in 0..n {
        let row_sum: f64 = laplacian[..assembled.nnz]
            .iter()
            .filter(|e| e.row as usize == r)
            .map(|e| e.value)
            .sum();
        assert!(row_sum.abs() < 1e-12);
        assert!(mass[r] > 0.0);
    }
}

#[test]
fn constant_field_is_the_neumann_nullspace() {
    let cancel = Cancellation::new();
    let mut budget = budget(&cancel, u64::MAX);
    let mut vertices = [Point3::default(); 4];
    let mut triangles = [[0; 3]; 2];
    let (n, t) = fill_unit_square(&mut vertices, &mut triangles).expect("square");
    let mut laplacian = [SparseEntry {
        row: 0,
        col: 0,
        value: 0.0,
    }; 18];
    let mut mass = [0.0; 4];
    let mut residual = [0.0; 4];
    let assembled = assemble_cotangent_admitted(
        &vertices[..n],
        &triangles[..t],
        OperatorScratch {
            laplacian: &mut laplacian,
            mass: &mut mass,
            residual: &mut residual,
        },
        conventions_from_fixed(&[false; 4]),
        SolverLimits::default(),
        &mut budget,
    )
    .unwrap();
    let values = [1.0; 4];
    let rhs = [0.0; 4];
    let fixed = [false; 4];
    let report = algebraic_residual(
        n,
        &laplacian[..assembled.nnz],
        &rhs,
        &values,
        &fixed,
        &mut residual,
        ResidualRequest::two_norm(),
        &mut budget,
    )
    .unwrap();
    assert_eq!(report.norm, ResidualNorm::EuclideanTwo);
    assert!(report.value < 1e-12);
    assert_eq!(report.free_count, 4);
}

#[test]
fn planar_grid_has_linear_precision_on_free_dofs() {
    let cancel = Cancellation::new();
    let mut budget = budget(&cancel, u64::MAX);
    let mut vertices = [Point3::default(); 9];
    let mut triangles = [[0; 3]; 8];
    let (n, t) = fill_planar_grid(2, 2, &mut vertices, &mut triangles).expect("grid");
    let mut laplacian = [SparseEntry {
        row: 0,
        col: 0,
        value: 0.0,
    }; 72];
    let mut mass = [0.0; 9];
    let mut residual = [0.0; 9];
    let mut fixed = [false; 9];
    for i in 0..n {
        let p = vertices[i];
        fixed[i] = p.x == 0.0 || p.x == 1.0 || p.y == 0.0 || p.y == 1.0;
    }
    let conventions = conventions_from_fixed(&fixed);
    let assembled = assemble_cotangent_admitted(
        &vertices[..n],
        &triangles[..t],
        OperatorScratch {
            laplacian: &mut laplacian,
            mass: &mut mass,
            residual: &mut residual,
        },
        conventions,
        SolverLimits::default(),
        &mut budget,
    )
    .unwrap();
    let mut values = [0.0; 9];
    for i in 0..n {
        values[i] = vertices[i].x;
    }
    let rhs = [0.0; 9];
    let report = algebraic_residual(
        n,
        &laplacian[..assembled.nnz],
        &rhs,
        &values,
        &fixed,
        &mut residual,
        ResidualRequest::inf_norm(),
        &mut budget,
    )
    .unwrap();
    assert_eq!(report.norm, ResidualNorm::Infinity);
    assert_eq!(report.free_count, 1);
    assert!(report.value < 1e-12, "linear precision residual {}", report.value);
}

#[test]
fn algebraic_residual_is_not_iterate_change() {
    let cancel = Cancellation::new();
    let mut budget = budget(&cancel, u64::MAX);
    let lap = spd2();
    let rhs = [0.0, 0.0];
    let fixed = [false, false];
    let mut values = [0.0, 1.0];
    let mut residual = [0.0; 2];
    let request = ResidualRequest::two_norm();
    let report = solve_poisson_instrumented(
        2,
        &lap,
        &rhs,
        &fixed,
        &mut values,
        &mut residual,
        1,
        request,
        definite_conventions(),
        SolverLimits::default(),
        &mut budget,
    )
    .unwrap();
    // DdgReport.residual stays iterate change; algebraic_residual is ||b−Ax||_2.
    assert!(report.jacobi.residual.is_finite());
    assert_eq!(report.algebraic_residual.norm, ResidualNorm::EuclideanTwo);
    assert!(
        (report.jacobi.residual - report.algebraic_residual.value).abs() > 1e-6,
        "iterate change {} must not be silently treated as ||b-Ax|| {}",
        report.jacobi.residual,
        report.algebraic_residual.value
    );
}

#[test]
fn zero_iteration_keeps_infinite_iterate_change_and_finite_algebraic_residual() {
    let cancel = Cancellation::new();
    let mut budget = budget(&cancel, u64::MAX);
    let lap = spd2();
    let rhs = [3.0, 1.0];
    let fixed = [false, false];
    let mut values = [0.0, 0.0];
    let mut residual = [0.0; 2];
    let report = solve_poisson_instrumented(
        2,
        &lap,
        &rhs,
        &fixed,
        &mut values,
        &mut residual,
        0,
        ResidualRequest::inf_norm(),
        definite_conventions(),
        SolverLimits::default(),
        &mut budget,
    )
    .unwrap();
    assert!(report.jacobi.residual.is_infinite());
    assert_eq!(report.jacobi.iterations, 0);
    assert!((report.algebraic_residual.value - 3.0).abs() < 1e-12);
    assert_eq!(report.algebraic_residual.norm, ResidualNorm::Infinity);
}

#[test]
fn scale_aware_relative_residual_accepts_large_rhs() {
    let cancel = Cancellation::new();
    let mut budget = budget(&cancel, u64::MAX);
    let lap = [
        SparseEntry {
            row: 0,
            col: 0,
            value: 1.0,
        },
    ];
    let rhs = [1.0e6];
    let fixed = [false];
    let values = [1.0e6 + 1.0];
    let mut residual = [0.0; 1];
    let mut request = ResidualRequest::inf_norm();
    request.absolute_tolerance = 1e-12;
    request.relative_tolerance = 1e-5;
    request.reference_scale = 1.0;
    let residual_report = algebraic_residual(
        1,
        &lap,
        &rhs,
        &values,
        &fixed,
        &mut residual,
        request,
        &mut budget,
    )
    .unwrap();
    assert!((residual_report.value - 1.0).abs() < 1e-9);
    assert!(residual_report.relative < 1e-5);
    let verdict = scale_aware_verdict(&residual_report, request, false);
    assert_eq!(verdict.status, ConvergenceStatus::Converged);
}

#[test]
fn oversized_dense_problem_is_rejected() {
    let mut absolute = SolverLimits::default();
    absolute.max_vertices = ABSOLUTE_DENSE_VERTEX_CAP;
    let err = admit_operator_size(ABSOLUTE_DENSE_VERTEX_CAP + 1, 1, absolute);
    assert!(matches!(
        err,
        Err(DdgProductionError::UnsupportedSize { vertices, limit })
            if vertices == ABSOLUTE_DENSE_VERTEX_CAP + 1 && limit == ABSOLUTE_DENSE_VERTEX_CAP
    ));
    let mut tight = SolverLimits::default();
    tight.max_vertices = 3;
    let cancel = Cancellation::new();
    let mut budget = budget(&cancel, u64::MAX);
    let mut vertices = [Point3::default(); 4];
    let mut triangles = [[0; 3]; 2];
    fill_unit_square(&mut vertices, &mut triangles).expect("square");
    let mut laplacian = [SparseEntry {
        row: 0,
        col: 0,
        value: 0.0,
    }; 18];
    let mut mass = [0.0; 4];
    let mut residual = [0.0; 4];
    let err = assemble_cotangent_admitted(
        &vertices,
        &triangles,
        OperatorScratch {
            laplacian: &mut laplacian,
            mass: &mut mass,
            residual: &mut residual,
        },
        definite_conventions(),
        tight,
        &mut budget,
    );
    assert!(matches!(err, Err(DdgProductionError::UnsupportedSize { .. })));
}

#[test]
fn ill_conditioned_diagonal_is_rejected() {
    let cancel = Cancellation::new();
    let mut budget = budget(&cancel, u64::MAX);
    let lap = [
        SparseEntry {
            row: 0,
            col: 0,
            value: 1.0e12,
        },
        SparseEntry {
            row: 1,
            col: 1,
            value: 1.0e-12,
        },
    ];
    let err = condition_estimate(2, &lap, &[false, false], &mut budget);
    assert!(matches!(
        err,
        Ok(est) if est > 1.0e20
    ));
    let mut values = [0.0, 0.0];
    let mut residual = [0.0; 2];
    let mut limits = SolverLimits::default();
    limits.max_condition_estimate = 1.0e8;
    let err = solve_poisson_instrumented(
        2,
        &lap,
        &[0.0, 0.0],
        &[false, false],
        &mut values,
        &mut residual,
        1,
        ResidualRequest::two_norm(),
        definite_conventions(),
        limits,
        &mut budget,
    );
    assert!(matches!(
        err,
        Err(DdgProductionError::IllConditioned { .. })
    ));
}

#[test]
fn neumann_load_not_orthogonal_to_constants_is_rejected() {
    let cancel = Cancellation::new();
    let mut budget = budget(&cancel, u64::MAX);
    let mut vertices = [Point3::default(); 4];
    let mut triangles = [[0; 3]; 2];
    let (n, t) = fill_unit_square(&mut vertices, &mut triangles).expect("square");
    let mut laplacian = [SparseEntry {
        row: 0,
        col: 0,
        value: 0.0,
    }; 18];
    let mut mass = [0.0; 4];
    let mut residual = [0.0; 4];
    let conventions = conventions_from_fixed(&[false; 4]);
    let assembled = assemble_cotangent_admitted(
        &vertices[..n],
        &triangles[..t],
        OperatorScratch {
            laplacian: &mut laplacian,
            mass: &mut mass,
            residual: &mut residual,
        },
        conventions,
        SolverLimits::default(),
        &mut budget,
    )
    .unwrap();
    let mut values = [0.0; 4];
    let err = solve_poisson_instrumented(
        n,
        &laplacian[..assembled.nnz],
        &[1.0, 0.0, 0.0, 0.0],
        &[false; 4],
        &mut values,
        &mut residual,
        4,
        ResidualRequest::two_norm(),
        conventions,
        SolverLimits::default(),
        &mut budget,
    );
    assert!(matches!(err, Err(DdgProductionError::SingularNullspace)));
}

#[test]
fn explicit_heat_route_stability_domain() {
    let cancel = Cancellation::new();
    let mut budget = budget(&cancel, u64::MAX);
    let mut vertices = [Point3::default(); 4];
    let mut triangles = [[0; 3]; 2];
    let (n, t) = fill_unit_square(&mut vertices, &mut triangles).expect("square");
    let mut laplacian = [SparseEntry {
        row: 0,
        col: 0,
        value: 0.0,
    }; 18];
    let mut mass = [0.0; 4];
    let mut residual = [0.0; 4];
    let assembled = assemble_cotangent_admitted(
        &vertices[..n],
        &triangles[..t],
        OperatorScratch {
            laplacian: &mut laplacian,
            mass: &mut mass,
            residual: &mut residual,
        },
        definite_conventions(),
        SolverLimits::default(),
        &mut budget,
    )
    .unwrap();
    let domain = heat_route_stability(
        n,
        &laplacian[..assembled.nnz],
        &mass,
        1.0e-4,
        HEAT_GEODESICS_EXPLICIT_STEPS,
        16,
        SolverLimits::default(),
        &mut budget,
    )
    .unwrap();
    assert_eq!(domain.integrator, HeatTimeIntegrator::ExplicitEuler);
    assert_eq!(domain.poisson, HeatPoissonSolver::BoundedJacobi);
    assert!(domain.numerically_stable);
    assert!(domain.within_declared_route);
    let mut values = [7.0; 4];
    heat_step(n, &laplacian[..assembled.nnz], &mass, domain.dt, &mut values).unwrap();
    assert!(values.iter().all(|v| (*v - 7.0).abs() < 1e-12));
    let unstable = heat_route_stability(
        n,
        &laplacian[..assembled.nnz],
        &mass,
        1.0e6,
        HEAT_GEODESICS_EXPLICIT_STEPS,
        16,
        SolverLimits::default(),
        &mut budget,
    );
    assert!(matches!(
        unstable,
        Err(DdgProductionError::UnstableHeatRoute { .. })
    ));
    let too_many_steps = heat_route_stability(
        n,
        &laplacian[..assembled.nnz],
        &mass,
        1.0e-4,
        HEAT_GEODESICS_EXPLICIT_STEPS + 1,
        16,
        SolverLimits::default(),
        &mut budget,
    );
    assert!(matches!(
        too_many_steps,
        Err(DdgProductionError::UnsupportedSize { .. })
    ));
}

#[test]
fn dirichlet_rows_do_not_enter_the_algebraic_residual() {
    let cancel = Cancellation::new();
    let mut budget = budget(&cancel, u64::MAX);
    let lap = spd2();
    let rhs = [100.0, 0.0];
    let fixed = [true, false];
    let values = [5.0, 0.0];
    let mut residual = [0.0; 2];
    let report = algebraic_residual(
        2,
        &lap,
        &rhs,
        &values,
        &fixed,
        &mut residual,
        ResidualRequest::inf_norm(),
        &mut budget,
    )
    .unwrap();
    assert_eq!(residual[0], 0.0);
    assert_eq!(report.free_count, 1);
    // Free residual is |0 - (L x)_1| = |0 - (-1*5 + 1*0)| = 5.
    assert!((report.value - 5.0).abs() < 1e-12);
}

#[test]
fn cancelled_budget_fails_closed() {
    let cancel = Cancellation::new();
    cancel.cancel();
    let err = ProductionBudget::admit(CapacityPlan::default(), usize::MAX, 8, &cancel);
    assert!(matches!(
        err,
        Err(super::super::geometry_workspace::production_budget::BudgetError::Cancelled)
    ));
}

#[test]
fn capacity_plan_and_heat_dt_helpers_are_finite() {
    let plan = operator_capacity_plan(4, 2, 18).unwrap();
    assert!(plan.total_bytes().unwrap() > 0);
    assert_eq!(heat_geodesics_dt(1.0, 1.0), Some(1.0));
    assert_eq!(recommended_explicit_dt(4.0), Some(0.5));
}

#[test]
fn two_norm_and_inf_norm_are_declared_distinct() {
    let cancel = Cancellation::new();
    let mut budget = budget(&cancel, u64::MAX);
    let lap = spd2();
    let rhs = [0.0, 0.0];
    let fixed = [false, false];
    let values = [1.0, 0.0];
    let mut residual = [0.0; 2];
    let two = algebraic_residual(
        2,
        &lap,
        &rhs,
        &values,
        &fixed,
        &mut residual,
        ResidualRequest::two_norm(),
        &mut budget,
    )
    .unwrap();
    let inf = algebraic_residual(
        2,
        &lap,
        &rhs,
        &values,
        &fixed,
        &mut residual,
        ResidualRequest::inf_norm(),
        &mut budget,
    )
    .unwrap();
    assert_eq!(two.norm, ResidualNorm::EuclideanTwo);
    assert_eq!(inf.norm, ResidualNorm::Infinity);
    // Ax = [4, -1], r = [-4, 1]; ||r||_2 = √17, ||r||_∞ = 4.
    assert!((inf.value - 4.0).abs() < 1e-12);
    assert!((two.value - 17.0_f64.sqrt()).abs() < 1e-12);
}
