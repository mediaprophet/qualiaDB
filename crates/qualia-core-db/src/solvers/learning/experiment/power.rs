//! Power analysis & sample-size (Practical Statistics ch 3) — how many observations
//! an experiment needs to detect an effect, and the power it achieves at a given
//! size. Honest experiment planning: state the effect you care about and the
//! confidence you need, get the sample size you must collect. Reuses the Normal
//! quantile/CDF from `statistics::distributions`.

use crate::solvers::statistics::distributions::normal;

/// Required sample size **per group** for a two-sample comparison of means, given a
/// standardized effect size `d` (Cohen's d = mean difference / pooled SD), two-sided
/// significance `alpha`, and desired `power` (e.g. 0.8). `None` if the inputs are
/// out of range. Uses the standard normal approximation
/// `n = 2·(z_{1−α/2} + z_power)² / d²`.
pub fn required_sample_size_two_sample(d: f64, alpha: f64, power: f64) -> Option<usize> {
    if d == 0.0
        || !(0.0..1.0).contains(&alpha)
        || alpha <= 0.0
        || !(0.0..1.0).contains(&power)
        || power <= 0.0
    {
        return None;
    }
    let za = normal::standard_quantile(1.0 - alpha / 2.0);
    let zb = normal::standard_quantile(power);
    let n = 2.0 * (za + zb).powi(2) / (d * d);
    Some(n.ceil() as usize)
}

/// Achieved power of a two-sample mean test with `n` per group, effect size `d`,
/// two-sided `alpha`. `None` on bad inputs.
pub fn power_two_sample(n: usize, d: f64, alpha: f64) -> Option<f64> {
    if n == 0 || !(0.0..1.0).contains(&alpha) || alpha <= 0.0 {
        return None;
    }
    let za = normal::standard_quantile(1.0 - alpha / 2.0);
    // Non-centrality on the standardized scale; one-direction normal approximation.
    let ncp = d.abs() * (n as f64 / 2.0).sqrt();
    Some(normal::standard_cdf(ncp - za))
}

/// Required sample size per group to detect a difference between two proportions
/// `p1` vs `p2` at two-sided `alpha` and `power`. `None` on bad inputs / equal props.
pub fn required_sample_size_two_proportion(
    p1: f64,
    p2: f64,
    alpha: f64,
    power: f64,
) -> Option<usize> {
    if !(0.0..=1.0).contains(&p1) || !(0.0..=1.0).contains(&p2) || (p1 - p2).abs() < 1e-12 {
        return None;
    }
    if !(0.0..1.0).contains(&alpha) || alpha <= 0.0 || !(0.0..1.0).contains(&power) || power <= 0.0
    {
        return None;
    }
    let za = normal::standard_quantile(1.0 - alpha / 2.0);
    let zb = normal::standard_quantile(power);
    let pbar = 0.5 * (p1 + p2);
    let num = (za * (2.0 * pbar * (1.0 - pbar)).sqrt()
        + zb * (p1 * (1.0 - p1) + p2 * (1.0 - p2)).sqrt())
    .powi(2);
    let n = num / (p1 - p2).powi(2);
    Some(n.ceil() as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_size_grows_as_effect_shrinks() {
        let big = required_sample_size_two_sample(0.8, 0.05, 0.8).unwrap();
        let small = required_sample_size_two_sample(0.2, 0.05, 0.8).unwrap();
        assert!(
            small > big,
            "smaller effect needs more data: {small} !> {big}"
        );
        // Known textbook value: d=0.5, α=0.05, power=0.8 → ~63–64 per group.
        let mid = required_sample_size_two_sample(0.5, 0.05, 0.8).unwrap();
        assert!((62..=65).contains(&mid), "n per group {mid}");
    }

    #[test]
    fn power_and_sample_size_are_consistent() {
        // Compute n for target power, then the achieved power should be ≥ target.
        let d = 0.5;
        let n = required_sample_size_two_sample(d, 0.05, 0.8).unwrap();
        let achieved = power_two_sample(n, d, 0.05).unwrap();
        assert!(achieved >= 0.8 - 1e-3, "achieved power {achieved}");
    }

    #[test]
    fn power_rises_with_sample_size() {
        let lo = power_two_sample(10, 0.4, 0.05).unwrap();
        let hi = power_two_sample(200, 0.4, 0.05).unwrap();
        assert!(hi > lo && hi <= 1.0);
    }

    #[test]
    fn proportion_sample_size() {
        // Detecting 0.10 vs 0.12 needs many samples; 0.10 vs 0.30 needs few.
        let subtle = required_sample_size_two_proportion(0.10, 0.12, 0.05, 0.8).unwrap();
        let obvious = required_sample_size_two_proportion(0.10, 0.30, 0.05, 0.8).unwrap();
        assert!(subtle > obvious);
    }

    #[test]
    fn guards() {
        assert_eq!(required_sample_size_two_sample(0.0, 0.05, 0.8), None);
        assert_eq!(power_two_sample(0, 0.5, 0.05), None);
        assert_eq!(
            required_sample_size_two_proportion(0.2, 0.2, 0.05, 0.8),
            None
        );
    }
}
