use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum SolveAction {
    Linalg {
        #[command(subcommand)]
        action: LinalgAction,
    },
    Optimize {
        #[command(subcommand)]
        action: OptimizeAction,
    },
    Ode {
        #[command(subcommand)]
        action: OdeAction,
    },
    Quantum {
        #[command(subcommand)]
        action: QuantumSolveAction,
    },
    Symbolic {
        #[command(subcommand)]
        action: SymbolicSolveAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum OdeAction {
    Rk4 {
        #[arg(long, default_value = "1.0")]
        lambda: f64,
        #[arg(long, default_value = "0.0")]
        t_start: f64,
        #[arg(long, default_value = "5.0")]
        t_end: f64,
        #[arg(long, default_value = "1.0")]
        y0: f64,
        #[arg(long, default_value = "0.01")]
        step_size: f64,
    },
    Harmonic {
        #[arg(long, default_value = "1.0")]
        omega: f64,
        #[arg(long, default_value = "0.0")]
        t_start: f64,
        #[arg(long, default_value = "6.28")]
        t_end: f64,
        #[arg(long, default_value = "1.0")]
        y0: f64,
        #[arg(long, default_value = "0.01")]
        step_size: f64,
    },
    Bvp {
        #[arg(long, default_value = "0.0")]
        t_start: f64,
        #[arg(long, default_value = "1.0")]
        t_end: f64,
        #[arg(long, default_value = "1.0")]
        y_left: f64,
        #[arg(long, default_value = "0.0")]
        y_right: f64,
        #[arg(long, default_value = "1e-6")]
        threshold: f64,
    },
    QuantumSpectrum {
        #[arg(long, default_value = "1.22e19")]
        planck_mass: f64,
        #[arg(long, default_value = "0.1")]
        coupling: f64,
        #[arg(long, default_value = "10")]
        max_n: u64,
        #[arg(long, default_value = "1.0")]
        frequency: f64,
    },
}

#[derive(Subcommand, Debug)]
pub enum QuantumSolveAction {
    Qaoa {
        #[arg(long, default_value = "3")]
        depth: u8,
        #[arg(long, default_value = "0.1,0.2,0.3")]
        beta: String,
        #[arg(long, default_value = "0.5,0.5,0.5")]
        gamma: String,
    },
    Spsa {
        #[arg(long, default_value = "4")]
        num_params: u8,
        #[arg(long, default_value = "1.0,2.0,3.0,4.0")]
        initial: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum SymbolicSolveAction {
    Defeasible {
        #[arg(long, default_value = "1,2")]
        facts: String,
        #[arg(long, default_value = "1,2:3")]
        rules: String,
    },
    Sat {
        #[arg(long, default_value = "1,-2|2,3|-1,3")]
        clauses: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum LinalgAction {
    Multiply {
        #[arg(long)]
        matrix_a: String,
        #[arg(long)]
        matrix_b: String,
    },
    Determinant {
        #[arg(long)]
        matrix: String,
    },
    SolveSystem {
        #[arg(long)]
        matrix: String,
        #[arg(long)]
        vector: String,
    },
    Eigenvalues {
        #[arg(long)]
        matrix: String,
        #[arg(long, default_value = "4")]
        count: usize,
    },
    TensorContract {
        #[arg(long)]
        tensor: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum OptimizeAction {
    Simplex {
        #[arg(long)]
        initial: String,
        #[arg(long, default_value = "200")]
        iterations: u32,
    },
    Root {
        #[arg(long)]
        initial: f64,
        #[arg(long)]
        lower: f64,
        #[arg(long)]
        upper: f64,
        #[arg(long, default_value = "1e-8")]
        tolerance: f64,
    },
    CurveFit {
        #[arg(long)]
        initial_params: String,
        #[arg(long)]
        x_data: String,
        #[arg(long)]
        y_data: String,
    },
}
