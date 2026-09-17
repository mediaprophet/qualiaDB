//! RFC 9002-style RTT estimator for QSession (not QUIC wire-compatible).
//!
//! Integer microseconds only. First sample seeds smoothed RTT and rttvar;
//! later samples use the 7/8 and 3/4 EWMA from RFC 9002 §5.

use crate::net::qdnf::errors::QdnfError;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RttEstimator {
    pub latest_us: u32,
    pub min_us: u32,
    pub smoothed_us: u32,
    pub rttvar_us: u32,
}

impl RttEstimator {
    pub const fn new() -> Self {
        Self {
            latest_us: 0,
            min_us: 0,
            smoothed_us: 0,
            rttvar_us: 0,
        }
    }
}

impl Default for RttEstimator {
    fn default() -> Self {
        Self::new()
    }
}

/// Absorb one ACK delay-adjusted sample. `sample_us == 0` is ignored.
pub fn on_ack_sample(est: &mut RttEstimator, sample_us: u32) {
    if sample_us == 0 {
        return;
    }
    est.latest_us = sample_us;
    if est.min_us == 0 || sample_us < est.min_us {
        est.min_us = sample_us;
    }
    if est.smoothed_us == 0 {
        est.smoothed_us = sample_us;
        est.rttvar_us = sample_us / 2;
        return;
    }
    let diff = abs_u32(est.smoothed_us, sample_us);
    est.rttvar_us = (est.rttvar_us.saturating_mul(3).saturating_add(diff)) / 4;
    est.smoothed_us = (est.smoothed_us.saturating_mul(7).saturating_add(sample_us)) / 8;
}

#[inline]
fn abs_u32(a: u32, b: u32) -> u32 {
    if a >= b {
        a - b
    } else {
        b - a
    }
}

/// Pacing / PTO helper. Zero smoothed RTT is not a usable estimate.
pub fn smoothed_or_err(est: &RttEstimator) -> Result<u32, QdnfError> {
    if est.smoothed_us == 0 {
        Err(QdnfError::Incomplete)
    } else {
        Ok(est.smoothed_us)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_sample_seeds_min_smoothed_and_rttvar() {
        let mut est = RttEstimator::new();
        on_ack_sample(&mut est, 1000);
        assert_eq!(est.latest_us, 1000);
        assert_eq!(est.min_us, 1000);
        assert_eq!(est.smoothed_us, 1000);
        assert_eq!(est.rttvar_us, 500);
        on_ack_sample(&mut est, 0);
        assert_eq!(est.smoothed_us, 1000);
    }

    #[test]
    fn later_sample_tracks_min_and_ewma() {
        let mut est = RttEstimator::new();
        on_ack_sample(&mut est, 1000);
        on_ack_sample(&mut est, 500);
        assert_eq!(est.min_us, 500);
        assert_eq!(est.latest_us, 500);
        assert!(est.smoothed_us < 1000);
        assert!(est.smoothed_us > 500);
        assert_eq!(smoothed_or_err(&est), Ok(est.smoothed_us));
    }

    #[test]
    fn unknown_rtt_is_incomplete() {
        assert_eq!(
            smoothed_or_err(&RttEstimator::new()),
            Err(QdnfError::Incomplete)
        );
    }
}
