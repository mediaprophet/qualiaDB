use crate::{NQuin, q_hash};

pub const OP_DEFEASIBLE_OVERRIDE: u8 = 0x50;
pub const DEFEATER_BIT: u64 = 1u64 << 63; // Shared with deontic logic

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DefeasibleStatus {
    Strict,      // No exceptions allowed
    Overridden,  // Defeater proved true
    Defeated,    // Normal rule defeated
    Active,      // Normal rule stands
}

#[derive(Debug)]
pub enum DefeasibleError {
    BufferOverflow,
}

#[derive(Debug, Clone, Copy)]
pub struct DefeasibleVerdict {
    pub claim: NQuin,
    pub status: DefeasibleStatus,
}

/// Evaluates a slice of Quins for non-monotonic (defeasible) reasoning.
pub fn evaluate_defeasible_frame(
    quins: &[NQuin],
    context_hash: u64,
    out: &mut [DefeasibleVerdict],
) -> Result<usize, DefeasibleError> {
    let mut count = 0;

    for q in quins {
        if context_hash != 0 && q.context != context_hash {
            continue;
        }

        let is_defeater = (q.predicate & DEFEATER_BIT) != 0;
        let opcode = (q.predicate & 0xFF) as u8;

        let status = if is_defeater {
            DefeasibleStatus::Overridden
        } else if opcode == OP_DEFEASIBLE_OVERRIDE {
            DefeasibleStatus::Defeated
        } else {
            DefeasibleStatus::Active
        };

        if count >= out.len() {
            return Err(DefeasibleError::BufferOverflow);
        }

        out[count] = DefeasibleVerdict {
            claim: *q,
            status,
        };
        count += 1;
    }

    Ok(count)
}

/// Negation-as-failure / closed-world assumption: a proposition holds "by default"
/// (its negation is concluded) exactly when it CANNOT be proven from the closed set
/// of `facts` — i.e. the `(subject, predicate, object)` triple is absent. This is
/// the non-monotonic primitive the positive forward-chainer (`fire_guard_rules`)
/// cannot express; the agent-honesty guard's "Unverified until proven" is an
/// instance. Zero-heap (single linear scan).
pub fn holds_by_default(facts: &[NQuin], goal: &NQuin) -> bool {
    !facts.iter().any(|q| {
        q.subject == goal.subject && q.predicate == goal.predicate && q.object == goal.object
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defeasible_evaluation() {
        let mut out = Vec::with_capacity(10);
        for _ in 0..10 {
            out.push(DefeasibleVerdict {
                claim: NQuin::default(),
                status: DefeasibleStatus::Strict,
            });
        }
        
        let ctx = q_hash("test_context");
        
        let mut q_normal = NQuin::default();
        q_normal.context = ctx;
        q_normal.predicate = OP_DEFEASIBLE_OVERRIDE as u64;

        let mut q_defeater = NQuin::default();
        q_defeater.context = ctx;
        q_defeater.predicate = DEFEATER_BIT | (OP_DEFEASIBLE_OVERRIDE as u64);

        let quins = [q_normal, q_defeater];

        let count = evaluate_defeasible_frame(&quins, ctx, &mut out).unwrap();
        assert_eq!(count, 2);
        assert_eq!(out[0].status, DefeasibleStatus::Defeated);
        assert_eq!(out[1].status, DefeasibleStatus::Overridden);
    }
}
