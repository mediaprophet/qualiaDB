pub fn median_f64(values: &[f64]) -> f64 {
    percentile_nearest_rank(values, 0.5)
}

pub fn percentile_nearest_rank(values: &[f64], quantile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let q = quantile.clamp(0.0, 1.0);
    let rank = ((q * sorted.len() as f64).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len() - 1);
    sorted[rank]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statistics_are_deterministic_for_odd_and_even_inputs() {
        assert_eq!(median_f64(&[5.0, 1.0, 3.0]), 3.0);
        assert_eq!(median_f64(&[4.0, 1.0, 3.0, 2.0]), 2.0);
        assert_eq!(percentile_nearest_rank(&[1.0, 2.0, 3.0, 4.0], 0.95), 4.0);
        assert_eq!(median_f64(&[]), 0.0);
    }
}
