//! Value-based Argumentation Frameworks (Bench-Capon) — attacks succeed only when the attacker's
//! value is not less preferred than the target's. This lets a **human-rights hierarchy** decide
//! conflicts: an attack from a less-important value cannot defeat an argument grounded in a
//! more-important (e.g. non-derogable) right. The result projects to a standard Dung framework of
//! successful *defeats*, over which the usual semantics apply.

use super::ArgumentationFramework;
use std::collections::{HashMap, HashSet};

/// A Dung framework augmented with a value per argument and an audience preference over values.
#[derive(Debug, Clone)]
pub struct ValueArgumentationFramework {
    pub af: ArgumentationFramework,
    /// argument id → its value (e.g. a human-rights principle hash).
    pub value_of: HashMap<u64, u64>,
    /// value → audience preference rank (higher = more important / more preferred).
    pub value_rank: HashMap<u64, u32>,
}

impl ValueArgumentationFramework {
    pub fn new(af: ArgumentationFramework) -> Self {
        Self { af, value_of: HashMap::new(), value_rank: HashMap::new() }
    }

    /// Assign argument `arg` the value `value`.
    pub fn set_value(&mut self, arg: u64, value: u64) {
        self.value_of.insert(arg, value);
    }

    /// Set the audience preference `rank` for `value` (higher = more preferred).
    pub fn set_rank(&mut self, value: u64, rank: u32) {
        self.value_rank.insert(value, rank);
    }

    fn rank_of_arg(&self, arg: u64) -> u32 {
        self.value_of
            .get(&arg)
            .and_then(|v| self.value_rank.get(v))
            .copied()
            .unwrap_or(0)
    }

    /// An attack `attacker → target` **defeats** `target` iff
    /// `rank(value(attacker)) >= rank(value(target))` — an attack from a less-preferred value
    /// cannot defeat a more-preferred (e.g. non-derogable rights) argument.
    pub fn defeats(&self, attacker: u64, target: u64) -> bool {
        self.rank_of_arg(attacker) >= self.rank_of_arg(target)
    }

    /// Project to the Dung framework of successful **defeats** (attacks that pass the value test).
    pub fn defeat_framework(&self) -> ArgumentationFramework {
        let mut out = ArgumentationFramework::new();
        for arg in self.af.arguments.values() {
            out.add_argument(arg.clone());
        }
        for atk in &self.af.attacks {
            if self.defeats(atk.attacker, atk.target) {
                out.add_attack(atk.clone());
            }
        }
        out
    }

    /// Grounded extension under the audience's value preference.
    pub fn grounded_extension(&self) -> HashSet<u64> {
        self.defeat_framework().grounded_extension()
    }

    /// Preferred extensions under the audience's value preference.
    pub fn preferred_extensions(&self) -> Vec<HashSet<u64>> {
        self.defeat_framework().preferred_extensions()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{Argument, Attack, AttackType};
    use crate::NQuin;

    fn arg(id: u64) -> Argument {
        Argument::new(id, String::new(), Vec::new(), NQuin::default())
    }
    fn atk(a: u64, b: u64) -> Attack {
        Attack { attacker: a, target: b, attack_type: AttackType::Rebuttal, strength: 1.0 }
    }

    #[test]
    fn higher_value_argument_survives_a_mutual_attack() {
        // a ↔ b mutually attack. In a plain AF, neither is grounded.
        let mut af = ArgumentationFramework::new();
        af.add_argument(arg(1));
        af.add_argument(arg(2));
        af.add_attack(atk(1, 2));
        af.add_attack(atk(2, 1));
        assert!(af.grounded_extension().is_empty(), "plain AF: mutual attack → no acceptance");

        // Now value(1)=rights (rank 10), value(2)=convenience (rank 1).
        let mut vaf = ValueArgumentationFramework::new(af);
        let rights = crate::q_hash("value:nonDerogableRight");
        let convenience = crate::q_hash("value:convenience");
        vaf.set_value(1, rights);
        vaf.set_value(2, convenience);
        vaf.set_rank(rights, 10);
        vaf.set_rank(convenience, 1);

        // 1 defeats 2 (10 >= 1); 2 does NOT defeat 1 (1 < 10). So only 1→2 survives → grounded {1}.
        assert!(vaf.defeats(1, 2));
        assert!(!vaf.defeats(2, 1));
        let g = vaf.grounded_extension();
        assert!(g.contains(&1) && !g.contains(&2), "the rights-grounded argument prevails");
    }
}
