use crate::QualiaQuin;

pub const SYNTHESIZED_BIT: u64 = 1u64 << 58;

pub fn synthesize_dialectical(thesis: &QualiaQuin, antithesis: &QualiaQuin) -> Option<QualiaQuin> {
    // A contradiction requires the same subject and predicate but different object
    if thesis.subject == antithesis.subject
        && thesis.predicate == antithesis.predicate
        && thesis.object != antithesis.object
    {
        let mut synthesized = *thesis;
        synthesized.context = thesis.context ^ antithesis.context;
        synthesized.metadata |= SYNTHESIZED_BIT;
        // The object becomes a combination, maybe just bitwise XOR for now?
        synthesized.object = thesis.object ^ antithesis.object;

        // Update parity to maintain structural integrity
        synthesized.parity =
            synthesized.subject ^ synthesized.predicate ^ synthesized.object ^ synthesized.context;

        return Some(synthesized);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthesize_dialectical() {
        let thesis = QualiaQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 10,
            metadata: 0,
            parity: 0,
        };
        let antithesis = QualiaQuin {
            subject: 1,
            predicate: 2,
            object: 4,
            context: 20,
            metadata: 0,
            parity: 0,
        };

        let syn = synthesize_dialectical(&thesis, &antithesis).unwrap();
        assert_eq!(syn.context, 10 ^ 20);
        assert_eq!(syn.metadata & SYNTHESIZED_BIT, SYNTHESIZED_BIT);

        let no_contradiction = QualiaQuin {
            subject: 1,
            predicate: 3,
            object: 4,
            context: 20,
            metadata: 0,
            parity: 0,
        };
        assert!(synthesize_dialectical(&thesis, &no_contradiction).is_none());
    }
}
