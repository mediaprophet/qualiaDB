//! 3D Helmholtz Frequency-Domain Solver & Green's Function Oracle Facade.
//!
//! Re-exports the canonical 3D Helmholtz frequency-domain operator, 7-point and 27-point isotropic
//! dispersion-minimizing stencils, zero-allocation damped Jacobi and BiCGSTAB solvers, free-space
//! 3D Green's function $G(r) = \frac{e^{ikr}}{4\pi r}$, and rectangular cavity eigenmode oracles
//! from [`etalon_sim::acoustics_continuum::helmholtz`].

pub use etalon_sim::acoustics_continuum::helmholtz::{
    apply_helmholtz_operator, helmholtz_greens_function_3d,
    rectangular_cavity_eigenfrequency_hz, rectangular_cavity_eigenmode_value,
    solve_helmholtz_bicgstab, step_helmholtz_jacobi, Complex64, HelmholtzBoundary,
    HelmholtzConfig, HelmholtzSolveReport, HelmholtzStencil, HelmholtzWorkspace,
};
