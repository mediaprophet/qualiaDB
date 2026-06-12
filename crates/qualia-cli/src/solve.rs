use qualia_core_db::solvers::linear_algebra::{
    Matrix4x4, Vector4, Tensor3x3x3,
    StaticLuDecomposition, FixedLanczosEigensolver, ConstTensorContractor,
};
use qualia_core_db::solvers::optimization::{
    NelderMeadSimplex, BoundedNewtonRaphson, LevenbergMarquardtStack,
    ObjectiveFunction, RootFunction, CurveFitFunction,
};
use qualia_core_db::solvers::SolverConfig;
use qualia_core_db::solvers::quantum_optimizers::{
    QAOAAngleOptimizer, SpsaOptimizer, QAOAAngles,
    QuantumCostFunction, SpsaCostFunction, QuantumOptimizerState,
};
use qualia_core_db::solvers::symbolic_logic::{
    ForwardChainingDefeasible, BoundedSatSolver,
    DefeasibleRule, Fact, Clause, Literal, RuleType,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_f64s(s: &str) -> Vec<f64> {
    s.split(',')
        .filter_map(|tok| tok.trim().parse::<f64>().ok())
        .collect()
}

fn build_matrix4x4(v: &[f64]) -> Matrix4x4 {
    let mut m = Matrix4x4::zero();
    for row in 0..4 {
        for col in 0..4 {
            m.data[row][col] = v[row * 4 + col];
        }
    }
    m
}

fn parse_matrix4x4(s: &str) -> Option<Matrix4x4> {
    let v = parse_f64s(s);
    if v.len() < 16 {
        eprintln!("Need 16 comma-separated values for a 4×4 matrix, got {}.", v.len());
        return None;
    }
    Some(build_matrix4x4(&v))
}

fn parse_vector4(s: &str) -> Option<Vector4> {
    let v = parse_f64s(s);
    if v.len() < 4 {
        eprintln!("Need 4 comma-separated values for a Vector4, got {}.", v.len());
        return None;
    }
    Some(Vector4::from_array([v[0], v[1], v[2], v[3]]))
}

fn build_tensor3x3x3(v: &[f64]) -> Tensor3x3x3 {
    let mut t = Tensor3x3x3::zero();
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                t.data[i][j][k] = v[i * 9 + j * 3 + k];
            }
        }
    }
    t
}

fn parse_tensor3x3x3(s: &str) -> Option<Tensor3x3x3> {
    let v = parse_f64s(s);
    if v.len() < 27 {
        eprintln!("Need 27 comma-separated values for a 3×3×3 tensor, got {}.", v.len());
        return None;
    }
    Some(build_tensor3x3x3(&v))
}

fn parse_params4(s: &str) -> Option<[f64; 4]> {
    let v = parse_f64s(s);
    if v.len() < 4 {
        eprintln!("Need 4 comma-separated values, got {}.", v.len());
        return None;
    }
    Some([v[0], v[1], v[2], v[3]])
}

fn default_config(max_iters: u32, tol: f64) -> SolverConfig {
    SolverConfig { max_iterations: max_iters, tolerance: tol, step_size: 0.01, verbose: false }
}

// ── Linear algebra runners ────────────────────────────────────────────────────

pub fn run_matrix_multiply(a_str: &str, b_str: &str) {
    let (Some(a), Some(b)) = (parse_matrix4x4(a_str), parse_matrix4x4(b_str)) else { return; };
    let result = a.multiply_matrix(&b);
    println!("Matrix A × B:");
    print_matrix4x4(&result);
}

pub fn run_determinant(m_str: &str) {
    let Some(m) = parse_matrix4x4(m_str) else { return; };
    let det = m.determinant();
    println!("Determinant: {det:.10e}");
}

pub fn run_solve_system(m_str: &str, v_str: &str) {
    let (Some(a), Some(b)) = (parse_matrix4x4(m_str), parse_vector4(v_str)) else { return; };
    let mut lu = StaticLuDecomposition::new(default_config(100, 1e-12));
    match lu.solve(&a, &b) {
        Ok(x) => {
            println!("Solution x for Ax = b:");
            println!("  [{:.6}, {:.6}, {:.6}, {:.6}]", x.data[0], x.data[1], x.data[2], x.data[3]);
        }
        Err(e) => eprintln!("Solve failed: {e:?}"),
    }
}

pub fn run_eigenvalues(m_str: &str, count: usize) {
    let Some(m) = parse_matrix4x4(m_str) else { return; };
    let mut solver = FixedLanczosEigensolver::new(default_config(100, 1e-8));
    match solver.find_lowest_eigenvalues(&m, count) {
        Ok(eigs) => {
            let n = count.min(4);
            println!("Lowest {n} eigenvalue(s) (Lanczos):");
            for i in 0..n {
                println!("  λ[{i}] = {:.10e}", eigs[i]);
            }
        }
        Err(e) => eprintln!("Eigenvalue computation failed: {e:?}"),
    }
}

pub fn run_tensor_contract(t_str: &str) {
    let Some(ta) = parse_tensor3x3x3(t_str) else { return; };
    let tb = Tensor3x3x3::zero();
    let indices: [(usize, usize); 3] = [(0, 0), (1, 1), (2, 2)];
    let mut contractor = ConstTensorContractor::new(default_config(1, 1e-10));
    match contractor.contract(&ta, &tb, &indices) {
        Ok(result) => {
            // Sum all elements as a scalar trace equivalent
            let scalar: f64 = (0..3).flat_map(|i| (0..3).flat_map(move |j| (0..3).map(move |k| (i, j, k))))
                .map(|(i, j, k)| result.data[i][j][k])
                .sum();
            println!("Tensor contraction A⊗0 trace-sum: {scalar:.10e}");
            println!("(pass two tensors via --tensor-a and --tensor-b for a full contraction)");
        }
        Err(e) => eprintln!("Tensor contraction failed: {e:?}"),
    }
}

fn print_matrix4x4(m: &Matrix4x4) {
    for row in 0..4 {
        println!("  [{:12.6}, {:12.6}, {:12.6}, {:12.6}]",
            m.data[row][0], m.data[row][1], m.data[row][2], m.data[row][3]);
    }
}

// ── ObjectiveFunction adapter for closures ────────────────────────────────────

struct ClosureFn<F: Fn(&[f64; 4]) -> f64>(F);
impl<F: Fn(&[f64; 4]) -> f64> ObjectiveFunction for ClosureFn<F> {
    fn evaluate(&self, params: &[f64; 4]) -> f64 { (self.0)(params) }
}

struct RootClosureFn<F: Fn(f64) -> f64, D: Fn(f64) -> f64>(F, D);
impl<F: Fn(f64) -> f64, D: Fn(f64) -> f64> RootFunction for RootClosureFn<F, D> {
    fn evaluate(&self, x: f64) -> f64 { (self.0)(x) }
    fn derivative(&self, x: f64) -> f64 { (self.1)(x) }
}

struct CurveFn;
impl CurveFitFunction for CurveFn {
    // cubic model: p[0]*x³ + p[1]*x² + p[2]*x + p[3]
    fn evaluate(&self, x: f64, p: &[f64; 4]) -> f64 {
        p[0] * x.powi(3) + p[1] * x.powi(2) + p[2] * x + p[3]
    }
    fn jacobian(&self, x: f64, _p: &[f64; 4]) -> [f64; 4] {
        [x.powi(3), x.powi(2), x, 1.0]
    }
}

// ── Optimization runners ──────────────────────────────────────────────────────

pub fn run_simplex(initial_str: &str, iterations: u32) {
    let Some(x0) = parse_params4(initial_str) else { return; };
    let cfg = default_config(iterations, 1e-8);
    let mut solver = NelderMeadSimplex::new(x0, cfg);
    // Rosenbrock 4D: f(x,y,z,w) = (1-x)² + 100(y-x²)² + (1-z)² + 100(w-z²)²
    let f = ClosureFn(|p: &[f64; 4]| {
        let a = (1.0 - p[0]).powi(2) + 100.0 * (p[1] - p[0].powi(2)).powi(2);
        let b = (1.0 - p[2]).powi(2) + 100.0 * (p[3] - p[2].powi(2)).powi(2);
        a + b
    });
    match solver.optimize(&f) {
        Ok(state) => {
            let bp = solver.best_point;
            println!("Nelder-Mead simplex (Rosenbrock 4D, {iterations} iters):");
            println!("  x = [{:.6}, {:.6}, {:.6}, {:.6}]", bp[0], bp[1], bp[2], bp[3]);
            println!("  f(x) = {:.10e}", state.objective_value);
            println!("  Converged: {}", state.converged);
        }
        Err(e) => eprintln!("Simplex failed: {e:?}"),
    }
}

pub fn run_root(initial: f64, lower: f64, upper: f64, tolerance: f64) {
    // f(x) = x³ − x − 1  (plastic constant, root ≈ 1.32472)
    let cfg = SolverConfig { max_iterations: 200, tolerance, step_size: 0.01, verbose: false };
    let mut solver = BoundedNewtonRaphson::new(initial, lower, upper, cfg);
    let f = RootClosureFn(
        |x: f64| x * x * x - x - 1.0,
        |x: f64| 3.0 * x * x - 1.0,
    );
    match solver.find_root(&f) {
        Ok(state) => {
            let root = solver.get_root();
            println!("Newton-Raphson root of f(x) = x³ − x − 1:");
            println!("  root ≈ {root:.10e}");
            println!("  f(root) = {:.4e}", x_cubed_minus_x_minus_one(root));
            println!("  Converged: {} ({} iters)", state.converged, state.iteration);
        }
        Err(e) => eprintln!("Root finder failed: {e:?}"),
    }
}

fn x_cubed_minus_x_minus_one(x: f64) -> f64 { x * x * x - x - 1.0 }

pub fn run_curve_fit(params_str: &str, x_str: &str, y_str: &str) {
    let Some(p0) = parse_params4(params_str) else { return; };
    let xs = parse_f64s(x_str);
    let ys = parse_f64s(y_str);
    if xs.is_empty() || ys.is_empty() || xs.len() != ys.len() {
        eprintln!("x-data and y-data must be equal-length, non-empty comma-separated lists.");
        return;
    }

    // Pad/truncate to exactly 10 points (LM stack uses fixed 10-point buffer)
    let mut xd = [0.0f64; 10];
    let mut yd = [0.0f64; 10];
    let n = xs.len().min(10);
    xd[..n].copy_from_slice(&xs[..n]);
    yd[..n].copy_from_slice(&ys[..n]);

    let cfg = default_config(200, 1e-8);
    let mut lm = LevenbergMarquardtStack::new(p0, cfg);
    let model = CurveFn;
    match lm.fit_curve(&model, &xd, &yd) {
        Ok(state) => {
            let p = lm.parameters;
            println!("Levenberg-Marquardt curve fit (cubic: p0·x³+p1·x²+p2·x+p3):");
            println!("  params = [{:.6}, {:.6}, {:.6}, {:.6}]", p[0], p[1], p[2], p[3]);
            println!("  χ²     = {:.6e}", state.chi_squared);
            println!("  Converged: {} ({} iters)", state.converged, state.iteration);
        }
        Err(e) => eprintln!("Curve fit failed: {e:?}"),
    }
}

// ── ODE runners ───────────────────────────────────────────────────────────────

pub fn run_ode_rk4(lambda: f64, t_start: f64, t_end: f64, y0: f64, step_size: f64) {
    use qualia_core_db::modalities::calculus::ode_solver::{ExponentialDecay, Rk4Solver};
    let system = ExponentialDecay::new(lambda);
    let mut solver = Rk4Solver::new(system, step_size);
    let result = solver.solve(t_start, t_end, y0);
    println!("RK4 ODE (exponential decay λ={lambda}):");
    println!("  t ∈ [{t_start}, {t_end}],  y(0)={y0},  h={step_size}");
    println!("  y(t_end) ≈ {result:.10e}");
    println!("  exact    = {:.10e}", y0 * (-lambda * (t_end - t_start)).exp());
}

pub fn run_ode_harmonic(omega: f64, t_start: f64, t_end: f64, y0: f64, step_size: f64) {
    use qualia_core_db::modalities::calculus::ode_solver::{HarmonicOscillator, Rk4Solver};
    let system = HarmonicOscillator::new(omega);
    let mut solver = Rk4Solver::new(system, step_size);
    let result = solver.solve(t_start, t_end, y0);
    println!("RK4 ODE (harmonic oscillator ω={omega}):");
    println!("  t ∈ [{t_start}, {t_end}],  y(0)={y0},  h={step_size}");
    println!("  y(t_end) ≈ {result:.10e}");
}

pub fn run_ode_bvp(t_start: f64, t_end: f64, y_left: f64, y_right: f64, threshold: f64) {
    use qualia_core_db::modalities::calculus::ode_solver::{
        ShootingMethod, BvpSystem, ExponentialDecay,
    };

    struct SimpleBvp { lambda: f64 }
    impl BvpSystem for SimpleBvp {
        fn derivative(&self, _t: f64, y: f64) -> f64 { -self.lambda * y }
        fn boundary_left(&self, _a: f64) -> f64 { 0.0 }
        fn boundary_right(&self, _b: f64) -> f64 { 0.0 }
    }

    let system = SimpleBvp { lambda: 1.0 };
    let mut solver = ShootingMethod::new(system, threshold)
        .with_max_iterations(500);
    match solver.solve(t_start, t_end, y_left, y_right) {
        Ok((ic, residual)) => {
            println!("BVP shooting method (f'=-y):");
            println!("  t ∈ [{t_start}, {t_end}]");
            println!("  Converged IC : {ic:.8e}");
            println!("  Residual     : {residual:.4e}");
        }
        Err(e) => eprintln!("BVP failed to converge: {e}"),
    }
}

pub fn run_ode_quantum_spectrum(planck_mass: f64, coupling: f64, max_n: u64, frequency: f64) {
    use qualia_core_db::modalities::calculus::ode_solver::QuantizationMapper;
    let qh = QuantizationMapper::new(planck_mass, coupling);
    let spectrum = qh.compute_mass_spectrum(max_n, frequency);
    println!("Quantum harmonic mass spectrum (M_p={planck_mass:.3e}, g={coupling:.3e}, f={frequency:.3e}):");
    for (n, mass) in spectrum.iter().take(10) {
        println!("  n={n:3}  mass = {mass:.10e}");
    }
    if spectrum.len() > 10 {
        println!("  … ({} total levels)", spectrum.len());
    }
}

// ── Quantum optimizer runners ─────────────────────────────────────────────────

struct DemoQaoa(u8);

impl QuantumCostFunction for DemoQaoa {
    fn evaluate_quantum(&self, angles: &QAOAAngles) -> qualia_core_db::solvers::SolverResult<f64> {
        // Demo MaxCut-like cost: minimize sum of angle squares
        let cost: f64 = angles.beta[..self.0 as usize].iter().map(|x| x * x).sum::<f64>()
                      + angles.gamma[..self.0 as usize].iter().map(|x| x * x).sum::<f64>();
        Ok(cost)
    }
    fn evaluate_perturbed(
        &self,
        angles: &QAOAAngles,
        perturbation: &QAOAAngles,
    ) -> qualia_core_db::solvers::SolverResult<(f64, f64)> {
        let mut plus  = *angles;
        let mut minus = *angles;
        for i in 0..self.0 as usize {
            plus.beta[i]  += perturbation.beta[i];
            minus.beta[i] -= perturbation.beta[i];
        }
        Ok((self.evaluate_quantum(&plus)?, self.evaluate_quantum(&minus)?))
    }
    fn problem_size(&self) -> u8 { self.0 }
}

pub fn run_quantum_qaoa(depth: u8, beta_str: &str, gamma_str: &str) {
    let cfg = default_config(100, 1e-6);
    let mut opt = QAOAAngleOptimizer::new(depth, cfg);
    let betas  = parse_f64s(beta_str);
    let gammas = parse_f64s(gamma_str);
    let mut init = QAOAAngles { beta: [0.0; 10], gamma: [0.0; 10] };
    for (i, &b) in betas.iter().take(10).enumerate()  { init.beta[i]  = b; }
    for (i, &g) in gammas.iter().take(10).enumerate() { init.gamma[i] = g; }
    let cost_fn = DemoQaoa(depth.min(10));
    match opt.optimize(&cost_fn, init) {
        Ok(state) => {
            let final_a = opt.get_angles();
            println!("QAOA angle optimizer (depth={depth}, demo MaxCut cost):");
            println!("  Iterations   : {}", state.iteration);
            println!("  Quantum calls: {}", state.quantum_calls);
            println!("  Final cost   : {:.8e}", state.cost_value);
            println!("  Converged    : {}", state.converged);
            println!("  β[0..d] = {:?}", &final_a.beta[..depth as usize]);
            println!("  γ[0..d] = {:?}", &final_a.gamma[..depth as usize]);
        }
        Err(e) => eprintln!("QAOA optimization failed: {e:?}"),
    }
}

struct DemoSpsa(u8);

impl SpsaCostFunction for DemoSpsa {
    fn evaluate(
        &self,
        params: &[f64; 20],
        n: u8,
    ) -> qualia_core_db::solvers::SolverResult<f64> {
        Ok(params[..n as usize].iter().map(|x| x * x).sum())
    }
    fn valid_parameters(&self, _p: &[f64; 20], _n: u8) -> bool { true }
}

pub fn run_quantum_spsa(num_params: u8, initial_str: &str) {
    let vals = parse_f64s(initial_str);
    let mut params = [0.0f64; 20];
    let n = (num_params as usize).min(20).min(vals.len());
    params[..n].copy_from_slice(&vals[..n]);
    let cfg = default_config(200, 1e-6);
    let mut opt = SpsaOptimizer::new(num_params, cfg);
    let cost_fn = DemoSpsa(num_params);
    match opt.optimize(&cost_fn, &params) {
        Ok(state) => {
            let p = opt.get_parameters();
            println!("SPSA optimizer (demo quadratic cost, {num_params} params):");
            println!("  Iterations: {}", state.iteration);
            println!("  Final cost: {:.8e}", state.cost_value);
            println!("  Converged : {}", state.converged);
            println!("  Params    : {:?}", &p[..n]);
        }
        Err(e) => eprintln!("SPSA optimization failed: {e:?}"),
    }
}

// ── Symbolic logic runners ────────────────────────────────────────────────────

fn parse_u8s(s: &str) -> Vec<u8> {
    s.split(',').filter_map(|t| t.trim().parse::<u8>().ok()).collect()
}

pub fn run_symbolic_defeasible(facts_str: &str, rules_str: &str) {
    let cfg = default_config(500, 1e-6);
    let mut solver = ForwardChainingDefeasible::new(cfg);

    // Parse facts: comma-separated variable indices (1-based)
    for (idx, var) in parse_u8s(facts_str).into_iter().enumerate() {
        let fact = Fact {
            id: (idx as u32) + 1,
            literal: Literal { variable: var, negated: false },
            supporting_rules: [0; 3],
            defeated: false,
            confidence: 1.0,
        };
        if let Err(e) = solver.add_fact(fact) {
            eprintln!("  Fact capacity exceeded: {e:?}");
            break;
        }
    }

    // Parse rules: "ant1,ant2:cons" pairs separated by ';'
    for (rid, rule_str) in rules_str.split(';').enumerate() {
        let parts: Vec<&str> = rule_str.splitn(2, ':').collect();
        if parts.len() != 2 { continue; }
        let ants: Vec<u8> = parse_u8s(parts[0]);
        let cons = parts[1].trim().parse::<u8>().unwrap_or(0);
        let mut antecedents = [Literal { variable: 0, negated: false }; 5];
        for (i, &a) in ants.iter().take(5).enumerate() {
            antecedents[i] = Literal { variable: a, negated: false };
        }
        let rule = DefeasibleRule {
            id: (rid as u32) + 1,
            rule_type: RuleType::Defeasible,
            antecedents,
            consequent: Literal { variable: cons, negated: false },
            priority: 500,
            active: true,
            fire_count: 0,
        };
        if let Err(e) = solver.add_rule(rule) {
            eprintln!("  Rule capacity exceeded: {e:?}");
            break;
        }
    }

    match solver.infer() {
        Ok(state) => {
            println!("Defeasible logic inference:");
            println!("  Facts at start : {}", parse_u8s(facts_str).len());
            println!("  Facts after    : {}", state.num_facts);
            println!("  Rules fired    : {}", state.rules_fired);
            println!("  Iterations     : {}", state.iteration);
            println!("  Converged      : {}", state.converged);
            let active: Vec<u8> = solver.get_facts()
                .iter()
                .filter(|f| f.id != 0 && !f.defeated)
                .map(|f| f.literal.variable)
                .collect();
            println!("  Active literals: {active:?}");
        }
        Err(e) => eprintln!("Defeasible inference failed: {e:?}"),
    }
}

pub fn run_symbolic_sat(clauses_str: &str) {
    let cfg = default_config(1000, 1e-8);
    let mut solver = BoundedSatSolver::new(cfg);

    // Parse clauses: "1,-2,3|4,-5" — pipe-separated clauses, comma-separated literals
    for (cid, clause_str) in clauses_str.split('|').enumerate() {
        let lits: Vec<Literal> = clause_str.split(',')
            .filter_map(|tok| {
                let tok = tok.trim();
                let (neg, var_str) = if tok.starts_with('-') {
                    (true, &tok[1..])
                } else {
                    (false, tok)
                };
                var_str.parse::<u8>().ok().map(|v| Literal { variable: v, negated: neg })
            })
            .collect();
        let mut literals = [Literal { variable: 0, negated: false }; 5];
        let n = lits.len().min(5) as u8;
        for (i, l) in lits.iter().take(5).enumerate() { literals[i] = *l; }
        let clause = Clause {
            id: (cid as u32) + 1,
            literals,
            num_literals: n,
            learned: false,
            activity: 1.0,
        };
        if let Err(e) = solver.add_clause(clause) {
            eprintln!("  Clause capacity exceeded: {e:?}");
            break;
        }
    }

    match solver.solve() {
        Ok(state) => {
            use qualia_core_db::solvers::symbolic_logic::AssignmentValue;
            println!("SAT solver (DPLL):");
            println!("  Iterations  : {}", state.iteration);
            println!("  Decisions   : {}", state.num_decisions);
            println!("  Satisfiable : {}", match state.satisfiable { Some(true) => "yes", Some(false) => "no", None => "unknown" });
            let assignments = solver.get_assignments();
            let assigned: Vec<_> = assignments.iter()
                .filter(|a| a.level != 0 || a.antecedent.is_some())
                .map(|a| {
                    let v = match a.value {
                        AssignmentValue::True => "T",
                        AssignmentValue::False => "F",
                        AssignmentValue::Unassigned => "?",
                    };
                    format!("x[{}]={}", a.level, v)
                })
                .take(10)
                .collect();
            if !assigned.is_empty() {
                println!("  Assignments : {}", assigned.join(", "));
            }
        }
        Err(e) => eprintln!("SAT solve failed: {e:?}"),
    }
}
