//! Demo `.10d` badge visual: a valid header so the collectable is a 10D handle.
//!
//! This is a **placeholder manifold**, not a finished animated mesh. SI-09 can
//! replace it with an authored badge. Axis `t` is present in the header taxonomy
//! so later frames/animation remain in-format.

use crate::container_10d::{Container10dHeader, HEADER_BYTE_SIZE};

/// Media type recorded on the manifest for a `.10d` badge.
pub const VISUAL_MEDIA_TYPE: &str = "application/vnd.qualia.10d";

/// A 64-byte valid `.10d` v1 header (Refuse default disposition). No section
/// table — inspectable as a living-container stub, not as artwork-as-proof.
pub fn demo_badge_10d() -> [u8; HEADER_BYTE_SIZE] {
    Container10dHeader::proposed().encode_to_vec64()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_10d::{Container10dHeader, MAGIC_10D};

    #[test]
    fn demo_visual_parses_as_10d_header() {
        let bytes = demo_badge_10d();
        assert_eq!(&bytes[..4], &MAGIC_10D);
        Container10dHeader::parse(&bytes).expect("valid 10d header");
    }
}
