//! Part of poet browser toolbox registration — clinical labs + Live LinearAlgebra + Physics.

use super::*;

fn sci_live_tool(
    id: &'static str,
    label: &'static str,
    scope: &'static str,
    description: &'static str,
) -> Box<dyn crate::tool_chest::core::tool::Tool> {
    Box::new(SimpleTool::new(
        ToolMetadata {
            id: id.into(),
            label: label.into(),
            icon: "lab".into(),
            kind: ToolKind::RunAction,
            capability_scope: Some(scope.into()),
            ontology_prefix: "sci".into(),
            description: description.into(),
        },
        ActionType::Invoke,
    ))
}

pub(super) fn register_scientific_toolbox(reg: &mut Registry) {
    let lab_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:place_health".into(),
                label: "+ Health & Clinical Node".into(),
                icon: "health".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "sci".into(),
                description: "Place the clinical workbench.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:place_3d".into(),
                label: "+ Molecular 3D Viewer".into(),
                icon: "3d".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "sci".into(),
                description: "Place the available 3D scientific viewer.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:thermodynamics".into(),
                label: "Thermodynamics MCMC".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: None,
                ontology_prefix: "sci".into(),
                description: "Run the bounded thermodynamics capability.".into(),
            },
            ActionType::Invoke,
        )),
    ];

    let linalg_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_matmul".into(),
                label: "Matrix multiply".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.matmul".into()),
                ontology_prefix: "sci".into(),
                description: "Multiply two row-major matrices from the selected surface."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_matvec".into(),
                label: "Matrix × vector".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.matvec".into()),
                ontology_prefix: "sci".into(),
                description: "Multiply a row-major matrix by a vector on this surface."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_transpose".into(),
                label: "Transpose".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.transpose".into()),
                ontology_prefix: "sci".into(),
                description: "Transpose a row-major matrix from the selected surface."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_determinant".into(),
                label: "Determinant".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.determinant".into()),
                ontology_prefix: "sci".into(),
                description: "Determinant of a square matrix on the selected surface."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_solve".into(),
                label: "Solve Ax=b".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.solve".into()),
                ontology_prefix: "sci".into(),
                description: "Solve a square linear system from matrix A then vector b."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_scale".into(),
                label: "Scale vector".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.scale".into()),
                ontology_prefix: "sci".into(),
                description: "Scale surface numbers by data-s (default 2)."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_add_into".into(),
                label: "Add vectors".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.add_into".into()),
                ontology_prefix: "sci".into(),
                description: "Element-wise sum of two equal-length halves on this surface."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_axpy".into(),
                label: "AXPY".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.axpy".into()),
                ontology_prefix: "sci".into(),
                description: "BLAS axpy: y += α·x from equal halves and data-alpha."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_hadamard_into".into(),
                label: "Hadamard product".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.hadamard_into".into()),
                ontology_prefix: "sci".into(),
                description: "Element-wise product of two equal-length halves."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_qr_factor".into(),
                label: "QR factor".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.qr_factor".into()),
                ontology_prefix: "sci".into(),
                description: "Householder QR factor of a tall-or-square matrix."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_cholesky_factor".into(),
                label: "Cholesky factor".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.cholesky_factor".into()),
                ontology_prefix: "sci".into(),
                description: "Cholesky factor of a small SPD square matrix.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_cholesky_solve".into(),
                label: "Cholesky solve".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.cholesky_solve".into()),
                ontology_prefix: "sci".into(),
                description: "Solve Ax=b via Cholesky after factoring SPD A.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_lu_decompose".into(),
                label: "LU decompose".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.lu_decompose".into()),
                ontology_prefix: "sci".into(),
                description: "LU factorization with partial pivoting.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_lu_solve".into(),
                label: "LU solve".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.lu_solve".into()),
                ontology_prefix: "sci".into(),
                description: "Solve Ax=b using LU decomposition.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_qr_form_q".into(),
                label: "QR form Q".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.qr_form_q".into()),
                ontology_prefix: "sci".into(),
                description: "Form thin Q from a Householder QR factor.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_qr_solve_ls".into(),
                label: "QR least squares".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.qr_solve_least_squares".into()),
                ontology_prefix: "sci".into(),
                description: "Least-squares solve via factored QR.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_svd".into(),
                label: "SVD".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.svd".into()),
                ontology_prefix: "sci".into(),
                description: "Thin singular value decomposition of a matrix.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_eigenvalues".into(),
                label: "Eigenvalues".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.eigenvalues".into()),
                ontology_prefix: "sci".into(),
                description: "Eigenvalues of a square matrix (complex pairs).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_add_assign".into(),
                label: "Add-assign".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.add_assign".into()),
                ontology_prefix: "sci".into(),
                description: "In-place element-wise sum of two equal-length halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_hadamard_assign".into(),
                label: "Hadamard-assign".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.hadamard_assign".into()),
                ontology_prefix: "sci".into(),
                description: "In-place element-wise product of two equal-length halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_cholesky_det".into(),
                label: "Cholesky det".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.cholesky_determinant".into()),
                ontology_prefix: "sci".into(),
                description: "Determinant via Cholesky factor of an SPD matrix.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_charpoly".into(),
                label: "Char poly".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.characteristic_polynomial".into()),
                ontology_prefix: "sci".into(),
                description: "Characteristic polynomial coefficients of a square matrix.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_eigenvalues_general".into(),
                label: "General eigenvalues".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.eigenvalues_general".into()),
                ontology_prefix: "sci".into(),
                description: "General-matrix eigenvalues as complex pairs.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_eigen_symmetric".into(),
                label: "Symmetric eigen".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.eigen_symmetric".into()),
                ontology_prefix: "sci".into(),
                description: "Symmetric eigendecomposition (Jacobi) of a square matrix.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_polynomial_roots".into(),
                label: "Poly roots".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.polynomial_roots".into()),
                ontology_prefix: "sci".into(),
                description: "Complex roots of a real polynomial (descending coeffs).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_solve_linear_system".into(),
                label: "Solve system".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.solve_linear_system".into()),
                ontology_prefix: "sci".into(),
                description: "Gaussian elimination solve with partial pivoting.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:la_symmetric_eigen_3x3".into(),
                label: "Sym eigen 3×3".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("LinearAlgebra.symmetric_eigen_3x3".into()),
                ontology_prefix: "sci".into(),
                description: "Closed-form symmetric 3×3 eigenvalues (Smith).".into(),
            },
            ActionType::Invoke,
        )),
    ];

    let physics_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:phys_doppler".into(),
                label: "Doppler shift".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Physics.doppler_shift".into()),
                ontology_prefix: "sci".into(),
                description: "Relativistic Doppler shift from frequency and relative velocity."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:phys_emf_attenuation".into(),
                label: "EMF attenuation".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Physics.emf_attenuation".into()),
                ontology_prefix: "sci".into(),
                description: "Inverse-square EMF attenuation with optional absorption."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:phys_harmonic".into(),
                label: "Harmonic oscillator".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Physics.harmonic_oscillator".into()),
                ontology_prefix: "sci".into(),
                description: "Spring–mass oscillator trajectory (symplectic sketch / Host)."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:phys_pendulum".into(),
                label: "Pendulum".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Physics.pendulum".into()),
                ontology_prefix: "sci".into(),
                description: "Nonlinear pendulum angle trajectory from length and g."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:phys_logistic".into(),
                label: "Logistic growth".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Physics.logistic_growth".into()),
                ontology_prefix: "sci".into(),
                description: "Logistic population growth toward a carrying capacity."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:phys_cfd_step".into(),
                label: "CFD residual".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Physics.cfd_step".into()),
                ontology_prefix: "sci".into(),
                description: "Burgers steady-state residual of a 1D velocity field."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:phys_heat_1d".into(),
                label: "Heat diffusion 1D".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Physics.heat_diffusion_1d".into()),
                ontology_prefix: "sci".into(),
                description: "1D heat diffusion on a temperature field from this surface."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:phys_wave_1d".into(),
                label: "Wave equation 1D".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Physics.wave_1d".into()),
                ontology_prefix: "sci".into(),
                description: "1D scalar wave evolution from displacement (and optional velocity)."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:phys_advection_1d".into(),
                label: "Advection–diffusion 1D".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Physics.advection_diffusion_1d".into()),
                ontology_prefix: "sci".into(),
                description: "1D advection–diffusion on a scalar field from this surface."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:phys_quantum_1d".into(),
                label: "Quantum states 1D".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Physics.quantum_states_1d".into()),
                ontology_prefix: "sci".into(),
                description: "Classical TISE eigenproblem on a 1D potential (natural units)."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:phys_n_body".into(),
                label: "N-body gravity".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Physics.n_body".into()),
                ontology_prefix: "sci".into(),
                description: "2D Newtonian N-body gravitation (direct sum).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:phys_molecular_dynamics".into(),
                label: "Molecular dynamics".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Physics.molecular_dynamics".into()),
                ontology_prefix: "sci".into(),
                description: "2D Lennard-Jones particles (velocity-Verlet).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:phys_emf_interference".into(),
                label: "EMF interference".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Physics.emf_interference".into()),
                ontology_prefix: "sci".into(),
                description: "Superpose N EMF sources at a 3D observation point.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:phys_emf_field_grid".into(),
                label: "EMF field grid 3D".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Physics.emf_field_grid_3d".into()),
                ontology_prefix: "sci".into(),
                description: "4D EMF physics grid (x×y×z×t) with manifold tags.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:phys_emf_sample_depth".into(),
                label: "EMF sample at depth".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Physics.emf_sample_at_depth".into()),
                ontology_prefix: "sci".into(),
                description: "Depth-aware EMF sampling along a camera ray.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:phys_field_sample".into(),
                label: "Field sample".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Physics.field_sample".into()),
                ontology_prefix: "sci".into(),
                description: "Sample an ambient field at a 3D position.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:phys_material_query".into(),
                label: "Material query".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Physics.material_query".into()),
                ontology_prefix: "sci".into(),
                description: "Faceted signature traits for a physical material.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:phys_evaluate_interaction".into(),
                label: "Evaluate interaction".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Physics.evaluate_interaction".into()),
                ontology_prefix: "sci".into(),
                description: "Apply field interaction laws to a continuant.".into(),
            },
            ActionType::Invoke,
        )),
    ];

    let cosmic_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_geodetic_distance".into(),
                label: "Geodetic distance".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.geodetic_distance".into()),
                ontology_prefix: "sci".into(),
                description: "Great-circle distance between two geodetic points (metres)."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_surface_gravity".into(),
                label: "Surface gravity".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.surface_gravity".into()),
                ontology_prefix: "sci".into(),
                description: "Surface gravity for a named celestial body (m/s²).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_flrw_distance".into(),
                label: "FLRW distance".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.flrw_distance".into()),
                ontology_prefix: "sci".into(),
                description: "Low-z FLRW comoving distance from redshift z.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_flrw_redshift".into(),
                label: "FLRW redshift".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.flrw_redshift".into()),
                ontology_prefix: "sci".into(),
                description: "Cosmological redshift from emit-scale factor a_emit.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_flrw_hubble".into(),
                label: "Hubble velocity".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.flrw_hubble_velocity".into()),
                ontology_prefix: "sci".into(),
                description: "Hubble recession velocity for a distance (m/s).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_warp_velocity".into(),
                label: "Warp velocity".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.warp_velocity".into()),
                ontology_prefix: "sci".into(),
                description: "Warp-factor velocity (TOS/TNG scales) in m/s.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_warp_factor_c".into(),
                label: "Warp factor c".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.warp_factor_c".into()),
                ontology_prefix: "sci".into(),
                description: "Dimensionless warp velocity in units of c.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_typical_length".into(),
                label: "Typical length".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.typical_length".into()),
                ontology_prefix: "sci".into(),
                description: "Typical length scale for hierarchy level L-2…L12.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_observe_redshift".into(),
                label: "Observe redshift".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.observe_redshift".into()),
                ontology_prefix: "sci".into(),
                description: "FLRW observation record from measured redshift z.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_compton".into(),
                label: "Compton wavelength".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.compton_wavelength".into()),
                ontology_prefix: "sci".into(),
                description: "Compton wavelength for electron or proton.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_de_broglie".into(),
                label: "de Broglie wavelength".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.de_broglie_wavelength".into()),
                ontology_prefix: "sci".into(),
                description: "de Broglie wavelength from particle and velocity.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_atm_pressure".into(),
                label: "Atmosphere pressure".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.atmosphere_pressure".into()),
                ontology_prefix: "sci".into(),
                description: "Atmospheric pressure at altitude for earth/mars/venus."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_geodetic_to_ecef".into(),
                label: "Geodetic to ECEF".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.geodetic_to_ecef".into()),
                ontology_prefix: "sci".into(),
                description: "WGS84 geodetic latitude/longitude/alt → ECEF metres.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_ecef_to_geodetic".into(),
                label: "ECEF to geodetic".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.ecef_to_geodetic".into()),
                ontology_prefix: "sci".into(),
                description: "ECEF metres → WGS84 latitude/longitude/altitude.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_ecef_to_enu".into(),
                label: "ECEF to ENU".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.ecef_to_enu".into()),
                ontology_prefix: "sci".into(),
                description: "ECEF to local East-North-Up about a reference geodetic."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_enu_to_ecef".into(),
                label: "ENU to ECEF".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.enu_to_ecef".into()),
                ontology_prefix: "sci".into(),
                description: "Local East-North-Up to ECEF about a reference geodetic."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_body_profile".into(),
                label: "Body profile".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.body_profile".into()),
                ontology_prefix: "sci".into(),
                description: "Named celestial body radius, mass, and class profile.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_stardate".into(),
                label: "Stardate to Gregorian".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.stardate_to_gregorian".into()),
                ontology_prefix: "sci".into(),
                description: "Convert a stardate to approximate Gregorian year.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_cochrane".into(),
                label: "Cochrane units".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.cochrane_units".into()),
                ontology_prefix: "sci".into(),
                description: "Warp-field Cochrane units (dimensionless v/c).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_atm_temperature".into(),
                label: "Atmosphere temperature".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.atmosphere_temperature".into()),
                ontology_prefix: "sci".into(),
                description: "Atmospheric temperature at altitude for earth/mars/venus."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_magnetosphere".into(),
                label: "Magnetosphere field".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.magnetosphere_field".into()),
                ontology_prefix: "sci".into(),
                description: "Dipole magnetosphere field strength at distance (Tesla)."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_scale_factor".into(),
                label: "Scale factor".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.scale_factor".into()),
                ontology_prefix: "sci".into(),
                description: "Hierarchy scale factor between two levels L-2…L12.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cosmic_usri_parse".into(),
                label: "Parse USRI".into(),
                icon: "physics".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Cosmic.usri_parse".into()),
                ontology_prefix: "sci".into(),
                description: "Parse a Universal Spacetime & Reality Identifier (USRI)."
                    .into(),
            },
            ActionType::Invoke,
        )),
    ];

    let sf_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:sf_airy_ai".into(),
                label: "Airy Ai".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("SpecialFunctions.airy_ai".into()),
                ontology_prefix: "sci".into(),
                description: "Airy Ai(x) from data-x or one surface number.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:sf_airy_bi".into(),
                label: "Airy Bi".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("SpecialFunctions.airy_bi".into()),
                ontology_prefix: "sci".into(),
                description: "Airy Bi(x) from data-x or one surface number.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:sf_zeta".into(),
                label: "Riemann zeta".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("SpecialFunctions.zeta".into()),
                ontology_prefix: "sci".into(),
                description: "ζ(s) for s>1 from data-s or one surface number.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:sf_legendre".into(),
                label: "Legendre P_n".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("SpecialFunctions.legendre".into()),
                ontology_prefix: "sci".into(),
                description: "Legendre P_n(x) from data-n/data-x or two numbers.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:sf_chebyshev_t".into(),
                label: "Chebyshev T_n".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("SpecialFunctions.chebyshev_t".into()),
                ontology_prefix: "sci".into(),
                description: "Chebyshev T_n(x) from data-n/data-x or two numbers.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:sf_chebyshev_u".into(),
                label: "Chebyshev U_n".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("SpecialFunctions.chebyshev_u".into()),
                ontology_prefix: "sci".into(),
                description: "Chebyshev U_n(x) from data-n/data-x or two numbers.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:sf_hermite".into(),
                label: "Hermite H_n".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("SpecialFunctions.hermite".into()),
                ontology_prefix: "sci".into(),
                description: "Physicists' Hermite H_n(x) from data-n/data-x.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:sf_laguerre".into(),
                label: "Laguerre L_n".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("SpecialFunctions.laguerre".into()),
                ontology_prefix: "sci".into(),
                description: "Laguerre L_n(x) from data-n/data-x or two numbers.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:sf_bessel_j".into(),
                label: "Bessel J_n".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("SpecialFunctions.bessel_j".into()),
                ontology_prefix: "sci".into(),
                description: "Bessel J_n(x) from data-n/data-x or two numbers.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:sf_bessel_i".into(),
                label: "Bessel I_n".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("SpecialFunctions.bessel_i".into()),
                ontology_prefix: "sci".into(),
                description: "Modified Bessel I_n(x) from data-n/data-x.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:sf_bessel_y".into(),
                label: "Bessel Y_n".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("SpecialFunctions.bessel_y".into()),
                ontology_prefix: "sci".into(),
                description: "Bessel Y_n(x) for x>0 from data-n/data-x.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:sf_bessel_k".into(),
                label: "Bessel K_n".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("SpecialFunctions.bessel_k".into()),
                ontology_prefix: "sci".into(),
                description: "Modified Bessel K_n(x) for x>0 from data-n/data-x.".into(),
            },
            ActionType::Invoke,
        )),
    ];

    let xform_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:xform_dft".into(),
                label: "DFT (real)".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("IntegralTransforms.dft".into()),
                ontology_prefix: "sci".into(),
                description: "Forward DFT of a real signal from surface numbers.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:xform_dft_complex".into(),
                label: "DFT (complex)".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("IntegralTransforms.dft_complex".into()),
                ontology_prefix: "sci".into(),
                description: "Forward DFT of [re,im] sample pairs.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:xform_idft".into(),
                label: "IDFT".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("IntegralTransforms.idft".into()),
                ontology_prefix: "sci".into(),
                description: "Inverse DFT from [re,im] spectrum pairs.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:xform_z_transform_finite".into(),
                label: "Z-transform (finite)".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("IntegralTransforms.z_transform_finite".into()),
                ontology_prefix: "sci".into(),
                description: "Finite-sequence Z-transform at complex z.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:xform_unit_step_z".into(),
                label: "Unit-step Z".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("IntegralTransforms.unit_step_z".into()),
                ontology_prefix: "sci".into(),
                description: "Closed-form Z{u[n]} at complex z (|z|>1).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:xform_geometric_z".into(),
                label: "Geometric Z".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("IntegralTransforms.geometric_z".into()),
                ontology_prefix: "sci".into(),
                description: "Closed-form Z{a^n} at complex z.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:xform_laplace_numeric".into(),
                label: "Laplace (numeric)".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("IntegralTransforms.laplace_numeric".into()),
                ontology_prefix: "sci".into(),
                description: "Numerical Laplace transform of expr at s.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:xform_laplace_symbolic".into(),
                label: "Laplace (symbolic)".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("IntegralTransforms.laplace_symbolic".into()),
                ontology_prefix: "sci".into(),
                description: "Table Laplace transform of a simple expr.".into(),
            },
            ActionType::Invoke,
        )),
    ];

    let calc_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_hermite_dense".into(),
                label: "Hermite dense output".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.hermite_dense_output".into()),
                ontology_prefix: "sci".into(),
                description: "Cubic Hermite state at θ∈[0,1] from endpoint values and slopes."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_bdf1".into(),
                label: "BDF1 step".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.bdf1_step".into()),
                ontology_prefix: "sci".into(),
                description: "Implicit Euler step (power-law RHS; default decay −y).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_bdf2".into(),
                label: "BDF2 step".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.bdf2_step".into()),
                ontology_prefix: "sci".into(),
                description: "BDF2 stiff step from two prior states.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_verlet".into(),
                label: "Verlet step".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.verlet_step".into()),
                ontology_prefix: "sci".into(),
                description: "Störmer–Verlet step for a harmonic oscillator.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_ruth3".into(),
                label: "Ruth3 step".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.ruth3_step".into()),
                ontology_prefix: "sci".into(),
                description: "Ruth 3rd-order symplectic step (harmonic defaults).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_yoshida4".into(),
                label: "Yoshida4 step".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.yoshida4_step".into()),
                ontology_prefix: "sci".into(),
                description: "Yoshida 4th-order symplectic step (harmonic defaults).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_integrate_bdf".into(),
                label: "Integrate BDF".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.integrate_bdf".into()),
                ontology_prefix: "sci".into(),
                description: "BDF2 integration over steps (power-law RHS; default −y)."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_integrate_sens".into(),
                label: "Integrate + sensitivity".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.integrate_with_sensitivity".into()),
                ontology_prefix: "sci".into(),
                description: "RK4 state and ∂y/∂y₀ for a power-law ODE.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_invariant_drift".into(),
                label: "Invariant drift".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.invariant_drift".into()),
                ontology_prefix: "sci".into(),
                description: "Absolute and relative drift of a conserved quantity.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_perm_parity".into(),
                label: "Permutation parity".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.permutation_parity".into()),
                ontology_prefix: "sci".into(),
                description: "Even (+1) or odd (−1) parity of a permutation list.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_pack_f32".into(),
                label: "Pack f32 pair".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.pack_f32_pair".into()),
                ontology_prefix: "sci".into(),
                description: "Pack step and compensation floats into one u64.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_unpack_f32".into(),
                label: "Unpack f32 pair".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.unpack_f32_pair".into()),
                ontology_prefix: "sci".into(),
                description: "Unpack a u64 back into step and compensation floats.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_poisson_bracket".into(),
                label: "Poisson bracket".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.canonical_poisson_bracket".into()),
                ontology_prefix: "sci".into(),
                description: "Canonical Poisson bracket from 2-D ∂f/∂q,∂f/∂p,∂g/∂q,∂g/∂p."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_stormer_verlet".into(),
                label: "Störmer–Verlet step".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.stormer_verlet_step".into()),
                ontology_prefix: "sci".into(),
                description: "Störmer–Verlet harmonic step (q, p, h; optional k, mass).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_gauss_kronrod".into(),
                label: "Gauss–Kronrod 15".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.adaptive_gauss_kronrod_15".into()),
                ontology_prefix: "sci".into(),
                description: "Adaptive G7-K15 quadrature of data-expr over [a,b].".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_jvp".into(),
                label: "JVP (2×2)".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.jvp".into()),
                ontology_prefix: "sci".into(),
                description: "Jacobian-vector product for a fixed 2×2 linear map.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_vjp".into(),
                label: "VJP (2×2)".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.vjp".into()),
                ontology_prefix: "sci".into(),
                description: "Vector-Jacobian product for a fixed 2×2 linear map.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_adaptive_simpson".into(),
                label: "Adaptive Simpson".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.adaptive_simpson".into()),
                ontology_prefix: "sci".into(),
                description: "Adaptive Simpson quadrature of data-expr over [a,b].".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_adaptive_deriv".into(),
                label: "Adaptive derivative".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.adaptive_derivative".into()),
                ontology_prefix: "sci".into(),
                description: "Adaptive central difference of data-expr at x.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_newton_solve".into(),
                label: "Newton solve".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.newton_solve".into()),
                ontology_prefix: "sci".into(),
                description: "Newton–Raphson root finder from data-exprs, vars, and guess."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_num_jacobian".into(),
                label: "Numerical Jacobian".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.numerical_jacobian".into()),
                ontology_prefix: "sci".into(),
                description: "Finite-difference Jacobian of data-exprs at a point.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:calc_num_hessian".into(),
                label: "Numerical Hessian".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Calculus.numerical_hessian".into()),
                ontology_prefix: "sci".into(),
                description: "Finite-difference Hessian of data-expr at a point.".into(),
            },
            ActionType::Invoke,
        )),
    ];

    let cg_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_distance_2d".into(),
                label: "Distance 2D".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.distance_2d".into()),
                ontology_prefix: "sci".into(),
                description: "Euclidean distance between two 2D points (ax ay bx by).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_distance_3d".into(),
                label: "Distance 3D".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.distance_3d".into()),
                ontology_prefix: "sci".into(),
                description: "Euclidean distance between two 3D points.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_point_segment_2d".into(),
                label: "Point–segment 2D".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.point_segment_distance_2d".into()),
                ontology_prefix: "sci".into(),
                description: "Distance from a 2D point to a line segment.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_orientation_2".into(),
                label: "Orientation 2D".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.orientation_2".into()),
                ontology_prefix: "sci".into(),
                description: "Orientation of three 2D points (CCW / CW / collinear).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_orient_3d".into(),
                label: "Orient 3D".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.orient_3d".into()),
                ontology_prefix: "sci".into(),
                description: "Signed orientation of a 3D tetrahedron.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_morton_encode_2d".into(),
                label: "Morton encode 2D".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.morton_encode_2d".into()),
                ontology_prefix: "sci".into(),
                description: "Morton (Z-order) code from integer x,y.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_morton_decode_2d".into(),
                label: "Morton decode 2D".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.morton_decode_2d".into()),
                ontology_prefix: "sci".into(),
                description: "Decode a 2D Morton code to integer x,y.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_morton_encode_3d".into(),
                label: "Morton encode 3D".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.morton_encode_3d".into()),
                ontology_prefix: "sci".into(),
                description: "Morton code from integer x,y,z.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_hilbert_encode_2d".into(),
                label: "Hilbert encode 2D".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.hilbert_encode_2d".into()),
                ontology_prefix: "sci".into(),
                description: "Hilbert curve code from integer x,y.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_circumcenter".into(),
                label: "Circumcenter".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.circumcenter".into()),
                ontology_prefix: "sci".into(),
                description: "Circumcenter of a 2D triangle from three points.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_point_segment_3d".into(),
                label: "Point–segment 3D".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.point_segment_distance_3d".into()),
                ontology_prefix: "sci".into(),
                description: "Distance from a 3D point to a line segment.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_point_triangle_3d".into(),
                label: "Point–triangle 3D".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.point_triangle_distance_3d".into()),
                ontology_prefix: "sci".into(),
                description: "Squared distance from a 3D point to a triangle.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_convex_hull_2".into(),
                label: "Convex hull 2D".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.convex_hull_2".into()),
                ontology_prefix: "sci".into(),
                description: "2D convex hull from flat x,y point pairs.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_triangulate".into(),
                label: "Triangulate polygon".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.triangulate_polygon".into()),
                ontology_prefix: "sci".into(),
                description: "Ear-clip triangulation of a 2D polygon.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_surface_area".into(),
                label: "Surface area".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.surface_area".into()),
                ontology_prefix: "sci".into(),
                description: "Triangle-mesh surface area from vertices and indices.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_signed_volume".into(),
                label: "Signed volume".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.signed_volume".into()),
                ontology_prefix: "sci".into(),
                description: "Signed volume of a closed triangle mesh.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_segment_intersect_2".into(),
                label: "Segment ∩ 2D".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.line_segment_intersection_2".into()),
                ontology_prefix: "sci".into(),
                description: "Intersection of two 2D line segments.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_bezier_eval".into(),
                label: "Bézier eval".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.bezier_eval".into()),
                ontology_prefix: "sci".into(),
                description: "Evaluate a 3D Bézier curve at parameter t.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:cg_nearest_site".into(),
                label: "Nearest site".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputationalGeometry.nearest_site_brute_force".into()),
                ontology_prefix: "sci".into(),
                description: "Nearest Voronoi site by brute-force distance.".into(),
            },
            ActionType::Invoke,
        )),
    ];

    let eng_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:eng_natural_freq".into(),
                label: "SDOF natural frequency".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("EngineeringAnalysis.natural_frequency_sdof".into()),
                ontology_prefix: "sci".into(),
                description: "Undamped SDOF ωₙ = √(k/m) from stiffness and mass.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:eng_harmonic_sdof".into(),
                label: "Harmonic SDOF FRF".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("EngineeringAnalysis.analyze_harmonic_sdof".into()),
                ontology_prefix: "sci".into(),
                description: "Forced SDOF amplitude and phase at excitation frequencies."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:eng_euler".into(),
                label: "Euler buckling".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("EngineeringAnalysis.analyze_euler".into()),
                ontology_prefix: "sci".into(),
                description: "Euler column critical loads P_cr,n for modes 1..=N.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:eng_reliability".into(),
                label: "Reliability index β".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("EngineeringAnalysis.compute_reliability_index".into()),
                ontology_prefix: "sci".into(),
                description: "β = −Φ⁻¹(p_f) from a failure probability.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:eng_kinematics".into(),
                label: "Kinematics".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("EngineeringAnalysis.kinematics".into()),
                ontology_prefix: "sci".into(),
                description: "Constant-acceleration positions and velocities over times."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:eng_cauchy".into(),
                label: "Cauchy stress".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("EngineeringAnalysis.cauchy_stress".into()),
                ontology_prefix: "sci".into(),
                description: "von Mises / principals from a 3×3 row-major stress tensor."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:eng_drag".into(),
                label: "Drag force".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("EngineeringAnalysis.drag_force".into()),
                ontology_prefix: "sci".into(),
                description: "Aerodynamic drag F = ½ρv²C_dA.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:eng_reynolds".into(),
                label: "Reynolds number".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("EngineeringAnalysis.reynolds_number".into()),
                ontology_prefix: "sci".into(),
                description: "Re = ρvL/μ for laminar/turbulent regime.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:eng_fatigue".into(),
                label: "Fatigue cycles".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("EngineeringAnalysis.fatigue_cycles".into()),
                ontology_prefix: "sci".into(),
                description: "Basquin cycles-to-failure from stress amplitude.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:eng_miner".into(),
                label: "Miner damage".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("EngineeringAnalysis.miner_damage".into()),
                ontology_prefix: "sci".into(),
                description: "Palmgren–Miner cumulative damage from load-block pairs."
                    .into(),
            },
            ActionType::Invoke,
        )),
    ];

    let ga_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:ga_dot".into(),
                label: "Dot product".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("GeometricAlgebra.dot".into()),
                ontology_prefix: "sci".into(),
                description: "Dot product of two 3-vectors.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:ga_cross".into(),
                label: "Cross product".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("GeometricAlgebra.cross_product".into()),
                ontology_prefix: "sci".into(),
                description: "Cross product of two 3-vectors.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:ga_normalize".into(),
                label: "Normalize vector".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("GeometricAlgebra.normalize_vector".into()),
                ontology_prefix: "sci".into(),
                description: "Unit-length 3-vector.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:ga_angle".into(),
                label: "Angle between vectors".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("GeometricAlgebra.angle_between_vectors".into()),
                ontology_prefix: "sci".into(),
                description: "Angle in radians between two 3-vectors.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:ga_geometric_product".into(),
                label: "Geometric product".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("GeometricAlgebra.geometric_product".into()),
                ontology_prefix: "sci".into(),
                description: "Cl(3,0) geometric product of two 8-coeff multivectors.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:ga_outer_product".into(),
                label: "Outer product".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("GeometricAlgebra.outer_product".into()),
                ontology_prefix: "sci".into(),
                description: "Cl(3,0) outer (wedge) product of two multivectors.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:ga_rotor".into(),
                label: "Rotor from angle/axis".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("GeometricAlgebra.rotor_from_angle_axis".into()),
                ontology_prefix: "sci".into(),
                description: "Build a rotor from angle (rad) and axis.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:ga_apply_rotor".into(),
                label: "Apply rotor".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("GeometricAlgebra.apply_rotor".into()),
                ontology_prefix: "sci".into(),
                description: "Rotate a 3-vector by a 4-component rotor.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:ga_translator".into(),
                label: "Translator from displacement".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("GeometricAlgebra.translator_from_displacement".into()),
                ontology_prefix: "sci".into(),
                description: "Build a translator from a 3-vector displacement.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:ga_apply_translator".into(),
                label: "Apply translator".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("GeometricAlgebra.apply_translator".into()),
                ontology_prefix: "sci".into(),
                description: "Translate a 3-vector by a 4-component translator.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:ga_is_simd".into(),
                label: "GA SIMD available?".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("GeometricAlgebra.is_simd_available".into()),
                ontology_prefix: "sci".into(),
                description: "Whether AVX2 GA kernels are available on the Host.".into(),
            },
            ActionType::Invoke,
        )),
    ];

    let vc_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        sci_live_tool(
            "scientific:vc_gradient",
            "Gradient",
            "VectorCalculus.gradient",
            "Symbolic gradient of a scalar field.",
        ),
        sci_live_tool(
            "scientific:vc_divergence",
            "Divergence",
            "VectorCalculus.divergence",
            "Divergence of a vector field.",
        ),
        sci_live_tool(
            "scientific:vc_curl",
            "Curl",
            "VectorCalculus.curl",
            "Curl of a 3-component vector field.",
        ),
        sci_live_tool(
            "scientific:vc_laplacian",
            "Laplacian",
            "VectorCalculus.laplacian",
            "Scalar Laplacian.",
        ),
        sci_live_tool(
            "scientific:vc_line_integral_scalar",
            "Scalar line integral",
            "VectorCalculus.line_integral_scalar",
            "Scalar line integral along a parametric curve.",
        ),
        sci_live_tool(
            "scientific:vc_line_integral_work",
            "Work line integral",
            "VectorCalculus.line_integral_work",
            "Work line integral of a vector field.",
        ),
        sci_live_tool(
            "scientific:vc_surface_flux",
            "Surface flux",
            "VectorCalculus.surface_flux",
            "Flux through a parametric surface.",
        ),
    ];

    let interp_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        sci_live_tool(
            "scientific:interp_linear",
            "Linear interpolate",
            "Interpolation.linear_interp",
            "Piecewise linear interpolation.",
        ),
        sci_live_tool(
            "scientific:interp_lagrange",
            "Lagrange evaluate",
            "Interpolation.lagrange_eval",
            "Lagrange polynomial evaluation.",
        ),
        sci_live_tool(
            "scientific:interp_newton_coef",
            "Newton coefficients",
            "Interpolation.newton_coefficients",
            "Newton divided-difference coefficients.",
        ),
        sci_live_tool(
            "scientific:interp_newton_eval",
            "Newton evaluate",
            "Interpolation.newton_eval",
            "Evaluate a Newton interpolant.",
        ),
        sci_live_tool(
            "scientific:interp_poly_fit",
            "Polynomial fit",
            "Interpolation.poly_fit",
            "Least-squares polynomial fit.",
        ),
        sci_live_tool(
            "scientific:interp_poly_eval",
            "Polynomial evaluate",
            "Interpolation.poly_eval",
            "Evaluate a polynomial from coefficients.",
        ),
    ];

    let spectral_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        sci_live_tool(
            "scientific:spectral_emf_to_spd",
            "EMF to SPD",
            "Spectral.emf_to_spd",
            "Emission spectrum from EMF parameters.",
        ),
        sci_live_tool(
            "scientific:spectral_spd_to_xyz",
            "SPD to XYZ",
            "Spectral.spd_to_xyz",
            "CIE XYZ from a 41-sample SPD.",
        ),
        sci_live_tool(
            "scientific:spectral_emf_to_rgb",
            "EMF to RGB",
            "Spectral.emf_to_rgb",
            "Display RGB from EMF parameters.",
        ),
        sci_live_tool(
            "scientific:spectral_blend",
            "Blend spectra",
            "Spectral.blend",
            "Blend two EMF spectra.",
        ),
        sci_live_tool(
            "scientific:spectral_gamut_map",
            "Gamut map",
            "Spectral.gamut_map",
            "Map XYZ into the display gamut.",
        ),
    ];

    let ode_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        sci_live_tool(
            "scientific:ode_lin1",
            "Linear first-order ODE",
            "SymbolicODE.solve_linear_first_order",
            "Closed form for y' + a·y = b.",
        ),
        sci_live_tool(
            "scientific:ode_lin2",
            "Linear second-order ODE",
            "SymbolicODE.solve_linear_second_order",
            "Closed form for a·y'' + b·y' + c·y = 0.",
        ),
        sci_live_tool(
            "scientific:ode_classify_pde",
            "Classify PDE",
            "SymbolicODE.classify_second_order_pde",
            "Elliptic / parabolic / hyperbolic by B²−4AC.",
        ),
        sci_live_tool(
            "scientific:ode_separable",
            "Separable ODE",
            "SymbolicODE.solve_separable",
            "Implicit solution for y' = g(x)·h(y).",
        ),
        sci_live_tool(
            "scientific:ode_pde1",
            "Linear first-order PDE",
            "SymbolicODE.solve_first_order_linear_pde",
            "Characteristics for a·uₓ + b·u_y = 0.",
        ),
    ];

    let chem_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_boys".into(),
                label: "Boys function".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.boys_function".into()),
                ontology_prefix: "sci".into(),
                description: "Boys F_n(t) from data-n/data-t or surface numbers.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_overlap_s".into(),
                label: "s-GTO overlap".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.overlap_s".into()),
                ontology_prefix: "sci".into(),
                description: "s-type overlap (a|b) for two primitives.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_kinetic_s".into(),
                label: "s-GTO kinetic".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.kinetic_s".into()),
                ontology_prefix: "sci".into(),
                description: "s-type kinetic integral for two primitives.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_nuclear_s".into(),
                label: "s-GTO nuclear".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.nuclear_s".into()),
                ontology_prefix: "sci".into(),
                description: "s-type nuclear attraction with center and Z.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_dipole_s".into(),
                label: "s-GTO dipole".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.dipole_s".into()),
                ontology_prefix: "sci".into(),
                description: "s-type dipole moment for two primitives.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_evaluate_eri".into(),
                label: "Two-electron ERI".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.evaluate_eri".into()),
                ontology_prefix: "sci".into(),
                description: "Electron-repulsion (ab|cd) for four primitives.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_total_angular_momentum".into(),
                label: "GTO angular momentum".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.total_angular_momentum".into()),
                ontology_prefix: "sci".into(),
                description: "lx+ly+lz for a primitive (data-lx/ly/lz).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_letter".into(),
                label: "Spectroscopic letter".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.letter".into()),
                ontology_prefix: "sci".into(),
                description: "Map angular momentum l to s/p/d/f/g/h.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_n_cartesian".into(),
                label: "Cartesian shells".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.n_cartesian".into()),
                ontology_prefix: "sci".into(),
                description: "Cartesian component count (l+1)(l+2)/2.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_n_spherical".into(),
                label: "Spherical shells".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.n_spherical".into()),
                ontology_prefix: "sci".into(),
                description: "Spherical component count 2l+1.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_from_letter".into(),
                label: "Letter to l".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.from_letter".into()),
                ontology_prefix: "sci".into(),
                description: "Map spectroscopic letter to angular momentum l.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_gaussian_elim".into(),
                label: "Gaussian elimination".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.gaussian_elimination".into()),
                ontology_prefix: "sci".into(),
                description: "Solve A x = b for 2×2/3×3 (SCF stack path).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_jacobi".into(),
                label: "Jacobi diagonalization".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.jacobi_diagonalization".into()),
                ontology_prefix: "sci".into(),
                description: "Eigenpairs of real symmetric 2×2/3×3.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_transpose".into(),
                label: "Matrix transpose".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.transpose".into()),
                ontology_prefix: "sci".into(),
                description: "Transpose a 2×2/3×3 matrix.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_orthogonalize".into(),
                label: "Löwdin orthogonalize".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.orthogonalization_matrix".into()),
                ontology_prefix: "sci".into(),
                description: "Löwdin X = S^{-1/2} for 2×2/3×3 overlap.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_element_symbol".into(),
                label: "Element symbol".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.element_symbol".into()),
                ontology_prefix: "sci".into(),
                description: "Map atomic number Z to element symbol.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_atomic_number".into(),
                label: "Atomic number".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.atomic_number".into()),
                ontology_prefix: "sci".into(),
                description: "Map element symbol to atomic number Z.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_atomic_weight".into(),
                label: "Atomic weight".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.standard_atomic_weight".into()),
                ontology_prefix: "sci".into(),
                description: "Standard atomic weight for an element symbol.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_lda_exchange".into(),
                label: "LDA exchange".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.lda_exchange".into()),
                ontology_prefix: "sci".into(),
                description: "LDA exchange energy and potential from density ρ.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_lda_vwn".into(),
                label: "LDA VWN correlation".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.lda_correlation_vwn".into()),
                ontology_prefix: "sci".into(),
                description: "VWN LDA correlation energy and potential from ρ.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "scientific:chem_sto3g_h2".into(),
                label: "STO-3G H₂ summary".into(),
                icon: "lab".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Chemistry.sto3g_h2".into()),
                ontology_prefix: "sci".into(),
                description: "STO-3G H₂ minimal-basis summary constants.".into(),
            },
            ActionType::Invoke,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "scientific".into(),
            label: "Scientific Labs & Physics".into(),
            icon: "lab".into(),
            ontology_prefix: "sci".into(),
            description:
                "Clinical, molecular, Live LinearAlgebra.*, Chemistry.*, Physics.*, Cosmic.*, SpecialFunctions.*, IntegralTransforms.*, Calculus.*, ComputationalGeometry.*, and EngineeringAnalysis.* kernels."
                    .into(),
            enabled_by_default: true,
            family: "lab".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "scientific:tools".into(),
                    label: "Clinical & Physics Labs".into(),
                    icon: "lab".into(),
                    description: "Clinical, molecular, and bounded physics laboratory surfaces."
                        .into(),
                },
                lab_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "scientific:linalg".into(),
                    label: "Live linear algebra".into(),
                    icon: "lab".into(),
                    description: "Curated LinearAlgebra.* binds — matmul through QR/LU/SVD, assign, spectrum, roots."
                        .into(),
                },
                linalg_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "scientific:chem".into(),
                    label: "Live chemistry integrals".into(),
                    icon: "lab".into(),
                    description:
                        "Curated Chemistry.* integrals, angular helpers, SCF LA, element/LDA."
                            .into(),
                },
                chem_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "scientific:physics".into(),
                    label: "Live physics".into(),
                    icon: "physics".into(),
                    description: "Curated Physics.* binds — Doppler, EMF, N-body/MD, fields, materials."
                        .into(),
                },
                physics_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "scientific:cosmic".into(),
                    label: "Live cosmic".into(),
                    icon: "physics".into(),
                    description: "Curated Cosmic.* geodesy, FLRW, warp, hierarchy, atmosphere, and USRI binds."
                        .into(),
                },
                cosmic_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "scientific:sf".into(),
                    label: "Live special functions".into(),
                    icon: "lab".into(),
                    description: "Curated SpecialFunctions.* Airy, zeta, orthogonal, and Bessel binds."
                        .into(),
                },
                sf_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "scientific:xform".into(),
                    label: "Live integral transforms".into(),
                    icon: "lab".into(),
                    description:
                        "Curated IntegralTransforms.* DFT/IDFT, Z-transform, and Laplace binds."
                            .into(),
                },
                xform_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "scientific:calc".into(),
                    label: "Live calculus".into(),
                    icon: "lab".into(),
                    description: "Curated Calculus.* Hermite/BDF/symplectic/sensitivity and grid helpers."
                        .into(),
                },
                calc_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "scientific:cg".into(),
                    label: "Live computational geometry".into(),
                    icon: "lab".into(),
                    description: "Curated ComputationalGeometry.* distance, hull, mesh, curves, and spatial codes."
                        .into(),
                },
                cg_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "scientific:cg_live".into(),
                    label: "Live CG remainder".into(),
                    icon: "lab".into(),
                    description: "Host-bound ComputationalGeometry leftovers (point-set, predicates, polygons)."
                        .into(),
                },
                super::register_cg_live::cg_live_tools(),
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "scientific:eng".into(),
                    label: "Live engineering analysis".into(),
                    icon: "lab".into(),
                    description: "Curated EngineeringAnalysis.* vibration, buckling, stress, drag, fatigue."
                        .into(),
                },
                eng_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "scientific:ga".into(),
                    label: "Live geometric algebra".into(),
                    icon: "lab".into(),
                    description: "Curated GeometricAlgebra.* vector, product, rotor, and translator binds."
                        .into(),
                },
                ga_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "scientific:vc".into(),
                    label: "Live vector calculus".into(),
                    icon: "lab".into(),
                    description: "Curated VectorCalculus.* gradient, divergence, curl, Laplacian, and integrals."
                        .into(),
                },
                vc_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "scientific:interp".into(),
                    label: "Live interpolation".into(),
                    icon: "lab".into(),
                    description: "Curated Interpolation.* linear, Lagrange, Newton, and polynomial binds."
                        .into(),
                },
                interp_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "scientific:spectral".into(),
                    label: "Live spectral colour".into(),
                    icon: "lab".into(),
                    description: "Curated Spectral.* EMF, SPD, RGB, blend, and gamut binds.".into(),
                },
                spectral_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "scientific:ode".into(),
                    label: "Live symbolic ODE".into(),
                    icon: "lab".into(),
                    description: "Curated SymbolicODE.* linear ODE, separable, and PDE binds.".into(),
                },
                ode_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "scientific:num_ode".into(),
                    label: "Live numeric ODE".into(),
                    icon: "lab".into(),
                    description: "Curated Ode.* RK4, DOPRI5, BDF, and symplectic binds.".into(),
                },
                super::register_wave33_live::ode_num_tools(),
            ),
        ],
    ));
}
