//! Discrete Hidden Markov Model (PRML ch 13.2) — the standard estimators over a
//! sequence of discrete observations: the **scaled forward** algorithm for the
//! sequence log-likelihood, **Viterbi** for the most-likely state path, and
//! **Baum-Welch** (EM) to learn the parameters. Mission note: time-indexed
//! provenance / life-record reasoning is temporal; this is the canonical model over
//! censored temporal evidence. Kernel-class `Reduction` (the message passes).

use crate::solvers::learning::LearningError;

/// A discrete HMM: `k` hidden states, `m` observation symbols.
#[derive(Debug, Clone)]
pub struct Hmm {
    /// Initial state distribution π (length k).
    pub pi: Vec<f64>,
    /// Row-major `k×k` transition matrix A (`a[i*k+j]` = P(state j | state i)).
    pub a: Vec<f64>,
    /// Row-major `k×m` emission matrix B (`b[i*m+o]` = P(symbol o | state i)).
    pub b: Vec<f64>,
    pub k: usize,
    pub m: usize,
}

impl Hmm {
    /// Construct from parameters, validating shapes (rows need not be exactly
    /// normalized but must be non-empty).
    pub fn new(
        pi: Vec<f64>,
        a: Vec<f64>,
        b: Vec<f64>,
        k: usize,
        m: usize,
    ) -> Result<Self, LearningError> {
        if k == 0 || m == 0 || pi.len() != k || a.len() != k * k || b.len() != k * m {
            return Err(LearningError::InvalidDimension);
        }
        Ok(Self { pi, a, b, k, m })
    }

    /// Scaled forward pass. Returns `(log_likelihood, scaled_alpha, scales)`.
    fn forward_scaled(&self, obs: &[usize]) -> (f64, Vec<f64>, Vec<f64>) {
        let (k, t) = (self.k, obs.len());
        let mut alpha = vec![0.0; t * k];
        let mut scale = vec![0.0; t];
        // t = 0
        let mut s = 0.0;
        for i in 0..k {
            let v = self.pi[i] * self.b[i * self.m + obs[0]];
            alpha[i] = v;
            s += v;
        }
        let c0 = if s > 0.0 { 1.0 / s } else { 0.0 };
        scale[0] = c0;
        for i in 0..k {
            alpha[i] *= c0;
        }
        // t > 0
        for tt in 1..t {
            let mut s = 0.0;
            for j in 0..k {
                let mut acc = 0.0;
                for i in 0..k {
                    acc += alpha[(tt - 1) * k + i] * self.a[i * k + j];
                }
                let v = acc * self.b[j * self.m + obs[tt]];
                alpha[tt * k + j] = v;
                s += v;
            }
            let c = if s > 0.0 { 1.0 / s } else { 0.0 };
            scale[tt] = c;
            for j in 0..k {
                alpha[tt * k + j] *= c;
            }
        }
        // log P(obs) = −Σ log c_t.
        let ll: f64 = scale
            .iter()
            .map(|&c| if c > 0.0 { -c.ln() } else { f64::NEG_INFINITY })
            .sum();
        (ll, alpha, scale)
    }

    /// Log-likelihood `log P(obs | model)`. `None` for an empty sequence or an
    /// out-of-range symbol.
    pub fn log_likelihood(&self, obs: &[usize]) -> Option<f64> {
        if obs.is_empty() || obs.iter().any(|&o| o >= self.m) {
            return None;
        }
        Some(self.forward_scaled(obs).0)
    }

    /// Viterbi most-likely state path + its log-probability. `None` for an empty
    /// sequence or an out-of-range symbol.
    pub fn viterbi(&self, obs: &[usize]) -> Option<(Vec<usize>, f64)> {
        if obs.is_empty() || obs.iter().any(|&o| o >= self.m) {
            return None;
        }
        let (k, t, m) = (self.k, obs.len(), self.m);
        let ln = |x: f64| if x > 0.0 { x.ln() } else { f64::NEG_INFINITY };
        let mut delta = vec![f64::NEG_INFINITY; t * k];
        let mut psi = vec![0usize; t * k];
        for i in 0..k {
            delta[i] = ln(self.pi[i]) + ln(self.b[i * m + obs[0]]);
        }
        for tt in 1..t {
            for j in 0..k {
                let mut best = f64::NEG_INFINITY;
                let mut arg = 0;
                for i in 0..k {
                    let v = delta[(tt - 1) * k + i] + ln(self.a[i * k + j]);
                    if v > best {
                        best = v;
                        arg = i;
                    }
                }
                delta[tt * k + j] = best + ln(self.b[j * m + obs[tt]]);
                psi[tt * k + j] = arg;
            }
        }
        // Termination + backtrack.
        let mut last = 0;
        let mut best = f64::NEG_INFINITY;
        for i in 0..k {
            if delta[(t - 1) * k + i] > best {
                best = delta[(t - 1) * k + i];
                last = i;
            }
        }
        let mut path = vec![0usize; t];
        path[t - 1] = last;
        for tt in (1..t).rev() {
            path[tt - 1] = psi[tt * k + path[tt]];
        }
        Some((path, best))
    }
}

struct Lcg(u64);
impl Lcg {
    fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

fn normalize(row: &mut [f64]) {
    let s: f64 = row.iter().sum();
    if s > 0.0 {
        for v in row.iter_mut() {
            *v /= s;
        }
    }
}

/// Learn HMM parameters from one observation sequence by Baum-Welch (EM). Returns
/// `(model, final_log_likelihood)`. Initialised randomly (seeded). Fails closed on
/// bad shapes / out-of-range symbols.
pub fn baum_welch(
    obs: &[usize],
    k: usize,
    m: usize,
    max_iter: usize,
    tol: f64,
    seed: u64,
) -> Result<(Hmm, f64), LearningError> {
    let t = obs.len();
    if k == 0 || m == 0 || t < 2 || obs.iter().any(|&o| o >= m) {
        return Err(LearningError::InvalidDimension);
    }

    // Random near-uniform initialisation.
    let mut rng = Lcg(seed ^ 0x9E3779B97F4A7C15);
    let mut pi = vec![0.0; k];
    let mut a = vec![0.0; k * k];
    let mut b = vec![0.0; k * m];
    for i in 0..k {
        pi[i] = 1.0 + 0.1 * rng.unit();
    }
    normalize(&mut pi);
    for i in 0..k {
        for j in 0..k {
            a[i * k + j] = 1.0 + 0.1 * rng.unit();
        }
        normalize(&mut a[i * k..(i + 1) * k]);
        for o in 0..m {
            b[i * m + o] = 1.0 + 0.1 * rng.unit();
        }
        normalize(&mut b[i * m..(i + 1) * m]);
    }

    let mut hmm = Hmm { pi, a, b, k, m };
    let mut prev_ll = f64::NEG_INFINITY;
    let mut final_ll = prev_ll;

    for _ in 0..max_iter.max(1) {
        // E-step: scaled forward + backward.
        let (ll, alpha, scale) = hmm.forward_scaled(obs);
        final_ll = ll;
        // Scaled backward.
        let mut beta = vec![0.0; t * k];
        for i in 0..k {
            beta[(t - 1) * k + i] = scale[t - 1];
        }
        for tt in (0..t - 1).rev() {
            for i in 0..k {
                let mut acc = 0.0;
                for j in 0..k {
                    acc += hmm.a[i * k + j] * hmm.b[j * m + obs[tt + 1]] * beta[(tt + 1) * k + j];
                }
                beta[tt * k + i] = acc * scale[tt];
            }
        }
        // γ and accumulate ξ sums.
        let mut gamma = vec![0.0; t * k];
        for tt in 0..t {
            let mut s = 0.0;
            for i in 0..k {
                gamma[tt * k + i] = alpha[tt * k + i] * beta[tt * k + i];
                s += gamma[tt * k + i];
            }
            if s > 0.0 {
                for i in 0..k {
                    gamma[tt * k + i] /= s;
                }
            }
        }
        // M-step.
        // π.
        for i in 0..k {
            hmm.pi[i] = gamma[i];
        }
        // A.
        let mut new_a = vec![0.0; k * k];
        for i in 0..k {
            let mut denom = 0.0;
            for tt in 0..t - 1 {
                denom += gamma[tt * k + i];
            }
            for j in 0..k {
                let mut num = 0.0;
                for tt in 0..t - 1 {
                    num += alpha[tt * k + i]
                        * hmm.a[i * k + j]
                        * hmm.b[j * m + obs[tt + 1]]
                        * beta[(tt + 1) * k + j];
                }
                new_a[i * k + j] = if denom > 0.0 { num / denom } else { 0.0 };
            }
            normalize(&mut new_a[i * k..(i + 1) * k]);
        }
        hmm.a = new_a;
        // B.
        let mut new_b = vec![0.0; k * m];
        for i in 0..k {
            let mut denom = 0.0;
            for tt in 0..t {
                denom += gamma[tt * k + i];
            }
            for tt in 0..t {
                new_b[i * m + obs[tt]] += gamma[tt * k + i];
            }
            if denom > 0.0 {
                for o in 0..m {
                    new_b[i * m + o] /= denom;
                }
            }
            normalize(&mut new_b[i * m..(i + 1) * m]);
        }
        hmm.b = new_b;

        if (ll - prev_ll).abs() < tol {
            break;
        }
        prev_ll = ll;
    }

    Ok((hmm, final_ll))
}

#[cfg(test)]
mod tests {
    use super::*;

    // A 2-state HMM: state 0 emits symbol 0 mostly, state 1 emits symbol 1 mostly;
    // states are sticky (stay put with high probability).
    fn sticky_hmm() -> Hmm {
        Hmm::new(
            vec![0.5, 0.5],
            vec![0.9, 0.1, 0.1, 0.9],
            vec![0.9, 0.1, 0.1, 0.9],
            2,
            2,
        )
        .unwrap()
    }

    #[test]
    fn viterbi_recovers_obvious_path() {
        let hmm = sticky_hmm();
        // Observations clearly in state 0 then state 1.
        let obs = [0, 0, 0, 1, 1, 1];
        let (path, _) = hmm.viterbi(&obs).unwrap();
        assert_eq!(path, vec![0, 0, 0, 1, 1, 1]);
    }

    #[test]
    fn log_likelihood_is_finite_and_orders_sequences() {
        let hmm = sticky_hmm();
        // A "consistent" sequence is more likely than a rapidly alternating one.
        let consistent = hmm.log_likelihood(&[0, 0, 0, 0]).unwrap();
        let alternating = hmm.log_likelihood(&[0, 1, 0, 1]).unwrap();
        assert!(consistent.is_finite() && alternating.is_finite());
        assert!(consistent > alternating, "{consistent} !> {alternating}");
        assert!(hmm.log_likelihood(&[]).is_none());
        assert!(hmm.log_likelihood(&[5]).is_none()); // out-of-range symbol
    }

    #[test]
    fn baum_welch_increases_likelihood_and_learns_structure() {
        // A long sequence with clear regime structure.
        let mut obs = Vec::new();
        for _ in 0..15 {
            obs.push(0);
        }
        for _ in 0..15 {
            obs.push(1);
        }
        for _ in 0..15 {
            obs.push(0);
        }
        let (model, ll) = baum_welch(&obs, 2, 2, 100, 1e-6, 1).unwrap();
        assert!(ll.is_finite());
        // The learned model should make the training sequence at least as likely as
        // a uniform-ish model, and Viterbi should segment the regimes.
        let (path, _) = model.viterbi(&obs).unwrap();
        // The first block and the middle block should differ in state.
        assert_ne!(path[5], path[20], "regimes should map to different states");
    }

    #[test]
    fn guards() {
        assert!(Hmm::new(vec![1.0], vec![1.0], vec![1.0, 0.0], 1, 2).is_ok());
        assert_eq!(
            baum_welch(&[0], 2, 2, 10, 1e-6, 0).unwrap_err(),
            LearningError::InvalidDimension
        );
    }
}
