//! E21.4 — bounded-seconds churn. Not days-long soak.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use crate::net::qdnf::bearer::{ipc_pair, Bearer};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::frame::{decode_frame, encode_frame, FrameHeader};
use crate::net::qdnf::harness::allocation::ReservationTracker;
use crate::net::qdnf::harness::oracles::{plaintext_on_wire, ProtectedView};
use crate::net::qdnf::harness::partition::PartitionGate;
use crate::net::qdnf::registries::{FrameType, NextProtocol};
use crate::net::qdnf::types::ScopeEpoch;

/// Multi-day churn / soak has not been executed.
pub fn long_duration_churn_days_executed() -> bool {
    false
}

static CHURN_RAN: AtomicBool = AtomicBool::new(false);
static CHURN_MS: AtomicU64 = AtomicU64::new(0);

/// True after [`run_bounded_churn`] completes in this process.
pub fn bounded_churn_executed() -> bool {
    CHURN_RAN.load(Ordering::SeqCst)
}

/// Wall time actually waited in the last [`run_bounded_churn`] call, or 0.
pub fn last_churn_duration_ms() -> u64 {
    CHURN_MS.load(Ordering::SeqCst)
}

/// Cap so CI stays sane. Seconds, not days.
pub const CHURN_CAP_MS: u64 = 2_000;

const APP_SECRET: &[u8] = b"qpr-app-secret-never-on-wire";
const PAYLOAD: [u8; 8] = [0x9a, 0x13, 0x44, 0x80, 0x02, 0x77, 0x11, 0xce];

fn one_round(pool: &mut ReservationTracker, round: u32) -> Result<(), QdnfError> {
    let gate = PartitionGate::open();
    gate.admit()?;
    let lease = pool.reserve(1, 64)?;
    if pool.live_bytes() != pool.occupied_bytes() {
        return Err(QdnfError::Conflict);
    }

    let mut header = FrameHeader::new(FrameType::DiscoveryBeacon, NextProtocol::QLink);
    header.payload_len = PAYLOAD.len() as u16;
    let mut wire = [0u8; 128];
    let n = encode_frame(&header, &PAYLOAD, &mut wire)?;
    let (decoded, off, len) = decode_frame(&wire[..n])?;
    if decoded.payload_len as usize != PAYLOAD.len() || off != 80 || len != PAYLOAD.len() {
        return Err(QdnfError::Malformed);
    }
    let view = ProtectedView {
        wire_payload: &wire[..n],
        application: APP_SECRET,
    };
    if plaintext_on_wire(view) {
        return Err(QdnfError::Denied);
    }

    let scope = ScopeEpoch {
        scope: 1,
        epoch: 1u64.wrapping_add(round as u64),
    };
    let (mut a, mut b) = ipc_pair(scope, 1280)?;
    a.send(&b.locator(), &wire[..n])?;
    let mut out = [0u8; 128];
    let (got, _meta) = b.recv(&mut out)?;
    let _ = decode_frame(&out[..got])?;
    a.shutdown()?;
    b.shutdown()?;

    pool.release(lease)?;
    if pool.live_bytes() != 0 || pool.occupied_bytes() != 0 {
        return Err(QdnfError::Conflict);
    }
    Ok(())
}

/// Timed pair/encode/decode loop with conservation and confidentiality checks.
pub fn run_bounded_churn() -> Result<u64, QdnfError> {
    CHURN_MS.store(0, Ordering::SeqCst);
    let start = Instant::now();
    let mut pool = ReservationTracker::new();
    let mut round = 0u32;
    loop {
        let elapsed = start.elapsed();
        if elapsed.as_millis() as u64 >= CHURN_CAP_MS {
            break;
        }
        one_round(&mut pool, round)?;
        round = round.wrapping_add(1);
        if round == 0 {
            break;
        }
    }
    let waited = start.elapsed().as_millis() as u64;
    CHURN_MS.store(waited, Ordering::SeqCst);
    CHURN_RAN.store(true, Ordering::SeqCst);
    if waited == 0 && round == 0 {
        return Err(QdnfError::Incomplete);
    }
    Ok(waited)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_churn_waits_seconds_not_days_and_conserves() {
        assert!(!long_duration_churn_days_executed());
        let waited = run_bounded_churn().unwrap();
        assert!(bounded_churn_executed());
        assert!(!long_duration_churn_days_executed());
        assert_eq!(last_churn_duration_ms(), waited);
        assert!(
            waited >= CHURN_CAP_MS,
            "must actually wait the cap, got {waited} ms"
        );
        assert!(
            waited < 5_000,
            "must stay under five seconds for CI, got {waited} ms"
        );
    }
}
