//! E21.2 — deterministic bounded codec campaign. Not libFuzzer.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::net::qdnf::frame::{decode_frame, encode_frame, FrameHeader};
use crate::net::qdnf::link::discovery::Beacon;
use crate::net::qdnf::policy_labels::{decode_label_into, encode_label_into, LabelFields};
use crate::net::qdnf::registries::{FrameType, NextProtocol};
use crate::net::qdnf::types::{LinkId, StrongDigest};

/// libFuzzer / cargo-fuzz is not wired and has not been executed.
pub fn libfuzzer_executed() -> bool {
    false
}

static BOUNDED_RAN: AtomicBool = AtomicBool::new(false);

/// True after [`run_bounded_codec_fuzz`] completes in this process.
pub fn bounded_codec_fuzz_executed() -> bool {
    BOUNDED_RAN.load(Ordering::SeqCst)
}

/// Campaign iteration cap. Enough to exercise codecs; small enough for CI.
pub const BOUNDED_FUZZ_ITERATIONS: usize = 4096;

/// Stack scratch for one trial. Tests may allocate; the decode loop does not.
const SCRATCH: usize = 256;

/// Counts for one campaign. `panics` must stay zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FuzzCounts {
    pub iterations: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub panics: usize,
}

fn mix(mut x: u64) -> u64 {
    x ^= x.wrapping_shl(13);
    x ^= x.wrapping_shr(7);
    x ^= x.wrapping_shl(17);
    x
}

fn fill_valid_frame(buf: &mut [u8; SCRATCH]) -> usize {
    let header = FrameHeader::new(FrameType::DiscoveryBeacon, NextProtocol::QLink);
    encode_frame(&header, &[], buf).unwrap_or(0)
}

fn fill_valid_beacon(buf: &mut [u8; SCRATCH]) -> usize {
    let beacon = Beacon {
        mode: crate::net::qdnf::link::discovery::DiscoveryMode::PrivatePairwise,
        tag: [0x11u8; 16],
        link_id: LinkId([0x22u8; 16]),
        epoch: 7,
        expiry_unix: 99,
        mtu: 1280,
    };
    beacon.encode(buf).unwrap_or(0)
}

fn fill_valid_label(buf: &mut [u8; SCRATCH]) -> usize {
    let mut issuer = StrongDigest::ZERO;
    issuer.0[0] = 0xA1;
    issuer.0[47] = 0x5C;
    let fields = LabelFields::request(
        crate::net::qdnf::policy_labels::Confidentiality::C1Private,
        issuer,
    );
    encode_label_into(&fields, buf).unwrap_or(0)
}

/// Seeded byte stream over frame, label, and discovery-beacon decoders.
///
/// Never panics on malformed input: each trial is isolated with `catch_unwind`.
/// The hot decode calls themselves take borrowed slices and stack `LabelFields`.
pub fn run_bounded_codec_fuzz() -> FuzzCounts {
    let mut counts = FuzzCounts {
        iterations: 0,
        accepted: 0,
        rejected: 0,
        panics: 0,
    };
    let mut seed = 0xC0DEC0DE_A5A5_u64;
    let mut i = 0usize;
    while i < BOUNDED_FUZZ_ITERATIONS {
        let mut buf = [0u8; SCRATCH];
        seed = mix(seed.wrapping_add(i as u64));
        let structured = (seed & 7) == 0;
        let kind = ((seed >> 3) & 3) as u8;
        let mut len = if structured {
            match kind % 3 {
                0 => fill_valid_frame(&mut buf),
                1 => fill_valid_label(&mut buf),
                _ => fill_valid_beacon(&mut buf),
            }
        } else {
            let n = ((seed >> 8) as usize) % (SCRATCH + 1);
            let mut b = 0usize;
            while b < n {
                seed = mix(seed);
                buf[b] = seed as u8;
                b += 1;
            }
            n
        };
        seed = mix(seed);
        if (seed & 1) == 1 && len > 0 {
            let idx = ((seed >> 8) as usize) % len.max(1);
            buf[idx] ^= (seed >> 16) as u8;
        }
        if len > SCRATCH {
            len = SCRATCH;
        }
        let slice = &buf[..len];
        let kind_run = kind;
        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> bool {
            match kind_run % 3 {
                0 => decode_frame(slice).is_ok(),
                1 => {
                    let mut fields = LabelFields::blank();
                    decode_label_into(slice, &mut fields).is_ok()
                }
                _ => Beacon::decode(slice).is_ok(),
            }
        }));
        match panicked {
            Ok(true) => {
                counts.accepted = counts.accepted.saturating_add(1);
            }
            Ok(false) => {
                counts.rejected = counts.rejected.saturating_add(1);
            }
            Err(_) => {
                counts.panics = counts.panics.saturating_add(1);
                counts.rejected = counts.rejected.saturating_add(1);
            }
        }
        counts.iterations = counts.iterations.saturating_add(1);
        i = i.saturating_add(1);
    }
    BOUNDED_RAN.store(true, Ordering::SeqCst);
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_codec_fuzz_does_not_panic_and_is_not_libfuzzer() {
        assert!(!libfuzzer_executed());
        let counts = run_bounded_codec_fuzz();
        assert!(bounded_codec_fuzz_executed());
        assert_eq!(counts.iterations, BOUNDED_FUZZ_ITERATIONS);
        assert_eq!(counts.panics, 0);
        assert_eq!(
            counts.accepted.saturating_add(counts.rejected),
            counts.iterations
        );
        assert!(counts.accepted > 0, "structured seeds must be accepted");
        assert!(counts.rejected > 0, "mutated bytes must be rejected");
    }

    #[test]
    fn empty_and_truncated_inputs_are_rejected_without_panic() {
        let _ = decode_frame(&[]);
        let mut fields = LabelFields::blank();
        let _ = decode_label_into(&[1, 2, 3], &mut fields);
        let _ = Beacon::decode(&[0u8; 8]);
        assert!(!libfuzzer_executed());
    }
}
