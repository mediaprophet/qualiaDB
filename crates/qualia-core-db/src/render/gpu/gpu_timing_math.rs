const MARKERS: usize = 6;
const BYTES: u64 = 48;
pub(super) fn decode(bytes: &[u8], period_ns: f64) -> Option<[f64; MARKERS]> {
    if bytes.len() != BYTES as usize || !period_ns.is_finite() || period_ns <= 0.0 {
        return None;
    }
    let mut ticks = [0u64; MARKERS];
    for (out, b) in ticks.iter_mut().zip(bytes.chunks_exact(8)) {
        *out = u64::from_le_bytes(b.try_into().ok()?);
    }
    if ticks.iter().all(|&t| t == 0) || ticks.windows(2).any(|p| p[1] < p[0]) {
        return None;
    }
    let mut result = [0.0; MARKERS];
    for i in 0..MARKERS - 1 {
        result[i] = (ticks[i + 1] - ticks[i]) as f64 * period_ns / 1_000_000.0;
    }
    result[MARKERS - 1] = (ticks[MARKERS - 1] - ticks[0]) as f64 * period_ns / 1_000_000.0;
    result.iter().all(|t| t.is_finite()).then_some(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_ticks_are_converted_with_actual_queue_period() {
        let ticks = [100u64, 110, 130, 130, 230, 240];
        let mut bytes = [0u8; 48];
        for (out, value) in bytes.chunks_exact_mut(8).zip(ticks) {
            out.copy_from_slice(&value.to_le_bytes());
        }
        assert_eq!(
            decode(&bytes, 1000.0),
            Some([0.01, 0.02, 0.0, 0.1, 0.01, 0.14])
        );
    }
    #[test]
    fn absent_disjoint_or_invalid_timestamps_do_not_become_measurements() {
        assert!(decode(&[0; 48], 1.0).is_none());
        assert!(decode(&[1; 47], 1.0).is_none());
        assert!(decode(&[1; 48], f64::NAN).is_none());
        let mut bytes = [0u8; 48];
        for (out, value) in bytes
            .chunks_exact_mut(8)
            .zip([100u64, 110, 105, 120, 130, 140])
        {
            out.copy_from_slice(&value.to_le_bytes());
        }
        assert!(decode(&bytes, 1.0).is_none());
    }
}
