//! Content hash for spans — same 60-bit FNV-1a as `lexicon::generate_60bit_token`.

pub use crate::lexicon::generate_60bit_token as hash60;

#[cfg(test)]
mod tests {
    use super::hash60;

    #[test]
    fn matches_engine_lexicon_hash() {
        let a = hash60(b"North Spring");
        let b = crate::lexicon::generate_60bit_token(b"North Spring");
        assert_eq!(a, b);
        assert_eq!(a & 0xF000_0000_0000_0000, 0);
    }
}
