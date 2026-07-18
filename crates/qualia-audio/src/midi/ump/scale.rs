//! MIDI 2.0 default resolution scaling (Min-Center-Max) between bit widths.
//!
//! MIDI 1.0 carries 7-bit / 14-bit values; MIDI 2.0 carries 16-bit / 32-bit
//! values. Up-scaling uses the "bit-repeat" expansion from the MIDI 2.0 UMP
//! specification so that minimum, center, and maximum map exactly (e.g. 7-bit
//! `0x40` center scales to the 16-bit center). Down-scaling is a right shift.

/// Scale `value` from `src_bits` up to `dst_bits` using the MIDI 2.0
/// Min-Center-Max bit-repeat algorithm. `src_bits <= dst_bits <= 32`.
///
/// Behaviour matches the MIDI Association reference: 0 maps to 0; values at or
/// below the source center are a plain left shift; values above center repeat
/// their low bits into the freed low bits so the maximum saturates.
pub fn scale_up(value: u32, src_bits: u32, dst_bits: u32) -> u32 {
    if value == 0 {
        return 0;
    }
    if src_bits >= dst_bits {
        return value;
    }
    if src_bits == 1 {
        // A single bit set expands to all-ones of the destination width.
        return (1u32 << dst_bits).wrapping_sub(1);
    }
    let scale_bits = dst_bits - src_bits;
    let mut bit_shifted = value << scale_bits;
    let src_center = 1u32 << (src_bits - 1);
    if value <= src_center {
        return bit_shifted;
    }
    // Expanded bit-repeat for the upper half.
    let repeat_bits = src_bits - 1;
    let repeat_mask = (1u32 << repeat_bits) - 1;
    let mut repeat_value = value & repeat_mask;
    if scale_bits > repeat_bits {
        repeat_value <<= scale_bits - repeat_bits;
    } else {
        repeat_value >>= repeat_bits - scale_bits;
    }
    while repeat_value != 0 {
        bit_shifted |= repeat_value;
        repeat_value >>= repeat_bits;
    }
    bit_shifted
}

/// Scale `value` from `src_bits` down to `dst_bits` (a right shift by the width
/// difference). `dst_bits <= src_bits`.
#[inline]
pub fn scale_down(value: u32, src_bits: u32, dst_bits: u32) -> u32 {
    if dst_bits >= src_bits {
        return value;
    }
    value >> (src_bits - dst_bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn velocity_7_to_16_reference_value() {
        // MIDI 1.0 velocity 100 (0x64) -> 16-bit 0xC924 per the reference algo.
        assert_eq!(scale_up(100, 7, 16), 0xC924);
        // ... and back to 100.
        assert_eq!(scale_down(0xC924, 16, 7), 100);
    }

    #[test]
    fn endpoints_and_center() {
        assert_eq!(scale_up(0, 7, 16), 0);
        assert_eq!(scale_up(127, 7, 16), 0xFFFF); // max saturates
        assert_eq!(scale_up(0x40, 7, 16), 0x8000); // center -> center
    }

    #[test]
    fn pitch_bend_14_to_32() {
        // 14-bit center 8192 (0x2000) -> 32-bit center 0x80000000.
        assert_eq!(scale_up(8192, 14, 32), 0x8000_0000);
        assert_eq!(scale_down(0x8000_0000, 32, 14), 8192);
        assert_eq!(scale_up(0, 14, 32), 0);
        assert_eq!(scale_up(0x3FFF, 14, 32), 0xFFFF_FFFF); // max
    }

    #[test]
    fn roundtrip_all_7bit() {
        for v in 0u32..=127 {
            let up = scale_up(v, 7, 16);
            assert_eq!(scale_down(up, 16, 7), v, "roundtrip failed for {v}");
        }
    }
}
