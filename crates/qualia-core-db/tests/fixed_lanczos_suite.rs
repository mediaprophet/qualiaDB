use qualia_core_db::solvers::linear_algebra::{FixedLanczosEigensolver, Matrix4x4};
use qualia_core_db::solvers::SolverConfig;

#[test]
fn test_lanczos_convergence() {
    let mut solver = FixedLanczosEigensolver::new(SolverConfig::default());

    // A symmetric, positive-definite matrix
    // Eigenvalues should be approximately real and positive.
    let matrix = Matrix4x4 {
        data: [
            [4.0, 1.0, 0.0, 0.0],
            [1.0, 4.0, 1.0, 0.0],
            [0.0, 1.0, 4.0, 1.0],
            [0.0, 0.0, 1.0, 4.0],
        ],
    };

    let result = solver.solve_smallest_eigenvector(&matrix);
    assert!(
        result.is_ok(),
        "Solver should converge on a positive definite matrix"
    );

    let vec = result.unwrap();
    // Normalization check
    let norm_sq = vec.data[0] * vec.data[0]
        + vec.data[1] * vec.data[1]
        + vec.data[2] * vec.data[2]
        + vec.data[3] * vec.data[3];
    assert!(
        (norm_sq - 1.0).abs() < 1e-4,
        "Eigenvector should be normalized, got {}",
        norm_sq
    );

    // Check that Ax = lambda x for smallest lambda
    // We don't explicitly know lambda, but we can verify Rayleigh quotient is an eigenvalue.
    let mut ax = [0.0; 4];
    for i in 0..4 {
        for j in 0..4 {
            ax[i] += matrix.data[i][j] * vec.data[j];
        }
    }

    // lambda = x^T A x
    let mut lambda = 0.0;
    for i in 0..4 {
        lambda += vec.data[i] * ax[i];
    }

    // Smallest eigenvalue for tridiag(1, 4, 1) is 4 - 2 cos(pi/5) ~ 2.381966
    assert!(
        (lambda - 2.381966).abs() < 1e-2,
        "Smallest eigenvalue should be ~2.382, got {}",
        lambda
    );

    // Ensure Ax ≈ lambda x
    for i in 0..4 {
        assert!(
            (ax[i] - lambda * vec.data[i]).abs() < 1e-2,
            "Ax != lambda x at index {}",
            i
        );
    }
}

#[test]
fn test_lanczos_eigenvalues() {
    let mut solver = FixedLanczosEigensolver::new(SolverConfig::default());

    let matrix = Matrix4x4 {
        data: [
            [4.0, 1.0, 0.0, 0.0],
            [1.0, 4.0, 1.0, 0.0],
            [0.0, 1.0, 4.0, 1.0],
            [0.0, 0.0, 1.0, 4.0],
        ],
    };

    let result = solver.find_lowest_eigenvalues(&matrix, 2);
    assert!(result.is_ok(), "Should find lowest eigenvalues");

    let eigs = result.unwrap();
    // The eigenvalues of this matrix are approx: 2.382, 3.382, 4.618, 5.618
    assert!(
        (eigs[0] - 2.381966).abs() < 1e-2,
        "First eigenvalue should be ~2.382, got {}",
        eigs[0]
    );
    assert!(
        (eigs[1] - 3.381966).abs() < 1e-2,
        "Second eigenvalue should be ~3.382, got {}",
        eigs[1]
    );
}
