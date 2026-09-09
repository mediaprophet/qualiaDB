//! Tokenless send pacing. Eligibility is a deadline, not a credit ledger.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::rtt::RttEstimator;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pacer {
    pub next_send_us: u64,
    pub rate_bytes_per_s: u64,
}

impl Pacer {
    pub const fn new(rate_bytes_per_s: u64) -> Self {
        Self {
            next_send_us: 0,
            rate_bytes_per_s,
        }
    }
}

impl Default for Pacer {
    fn default() -> Self {
        Self::new(0)
    }
}

/// [`QdnfError::WouldBlock`] when `now_us` is still before the pacing deadline.
pub fn wait_until(pacer: &Pacer, now_us: u64) -> Result<(), QdnfError> {
    if now_us < pacer.next_send_us {
        Err(QdnfError::WouldBlock)
    } else {
        Ok(())
    }
}

/// Charge `bytes` against the pacing rate and push `next_send_us`.
pub fn on_paced_send(pacer: &mut Pacer, bytes: u64, now_us: u64) -> Result<(), QdnfError> {
    wait_until(pacer, now_us)?;
    if pacer.rate_bytes_per_s == 0 {
        return Err(QdnfError::Incomplete);
    }
    if bytes == 0 {
        return Err(QdnfError::Range);
    }
    let interval = bytes.saturating_mul(1_000_000) / pacer.rate_bytes_per_s;
    pacer.next_send_us = now_us.saturating_add(interval.max(1));
    Ok(())
}

/// Rate ≈ window / smoothed_rtt (RFC 9002 pacing). Unknown RTT fails closed.
pub fn set_rate_from_window_rtt(
    pacer: &mut Pacer,
    window_bytes: u64,
    rtt: &RttEstimator,
) -> Result<(), QdnfError> {
    let rtt_us = crate::net::qdnf::session::rtt::smoothed_or_err(rtt)? as u64;
    if window_bytes == 0 {
        return Err(QdnfError::Range);
    }
    pacer.rate_bytes_per_s = window_bytes.saturating_mul(1_000_000) / rtt_us;
    if pacer.rate_bytes_per_s == 0 {
        pacer.rate_bytes_per_s = 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::session::rtt::on_ack_sample;

    #[test]
    fn wait_until_would_block_before_deadline() {
        let pacer = Pacer {
            next_send_us: 1000,
            rate_bytes_per_s: 1200,
        };
        assert_eq!(wait_until(&pacer, 999), Err(QdnfError::WouldBlock));
        assert_eq!(wait_until(&pacer, 1000), Ok(()));
        assert_eq!(wait_until(&pacer, 1001), Ok(()));
    }

    #[test]
    fn paced_send_advances_deadline() {
        let mut pacer = Pacer::new(1_000_000);
        on_paced_send(&mut pacer, 500, 0).unwrap();
        assert_eq!(pacer.next_send_us, 500);
        assert_eq!(on_paced_send(&mut pacer, 1, 0), Err(QdnfError::WouldBlock));
        on_paced_send(&mut pacer, 1, 500).unwrap();
        assert!(pacer.next_send_us > 500);
    }

    #[test]
    fn unknown_rtt_does_not_invent_a_rate() {
        let mut pacer = Pacer::new(0);
        let rtt = RttEstimator::new();
        assert_eq!(
            set_rate_from_window_rtt(&mut pacer, 1200, &rtt),
            Err(QdnfError::Incomplete)
        );
        let mut rtt = RttEstimator::new();
        on_ack_sample(&mut rtt, 1000);
        set_rate_from_window_rtt(&mut pacer, 1200, &rtt).unwrap();
        assert_eq!(pacer.rate_bytes_per_s, 1_200_000);
    }
}
