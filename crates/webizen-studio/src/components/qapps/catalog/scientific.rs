use super::{Cat, QApp, Stat};

pub(super) fn apps() -> Vec<QApp> {
    vec![
        // â”€â”€ Scientific Computing â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        QApp {
            id: "physics-simulator",
            name: "Physics Simulator",
            tagline: "Physical Modelling",
            desc: "Clifford geometric algebra, Lorentz vectors, Voronoi tessellations, Burgers-equation \
                   CFD, and MCMC thermodynamic sampling â€” all zero-copy via physics_simulation and thermodynamics libraries.",
            icon: "lightning-charge",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Scientific,
        },
        QApp {
            id: "chemistry-modeler",
            name: "Chemistry Modeler",
            tagline: "Molecular Science",
            desc: "SMILES parsing, Lipinski/Veber/Ghose drug-likeness filters, LogP/TPSA, Morgan \
                   fingerprints, functional group detection, and thermochemistry via the organic_chemistry domain engine.",
            icon: "droplet",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Scientific,
        },
        QApp {
            id: "ode-lab",
            name: "ODE & Calculus Lab",
            tagline: "Numerical Methods",
            desc: "Runge-Kutta 4th-order integrator, shooting-method BVP solver, Simpson quadrature, \
                   and symbolic ODE parsing â€” each result stamped with a tensor-provenance NQuin.",
            icon: "graph-up",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Scientific,
        },
        QApp {
            id: "matrix-lab",
            name: "Matrix & Linear Algebra",
            tagline: "Numerical Computing",
            desc: "Hardware-sympathetic zero-copy matrix ops: Lanczos eigensolver, LU decomposition, \
                   tensor contraction, and CSD-accelerated dense kernels via the linear_algebra library.",
            icon: "grid-3x3",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Scientific,
        },
        QApp {
            id: "stats-lab",
            name: "Statistical Analysis Lab",
            tagline: "Data Science",
            desc: "Privacy-preserving statistics with ML-DSA fiduciary signatures: distributions, \
                   hypothesis tests, regression, Bayesian inference, and Monte Carlo sampling.",
            icon: "bar-chart-line",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Scientific,
        },
        QApp {
            id: "bioinformatics-lab",
            name: "Bioinformatics Lab",
            tagline: "Sequence Analysis",
            desc: "SIMD-accelerated Smith-Waterman and Needleman-Wunsch alignment, k-mer indexing, \
                   metabolite fingerprinting, and phylogenetic tree construction via the bioinformatics domain.",
            icon: "bezier2",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Scientific,
        },
    ]
}
