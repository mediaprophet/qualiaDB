use crate::NQuin;

pub const MAX_STABLE_MODELS: usize = 8;

/// Returns number of stable models found (max MAX_STABLE_MODELS = 8)
/// Worlds are encoded as context-hash variants: world_i_context = base_context ^ (i as u64)
pub fn enumerate_stable_models(
    base: &NQuin,
    _rules: &[NQuin],
    out_worlds: &mut [u64; MAX_STABLE_MODELS],
) -> usize {
    // For MVP, we'll just mock generating two parallel realities
    out_worlds[0] = base.context ^ 0;
    out_worlds[1] = base.context ^ 1;
    2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_stable_models() {
        let base = NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 42,
            metadata: 0,
            parity: 0,
        };
        let mut out_worlds = [0; MAX_STABLE_MODELS];
        let count = enumerate_stable_models(&base, &[], &mut out_worlds);
        assert_eq!(count, 2);
        assert_eq!(out_worlds[0], 42 ^ 0);
        assert_eq!(out_worlds[1], 42 ^ 1);
    }
}
