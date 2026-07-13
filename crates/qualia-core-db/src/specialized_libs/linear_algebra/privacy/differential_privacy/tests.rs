use super::*;

struct LcgNoise(u64);

impl NoiseSource for LcgNoise {
    fn fill_bytes(&mut self, destination: &mut [u8]) -> Result<(), PrivacyError> {
        for chunk in destination.chunks_mut(8) {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            chunk.copy_from_slice(&self.0.to_le_bytes()[..chunk.len()]);
        }
        Ok(())
    }
}

#[test]
fn laplace_release_is_deterministic_for_a_supplied_noise_stream() {
    let mut first =
        DifferentialPrivacy::with_budget(2.0, 1e-5, CompositionMethod::BasicComposition).unwrap();
    let mut second =
        DifferentialPrivacy::with_budget(2.0, 1e-5, CompositionMethod::BasicComposition).unwrap();
    let query = [2.0, 4.0, 8.0];
    let mut out_a = [0.0; 3];
    let mut out_b = [0.0; 3];
    first
        .release_laplace_with_noise_into(&query, 1.0, 0.5, &mut LcgNoise(7), &mut out_a)
        .unwrap();
    second
        .release_laplace_with_noise_into(&query, 1.0, 0.5, &mut LcgNoise(7), &mut out_b)
        .unwrap();
    assert_eq!(out_a, out_b);
    assert_ne!(out_a, query);
}

#[test]
fn basic_composition_fails_closed_at_the_budget_boundary() {
    let mut dp =
        DifferentialPrivacy::with_budget(1.0, 1e-5, CompositionMethod::BasicComposition).unwrap();
    let mut out = [0.0; 1];
    dp.release_laplace_with_noise_into(&[10.0], 1.0, 0.6, &mut LcgNoise(1), &mut out)
        .unwrap();
    assert_eq!(
        dp.release_laplace_with_noise_into(&[10.0], 1.0, 0.5, &mut LcgNoise(2), &mut out,),
        Err(PrivacyError::BudgetExceeded)
    );
    assert_eq!(dp.privacy_accountant.releases(), 1);
    assert!((dp.privacy_budget.remaining_epsilon - 0.4).abs() < 1e-12);
}

#[test]
fn gaussian_calibration_matches_the_classic_formula() {
    let sigma = gaussian_sigma(2.0, 0.5, 1e-6).unwrap();
    let expected = 2.0 * (2.0 * (1.25_f64 / 1e-6).ln()).sqrt() / 0.5;
    assert!((sigma - expected).abs() < 1e-12);
    assert_eq!(
        gaussian_sigma(1.0, 1.1, 1e-6),
        Err(PrivacyError::InvalidEpsilon)
    );
}

#[test]
fn gaussian_release_charges_epsilon_and_delta_once_per_vector() {
    let mut dp =
        DifferentialPrivacy::with_budget(1.0, 1e-5, CompositionMethod::BasicComposition).unwrap();
    let mut out = [0.0; 4];
    assert_eq!(
        dp.release_gaussian_with_noise_into(
            &[1.0, 2.0, 3.0, 4.0],
            2.0,
            0.4,
            2e-6,
            &mut LcgNoise(9),
            &mut out,
        )
        .unwrap(),
        4
    );
    assert_eq!(dp.privacy_accountant.releases(), 1);
    assert!((dp.privacy_budget.remaining_epsilon - 0.6).abs() < 1e-12);
    assert!((dp.privacy_budget.remaining_delta - 8e-6).abs() < 1e-15);
}

#[test]
fn zero_sensitivity_copies_without_spending_budget() {
    let mut dp = DifferentialPrivacy::new();
    let mut out = [0.0; 2];
    dp.release_laplace_with_noise_into(&[3.0, 4.0], 0.0, 0.5, &mut LcgNoise(1), &mut out)
        .unwrap();
    assert_eq!(out, [3.0, 4.0]);
    assert_eq!(dp.privacy_accountant.releases(), 0);
}

#[test]
fn invalid_inputs_and_short_buffers_fail_before_release() {
    let mut dp = DifferentialPrivacy::new();
    assert_eq!(
        dp.release_laplace_with_noise_into(&[1.0, 2.0], 1.0, 0.5, &mut LcgNoise(1), &mut [0.0],),
        Err(PrivacyError::OutputBufferTooSmall)
    );
    assert_eq!(
        dp.release_gaussian_with_noise_into(
            &[f64::NAN],
            1.0,
            0.5,
            1e-7,
            &mut LcgNoise(1),
            &mut [0.0],
        ),
        Err(PrivacyError::NonFiniteInput)
    );
    assert_eq!(dp.privacy_accountant.releases(), 0);
}

#[test]
fn rdp_accounting_accepts_gaussian_and_rejects_laplace() {
    let method = CompositionMethod::RdpComposition {
        order: 8.0,
        target_delta: 1e-6,
    };
    let mut gaussian = DifferentialPrivacy::with_budget(10.0, 1e-5, method).unwrap();
    gaussian
        .release_gaussian_with_noise_into(&[1.0], 1.0, 0.5, 1e-7, &mut LcgNoise(1), &mut [0.0])
        .unwrap();
    assert_eq!(gaussian.privacy_accountant.releases(), 1);

    let mut laplace = DifferentialPrivacy::with_budget(10.0, 1e-5, method).unwrap();
    assert_eq!(
        laplace.release_laplace_with_noise_into(&[1.0], 1.0, 0.5, &mut LcgNoise(1), &mut [0.0],),
        Err(PrivacyError::UnsupportedComposition)
    );
}

#[test]
fn advanced_composition_uses_generalized_bound() {
    let slack = 1e-6;
    let mut dp = DifferentialPrivacy::with_budget(
        10.0,
        1e-4,
        CompositionMethod::AdvancedComposition { delta_slack: slack },
    )
    .unwrap();
    let mut noise = LcgNoise(5);
    let mut out = [0.0; 1];
    dp.release_laplace_with_noise_into(&[1.0], 1.0, 0.2, &mut noise, &mut out)
        .unwrap();
    dp.release_laplace_with_noise_into(&[1.0], 1.0, 0.3, &mut noise, &mut out)
        .unwrap();

    let sum_squared = 0.2_f64.powi(2) + 0.3_f64.powi(2);
    let correction = 0.2 * 0.2_f64.exp_m1() + 0.3 * 0.3_f64.exp_m1();
    let expected = (2.0 * (1.0 / slack).ln() * sum_squared).sqrt() + correction;
    assert!((dp.privacy_accountant.total_epsilon_spent - expected).abs() < 1e-12);
    assert_eq!(dp.privacy_accountant.total_delta_spent, slack);
    assert_eq!(dp.privacy_accountant.releases(), 2);
}
