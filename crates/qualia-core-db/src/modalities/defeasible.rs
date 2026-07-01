use crate::NQuin;

pub const OP_DEFEASIBLE_OVERRIDE: u8 = 0x50;
// Shared with deontic logic; canonical bit position lives in the FrameLayout ABI.
pub use crate::frame_layout::DEFEATER_BIT;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DefeasibleStatus {
    Strict,     // No exceptions allowed
    Overridden, // Defeater proved true
    Defeated,   // Normal rule defeated
    Active,     // Normal rule stands
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

        out[count] = DefeasibleVerdict { claim: *q, status };
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

// ─── Defeasible Logic: strict / defeasible / defeater rules + superiority ───────────
//
// Defeasible Logic (Nute / Governatori) layers three rule kinds and a superiority relation:
//   * STRICT rules — indefeasible (their conclusion always holds when fired).
//   * DEFEASIBLE rules — hold unless defeated by a superior opposing rule.
//   * DEFEATERS — cannot conclude on their own; they only BLOCK an opposing defeasible rule.
// When two rules conclude complementary literals, the superiority relation decides; if neither
// is superior the outcome is ambiguous, handled either by blocking or propagating ambiguity.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleKind {
    Strict,
    Defeasible,
    Defeater,
}

/// A minimal defeasible rule: an id, its kind, the literal it concludes, and the polarity
/// (`positive`: concludes `literal`; else concludes `¬literal`).
#[derive(Debug, Clone, Copy)]
pub struct DefeasibleRule {
    pub id: u64,
    pub kind: RuleKind,
    pub literal: u64,
    pub positive: bool,
}

/// How unresolved ambiguity (neither conflicting rule superior) is treated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbiguityMode {
    /// Ambiguity is **blocked**: neither conclusion is drawn (`Undecided`).
    Blocking,
    /// Ambiguity is **propagated**: the literal is marked `Ambiguous` downstream.
    Propagating,
}

/// The conclusion drawn for a literal after conflict resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conclusion {
    Positive,
    Negative,
    Ambiguous,
    Undecided,
}

/// Two rules **conflict** iff they conclude the same literal with opposite polarity.
pub fn rules_conflict(a: &DefeasibleRule, b: &DefeasibleRule) -> bool {
    a.literal == b.literal && a.positive != b.positive
}

/// Superiority lookup: is `a` superior to `b` in the supplied relation `sup` (pairs
/// `(higher, lower)`)?
pub fn is_superior(sup: &[(u64, u64)], a: u64, b: u64) -> bool {
    sup.iter().any(|&(hi, lo)| hi == a && lo == b)
}

#[inline]
fn polarity(r: &DefeasibleRule) -> Conclusion {
    if r.positive {
        Conclusion::Positive
    } else {
        Conclusion::Negative
    }
}

/// A `Defeater` can only block an opponent — it never supports a conclusion. Strict and
/// defeasible rules can conclude.
#[inline]
fn can_conclude(kind: RuleKind) -> bool {
    matches!(kind, RuleKind::Strict | RuleKind::Defeasible)
}

/// Resolve a conflict between two opposing rules, given the superiority relation and ambiguity
/// mode. Non-conflicting inputs yield `a`'s polarity. Semantics (Nute / Governatori):
///  - A `Strict` rule dominates a non-strict opponent.
///  - Otherwise a side concludes only if its rule is **superior** to the opponent AND can
///    conclude (a superior `Defeater` merely blocks → `Undecided`, never its own polarity).
///  - With neither superior, the stand-off is `Undecided` (blocking) or `Ambiguous` (propagating).
pub fn resolve_conflict(
    a: &DefeasibleRule,
    b: &DefeasibleRule,
    sup: &[(u64, u64)],
    mode: AmbiguityMode,
) -> Conclusion {
    if !rules_conflict(a, b) {
        return polarity(a);
    }
    // Strict indefeasibility.
    match (a.kind, b.kind) {
        (RuleKind::Strict, k) if k != RuleKind::Strict => return polarity(a),
        (k, RuleKind::Strict) if k != RuleKind::Strict => return polarity(b),
        _ => {}
    }
    // Explicit superiority — but a superior Defeater only blocks; it cannot conclude.
    if is_superior(sup, a.id, b.id) {
        return if can_conclude(a.kind) {
            polarity(a)
        } else {
            Conclusion::Undecided
        };
    }
    if is_superior(sup, b.id, a.id) {
        return if can_conclude(b.kind) {
            polarity(b)
        } else {
            Conclusion::Undecided
        };
    }
    // Neither superior → genuine stand-off (an applicable opponent, even a defeater, blocks).
    match mode {
        AmbiguityMode::Blocking => Conclusion::Undecided,
        AmbiguityMode::Propagating => Conclusion::Ambiguous,
    }
}

// ─── Integration with the Dung argumentation framework (grounded extension) ─────────
//
// Defeasible rules map naturally onto Dung's abstract argumentation: each rule is an argument,
// conflicting rules attack each other, and the superiority relation orients the attack (the
// superior rule defeats the inferior; a strict rule is never attacked back). The skeptical
// GROUNDED extension then resolves which conclusions are justified — and is exactly the
// ambiguity-BLOCKING reading (an un-oriented conflict leaves both out). Defeaters block but never
// conclude, so they are excluded from the returned conclusions.
//
// NOTE: this bridge intentionally uses the heap-based `argumentation` module; the defeasible
// *core* above stays zero-heap. It is an additive convenience over `grounded_extension`.

/// Resolve a defeasible rule set against the Dung **grounded extension**: returns the set of rule
/// ids whose conclusion is justified (skeptically), excluding defeaters. Conflicting rules attack
/// mutually unless the superiority relation (or strictness) orients the attack one way.
pub fn grounded_justified_rules(
    rules: &[DefeasibleRule],
    sup: &[(u64, u64)],
) -> std::collections::HashSet<u64> {
    use crate::modalities::argumentation::{Argument, ArgumentationFramework, Attack, AttackType};

    let mut af = ArgumentationFramework::new();
    for r in rules {
        // Encode the conclusion literal+polarity into a NQuin (polarity in the predicate).
        let concl = NQuin {
            subject: r.literal,
            predicate: r.positive as u64,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        af.add_argument(Argument::new(r.id, String::new(), Vec::new(), concl));
    }
    for (i, a) in rules.iter().enumerate() {
        for b in &rules[i + 1..] {
            if !rules_conflict(a, b) {
                continue;
            }
            let a_strict = a.kind == RuleKind::Strict;
            let b_strict = b.kind == RuleKind::Strict;
            let a_sup = is_superior(sup, a.id, b.id);
            let b_sup = is_superior(sup, b.id, a.id);
            // `a` attacks `b` unless `b` is a strict rule `a` is not, or `b` is strictly superior.
            let a_attacks_b = !(b_strict && !a_strict) && !b_sup;
            let b_attacks_a = !(a_strict && !b_strict) && !a_sup;
            if a_attacks_b {
                af.add_attack(Attack {
                    attacker: a.id,
                    target: b.id,
                    attack_type: AttackType::Rebuttal,
                    strength: 1.0,
                });
            }
            if b_attacks_a {
                af.add_attack(Attack {
                    attacker: b.id,
                    target: a.id,
                    attack_type: AttackType::Rebuttal,
                    strength: 1.0,
                });
            }
        }
    }
    af.grounded_extension()
        .into_iter()
        .filter(|id| {
            !rules
                .iter()
                .any(|r| r.id == *id && r.kind == RuleKind::Defeater)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(id: u64, kind: RuleKind, lit: u64, positive: bool) -> DefeasibleRule {
        DefeasibleRule {
            id,
            kind,
            literal: lit,
            positive,
        }
    }

    #[test]
    fn strict_dominates_and_superiority_decides() {
        let lit = q_hash("penguin:flies");
        let r_pos = rule(1, RuleKind::Defeasible, lit, true); // birds fly
        let r_neg = rule(2, RuleKind::Defeasible, lit, false); // penguins don't
        assert!(rules_conflict(&r_pos, &r_neg));

        // No superiority, both defeasible → ambiguity (mode-dependent).
        assert_eq!(
            resolve_conflict(&r_pos, &r_neg, &[], AmbiguityMode::Blocking),
            Conclusion::Undecided
        );
        assert_eq!(
            resolve_conflict(&r_pos, &r_neg, &[], AmbiguityMode::Propagating),
            Conclusion::Ambiguous
        );

        // "penguins don't fly" is superior → Negative concluded.
        let sup = [(2u64, 1u64)];
        assert_eq!(
            resolve_conflict(&r_pos, &r_neg, &sup, AmbiguityMode::Blocking),
            Conclusion::Negative
        );

        // A strict opposing rule dominates regardless of superiority.
        let r_strict = rule(3, RuleKind::Strict, lit, false);
        assert_eq!(
            resolve_conflict(&r_pos, &r_strict, &[], AmbiguityMode::Blocking),
            Conclusion::Negative
        );
    }

    #[test]
    fn defeater_only_blocks_it_cannot_conclude() {
        let lit = q_hash("claim:x");
        let r = rule(1, RuleKind::Defeasible, lit, true);
        let d = rule(2, RuleKind::Defeater, lit, false);
        // No superiority: the applicable defeater blocks r; nothing is concluded.
        assert_eq!(
            resolve_conflict(&r, &d, &[], AmbiguityMode::Blocking),
            Conclusion::Undecided
        );
        // A SUPERIOR defeater still cannot conclude its own polarity — r is defeated → Undecided.
        let sup_d = [(2u64, 1u64)];
        assert_eq!(
            resolve_conflict(&r, &d, &sup_d, AmbiguityMode::Blocking),
            Conclusion::Undecided
        );
        // When the defeasible rule is superior to the defeater, it concludes.
        let sup_r = [(1u64, 2u64)];
        assert_eq!(
            resolve_conflict(&r, &d, &sup_r, AmbiguityMode::Blocking),
            Conclusion::Positive
        );
    }

    #[test]
    fn non_conflicting_rules_just_conclude() {
        let r1 = rule(1, RuleKind::Defeasible, q_hash("a"), true);
        let r2 = rule(2, RuleKind::Defeasible, q_hash("b"), false);
        assert!(!rules_conflict(&r1, &r2));
        assert_eq!(
            resolve_conflict(&r1, &r2, &[], AmbiguityMode::Blocking),
            Conclusion::Positive
        );
    }

    #[test]
    fn grounded_extension_resolves_defeasible_conflict() {
        let flies = q_hash("penguin:flies");
        let r_bird = rule(1, RuleKind::Defeasible, flies, true); // birds fly
        let r_peng = rule(2, RuleKind::Defeasible, flies, false); // penguins don't

        // No superiority → mutual attack → grounded extension is skeptical → neither justified.
        let none = grounded_justified_rules(&[r_bird, r_peng], &[]);
        assert!(
            !none.contains(&1) && !none.contains(&2),
            "un-oriented conflict: neither justified"
        );

        // "penguins don't fly" superior → only r2 attacks r1 → r2 justified, r1 defeated.
        let sup = [(2u64, 1u64)];
        let g = grounded_justified_rules(&[r_bird, r_peng], &sup);
        assert!(g.contains(&2) && !g.contains(&1));

        // A non-conflicting rule is always justified (no attackers).
        let r_other = rule(3, RuleKind::Defeasible, q_hash("swims"), true);
        let g2 = grounded_justified_rules(&[r_bird, r_peng, r_other], &sup);
        assert!(g2.contains(&3));

        // A defeater blocks but is never itself a justified conclusion.
        let d = rule(4, RuleKind::Defeater, q_hash("claim"), false);
        let r_c = rule(5, RuleKind::Defeasible, q_hash("claim"), true);
        let gd = grounded_justified_rules(&[r_c, d], &[]);
        assert!(!gd.contains(&4), "defeater never concludes");
    }

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
