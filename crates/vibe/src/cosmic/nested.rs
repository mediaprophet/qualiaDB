//! Nested realm context hashing and time dilation (OCS §7).
//!
//! Reference: OCS Specification v2.2.0 §7.

use crate::value::Value;
use std::collections::BTreeMap;

/// FNV-1a 64-bit hash (matching q_hash used throughout QualiaDB).
fn fnv1a_64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// A nested realm level (OCS §7).
///
/// Realities can be nested: a holodeck simulation inside a starship,
/// a dream level inside a character, etc. Each level has its own
/// coordinate system, clock, and paraconsistent isolation context.
#[derive(Debug, Clone, PartialEq)]
pub struct NestedRealm {
    /// The nesting depth (0 = physical base, 1 = first nested, etc.)
    pub depth: u32,
    /// The USRI of this realm.
    pub realm_usri: String,
    /// Time dilation factor ζ = dτ_nested / dτ_parent (OCS §7.1).
    /// ζ > 1 = accelerated (e.g. dream time), ζ < 1 = decelerated.
    pub time_dilation: f64,
    /// The parent realm's context hash (for isolation).
    pub parent_context: u64,
}

impl NestedRealm {
    pub fn new(depth: u32, realm_usri: &str, time_dilation: f64, parent_context: u64) -> Self {
        Self {
            depth,
            realm_usri: realm_usri.into(),
            time_dilation,
            parent_context,
        }
    }

    /// Compute the paraconsistent isolation context hash (OCS §7.1).
    ///
    /// context_Nk = context_N(k-1) ⊕ q_hash(realm_usi_Nk) ⊕ q_hash("q42:nested:level" || k)
    ///
    /// This prevents nested contradictions from escaping into parent realities.
    pub fn context_hash(&self) -> u64 {
        let realm_hash = fnv1a_64(&self.realm_usri);
        let level_tag = format!("q42:nested:level{}", self.depth);
        let level_hash = fnv1a_64(&level_tag);
        self.parent_context ^ realm_hash ^ level_hash
    }

    /// Convert parent proper time to nested proper time (OCS §7.1).
    /// τ_nested = τ_parent * ζ
    pub fn parent_to_nested_time(&self, parent_tau_secs: f64) -> f64 {
        parent_tau_secs * self.time_dilation
    }

    /// Convert nested proper time to parent proper time.
    pub fn nested_to_parent_time(&self, nested_tau_secs: f64) -> f64 {
        nested_tau_secs / self.time_dilation
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("depth".into(), Value::U64(self.depth as u64));
        rec.insert("realm_usri".into(), Value::String(self.realm_usri.clone()));
        rec.insert("time_dilation".into(), Value::F64(self.time_dilation));
        rec.insert("context_hash".into(), Value::U64(self.context_hash()));
        Value::Record(rec)
    }
}

/// A stack of nested realms (OCS §7).
#[derive(Debug, Clone)]
pub struct NestingStack {
    realms: Vec<NestedRealm>,
}

impl NestingStack {
    pub fn new() -> Self {
        Self { realms: Vec::new() }
    }

    /// Push a new nested realm onto the stack.
    pub fn push(&mut self, realm_usri: &str, time_dilation: f64) -> &NestedRealm {
        let depth = self.realms.len() as u32;
        let parent_context = self.realms.last().map(|r| r.context_hash()).unwrap_or(0); // Physical base has context 0
        self.realms.push(NestedRealm::new(
            depth,
            realm_usri,
            time_dilation,
            parent_context,
        ));
        self.realms.last().unwrap()
    }

    /// Pop the top realm off the stack.
    pub fn pop(&mut self) -> Option<NestedRealm> {
        self.realms.pop()
    }

    /// Current nesting depth.
    pub fn depth(&self) -> u32 {
        self.realms.len() as u32
    }

    /// Get the current (top) realm.
    pub fn current(&self) -> Option<&NestedRealm> {
        self.realms.last()
    }

    /// Compute the cumulative time dilation from base to current level.
    pub fn cumulative_time_dilation(&self) -> f64 {
        self.realms.iter().map(|r| r.time_dilation).product()
    }

    /// Convert base time to time at the current nesting level.
    pub fn base_to_current_time(&self, base_tau_secs: f64) -> f64 {
        base_tau_secs * self.cumulative_time_dilation()
    }
}

impl Default for NestingStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_realm_context_hash() {
        let r = NestedRealm::new(
            1,
            "urn:omni:v1:simulation:holodeck:vicorian-london",
            20.0,
            0,
        );
        let h = r.context_hash();
        // Should be deterministic
        assert_eq!(h, r.context_hash());
        // Should differ from parent
        assert_ne!(h, 0);
    }

    #[test]
    fn context_hash_isolates_levels() {
        let r1 = NestedRealm::new(1, "urn:omni:v1:simulation:A", 1.0, 0);
        let r2 = NestedRealm::new(1, "urn:omni:v1:simulation:B", 1.0, 0);
        // Same depth, different realm → different context
        assert_ne!(r1.context_hash(), r2.context_hash());
    }

    #[test]
    fn time_dilation_accelerated() {
        let r = NestedRealm::new(1, "dream", 20.0, 0);
        // 1 second parent → 20 seconds nested
        assert!((r.parent_to_nested_time(1.0) - 20.0).abs() < 1e-10);
        // 20 seconds nested → 1 second parent
        assert!((r.nested_to_parent_time(20.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn time_dilation_decelerated() {
        let r = NestedRealm::new(1, "slow", 0.1, 0);
        // 1 second parent → 0.1 seconds nested
        assert!((r.parent_to_nested_time(1.0) - 0.1).abs() < 1e-10);
    }

    #[test]
    fn nesting_stack_push_pop() {
        let mut stack = NestingStack::new();
        assert_eq!(stack.depth(), 0);
        stack.push("level1", 10.0);
        assert_eq!(stack.depth(), 1);
        stack.push("level2", 2.0);
        assert_eq!(stack.depth(), 2);
        stack.pop();
        assert_eq!(stack.depth(), 1);
    }

    #[test]
    fn cumulative_time_dilation() {
        let mut stack = NestingStack::new();
        stack.push("level1", 10.0);
        stack.push("level2", 2.0);
        // 10 * 2 = 20
        assert!((stack.cumulative_time_dilation() - 20.0).abs() < 1e-10);
    }

    #[test]
    fn base_to_current_time() {
        let mut stack = NestingStack::new();
        stack.push("dream", 20.0);
        // 1 second base → 20 seconds at dream level
        assert!((stack.base_to_current_time(1.0) - 20.0).abs() < 1e-10);
    }

    #[test]
    fn nesting_context_chains() {
        let mut stack = NestingStack::new();
        stack.push("level1", 1.0);
        let ctx1 = stack.current().unwrap().context_hash();
        stack.push("level2", 1.0);
        let ctx2 = stack.current().unwrap().context_hash();
        // Each level should have a different context
        assert_ne!(ctx1, ctx2);
        assert_ne!(ctx1, 0);
        assert_ne!(ctx2, 0);
    }

    #[test]
    fn nested_realm_to_value() {
        let r = NestedRealm::new(2, "urn:omni:v1:simulation:test", 5.0, 12345);
        let v = r.to_value();
        match v {
            Value::Record(rec) => {
                assert_eq!(rec.get("depth"), Some(&Value::U64(2)));
                assert_eq!(rec.get("time_dilation"), Some(&Value::F64(5.0)));
                assert!(rec.contains_key("context_hash"));
            }
            _ => panic!("expected Record"),
        }
    }
}
