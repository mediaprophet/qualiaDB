//! Discrete factor graphs + the sum-product algorithm (PRML ch 8.4) — belief
//! propagation for marginal inference. Exact on a tree, loopy (iterated) otherwise.
//!
//! Mission note: the governance topology is relational and lives in the *edges*
//! (guardianship → agency → credentials); a factor graph with sum-product is the
//! native inference for exactly that structure. Kernel-class `Reduction` (the
//! message sums).

use crate::solvers::learning::LearningError;

/// A non-negative factor over an ordered list of variables. `table` is indexed in
/// mixed-radix with the first variable fastest: `idx = Σ_j x_j · stride_j`,
/// `stride_0 = 1`, `stride_j = stride_{j-1}·card_{j-1}`.
#[derive(Debug, Clone)]
pub struct Factor {
    pub vars: Vec<usize>,
    pub table: Vec<f64>,
}

/// A discrete factor graph: variable cardinalities + factors.
#[derive(Debug, Clone)]
pub struct FactorGraph {
    cardinalities: Vec<usize>,
    factors: Vec<Factor>,
    /// For each variable, the `(factor index, position-in-factor)` it appears in.
    var_factors: Vec<Vec<(usize, usize)>>,
}

impl FactorGraph {
    /// Build and validate a factor graph. Each factor's table length must equal the
    /// product of its variables' cardinalities.
    pub fn new(cardinalities: Vec<usize>, factors: Vec<Factor>) -> Result<Self, LearningError> {
        if cardinalities.is_empty() || cardinalities.iter().any(|&c| c == 0) {
            return Err(LearningError::InvalidDimension);
        }
        for f in &factors {
            let expect: usize = f
                .vars
                .iter()
                .map(|&v| *cardinalities.get(v).unwrap_or(&0))
                .product();
            if f.vars.is_empty() || f.table.len() != expect || expect == 0 {
                return Err(LearningError::InvalidDimension);
            }
        }
        let mut var_factors = vec![Vec::new(); cardinalities.len()];
        for (fi, f) in factors.iter().enumerate() {
            for (pos, &v) in f.vars.iter().enumerate() {
                if v >= cardinalities.len() {
                    return Err(LearningError::InvalidDimension);
                }
                var_factors[v].push((fi, pos));
            }
        }
        Ok(Self {
            cardinalities,
            factors,
            var_factors,
        })
    }

    fn strides(&self, f: &Factor) -> Vec<usize> {
        let mut s = vec![1usize; f.vars.len()];
        for j in 1..f.vars.len() {
            s[j] = s[j - 1] * self.cardinalities[f.vars[j - 1]];
        }
        s
    }

    /// Sum-product belief propagation. Returns the normalized marginal distribution
    /// for each variable. Exact on a tree; `max_iter` loopy sweeps otherwise.
    pub fn marginals(&self, max_iter: usize, tol: f64) -> Vec<Vec<f64>> {
        let nf = self.factors.len();
        // Messages, initialised uniform.
        // m_v2f[fi][pos] : variable → factor (length card of that var).
        // m_f2v[fi][pos] : factor → variable.
        let mut m_v2f: Vec<Vec<Vec<f64>>> = Vec::with_capacity(nf);
        let mut m_f2v: Vec<Vec<Vec<f64>>> = Vec::with_capacity(nf);
        for f in &self.factors {
            let mut a = Vec::new();
            let mut b = Vec::new();
            for &v in &f.vars {
                a.push(vec![1.0; self.cardinalities[v]]);
                b.push(vec![1.0; self.cardinalities[v]]);
            }
            m_v2f.push(a);
            m_f2v.push(b);
        }

        for _ in 0..max_iter.max(1) {
            let mut max_delta = 0.0_f64;

            // ── variable → factor ──
            for (fi, f) in self.factors.iter().enumerate() {
                for (pos, &v) in f.vars.iter().enumerate() {
                    let card = self.cardinalities[v];
                    let mut msg = vec![1.0; card];
                    // product of m_f2v from every OTHER factor containing v
                    for &(gf, gpos) in &self.var_factors[v] {
                        if gf == fi {
                            continue;
                        }
                        for x in 0..card {
                            msg[x] *= m_f2v[gf][gpos][x];
                        }
                    }
                    normalize(&mut msg);
                    let delta = max_abs_diff(&m_v2f[fi][pos], &msg);
                    max_delta = max_delta.max(delta);
                    m_v2f[fi][pos] = msg;
                }
            }

            // ── factor → variable ──
            for (fi, f) in self.factors.iter().enumerate() {
                let strides = self.strides(f);
                let total: usize = f.table.len();
                for (pos, &v) in f.vars.iter().enumerate() {
                    let card = self.cardinalities[v];
                    let mut msg = vec![0.0; card];
                    // Sum over all joint configurations of the factor's variables.
                    for idx in 0..total {
                        // Decode per-variable values + accumulate the product of
                        // incoming var→factor messages for the OTHER variables.
                        let mut prod = f.table[idx];
                        let mut xv = 0;
                        for (j, &vj) in f.vars.iter().enumerate() {
                            let xj = (idx / strides[j]) % self.cardinalities[vj];
                            if j == pos {
                                xv = xj;
                            } else {
                                prod *= m_v2f[fi][j][xj];
                            }
                        }
                        msg[xv] += prod;
                    }
                    normalize(&mut msg);
                    let delta = max_abs_diff(&m_f2v[fi][pos], &msg);
                    max_delta = max_delta.max(delta);
                    m_f2v[fi][pos] = msg;
                }
            }

            if max_delta < tol {
                break;
            }
        }

        // Beliefs: product of incoming factor→var messages.
        (0..self.cardinalities.len())
            .map(|v| {
                let card = self.cardinalities[v];
                let mut belief = vec![1.0; card];
                for &(fi, pos) in &self.var_factors[v] {
                    for x in 0..card {
                        belief[x] *= m_f2v[fi][pos][x];
                    }
                }
                normalize(&mut belief);
                belief
            })
            .collect()
    }
}

fn normalize(v: &mut [f64]) {
    let s: f64 = v.iter().sum();
    if s > 0.0 {
        for x in v.iter_mut() {
            *x /= s;
        }
    } else {
        let u = 1.0 / v.len() as f64;
        v.iter_mut().for_each(|x| *x = u);
    }
}

fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Brute-force marginals of a list of factors over `cards`, for cross-checking.
    fn brute_force(cards: &[usize], factors: &[Factor]) -> Vec<Vec<f64>> {
        let total: usize = cards.iter().product();
        let mut joint = vec![0.0; total];
        let mut strides_global = vec![1usize; cards.len()];
        for j in 1..cards.len() {
            strides_global[j] = strides_global[j - 1] * cards[j - 1];
        }
        for idx in 0..total {
            let x: Vec<usize> = (0..cards.len())
                .map(|j| (idx / strides_global[j]) % cards[j])
                .collect();
            let mut p = 1.0;
            for f in factors {
                // local index in the factor
                let mut s = vec![1usize; f.vars.len()];
                for j in 1..f.vars.len() {
                    s[j] = s[j - 1] * cards[f.vars[j - 1]];
                }
                let li: usize = f.vars.iter().enumerate().map(|(j, &v)| x[v] * s[j]).sum();
                p *= f.table[li];
            }
            joint[idx] = p;
        }
        let z: f64 = joint.iter().sum();
        cards
            .iter()
            .enumerate()
            .map(|(v, &c)| {
                let mut m = vec![0.0; c];
                for idx in 0..total {
                    let xv = (idx / strides_global[v]) % c;
                    m[xv] += joint[idx];
                }
                for x in m.iter_mut() {
                    *x /= z;
                }
                m
            })
            .collect()
    }

    #[test]
    fn chain_marginals_match_brute_force() {
        // X0 — X1 — X2 chain (a tree): a prior on X0 and two pairwise factors.
        let cards = vec![2usize, 2, 2];
        let factors = vec![
            Factor {
                vars: vec![0],
                table: vec![0.7, 0.3],
            },
            Factor {
                vars: vec![0, 1],
                table: vec![0.8, 0.2, 0.3, 0.7],
            }, // [x0=0..,x1]
            Factor {
                vars: vec![1, 2],
                table: vec![0.6, 0.4, 0.1, 0.9],
            },
        ];
        let fg = FactorGraph::new(cards.clone(), factors.clone()).unwrap();
        let bp = fg.marginals(50, 1e-12);
        let bf = brute_force(&cards, &factors);
        for v in 0..3 {
            for x in 0..2 {
                assert!(
                    (bp[v][x] - bf[v][x]).abs() < 1e-9,
                    "var {v} val {x}: {} vs {}",
                    bp[v][x],
                    bf[v][x]
                );
            }
        }
    }

    #[test]
    fn star_tree_marginals_match_brute_force() {
        // Center X0 connected to X1 and X2 (a tree).
        let cards = vec![3usize, 2, 2];
        let factors = vec![
            Factor {
                vars: vec![0],
                table: vec![0.2, 0.5, 0.3],
            },
            Factor {
                vars: vec![0, 1],
                table: vec![0.9, 0.1, 0.5, 0.5, 0.2, 0.8],
            },
            Factor {
                vars: vec![0, 2],
                table: vec![0.3, 0.7, 0.6, 0.4, 0.5, 0.5],
            },
        ];
        let fg = FactorGraph::new(cards.clone(), factors.clone()).unwrap();
        let bp = fg.marginals(50, 1e-12);
        let bf = brute_force(&cards, &factors);
        for v in 0..3 {
            for x in 0..cards[v] {
                assert!((bp[v][x] - bf[v][x]).abs() < 1e-9, "var {v} val {x}");
            }
        }
    }

    #[test]
    fn guards() {
        // Factor table of the wrong length is rejected.
        let bad = FactorGraph::new(
            vec![2, 2],
            vec![Factor {
                vars: vec![0, 1],
                table: vec![1.0, 2.0],
            }],
        );
        assert_eq!(bad.unwrap_err(), LearningError::InvalidDimension);
    }
}
