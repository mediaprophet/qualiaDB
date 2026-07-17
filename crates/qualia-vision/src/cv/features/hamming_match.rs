//! Brute-force Hamming match for binary descriptors.

use crate::cv::error::CvError;
use crate::cv::features::brief_desc_u8::DESC_LEN;

#[derive(Debug, Clone, Copy, Default)]
pub struct Match {
    pub query_idx: u32,
    pub train_idx: u32,
    pub distance: u16,
}

fn hamming(a: &[u8], b: &[u8]) -> u16 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x ^ y).count_ones() as u16)
        .sum()
}

/// Match query descriptors to train. `out` one match per query (best).
pub fn hamming_match(
    query: &[u8],
    n_query: usize,
    train: &[u8],
    n_train: usize,
    out: &mut [Match],
) -> Result<usize, CvError> {
    if n_query == 0 || n_train == 0 {
        return Ok(0);
    }
    if query.len() < n_query * DESC_LEN || train.len() < n_train * DESC_LEN {
        return Err(CvError::BufferTooSmall);
    }
    let n = n_query.min(out.len());
    for qi in 0..n {
        let q = &query[qi * DESC_LEN..(qi + 1) * DESC_LEN];
        let mut best_d = u16::MAX;
        let mut best_t = 0u32;
        for ti in 0..n_train {
            let t = &train[ti * DESC_LEN..(ti + 1) * DESC_LEN];
            let d = hamming(q, t);
            if d < best_d {
                best_d = d;
                best_t = ti as u32;
            }
        }
        out[qi] = Match {
            query_idx: qi as u32,
            train_idx: best_t,
            distance: best_d,
        };
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identical_zero() {
        let d = [0u8; DESC_LEN];
        let mut m = [Match::default(); 1];
        let n = hamming_match(&d, 1, &d, 1, &mut m).unwrap();
        assert_eq!(n, 1);
        assert_eq!(m[0].distance, 0);
    }
}
