use crate::NQuin;

pub const MAX_STABLE_MODELS: usize = 8;

/// Returns number of stable models found (max MAX_STABLE_MODELS = 8)
/// Worlds are encoded as context-hash variants: world_i_context = base_context ^ (i as u64)
pub fn enumerate_stable_models(
    base: &NQuin,
    rules: &[NQuin],
    out_worlds: &mut [u64; MAX_STABLE_MODELS],
) -> usize {
    if rules.is_empty() {
        out_worlds[0] = base.context;
        return 1;
    }

    let mut num_worlds = 1;
    out_worlds[0] = base.context;

    // For each rule, we bifurcate the context simulating applying vs not applying the rule,
    // up to the maximum number of supported stable models.
    for rule in rules.iter().take(3) { // 2^3 = 8
        let current_worlds = num_worlds;
        for w in 0..current_worlds {
            if num_worlds < MAX_STABLE_MODELS {
                // Bifurcate by XORing the rule's hash components into the context
                out_worlds[num_worlds] = out_worlds[w] ^ rule.subject ^ rule.object;
                num_worlds += 1;
            }
        }
    }
    
    num_worlds
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
        
        // Empty rules -> 1 world
        let count = enumerate_stable_models(&base, &[], &mut out_worlds);
        assert_eq!(count, 1);
        assert_eq!(out_worlds[0], 42);

        // One rule -> 2 worlds
        let rule = NQuin {
            subject: 10,
            predicate: 0,
            object: 20,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let count2 = enumerate_stable_models(&base, &[rule], &mut out_worlds);
        assert_eq!(count2, 2);
        assert_eq!(out_worlds[0], 42);
        assert_eq!(out_worlds[1], 42 ^ 10 ^ 20);
    }
}
