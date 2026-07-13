//! General-dimension metaheuristic optimizers (CI-SKM ch 4/6) — global / non-convex
//! search beyond the fixed-`[f64;4]` local solvers in this category. Generic local
//! search + simulated annealing work over **any** state (continuous vectors or
//! combinatorial structures via a neighbour closure — the engine ontology alignment
//! consumes), plus a continuous population optimizer (Artificial Bee Colony).
//!
//! All minimize the objective. Kernel-class `Divergent` (branch-heavy search) with
//! the CPU path always present (§13). Deterministic given the seed.

/// Deterministic RNG (LCG + Box-Muller) so searches are reproducible.
pub struct Rng(pub u64);
impl Rng {
    pub fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
    pub fn below(&mut self, b: usize) -> usize {
        (self.unit() * b as f64) as usize % b.max(1)
    }
    pub fn gaussian(&mut self) -> f64 {
        let u1 = self.unit().max(1e-12);
        let u2 = self.unit();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
}

/// Generic hill-climbing: from `initial`, repeatedly move to the best improving
/// neighbour until none improves (a local minimum) or `max_iter` is reached.
/// `neighbors` enumerates candidate moves; `objective` is minimized.
pub fn hill_climbing<S, N, O>(initial: S, neighbors: N, objective: O, max_iter: usize) -> (S, f64)
where
    S: Clone,
    N: Fn(&S) -> Vec<S>,
    O: Fn(&S) -> f64,
{
    let mut current = initial;
    let mut current_val = objective(&current);
    for _ in 0..max_iter {
        let mut best: Option<(S, f64)> = None;
        for cand in neighbors(&current) {
            let v = objective(&cand);
            if v < current_val && best.as_ref().map(|(_, bv)| v < *bv).unwrap_or(true) {
                best = Some((cand, v));
            }
        }
        match best {
            Some((s, v)) => {
                current = s;
                current_val = v;
            }
            None => break, // local optimum
        }
    }
    (current, current_val)
}

/// Generic simulated annealing: accept worsening moves with probability
/// `exp(−Δ/T)`, cooling `T ← cooling·T` each step, to escape local minima.
/// `neighbor` proposes a single random move.
pub fn simulated_annealing<S, NB, O>(
    initial: S,
    neighbor: NB,
    objective: O,
    t0: f64,
    cooling: f64,
    max_iter: usize,
    seed: u64,
) -> (S, f64)
where
    S: Clone,
    NB: Fn(&S, &mut Rng) -> S,
    O: Fn(&S) -> f64,
{
    let mut rng = Rng(seed ^ 0x9E3779B97F4A7C15);
    let mut current = initial;
    let mut current_val = objective(&current);
    let mut best = current.clone();
    let mut best_val = current_val;
    let mut t = t0.max(1e-12);
    for _ in 0..max_iter {
        let cand = neighbor(&current, &mut rng);
        let v = objective(&cand);
        let delta = v - current_val;
        if delta < 0.0 || rng.unit() < (-delta / t).exp() {
            current = cand;
            current_val = v;
            if current_val < best_val {
                best = current.clone();
                best_val = current_val;
            }
        }
        t *= cooling;
    }
    (best, best_val)
}

/// Artificial Bee Colony (continuous): a swarm explores a box `[lower, upper]^d`,
/// exploiting good food sources and scouting new ones. Minimizes `objective`.
pub fn artificial_bee_colony<O>(
    objective: O,
    lower: &[f64],
    upper: &[f64],
    n_bees: usize,
    max_iter: usize,
    seed: u64,
) -> Option<(Vec<f64>, f64)>
where
    O: Fn(&[f64]) -> f64,
{
    let d = lower.len();
    if d == 0 || upper.len() != d || n_bees < 2 {
        return None;
    }
    let mut rng = Rng(seed ^ 0xD1B54A32D192ED03);
    let sample = |rng: &mut Rng| -> Vec<f64> {
        (0..d)
            .map(|j| lower[j] + rng.unit() * (upper[j] - lower[j]))
            .collect()
    };
    let mut foods: Vec<Vec<f64>> = (0..n_bees).map(|_| sample(&mut rng)).collect();
    let mut vals: Vec<f64> = foods.iter().map(|f| objective(f)).collect();
    let mut trials = vec![0usize; n_bees];
    let limit = (n_bees * d).max(20);

    let mut best = foods[0].clone();
    let mut best_val = vals[0];
    for (i, &v) in vals.iter().enumerate() {
        if v < best_val {
            best_val = v;
            best = foods[i].clone();
        }
    }

    for _ in 0..max_iter {
        // Employed + onlooker phase: perturb one coordinate toward a partner.
        for i in 0..n_bees {
            let k = {
                let mut k = rng.below(n_bees);
                if k == i {
                    k = (k + 1) % n_bees;
                }
                k
            };
            let j = rng.below(d);
            let phi = 2.0 * rng.unit() - 1.0;
            let mut cand = foods[i].clone();
            cand[j] = (cand[j] + phi * (cand[j] - foods[k][j])).clamp(lower[j], upper[j]);
            let cv = objective(&cand);
            if cv < vals[i] {
                foods[i] = cand;
                vals[i] = cv;
                trials[i] = 0;
                if cv < best_val {
                    best_val = cv;
                    best = foods[i].clone();
                }
            } else {
                trials[i] += 1;
            }
        }
        // Scout phase: abandon exhausted sources.
        for i in 0..n_bees {
            if trials[i] > limit {
                foods[i] = sample(&mut rng);
                vals[i] = objective(&foods[i]);
                trials[i] = 0;
                if vals[i] < best_val {
                    best_val = vals[i];
                    best = foods[i].clone();
                }
            }
        }
    }
    Some((best, best_val))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abc_minimizes_the_sphere() {
        // f(x) = Σ x² has its global minimum 0 at the origin.
        let f = |x: &[f64]| x.iter().map(|v| v * v).sum::<f64>();
        let (x, val) =
            artificial_bee_colony(f, &[-5.0, -5.0, -5.0], &[5.0, 5.0, 5.0], 30, 300, 1).unwrap();
        assert!(val < 1e-2, "ABC sphere min {val}");
        assert!(x.iter().all(|&xi| xi.abs() < 0.2));
    }

    #[test]
    fn sa_escapes_a_local_minimum() {
        // A 1-D double well with a *modest* barrier (×0.1 so the peak at x=0 is
        // ~1.6, crossable by annealing): global min near x=2, local near x=-2.
        let f = |s: &Vec<f64>| {
            let x = s[0];
            0.1 * (x * x - 4.0).powi(2) - 0.5 * x // right well (x≈2) is global
        };
        let neighbor = |s: &Vec<f64>, rng: &mut Rng| vec![s[0] + 0.5 * rng.gaussian()];
        let (best, _) = simulated_annealing(vec![-2.0], neighbor, f, 5.0, 0.997, 8000, 3);
        assert!(
            best[0] > 0.0,
            "SA should find the right well, got {}",
            best[0]
        );
    }

    #[test]
    fn hill_climbing_reaches_a_discrete_optimum() {
        // Combinatorial: maximize the number of 1s in a bit-vector (minimize its
        // negative) by single-bit flips — proves the generic combinatorial path.
        let objective = |s: &Vec<bool>| -(s.iter().filter(|&&b| b).count() as f64);
        let neighbors = |s: &Vec<bool>| {
            (0..s.len())
                .map(|i| {
                    let mut c = s.clone();
                    c[i] = !c[i];
                    c
                })
                .collect()
        };
        let (best, val) = hill_climbing(vec![false; 8], neighbors, objective, 100);
        assert!(best.iter().all(|&b| b), "should turn on all bits");
        assert_eq!(val, -8.0);
    }

    #[test]
    fn guards() {
        assert!(artificial_bee_colony(|_| 0.0, &[], &[], 10, 10, 0).is_none());
    }
}
