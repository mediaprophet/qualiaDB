//! Bipolar Argumentation (Cayrol & Lagasquie-Schiex) — adds a **support** relation alongside
//! attack and derives the *complex attacks* it induces (deductive-support semantics):
//!   * **supported attack:**  a supports* b  ∧  b attacks c   ⇒   a attacks c
//!   * **secondary attack:**  a attacks b    ∧  c supports* b  ⇒   a attacks c
//! Projecting these derived attacks back into a Dung framework lets the standard semantics apply.

use super::{ArgumentationFramework, Attack, AttackType};
use std::collections::HashSet;

/// A Dung framework augmented with a binary support relation.
#[derive(Debug, Clone)]
pub struct BipolarFramework {
    pub af: ArgumentationFramework,
    /// `(supporter, supported)` edges.
    pub supports: Vec<(u64, u64)>,
}

impl BipolarFramework {
    pub fn new(af: ArgumentationFramework) -> Self {
        Self {
            af,
            supports: Vec::new(),
        }
    }

    /// Add a support edge `supporter → supported`.
    pub fn add_support(&mut self, supporter: u64, supported: u64) {
        self.supports.push((supporter, supported));
    }

    /// Is there a support path `from →…→ to` of length ≥ 1?
    pub fn support_reaches(&self, from: u64, to: u64) -> bool {
        let mut stack = vec![from];
        let mut seen = HashSet::new();
        seen.insert(from);
        while let Some(x) = stack.pop() {
            for &(s, t) in &self.supports {
                if s == x {
                    if t == to {
                        return true;
                    }
                    if seen.insert(t) {
                        stack.push(t);
                    }
                }
            }
        }
        false
    }

    /// Project to a Dung framework whose attacks are the original attacks PLUS the derived
    /// supported and secondary attacks. Duplicate edges are harmless to the semantics.
    pub fn to_dung(&self) -> ArgumentationFramework {
        let mk = |a: u64, b: u64| Attack {
            attacker: a,
            target: b,
            attack_type: AttackType::Rebuttal,
            strength: 1.0,
        };
        let mut out = ArgumentationFramework::new();
        for arg in self.af.arguments.values() {
            out.add_argument(arg.clone());
        }
        for atk in &self.af.attacks {
            out.add_attack(atk.clone());
        }
        let ids: Vec<u64> = self.af.arguments.keys().copied().collect();

        for atk in &self.af.attacks {
            let (b, c) = (atk.attacker, atk.target);
            // supported attack: a supports* b ∧ b attacks c ⇒ a attacks c
            for &a in &ids {
                if a != b && self.support_reaches(a, b) {
                    out.add_attack(mk(a, c));
                }
            }
            // secondary attack: a attacks b ∧ c supports* b ⇒ a attacks c
            let (a, bb) = (atk.attacker, atk.target);
            for &c in &ids {
                if c != bb && self.support_reaches(c, bb) {
                    out.add_attack(mk(a, c));
                }
            }
        }
        out
    }

    /// Grounded extension over the derived (complex-attack) Dung framework.
    pub fn grounded_extension(&self) -> HashSet<u64> {
        self.to_dung().grounded_extension()
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Argument, Attack, AttackType};
    use super::*;
    use crate::NQuin;

    fn arg(id: u64) -> Argument {
        Argument::new(id, String::new(), Vec::new(), NQuin::default())
    }

    #[test]
    fn support_induces_a_supported_attack() {
        // a supports b; b attacks c. Deductive support ⇒ a attacks c.
        let mut af = ArgumentationFramework::new();
        af.add_argument(arg(1)); // a
        af.add_argument(arg(2)); // b
        af.add_argument(arg(3)); // c
        af.add_attack(Attack {
            attacker: 2,
            target: 3,
            attack_type: AttackType::Rebuttal,
            strength: 1.0,
        });

        let mut bf = BipolarFramework::new(af);
        bf.add_support(1, 2); // a supports b
        assert!(bf.support_reaches(1, 2));

        let dung = bf.to_dung();
        // The derived framework contains a→c (the supported attack).
        assert!(
            dung.attacks
                .iter()
                .any(|atk| atk.attacker == 1 && atk.target == 3),
            "a supports b, b attacks c ⇒ a attacks c"
        );
        // c has attackers {2,3-side}; with a supporting b, c is not accepted.
        let g = bf.grounded_extension();
        assert!(
            !g.contains(&3),
            "c is defeated through the support-backed attack"
        );
    }

    #[test]
    fn support_paths_are_transitive() {
        let af = {
            let mut a = ArgumentationFramework::new();
            a.add_argument(arg(1));
            a.add_argument(arg(2));
            a.add_argument(arg(3));
            a
        };
        let mut bf = BipolarFramework::new(af);
        bf.add_support(1, 2);
        bf.add_support(2, 3);
        assert!(bf.support_reaches(1, 3), "support is transitive (1→2→3)");
        assert!(!bf.support_reaches(3, 1));
    }
}
