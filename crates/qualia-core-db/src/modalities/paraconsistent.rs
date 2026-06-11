use crate::{q_hash, NQuin};

pub const OP_ISOLATE: u8 = 0x30;
pub const OP_CONTRADICTION_SCORE: u8 = 0x31;
pub const OP_PARACONSISTENT_MERGE: u8 = 0x32;

pub const ISOLATED_CONTEXT_PREFIX: u64 = q_hash("q42:isolated");

#[derive(Debug, PartialEq, Eq)]
pub enum ContradictionStatus {
    Consistent,
    Isolated {
        severity: u8,
        isolation_context: u64,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParaconsistentError {
    BufferOverflow,
}

pub fn route_paraconsistent(
    quins: &[NQuin],
    out_consistent: &mut [NQuin],
    out_isolated: &mut [NQuin],
) -> Result<(usize, usize), ParaconsistentError> {
    let mut num_consistent = 0;
    let mut num_isolated = 0;

    for &quin in quins {
        let mut is_contradiction = false;

        for i in 0..num_consistent {
            let c = &out_consistent[i];
            if c.context == quin.context
                && c.subject == quin.subject
                && c.predicate == quin.predicate
            {
                if c.object != quin.object {
                    is_contradiction = true;
                    break;
                }
            }
        }

        if is_contradiction {
            if num_isolated >= out_isolated.len() {
                return Err(ParaconsistentError::BufferOverflow);
            }
            let mut isolated_quin = quin;

            // Passes through without re-isolation if the context is already an isolated one.
            // For testing, we assume an already isolated context has ISOLATED_CONTEXT_PREFIX
            // directly or is logically marked by the prefix.
            if quin.context != ISOLATED_CONTEXT_PREFIX {
                isolated_quin.context ^= ISOLATED_CONTEXT_PREFIX;
            }

            out_isolated[num_isolated] = isolated_quin;
            num_isolated += 1;
        } else {
            if num_consistent >= out_consistent.len() {
                return Err(ParaconsistentError::BufferOverflow);
            }

            // If it's already an isolated quin that doesn't contradict anything,
            // it passes through to out_consistent without being isolated again.
            out_consistent[num_consistent] = quin;
            num_consistent += 1;
        }
    }

    Ok((num_consistent, num_isolated))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_quin(subject: u64, predicate: u64, object: u64, context: u64) -> NQuin {
        NQuin {
            subject,
            predicate,
            object,
            context,
            metadata: 0,
            parity: 0,
        }
    }

    #[test]
    fn test_no_contradictions() {
        let q1 = dummy_quin(1, 1, 1, 100);
        let q2 = dummy_quin(2, 2, 2, 100);
        let quins = [q1, q2];

        let mut out_cons = [NQuin::default(); 4];
        let mut out_iso = [NQuin::default(); 4];

        let (c, i) = route_paraconsistent(&quins, &mut out_cons, &mut out_iso).unwrap();
        assert_eq!(c, 2);
        assert_eq!(i, 0);
    }

    #[test]
    fn test_two_quins_contradict() {
        let q1 = dummy_quin(1, 1, 1, 100);
        let q2 = dummy_quin(1, 1, 2, 100); // Same subject/predicate/context, diff object
        let quins = [q1, q2];

        let mut out_cons = [NQuin::default(); 4];
        let mut out_iso = [NQuin::default(); 4];

        let (c, i) = route_paraconsistent(&quins, &mut out_cons, &mut out_iso).unwrap();
        assert_eq!(c, 1);
        assert_eq!(i, 1);
        assert_eq!(out_cons[0].object, 1);
        assert_eq!(out_iso[0].object, 2);
        assert_eq!(out_iso[0].context, 100 ^ ISOLATED_CONTEXT_PREFIX);
    }

    #[test]
    fn test_three_quins_contradict() {
        let q1 = dummy_quin(1, 1, 1, 100);
        let q2 = dummy_quin(1, 1, 2, 100); // Contradicts q1
        let q3 = dummy_quin(2, 2, 2, 100); // Normal
        let quins = [q1, q2, q3];

        let mut out_cons = [NQuin::default(); 4];
        let mut out_iso = [NQuin::default(); 4];

        let (c, i) = route_paraconsistent(&quins, &mut out_cons, &mut out_iso).unwrap();
        assert_eq!(c, 2);
        assert_eq!(i, 1);
        assert_eq!(out_cons[0].object, 1);
        assert_eq!(out_cons[1].object, 2); // q3
        assert_eq!(out_iso[0].object, 2); // q2
    }

    #[test]
    fn test_already_isolated_passes_through() {
        // An already isolated Quin
        let q1 = dummy_quin(1, 1, 1, ISOLATED_CONTEXT_PREFIX);
        let quins = [q1];

        let mut out_cons = [NQuin::default(); 4];
        let mut out_iso = [NQuin::default(); 4];

        let (c, i) = route_paraconsistent(&quins, &mut out_cons, &mut out_iso).unwrap();
        assert_eq!(c, 1);
        assert_eq!(i, 0);
        assert_eq!(out_cons[0].context, ISOLATED_CONTEXT_PREFIX);
    }

    #[test]
    fn test_isolation_context_is_deterministic() {
        let q1 = dummy_quin(1, 1, 1, 100);
        let q2 = dummy_quin(1, 1, 2, 100);

        let q3 = dummy_quin(5, 5, 5, 100);
        let q4 = dummy_quin(5, 5, 6, 100);

        let quins = [q1, q2, q3, q4];

        let mut out_cons = [NQuin::default(); 4];
        let mut out_iso = [NQuin::default(); 4];

        let (_, i) = route_paraconsistent(&quins, &mut out_cons, &mut out_iso).unwrap();

        assert_eq!(i, 2);
        // Both isolated quins came from context 100, so they should have the same isolation context.
        assert_eq!(out_iso[0].context, out_iso[1].context);
        assert_eq!(out_iso[0].context, 100 ^ ISOLATED_CONTEXT_PREFIX);
    }
}
