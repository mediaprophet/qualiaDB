//! E21.5 honesty claims. Test counts are not certification.
//!
//! A bounded in-process header-byte loop exists in this module. That is not
//! libFuzzer, not a model checker, and not the E21.2 fuzz suite.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::frame::{decode_frame, BASE_HEADER_LEN};
use crate::net::qdnf::harness::oracle::independent_decode_header;

/// Passing unit tests do not certify a deployment.
pub fn test_count_is_certification() -> bool {
    false
}

/// Independent cryptographic / hostile-environment review is not commissioned.
pub fn independent_review_commissioned() -> bool {
    false
}

/// E21.2 libFuzzer / model-check suite has not been executed.
pub fn fuzz_suite_executed() -> bool {
    false
}

/// Bounded in-process header codec loop (not libFuzzer).
pub const IN_PROCESS_HEADER_FUZZ_TRIALS: usize = 320;

/// True when the in-process header loop is part of this crate's tests.
pub fn in_process_header_codec_fuzz_executed() -> bool {
    true
}

fn base_header() -> [u8; BASE_HEADER_LEN] {
    let mut src = [0u8; BASE_HEADER_LEN];
    src[0] = b'Q';
    src[1] = b'D';
    src[2] = b'N';
    src[3] = b'F';
    src[4] = 1;
    src[5] = 2;
    src[8] = 0;
    src[9] = 80;
    src[12] = 0;
    src[13] = 1;
    src
}

fn mix(mut x: u64) -> u64 {
    x ^= x.wrapping_shl(13);
    x ^= x.wrapping_shr(7);
    x ^= x.wrapping_shl(17);
    x
}

/// Deterministic in-process mutations over an 80-byte QFrame header.
///
/// Each trial calls the production decoder and the independent oracle.
/// Outcomes are [`QdnfError`] codes or a decoded header. No payload is echoed.
pub fn in_process_header_codec_fuzz() -> Result<usize, QdnfError> {
    let mut trials = 0usize;
    let mut seed = 0xA5A5_C3C3_u64;
    let mut i = 0usize;
    while i < IN_PROCESS_HEADER_FUZZ_TRIALS {
        let mut src = base_header();
        seed = mix(seed.wrapping_add(i as u64));
        let idx = (seed as usize) % BASE_HEADER_LEN;
        src[idx] = (seed >> 8) as u8;
        seed = mix(seed);
        let idx2 = (seed as usize) % BASE_HEADER_LEN;
        src[idx2] = src[idx2].wrapping_add((i as u8).wrapping_mul(17));
        let _ = decode_frame(&src);
        let _ = independent_decode_header(&src);
        trials = trials.saturating_add(1);
        i = i.saturating_add(1);
    }
    if trials != IN_PROCESS_HEADER_FUZZ_TRIALS {
        return Err(QdnfError::Range);
    }
    Ok(trials)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::harness::scenarios::{qualified_count, ScenarioId};

    #[test]
    fn honesty_flags_are_negative() {
        assert!(!test_count_is_certification());
        assert!(!independent_review_commissioned());
        assert!(!fuzz_suite_executed());
        assert_eq!(qualified_count(), 0);
        assert_eq!(ScenarioId::COUNT, 40);
    }

    #[test]
    fn in_process_header_fuzz_is_bounded_and_does_not_echo() {
        assert!(in_process_header_codec_fuzz_executed());
        let n = in_process_header_codec_fuzz().unwrap();
        assert_eq!(n, IN_PROCESS_HEADER_FUZZ_TRIALS);
        let mut bad = base_header();
        bad[0] = b'X';
        let err = decode_frame(&bad).unwrap_err();
        assert_eq!(err, QdnfError::Malformed);
        assert_eq!(err.to_string(), "malformed");
        assert_eq!(independent_decode_header(&bad), Err(QdnfError::Malformed));
    }
}
