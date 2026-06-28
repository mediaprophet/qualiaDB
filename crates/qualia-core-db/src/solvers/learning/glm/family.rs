//! GLM exponential-family links — the per-family functions the IRLS loop needs.
//! Both families use their canonical link.

/// A generalized-linear-model family (canonical link).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// Logistic regression: Bernoulli response, logit link, `μ = σ(η)`.
    Binomial,
    /// Poisson regression: count response, log link, `μ = exp(η)`.
    Poisson,
}

impl Family {
    /// Inverse link `μ = g⁻¹(η)`.
    pub fn inv_link(self, eta: f64) -> f64 {
        match self {
            // Numerically stable logistic.
            Family::Binomial => {
                if eta >= 0.0 {
                    1.0 / (1.0 + (-eta).exp())
                } else {
                    let e = eta.exp();
                    e / (1.0 + e)
                }
            }
            Family::Poisson => eta.exp(),
        }
    }

    /// `dμ/dη` at the current mean.
    pub fn dmu_deta(self, mu: f64) -> f64 {
        match self {
            Family::Binomial => mu * (1.0 - mu),
            Family::Poisson => mu,
        }
    }

    /// Variance function `V(μ)`.
    pub fn variance(self, mu: f64) -> f64 {
        match self {
            Family::Binomial => mu * (1.0 - mu),
            Family::Poisson => mu,
        }
    }

    /// Unit deviance contribution `dᵢ` (so total deviance = Σ dᵢ). Used for the
    /// model deviance / convergence on the log-likelihood scale.
    pub fn unit_deviance(self, y: f64, mu: f64) -> f64 {
        const EPS: f64 = 1e-12;
        match self {
            Family::Binomial => {
                let m = mu.clamp(EPS, 1.0 - EPS);
                let a = if y > 0.0 { y * (y / m).ln() } else { 0.0 };
                let b = if y < 1.0 {
                    (1.0 - y) * ((1.0 - y) / (1.0 - m)).ln()
                } else {
                    0.0
                };
                2.0 * (a + b)
            }
            Family::Poisson => {
                let m = mu.max(EPS);
                let a = if y > 0.0 { y * (y / m).ln() } else { 0.0 };
                2.0 * (a - (y - m))
            }
        }
    }

    /// A safe starting mean for IRLS from the raw response.
    pub fn start_mu(self, y: f64) -> f64 {
        match self {
            Family::Binomial => (y + 0.5) / 2.0, // pull toward 0.5
            Family::Poisson => (y + 0.1).max(0.1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logistic_link_round_trip() {
        let f = Family::Binomial;
        assert!((f.inv_link(0.0) - 0.5).abs() < 1e-12);
        assert!(f.inv_link(20.0) > 0.999);
        assert!(f.inv_link(-20.0) < 0.001);
        // variance peaks at μ=0.5.
        assert!((f.variance(0.5) - 0.25).abs() < 1e-12);
    }

    #[test]
    fn poisson_link() {
        let f = Family::Poisson;
        assert!((f.inv_link(0.0) - 1.0).abs() < 1e-12);
        assert!((f.variance(3.0) - 3.0).abs() < 1e-12);
    }

    #[test]
    fn deviance_is_zero_at_perfect_fit() {
        assert!(Family::Poisson.unit_deviance(4.0, 4.0).abs() < 1e-9);
        assert!(Family::Binomial.unit_deviance(1.0, 1.0 - 1e-13).abs() < 1e-6);
    }
}
