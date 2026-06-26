//! Histogram binning — fills a caller-owned counts buffer, no allocation.
//!
//! Canonical home for equal-width binning. The caller owns the `counts` slice
//! (its length sets the bin count) and the returned range describes the binning.

use super::descriptive::{max, min};

/// Range/width metadata for a binning pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HistRange {
    pub min: f64,
    pub max: f64,
    pub bin_width: f64,
}

/// Equal-width histogram into the caller-owned `counts` buffer (zeroed first).
/// The number of bins is `counts.len()`. Values outside [min, max] are skipped;
/// a value landing on/above the top edge falls in the last bin. `None` if the
/// data is empty or `counts` is empty.
pub fn histogram_into(values: &[f64], counts: &mut [u32]) -> Option<HistRange> {
    let bins = counts.len();
    if values.is_empty() || bins == 0 {
        return None;
    }
    let min_v = min(values)?;
    let max_v = max(values)?;
    let bin_width = (max_v - min_v) / bins as f64;

    for c in counts.iter_mut() {
        *c = 0;
    }

    for &v in values {
        if v < min_v || v > max_v {
            continue;
        }
        let bin_index = if !bin_width.is_finite() || bin_width <= 0.0 {
            // Degenerate range (all values equal): everything in bin 0.
            0
        } else {
            let idx = ((v - min_v) / bin_width) as usize;
            if idx >= bins {
                bins - 1
            } else {
                idx
            }
        };
        counts[bin_index] += 1;
    }

    Some(HistRange {
        min: min_v,
        max: max_v,
        bin_width,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-12;

    #[test]
    fn guards_empty() {
        assert_eq!(histogram_into(&[], &mut [0u32; 4]), None);
        assert_eq!(histogram_into(&[1.0, 2.0], &mut []), None);
    }

    #[test]
    fn bins_uniform_data_and_conserves_count() {
        let v = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let mut counts = [0u32; 5];
        let r = histogram_into(&v, &mut counts).unwrap();
        assert!((r.min - 0.0).abs() < EPS);
        assert!((r.max - 10.0).abs() < EPS);
        assert!((r.bin_width - 2.0).abs() < EPS);
        assert_eq!(counts.iter().sum::<u32>(), v.len() as u32); // every point binned
    }

    #[test]
    fn degenerate_all_equal_goes_to_bin_zero() {
        let v = [5.0, 5.0, 5.0];
        let mut counts = [0u32; 3];
        let r = histogram_into(&v, &mut counts).unwrap();
        assert_eq!(r.bin_width, 0.0);
        assert_eq!(counts[0], 3);
    }

    #[test]
    fn buffer_is_zeroed_first() {
        let v = [1.0, 2.0];
        let mut counts = [99u32; 2]; // dirty buffer
        histogram_into(&v, &mut counts).unwrap();
        assert_eq!(counts.iter().sum::<u32>(), 2);
    }
}
