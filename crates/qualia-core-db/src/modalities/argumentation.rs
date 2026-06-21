// Argumentation Frameworks - Dung-style Abstract Argumentation
// Provides formal debate resolution mechanisms for Peace Infrastructure

use crate::NQuin;
use std::collections::{HashMap, HashSet};

pub const ARGUMENT_BIT: u64 = 1u64 << 55;
pub const ATTACK_BIT: u64 = 1u64 << 54;
pub const DEFENSE_BIT: u64 = 1u64 << 53;

/// Argument in an abstract argumentation framework
#[derive(Debug, Clone)]
pub struct Argument {
    pub id: u64,
    pub content: String,
    pub premise_quins: Vec<NQuin>,
    pub conclusion_quin: NQuin,
    pub strength: f32, // Argument strength for weighted argumentation
}

impl Argument {
    /// Create a new argument from premises and conclusion
    pub fn new(id: u64, content: String, premises: Vec<NQuin>, conclusion: NQuin) -> Self {
        Self {
            id,
            content,
            premise_quins: premises,
            conclusion_quin: conclusion,
            strength: 1.0, // Default strength
        }
    }
    
    /// Create an argument with specified strength
    pub fn with_strength(id: u64, content: String, premises: Vec<NQuin>, conclusion: NQuin, strength: f32) -> Self {
        Self {
            id,
            content,
            premise_quins: premises,
            conclusion_quin: conclusion,
            strength,
        }
    }
}

/// Attack relation between arguments
#[derive(Debug, Clone)]
pub struct Attack {
    pub attacker: u64,
    pub target: u64,
    pub attack_type: AttackType,
    pub strength: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttackType {
    /// Direct contradiction of conclusion
    Rebuttal,
    /// Attack on premises
    Undercut,
    /// Weakening argument strength
    Undermine,
}

/// Abstract argumentation framework
#[derive(Debug, Clone)]
pub struct ArgumentationFramework {
    pub arguments: HashMap<u64, Argument>,
    pub attacks: Vec<Attack>,
    pub metadata: u64,
}

impl ArgumentationFramework {
    /// Create a new empty framework
    pub fn new() -> Self {
        Self {
            arguments: HashMap::new(),
            attacks: Vec::new(),
            metadata: 0,
        }
    }
    
    /// Add an argument to the framework
    pub fn add_argument(&mut self, argument: Argument) {
        self.arguments.insert(argument.id, argument);
    }
    
    /// Add an attack relation
    pub fn add_attack(&mut self, attack: Attack) {
        self.attacks.push(attack);
    }
    
    /// Get all arguments that attack a given argument
    pub fn get_attackers(&self, target_id: u64) -> Vec<&Argument> {
        let mut attackers = Vec::new();
        for attack in &self.attacks {
            if attack.target == target_id {
                if let Some(attacker) = self.arguments.get(&attack.attacker) {
                    attackers.push(attacker);
                }
            }
        }
        attackers
    }
    
    /// Get all arguments that are attacked by a given argument
    pub fn get_attacked(&self, attacker_id: u64) -> Vec<&Argument> {
        let mut attacked = Vec::new();
        for attack in &self.attacks {
            if attack.attacker == attacker_id {
                if let Some(target) = self.arguments.get(&attack.target) {
                    attacked.push(target);
                }
            }
        }
        attacked
    }
    
    /// Compute grounded extension (unique least fixed point of the characteristic function).
    ///
    /// Algorithm (Dung 1995):
    ///   GE ← ∅
    ///   Repeat:
    ///     1. Collect all arguments that are *defeated* by GE (attacked by some member of GE).
    ///     2. For every remaining argument a not yet in GE:
    ///        if *every* attacker of a is defeated by GE, add a to GE.
    ///   Until no change.
    pub fn grounded_extension(&self) -> HashSet<u64> {
        let mut grounded: HashSet<u64> = HashSet::new();
        let mut changed = true;

        while changed {
            changed = false;

            // Build set of arguments defeated by the current grounded set
            // (i.e., arguments attacked by at least one member of grounded).
            let defeated: HashSet<u64> = self.attacks.iter()
                .filter(|atk| grounded.contains(&atk.attacker))
                .map(|atk| atk.target)
                .collect();

            for (&arg_id, _) in &self.arguments {
                if grounded.contains(&arg_id) {
                    continue;
                }
                // An argument is added to the grounded extension iff all of its
                // attackers are themselves defeated (i.e., counter-attacked by
                // something already in grounded).
                let all_attackers_defeated = self.attacks.iter()
                    .filter(|atk| atk.target == arg_id)
                    .all(|atk| defeated.contains(&atk.attacker));

                if all_attackers_defeated {
                    grounded.insert(arg_id);
                    changed = true;
                }
            }
        }

        grounded
    }
    
    /// Compute preferred extensions (maximal conflict-free sets)
    pub fn preferred_extensions(&self) -> Vec<HashSet<u64>> {
        // Start with grounded extension as base
        let grounded = self.grounded_extension();
        let mut extensions = vec![grounded.clone()];
        
        // Try to add unattacked arguments iteratively
        let mut changed = true;
        while changed {
            changed = false;
            let mut new_extensions = Vec::new();
            
            for extension in &extensions {
                for (&arg_id, _) in &self.arguments {
                    if !extension.contains(&arg_id) {
                        let mut candidate = extension.clone();
                        candidate.insert(arg_id);
                        
                        if self.is_conflict_free(&candidate) && self.is_admissible(&candidate) {
                            if !extensions.iter().any(|ext| ext.is_superset(&candidate)) {
                                new_extensions.push(candidate);
                                changed = true;
                            }
                        }
                    }
                }
            }
            
            extensions.extend(new_extensions);
        }
        
        // Return only maximal extensions
        let extensions_clone = extensions.clone();
        extensions.into_iter()
            .filter(|ext| {
                !extensions_clone.iter().any(|other| other != ext && other.is_superset(ext))
            })
            .collect()
    }
    
    /// Check if a set of arguments is conflict-free (no attacks within the set)
    pub fn is_conflict_free(&self, args: &HashSet<u64>) -> bool {
        for &arg_id in args {
            let attacked = self.get_attacked(arg_id);
            for attacked_arg in attacked {
                if args.contains(&attacked_arg.id) {
                    return false;
                }
            }
        }
        true
    }
    
    /// Check if a set of arguments is admissible (conflict-free and defends all its members)
    pub fn is_admissible(&self, args: &HashSet<u64>) -> bool {
        if !self.is_conflict_free(args) {
            return false;
        }
        
        // Check if the set defends all its members
        for &arg_id in args {
            let attackers = self.get_attackers(arg_id);
            for attacker in attackers {
                let is_defended = self.get_attacked(attacker.id)
                    .iter()
                    .any(|defender| args.contains(&defender.id));
                
                if !is_defended {
                    return false;
                }
            }
        }
        
        true
    }
    
    /// Compute the argumentation status of an argument
    pub fn argument_status(&self, arg_id: u64) -> ArgumentStatus {
        let grounded = self.grounded_extension();
        
        if grounded.contains(&arg_id) {
            ArgumentStatus::Accepted
        } else {
            let preferred = self.preferred_extensions();
            let accepted_in_all = preferred.iter().all(|ext| ext.contains(&arg_id));
            let accepted_in_some = preferred.iter().any(|ext| ext.contains(&arg_id));
            
            if accepted_in_all {
                ArgumentStatus::Accepted
            } else if accepted_in_some {
                ArgumentStatus::Undecided
            } else {
                ArgumentStatus::Rejected
            }
        }
    }
    
    /// Resolve a debate using skeptical reasoning (intersection of all preferred extensions)
    pub fn resolve_skeptically(&self) -> HashSet<u64> {
        let preferred = self.preferred_extensions();
        if preferred.is_empty() {
            return HashSet::new();
        }
        
        // Return intersection of all preferred extensions
        let mut result = preferred[0].clone();
        for extension in &preferred[1..] {
            result = result.intersection(extension).cloned().collect();
        }
        
        result
    }
    
    /// Resolve a debate using credulous reasoning (union of all preferred extensions)
    pub fn resolve_credulously(&self) -> HashSet<u64> {
        let preferred = self.preferred_extensions();
        let mut result = HashSet::new();
        
        for extension in preferred {
            result.extend(extension);
        }
        
        result
    }
}

/// Argument status in the framework
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgumentStatus {
    Accepted,
    Rejected,
    Undecided,
}

/// Convert argument framework to NQuin representation for storage
pub fn framework_to_quins(framework: &ArgumentationFramework, context: u64) -> Vec<NQuin> {
    let mut quins = Vec::new();
    
    // Store arguments
    for (arg_id, argument) in &framework.arguments {
        let mut quin = NQuin {
            subject: *arg_id,
            predicate: crate::q_hash("has_argument"),
            object: crate::q_hash(&argument.content),
            context,
            metadata: ARGUMENT_BIT | ((argument.strength as u64) << 32),
            parity: 0,
        };
        quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;
        quins.push(quin);
    }
    
    // Store attacks
    for attack in &framework.attacks {
        let mut quin = NQuin {
            subject: attack.attacker,
            predicate: crate::q_hash("attacks"),
            object: attack.target,
            context,
            metadata: ATTACK_BIT | ((attack.strength as u64) << 32),
            parity: 0,
        };
        quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;
        quins.push(quin);
    }
    
    quins
}

/// Create a simple debate about sanctuary boundaries
pub fn create_sanctuary_debate() -> ArgumentationFramework {
    let mut framework = ArgumentationFramework::new();
    
    // Argument 1: Sanctuary should protect all life
    let arg1 = Argument::new(
        1,
        "Sanctuary must protect all living beings".to_string(),
        vec![],
        NQuin {
            subject: crate::q_hash("sanctuary"),
            predicate: crate::q_hash("protects"),
            object: crate::q_hash("all_life"),
            context: 100,
            metadata: 0,
            parity: 0,
        }
    );
    framework.add_argument(arg1);
    
    // Argument 2: Resource constraints limit protection scope
    let arg2 = Argument::new(
        2,
        "Limited resources require prioritized protection".to_string(),
        vec![],
        NQuin {
            subject: crate::q_hash("sanctuary"),
            predicate: crate::q_hash("protects"),
            object: crate::q_hash("prioritized_life"),
            context: 101,
            metadata: 0,
            parity: 0,
        }
    );
    framework.add_argument(arg2);
    
    // Argument 2 attacks Argument 1 (undercut)
    framework.add_attack(Attack {
        attacker: 2,
        target: 1,
        attack_type: AttackType::Undercut,
        strength: 0.8,
    });
    
    framework
}

/// Max arguments considered by the zero-heap grounded-extension membership test.
pub const MAX_GROUNDED_ARGS: usize = 128;

/// Zero-heap Dung (1995) grounded-extension membership test over caller-supplied,
/// bounded argument and attack arrays. Returns whether `goal` is justified (in the
/// grounded extension). No allocation — fixed stack buffers only; suitable for the
/// Webizen VM hot path. (`ArgumentationFramework::grounded_extension` is the
/// heap-using batch variant for the cold path.)
pub fn grounded_contains(args: &[u64], attacks: &[(u64, u64)], goal: u64) -> bool {
    let n = args.len().min(MAX_GROUNDED_ARGS);
    let mut grounded = [false; MAX_GROUNDED_ARGS];
    let mut defeated = [false; MAX_GROUNDED_ARGS];

    let index_of = |id: u64| -> Option<usize> { args[..n].iter().position(|&a| a == id) };

    loop {
        // defeated = arguments attacked by a current grounded member.
        for d in defeated.iter_mut().take(n) {
            *d = false;
        }
        for &(attacker, target) in attacks {
            if let Some(ai) = index_of(attacker) {
                if grounded[ai] {
                    if let Some(ti) = index_of(target) {
                        defeated[ti] = true;
                    }
                }
            }
        }

        let mut changed = false;
        for i in 0..n {
            if grounded[i] {
                continue;
            }
            // args[i] joins the grounded set iff every attacker of it is defeated.
            let mut all_attackers_defeated = true;
            for &(attacker, target) in attacks {
                if target == args[i] {
                    match index_of(attacker) {
                        Some(ai) if defeated[ai] => {}
                        _ => {
                            all_attackers_defeated = false;
                            break;
                        }
                    }
                }
            }
            if all_attackers_defeated {
                grounded[i] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    match index_of(goal) {
        Some(gi) => grounded[gi],
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grounded_contains_zero_heap_matches_dung() {
        // A unattacked; A attacks B; B attacks C. Grounded = {A, C}.
        let args = [1u64, 2, 3];
        let attacks = [(1u64, 2u64), (2, 3)];
        assert!(grounded_contains(&args, &attacks, 1), "A is justified");
        assert!(!grounded_contains(&args, &attacks, 2), "B is defeated by A");
        assert!(grounded_contains(&args, &attacks, 3), "C is reinstated (its attacker B is defeated)");
        assert!(!grounded_contains(&args, &attacks, 99), "unknown argument is not justified");
    }
    
    #[test]
    fn test_grounded_extension() {
        let framework = create_sanctuary_debate();
        let grounded = framework.grounded_extension();
        
        // Argument 2 should be in grounded extension (unattacked)
        assert!(grounded.contains(&2));
        
        // Argument 1 should not be (attacked by 2)
        assert!(!grounded.contains(&1));
    }
    
    #[test]
    fn test_conflict_free() {
        let framework = create_sanctuary_debate();
        
        // Set with both arguments should not be conflict-free
        let both_args = HashSet::from([1, 2]);
        assert!(!framework.is_conflict_free(&both_args));
        
        // Set with only argument 2 should be conflict-free
        let only_arg2 = HashSet::from([2]);
        assert!(framework.is_conflict_free(&only_arg2));
    }
    
    #[test]
    fn test_argument_status() {
        let framework = create_sanctuary_debate();
        
        assert_eq!(framework.argument_status(2), ArgumentStatus::Accepted);
        assert_eq!(framework.argument_status(1), ArgumentStatus::Rejected);
    }
    
    #[test]
    fn test_skeptical_resolution() {
        let framework = create_sanctuary_debate();
        let skeptical = framework.resolve_skeptically();
        
        // Should only include arguments accepted in all preferred extensions
        assert!(skeptical.contains(&2));
        assert!(!skeptical.contains(&1));
    }
    
    #[test]
    fn test_framework_to_quins() {
        let framework = create_sanctuary_debate();
        let quins = framework_to_quins(&framework, 123);
        
        // Should have quins for arguments and attacks
        assert_eq!(quins.len(), 3); // 2 arguments + 1 attack
        
        // Check metadata bits
        let arg_quin = quins.iter().find(|q| q.predicate == crate::q_hash("has_argument")).unwrap();
        assert!(arg_quin.metadata & ARGUMENT_BIT != 0);
        
        let attack_quin = quins.iter().find(|q| q.predicate == crate::q_hash("attacks")).unwrap();
        assert!(attack_quin.metadata & ATTACK_BIT != 0);
    }
}
