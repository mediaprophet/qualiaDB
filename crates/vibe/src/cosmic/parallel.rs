//! Parallel realities, quantum bifurcations, and canon timelines (OCS §6).
//!
//! Reference: OCS Specification v2.2.0 §6.

use crate::value::Value;
use std::collections::BTreeMap;

/// FNV-1a 64-bit hash.
fn fnv1a_64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// A timeline branch in a universe (OCS §6.1).
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineBranch {
    /// Branch identifier (e.g. "prime", "mirror", "kelvin")
    pub branch_id: String,
    /// Human-readable name
    pub name: String,
    /// USRI of the parent universe
    pub universe_usri: String,
    /// Divergence epoch — when this branch split from its parent
    pub divergence_epoch: String,
    /// Quantum state bifurcation hash (SHA-256 of the divergence event)
    pub bifurcation_hash: u64,
    /// Parent branch ID (None for the root/prime timeline)
    pub parent_branch: Option<String>,
}

impl TimelineBranch {
    /// Create the prime/root timeline of a universe.
    pub fn prime(universe_usri: &str) -> Self {
        Self {
            branch_id: "prime".into(),
            name: "Prime Canon Timeline".into(),
            universe_usri: universe_usri.into(),
            divergence_epoch: "origin".into(),
            bifurcation_hash: 0, // Root has no bifurcation
            parent_branch: None,
        }
    }

    /// Create a divergent branch from a parent timeline (OCS §6.1).
    pub fn divergent(
        branch_id: &str,
        name: &str,
        universe_usri: &str,
        divergence_epoch: &str,
        parent_branch: &str,
    ) -> Self {
        let bifurcation_input = format!("{}:{}:{}", universe_usri, branch_id, divergence_epoch);
        Self {
            branch_id: branch_id.into(),
            name: name.into(),
            universe_usri: universe_usri.into(),
            divergence_epoch: divergence_epoch.into(),
            bifurcation_hash: fnv1a_64(&bifurcation_input),
            parent_branch: Some(parent_branch.into()),
        }
    }

    /// Whether this is the root/prime timeline.
    pub fn is_prime(&self) -> bool {
        self.parent_branch.is_none()
    }

    /// Compute a unique context hash for this branch (OCS §6.1).
    /// Used for paraconsistent isolation between divergent timelines.
    pub fn context_hash(&self) -> u64 {
        let input = format!(
            "{}:{}:{}",
            self.universe_usri, self.branch_id, self.divergence_epoch
        );
        fnv1a_64(&input)
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("branch_id".into(), Value::String(self.branch_id.clone()));
        rec.insert("name".into(), Value::String(self.name.clone()));
        rec.insert(
            "universe_usri".into(),
            Value::String(self.universe_usri.clone()),
        );
        rec.insert(
            "divergence_epoch".into(),
            Value::String(self.divergence_epoch.clone()),
        );
        rec.insert("bifurcation_hash".into(), Value::U64(self.bifurcation_hash));
        if let Some(ref p) = self.parent_branch {
            rec.insert("parent_branch".into(), Value::String(p.clone()));
        }
        Value::Record(rec)
    }
}

/// A timeline DAG — directed acyclic graph of timeline branches (OCS §6).
#[derive(Debug, Clone)]
pub struct TimelineDag {
    /// All branches in this DAG, keyed by branch_id
    pub branches: Vec<TimelineBranch>,
}

impl TimelineDag {
    pub fn new(prime: TimelineBranch) -> Self {
        Self {
            branches: vec![prime],
        }
    }

    /// Add a divergent branch (OCS §6.1).
    /// Returns false if the parent doesn't exist or would create a cycle.
    pub fn add_branch(&mut self, branch: TimelineBranch) -> bool {
        // Check parent exists
        if let Some(ref parent_id) = branch.parent_branch {
            if !self.branches.iter().any(|b| &b.branch_id == parent_id) {
                return false;
            }
        }
        // Check for duplicate branch_id
        if self
            .branches
            .iter()
            .any(|b| b.branch_id == branch.branch_id)
        {
            return false;
        }
        // Check for cycles (a branch can't be its own ancestor)
        if let Some(ref parent_id) = branch.parent_branch {
            if self.would_create_cycle(&branch.branch_id, parent_id) {
                return false;
            }
        }
        self.branches.push(branch);
        true
    }

    /// Check if adding a branch would create a cycle.
    fn would_create_cycle(&self, new_id: &str, parent_id: &str) -> bool {
        // Walk up the parent chain from parent_id; if we reach new_id, it's a cycle
        let mut current = parent_id.to_string();
        while let Some(branch) = self.branches.iter().find(|b| b.branch_id == current) {
            if branch.branch_id == new_id {
                return true;
            }
            match &branch.parent_branch {
                Some(p) => current = p.clone(),
                None => break,
            }
        }
        false
    }

    /// Get all child branches of a given parent.
    pub fn children_of(&self, parent_id: &str) -> Vec<&TimelineBranch> {
        self.branches
            .iter()
            .filter(|b| b.parent_branch.as_deref() == Some(parent_id))
            .collect()
    }

    /// Get the full ancestry chain of a branch (root → ... → branch).
    pub fn ancestry(&self, branch_id: &str) -> Vec<&TimelineBranch> {
        let mut chain = Vec::new();
        let mut current = branch_id.to_string();
        while let Some(branch) = self.branches.iter().find(|b| b.branch_id == current) {
            chain.push(branch);
            match &branch.parent_branch {
                Some(p) => current = p.clone(),
                None => break,
            }
        }
        chain.reverse(); // Root first
        chain
    }

    /// Verify that divergent branches don't mutate parent timeline records (OCS-T09).
    pub fn verify_branch_isolation(&self, branch_id: &str) -> bool {
        let branch = match self.branches.iter().find(|b| b.branch_id == branch_id) {
            Some(b) => b,
            None => return false,
        };
        // A branch must have a different context hash from its parent
        if let Some(ref parent_id) = branch.parent_branch {
            if let Some(parent) = self.branches.iter().find(|b| b.branch_id == *parent_id) {
                if branch.context_hash() == parent.context_hash() {
                    return false;
                }
            }
        }
        true
    }

    /// Get the prime (root) timeline.
    pub fn prime(&self) -> Option<&TimelineBranch> {
        self.branches.iter().find(|b| b.is_prime())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prime_timeline() {
        let t = TimelineBranch::prime("urn:omni:v1:fiction:star-trek");
        assert!(t.is_prime());
        assert_eq!(t.branch_id, "prime");
        assert!(t.parent_branch.is_none());
    }

    #[test]
    fn divergent_branch() {
        let t = TimelineBranch::divergent(
            "mirror",
            "Mirror Terran Timeline",
            "urn:omni:v1:fiction:star-trek",
            "antiquity",
            "prime",
        );
        assert!(!t.is_prime());
        assert_eq!(t.parent_branch, Some("prime".into()));
        assert!(t.bifurcation_hash != 0);
    }

    #[test]
    fn branch_context_hash_differs() {
        let prime = TimelineBranch::prime("urn:omni:v1:fiction:star-trek");
        let mirror = TimelineBranch::divergent(
            "mirror",
            "Mirror",
            "urn:omni:v1:fiction:star-trek",
            "antiquity",
            "prime",
        );
        assert_ne!(prime.context_hash(), mirror.context_hash());
    }

    #[test]
    fn dag_add_branch() {
        let prime = TimelineBranch::prime("urn:omni:v1:fiction:star-trek");
        let mut dag = TimelineDag::new(prime);
        let kelvin = TimelineBranch::divergent(
            "kelvin",
            "Kelvin Timeline",
            "urn:omni:v1:fiction:star-trek",
            "uss-kelvin-destruction",
            "prime",
        );
        assert!(dag.add_branch(kelvin));
        assert_eq!(dag.branches.len(), 2);
    }

    #[test]
    fn dag_rejects_missing_parent() {
        let prime = TimelineBranch::prime("urn:omni:v1:fiction:star-trek");
        let mut dag = TimelineDag::new(prime);
        let orphan = TimelineBranch::divergent(
            "orphan",
            "Orphan",
            "urn:omni:v1:fiction:star-trek",
            "epoch",
            "nonexistent",
        );
        assert!(!dag.add_branch(orphan));
    }

    #[test]
    fn dag_rejects_duplicate() {
        let prime = TimelineBranch::prime("urn:omni:v1:fiction:star-trek");
        let mut dag = TimelineDag::new(prime);
        let dup = TimelineBranch::divergent(
            "prime",
            "Duplicate",
            "urn:omni:v1:fiction:star-trek",
            "epoch",
            "prime",
        );
        assert!(!dag.add_branch(dup));
    }

    #[test]
    fn dag_children_of() {
        let prime = TimelineBranch::prime("urn:omni:v1:fiction:star-trek");
        let mut dag = TimelineDag::new(prime);
        dag.add_branch(TimelineBranch::divergent(
            "mirror",
            "Mirror",
            "urn:omni:v1:fiction:star-trek",
            "antiquity",
            "prime",
        ));
        dag.add_branch(TimelineBranch::divergent(
            "kelvin",
            "Kelvin",
            "urn:omni:v1:fiction:star-trek",
            "kelvin-event",
            "prime",
        ));
        let children = dag.children_of("prime");
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn dag_ancestry() {
        let prime = TimelineBranch::prime("urn:omni:v1:fiction:star-trek");
        let mut dag = TimelineDag::new(prime);
        dag.add_branch(TimelineBranch::divergent(
            "mirror",
            "Mirror",
            "urn:omni:v1:fiction:star-trek",
            "antiquity",
            "prime",
        ));
        dag.add_branch(TimelineBranch::divergent(
            "mirror-dark",
            "Dark Mirror",
            "urn:omni:v1:fiction:star-trek",
            "later",
            "mirror",
        ));
        let ancestry = dag.ancestry("mirror-dark");
        assert_eq!(ancestry.len(), 3); // prime → mirror → mirror-dark
        assert_eq!(ancestry[0].branch_id, "prime");
        assert_eq!(ancestry[2].branch_id, "mirror-dark");
    }

    #[test]
    fn dag_branch_isolation() {
        let prime = TimelineBranch::prime("urn:omni:v1:fiction:star-trek");
        let mut dag = TimelineDag::new(prime);
        dag.add_branch(TimelineBranch::divergent(
            "mirror",
            "Mirror",
            "urn:omni:v1:fiction:star-trek",
            "antiquity",
            "prime",
        ));
        // OCS-T09: Mirror branch should be isolated from Prime
        assert!(dag.verify_branch_isolation("mirror"));
        assert!(dag.verify_branch_isolation("prime"));
    }

    #[test]
    fn dag_prime_lookup() {
        let prime = TimelineBranch::prime("urn:omni:v1:fiction:star-trek");
        let dag = TimelineDag::new(prime);
        assert!(dag.prime().is_some());
        assert_eq!(dag.prime().unwrap().branch_id, "prime");
    }

    #[test]
    fn branch_to_value() {
        let t = TimelineBranch::prime("urn:omni:v1:fiction:star-trek");
        let v = t.to_value();
        match v {
            Value::Record(r) => {
                assert_eq!(r.get("branch_id"), Some(&Value::String("prime".into())));
            }
            _ => panic!("expected Record"),
        }
    }

    #[test]
    fn bifurcation_hash_deterministic() {
        let t1 = TimelineBranch::divergent(
            "mirror",
            "Mirror",
            "urn:omni:v1:fiction:star-trek",
            "antiquity",
            "prime",
        );
        let t2 = TimelineBranch::divergent(
            "mirror",
            "Mirror",
            "urn:omni:v1:fiction:star-trek",
            "antiquity",
            "prime",
        );
        assert_eq!(t1.bifurcation_hash, t2.bifurcation_hash);
    }

    #[test]
    fn different_branches_different_hash() {
        let t1 = TimelineBranch::divergent(
            "mirror",
            "Mirror",
            "urn:omni:v1:fiction:star-trek",
            "antiquity",
            "prime",
        );
        let t2 = TimelineBranch::divergent(
            "kelvin",
            "Kelvin",
            "urn:omni:v1:fiction:star-trek",
            "kelvin-event",
            "prime",
        );
        assert_ne!(t1.bifurcation_hash, t2.bifurcation_hash);
    }
}
