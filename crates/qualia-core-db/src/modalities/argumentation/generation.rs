//! Dynamic argument-generation engine — build a Dung framework directly from raw deontic / LTL
//! trace verdicts. Each verdict becomes an argument concluding a signed literal; verdicts with
//! **complementary** conclusions (same literal, opposite polarity) attack each other. This is the
//! bridge from the engine's modal/deontic traces into abstract argumentation, so a conflict in
//! the trace becomes a resolvable debate.

use super::{Argument, ArgumentationFramework, Attack, AttackType};
use crate::NQuin;

/// Build a framework from trace `entries` `(arg_id, conclusion_literal, positive)`: each becomes
/// an argument; any two with the same `conclusion_literal` and opposite `positive` mutually
/// attack (a rebuttal). The conclusion is encoded into the argument's `conclusion_quin`
/// (`subject = literal`, `predicate = polarity`).
pub fn framework_from_trace(entries: &[(u64, u64, bool)]) -> ArgumentationFramework {
    let mut af = ArgumentationFramework::new();
    for &(id, lit, positive) in entries {
        let mut concl = NQuin {
            subject: lit,
            predicate: positive as u64,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        concl.parity = concl.subject ^ concl.predicate ^ concl.object ^ concl.context;
        af.add_argument(Argument::new(id, String::new(), Vec::new(), concl));
    }
    for (i, &(id_a, lit_a, pos_a)) in entries.iter().enumerate() {
        for &(id_b, lit_b, pos_b) in &entries[i + 1..] {
            if lit_a == lit_b && pos_a != pos_b {
                af.add_attack(Attack {
                    attacker: id_a,
                    target: id_b,
                    attack_type: AttackType::Rebuttal,
                    strength: 1.0,
                });
                af.add_attack(Attack {
                    attacker: id_b,
                    target: id_a,
                    attack_type: AttackType::Rebuttal,
                    strength: 1.0,
                });
            }
        }
    }
    af
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complementary_verdicts_become_a_debate() {
        let permit = crate::q_hash("act:disclose");
        // Trace: arg 1 concludes (disclose, +); arg 2 concludes (disclose, −); arg 3 unrelated.
        let af = framework_from_trace(&[
            (1, permit, true),
            (2, permit, false),
            (3, crate::q_hash("act:other"), true),
        ]);
        assert_eq!(af.arguments.len(), 3);
        // 1 and 2 conflict (mutual attack); 3 is independent.
        assert!(af.attacks.iter().any(|a| a.attacker == 1 && a.target == 2));
        assert!(af.attacks.iter().any(|a| a.attacker == 2 && a.target == 1));
        let g = af.grounded_extension();
        assert!(
            !g.contains(&1) && !g.contains(&2),
            "the conflict is undecided in the grounded extension"
        );
        assert!(g.contains(&3), "the independent verdict stands");
    }

    #[test]
    fn agreeing_verdicts_do_not_attack() {
        let lit = crate::q_hash("act:x");
        // Two arguments concluding the SAME polarity do not conflict.
        let af = framework_from_trace(&[(1, lit, true), (2, lit, true)]);
        assert!(af.attacks.is_empty());
        let g = af.grounded_extension();
        assert!(g.contains(&1) && g.contains(&2));
    }
}
