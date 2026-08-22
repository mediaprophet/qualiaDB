//! Machine schema exports for all bound host call ids (T51).
//!
//! Every `ALL_BOUND` id exports a machine schema — not English prose.
//! Arguments, effect class, honesty label, and return type are
//! declared in the same table that the catalog uses. This is the
//! MCP replacement: the schema IS the protocol.
//!
//! The schema is generated from the dispatch table so it can never
//! drift from the actual implementation. A host can query
//! `host.schema()` to get the full catalog, or `host.schema_for(id)`
//! to get a single entry.
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` §8.11 T51,
//! recommendations §4.3.

#![allow(dead_code)]

use crate::value::Value;
use std::collections::BTreeMap;

/// The effect class of a host call (T51).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectClass {
    /// Pure — no side effects, deterministic, safe in cells.
    Pure,
    /// Read — reads from graph/store but does not mutate.
    Read,
    /// Write — mutates the graph or store.
    Write,
    /// External — performs I/O (network, HID, pulse, etc.).
    External,
}

impl EffectClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pure => "pure",
            Self::Read => "read",
            Self::Write => "write",
            Self::External => "external",
        }
    }
}

/// The honesty label for a host call (T51).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HonestyLabel {
    /// Always available — no capability lease required.
    Always,
    /// Requires a capability lease (capability.invoke or import).
    CapabilityLease,
    /// Fail-closed — returns an error when the host doesn't support it.
    FailClosed,
    /// Deprecated — kept for compatibility, will be removed.
    Deprecated,
}

impl HonestyLabel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::CapabilityLease => "capability_lease",
            Self::FailClosed => "fail_closed",
            Self::Deprecated => "deprecated",
        }
    }
}

/// A single argument in a host call schema (T51).
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaArg {
    /// The argument name (for named args) or position (for positional).
    pub name: String,
    /// The expected type name (e.g. "string", "i64", "Instant", "List<f64>").
    pub ty: String,
    /// Whether the argument is required.
    pub required: bool,
    /// Optional description.
    pub description: String,
}

impl SchemaArg {
    pub fn new(name: &str, ty: &str, required: bool) -> Self {
        Self {
            name: name.into(),
            ty: ty.into(),
            required,
            description: String::new(),
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.into();
        self
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("name".into(), Value::String(self.name.clone()));
        rec.insert("type".into(), Value::String(self.ty.clone()));
        rec.insert("required".into(), Value::Bool(self.required));
        if !self.description.is_empty() {
            rec.insert(
                "description".into(),
                Value::String(self.description.clone()),
            );
        }
        Value::Record(rec)
    }
}

/// A host call schema entry (T51).
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaEntry {
    /// The host call path (e.g. "time.now", "crypto.sha256").
    pub id: String,
    /// The effect class.
    pub effect: EffectClass,
    /// The honesty label.
    pub honesty: HonestyLabel,
    /// The arguments.
    pub args: Vec<SchemaArg>,
    /// The return type name.
    pub returns: String,
    /// Optional description.
    pub description: String,
}

impl SchemaEntry {
    pub fn new(id: &str, effect: EffectClass, honesty: HonestyLabel, returns: &str) -> Self {
        Self {
            id: id.into(),
            effect,
            honesty,
            args: Vec::new(),
            returns: returns.into(),
            description: String::new(),
        }
    }

    pub fn with_arg(mut self, arg: SchemaArg) -> Self {
        self.args.push(arg);
        self
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.into();
        self
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("id".into(), Value::String(self.id.clone()));
        rec.insert("effect".into(), Value::String(self.effect.as_str().into()));
        rec.insert(
            "honesty".into(),
            Value::String(self.honesty.as_str().into()),
        );
        rec.insert(
            "args".into(),
            Value::List(self.args.iter().map(|a| a.to_value()).collect()),
        );
        rec.insert("returns".into(), Value::String(self.returns.clone()));
        if !self.description.is_empty() {
            rec.insert(
                "description".into(),
                Value::String(self.description.clone()),
            );
        }
        Value::Record(rec)
    }
}

/// The full capability schema — all bound host call ids (T51).
///
/// This is generated from the dispatch table so it never drifts from
/// the actual implementation. New host calls must be added here AND
/// in the dispatch table — the schema test verifies they match.
pub fn all_schemas() -> Vec<SchemaEntry> {
    vec![
        // ── Graph ─────────────────────────────────────────────────────
        SchemaEntry::new(
            "graph.query",
            EffectClass::Read,
            HonestyLabel::CapabilityLease,
            "Value",
        )
        .with_arg(SchemaArg::new("query", "string", true))
        .with_description("Query the graph with a SPARQL-like pattern"),
        SchemaEntry::new(
            "graph.stage",
            EffectClass::Write,
            HonestyLabel::CapabilityLease,
            "Value",
        )
        .with_arg(SchemaArg::new("term", "Value", true))
        .with_description("Stage a term for later commit"),
        SchemaEntry::new(
            "graph.commit",
            EffectClass::Write,
            HonestyLabel::CapabilityLease,
            "Value",
        )
        .with_description("Commit staged terms to the graph"),
        SchemaEntry::new(
            "graph.snapshot",
            EffectClass::Read,
            HonestyLabel::CapabilityLease,
            "Value",
        )
        .with_description("Take a snapshot of the current graph state"),
        // ── Pulse ─────────────────────────────────────────────────────
        SchemaEntry::new(
            "pulse.publish",
            EffectClass::External,
            HonestyLabel::CapabilityLease,
            "Value",
        )
        .with_arg(SchemaArg::new("topic", "string", true))
        .with_arg(SchemaArg::new("payload", "Value", false))
        .with_description("Publish a payload to a pulse topic"),
        // ── Time ──────────────────────────────────────────────────────
        SchemaEntry::new(
            "time.now",
            EffectClass::Read,
            HonestyLabel::FailClosed,
            "Instant",
        )
        .with_description(
            "Primary time primitive — returns Instant with nanosecond resolution (X6)",
        ),
        SchemaEntry::new(
            "time.unix",
            EffectClass::Read,
            HonestyLabel::Deprecated,
            "i64",
        )
        .with_description("DEPRECATED (X6) — use time.now + instant.to_unix_secs"),
        SchemaEntry::new(
            "time.unix_nanos",
            EffectClass::Read,
            HonestyLabel::Deprecated,
            "Record",
        )
        .with_description("DEPRECATED (X6) — use time.now"),
        SchemaEntry::new(
            "time.monotonic_nanos",
            EffectClass::Read,
            HonestyLabel::FailClosed,
            "u64",
        )
        .with_description("Monotonic nanos for frame timing and physics dt"),
        SchemaEntry::new(
            "time.proper_time",
            EffectClass::Read,
            HonestyLabel::FailClosed,
            "Value",
        )
        .with_arg(SchemaArg::new("worldline_id", "u64", true))
        .with_description("Proper time along a worldline"),
        // ── Instant projections ────────────────────────────────────────
        SchemaEntry::new(
            "instant.to_unix_secs",
            EffectClass::Pure,
            HonestyLabel::Always,
            "i64",
        )
        .with_arg(SchemaArg::new("instant", "Instant", true))
        .with_description("Project an Instant to Unix seconds (X6)"),
        SchemaEntry::new(
            "instant.to_unix_nanos",
            EffectClass::Pure,
            HonestyLabel::Always,
            "u64",
        )
        .with_arg(SchemaArg::new("instant", "Instant", true))
        .with_description("Project an Instant to Unix nanoseconds (X6)"),
        // ── Host ──────────────────────────────────────────────────────
        SchemaEntry::new(
            "host.version",
            EffectClass::Pure,
            HonestyLabel::Always,
            "string",
        )
        .with_description("Get the host version string"),
        // ── Capability ────────────────────────────────────────────────
        SchemaEntry::new(
            "capability.resolve",
            EffectClass::Read,
            HonestyLabel::CapabilityLease,
            "Value",
        )
        .with_arg(SchemaArg::new("id", "string", true))
        .with_description("Resolve a capability id to its metadata"),
        SchemaEntry::new(
            "capability.invoke",
            EffectClass::External,
            HonestyLabel::CapabilityLease,
            "Value",
        )
        .with_arg(SchemaArg::new("id", "string", true))
        .with_arg(SchemaArg::new("payload", "Value", false))
        .with_description("Invoke a capability by id"),
        // ── Conservation ──────────────────────────────────────────────
        SchemaEntry::new(
            "conservation.check",
            EffectClass::Pure,
            HonestyLabel::FailClosed,
            "ConservationResult",
        )
        .with_arg(SchemaArg::new("quantity", "string", true))
        .with_arg(SchemaArg::new("before", "f64", true))
        .with_arg(SchemaArg::new("after", "f64", true))
        .with_arg(SchemaArg::new("tolerance", "f64", false))
        .with_description("Check if a transformation preserves a conserved quantity (T34)"),
        // ── Causal ────────────────────────────────────────────────────
        SchemaEntry::new(
            "causal.relation",
            EffectClass::Pure,
            HonestyLabel::FailClosed,
            "CausalRelation",
        )
        .with_arg(SchemaArg::new("event_a", "Value", true))
        .with_arg(SchemaArg::new("event_b", "Value", true))
        .with_description("Determine the causal relation between two events (T35)"),
        // ── DAG ───────────────────────────────────────────────────────
        SchemaEntry::new(
            "dag.execute",
            EffectClass::External,
            HonestyLabel::CapabilityLease,
            "Value",
        )
        .with_arg(SchemaArg::new("pipeline", "Value", true))
        .with_description("Execute a DAG pipeline in topological order (T24)"),
        SchemaEntry::new(
            "dag.validate",
            EffectClass::Pure,
            HonestyLabel::CapabilityLease,
            "Value",
        )
        .with_arg(SchemaArg::new("pipeline", "Value", true))
        .with_description("Validate a DAG pipeline without executing (T24)"),
        // ── Deontic ───────────────────────────────────────────────────
        SchemaEntry::new(
            "deontic.check",
            EffectClass::Pure,
            HonestyLabel::FailClosed,
            "Value",
        )
        .with_arg(SchemaArg::new("capability", "string", true))
        .with_arg(SchemaArg::new("phase", "string", true))
        .with_description("Check if a capability is permitted in a deontic phase (T25)"),
        // ── HID ───────────────────────────────────────────────────────
        SchemaEntry::new(
            "hid.poll",
            EffectClass::External,
            HonestyLabel::FailClosed,
            "Value",
        )
        .with_description("Poll for HID events (T42)"),
        SchemaEntry::new(
            "hid.wait",
            EffectClass::External,
            HonestyLabel::FailClosed,
            "Value",
        )
        .with_arg(SchemaArg::new("timeout_ms", "u64", false))
        .with_description("Wait for HID events with optional timeout (T42)"),
        // ── Cue ───────────────────────────────────────────────────────
        SchemaEntry::new(
            "cue.post",
            EffectClass::External,
            HonestyLabel::FailClosed,
            "Value",
        )
        .with_arg(SchemaArg::new("cue_id", "string", true))
        .with_arg(SchemaArg::new("payload", "Value", false))
        .with_description("Post an outbound cue (haptic/audio/visual/accessibility) (T45)"),
        // ── Crypto ────────────────────────────────────────────────────
        SchemaEntry::new(
            "crypto.sha256",
            EffectClass::Pure,
            HonestyLabel::FailClosed,
            "HashResult",
        )
        .with_arg(SchemaArg::new("data", "bytes", true))
        .with_description("SHA-256 hash"),
        SchemaEntry::new(
            "crypto.sha512",
            EffectClass::Pure,
            HonestyLabel::FailClosed,
            "HashResult",
        )
        .with_arg(SchemaArg::new("data", "bytes", true))
        .with_description("SHA-512 hash"),
        SchemaEntry::new(
            "crypto.blake3",
            EffectClass::Pure,
            HonestyLabel::FailClosed,
            "HashResult",
        )
        .with_arg(SchemaArg::new("data", "bytes", true))
        .with_description("BLAKE3 hash"),
        SchemaEntry::new(
            "crypto.hkdf_sha256",
            EffectClass::Pure,
            HonestyLabel::FailClosed,
            "bytes",
        )
        .with_arg(SchemaArg::new("ikm", "bytes", true))
        .with_arg(SchemaArg::new("info", "bytes", false))
        .with_arg(SchemaArg::new("len", "u64", true))
        .with_description("HKDF-SHA256 key derivation"),
        SchemaEntry::new(
            "crypto.aead_encrypt",
            EffectClass::Pure,
            HonestyLabel::FailClosed,
            "EncryptedData",
        )
        .with_arg(SchemaArg::new("algorithm", "string", true))
        .with_arg(SchemaArg::new("key", "bytes", true))
        .with_arg(SchemaArg::new("nonce", "bytes", true))
        .with_arg(SchemaArg::new("plaintext", "bytes", true))
        .with_arg(SchemaArg::new("aad", "bytes", false))
        .with_description("AEAD encrypt (AES-256-GCM, ChaCha20-Poly1305, XChaCha20-Poly1305)"),
        SchemaEntry::new(
            "crypto.aead_decrypt",
            EffectClass::Pure,
            HonestyLabel::FailClosed,
            "bytes",
        )
        .with_arg(SchemaArg::new("algorithm", "string", true))
        .with_arg(SchemaArg::new("key", "bytes", true))
        .with_arg(SchemaArg::new("nonce", "bytes", true))
        .with_arg(SchemaArg::new("ciphertext", "bytes", true))
        .with_arg(SchemaArg::new("tag", "bytes", true))
        .with_arg(SchemaArg::new("aad", "bytes", false))
        .with_description("AEAD decrypt"),
        SchemaEntry::new(
            "crypto.sign",
            EffectClass::Pure,
            HonestyLabel::FailClosed,
            "Signature",
        )
        .with_arg(SchemaArg::new("key_id", "string", true))
        .with_arg(SchemaArg::new("data", "bytes", true))
        .with_description("Sign data (fail-closed: key vault not wired)"),
        SchemaEntry::new(
            "crypto.verify",
            EffectClass::Pure,
            HonestyLabel::FailClosed,
            "bool",
        )
        .with_arg(SchemaArg::new("key_id", "string", true))
        .with_arg(SchemaArg::new("data", "bytes", true))
        .with_arg(SchemaArg::new("signature", "bytes", true))
        .with_description("Verify a signature (fail-closed: key vault not wired)"),
        SchemaEntry::new(
            "crypto.generate_key",
            EffectClass::Pure,
            HonestyLabel::FailClosed,
            "CryptoKey",
        )
        .with_arg(SchemaArg::new("algorithm", "string", true))
        .with_description("Generate a cryptographic key (fail-closed: key vault not wired)"),
        // ── ZK ────────────────────────────────────────────────────────
        SchemaEntry::new(
            "zk.prove_threshold",
            EffectClass::Pure,
            HonestyLabel::FailClosed,
            "ZkProof",
        )
        .with_arg(SchemaArg::new("value", "f64", true))
        .with_arg(SchemaArg::new("threshold", "f64", true))
        .with_description("Prove value >= threshold (Groth16/BLS12-381)"),
        SchemaEntry::new(
            "zk.verify_threshold",
            EffectClass::Pure,
            HonestyLabel::FailClosed,
            "ZkVerification",
        )
        .with_arg(SchemaArg::new("proof_hex", "string", true))
        .with_arg(SchemaArg::new("vk_hex", "string", true))
        .with_arg(SchemaArg::new("threshold", "f64", true))
        .with_description("Verify a threshold proof"),
        SchemaEntry::new(
            "zk.prove_range",
            EffectClass::Pure,
            HonestyLabel::FailClosed,
            "ZkProof",
        )
        .with_arg(SchemaArg::new("value", "f64", true))
        .with_arg(SchemaArg::new("lo", "f64", true))
        .with_arg(SchemaArg::new("hi", "f64", true))
        .with_description("Prove lo <= value <= hi"),
        SchemaEntry::new(
            "zk.verify_range",
            EffectClass::Pure,
            HonestyLabel::FailClosed,
            "ZkVerification",
        )
        .with_arg(SchemaArg::new("proof_hex", "string", true))
        .with_arg(SchemaArg::new("vk_hex", "string", true))
        .with_arg(SchemaArg::new("lo", "f64", true))
        .with_arg(SchemaArg::new("hi", "f64", true))
        .with_description("Verify a range proof"),
        SchemaEntry::new(
            "zk.prove_matmul",
            EffectClass::Pure,
            HonestyLabel::FailClosed,
            "ZkMatmulResult",
        )
        .with_arg(SchemaArg::new("m", "u64", true))
        .with_arg(SchemaArg::new("k", "u64", true))
        .with_arg(SchemaArg::new("n", "u64", true))
        .with_arg(SchemaArg::new("a", "List<f64>", true))
        .with_arg(SchemaArg::new("b", "List<f64>", true))
        .with_description("Prove matrix multiplication result"),
        SchemaEntry::new(
            "zk.list_circuits",
            EffectClass::Pure,
            HonestyLabel::Always,
            "List<string>",
        )
        .with_description("List available ZK circuits"),
    ]
}

/// Get the schema for a single host call id.
pub fn schema_for(id: &str) -> Option<SchemaEntry> {
    all_schemas().into_iter().find(|e| e.id == id)
}

/// Get all schema ids.
pub fn all_ids() -> Vec<String> {
    all_schemas().into_iter().map(|e| e.id).collect()
}

/// Get the full schema as a Value::List of Value::Records.
pub fn schema_to_value() -> Value {
    Value::List(all_schemas().iter().map(|e| e.to_value()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t51_schema_has_entries() {
        let schemas = all_schemas();
        assert!(
            schemas.len() > 30,
            "expected 30+ schema entries, got {}",
            schemas.len()
        );
    }

    #[test]
    fn t51_schema_ids_are_unique() {
        let schemas = all_schemas();
        let ids: Vec<&str> = schemas.iter().map(|e| e.id.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len(), "duplicate schema ids found");
    }

    #[test]
    fn t51_schema_for_known_id() {
        let entry = schema_for("time.now").unwrap();
        assert_eq!(entry.id, "time.now");
        assert_eq!(entry.effect, EffectClass::Read);
        assert_eq!(entry.returns, "Instant");
    }

    #[test]
    fn t51_schema_for_unknown_returns_none() {
        assert!(schema_for("nonexistent.call").is_none());
    }

    #[test]
    fn t51_all_ids_contains_core_calls() {
        let ids = all_ids();
        assert!(ids.contains(&"time.now".to_string()));
        assert!(ids.contains(&"crypto.sha256".to_string()));
        assert!(ids.contains(&"zk.prove_threshold".to_string()));
        assert!(ids.contains(&"graph.query".to_string()));
        assert!(ids.contains(&"pulse.publish".to_string()));
    }

    #[test]
    fn t51_deprecated_labels_on_time_unix() {
        let entry = schema_for("time.unix").unwrap();
        assert_eq!(entry.honesty, HonestyLabel::Deprecated);
    }

    #[test]
    fn t51_always_labels_on_pure_calls() {
        let entry = schema_for("host.version").unwrap();
        assert_eq!(entry.honesty, HonestyLabel::Always);
        assert_eq!(entry.effect, EffectClass::Pure);
    }

    #[test]
    fn t51_schema_entry_to_value() {
        let entry = schema_for("crypto.sha256").unwrap();
        let v = entry.to_value();
        match v {
            Value::Record(r) => {
                assert!(r.contains_key("id"));
                assert!(r.contains_key("effect"));
                assert!(r.contains_key("honesty"));
                assert!(r.contains_key("args"));
                assert!(r.contains_key("returns"));
            }
            _ => panic!("expected Record"),
        }
    }

    #[test]
    fn t51_schema_to_value_is_list() {
        let v = schema_to_value();
        match v {
            Value::List(xs) => assert!(xs.len() > 30),
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn t51_effect_class_as_str() {
        assert_eq!(EffectClass::Pure.as_str(), "pure");
        assert_eq!(EffectClass::Read.as_str(), "read");
        assert_eq!(EffectClass::Write.as_str(), "write");
        assert_eq!(EffectClass::External.as_str(), "external");
    }

    #[test]
    fn t51_honesty_label_as_str() {
        assert_eq!(HonestyLabel::Always.as_str(), "always");
        assert_eq!(HonestyLabel::CapabilityLease.as_str(), "capability_lease");
        assert_eq!(HonestyLabel::FailClosed.as_str(), "fail_closed");
        assert_eq!(HonestyLabel::Deprecated.as_str(), "deprecated");
    }

    #[test]
    fn t51_schema_arg_to_value() {
        let arg = SchemaArg::new("data", "bytes", true).with_description("the data to hash");
        let v = arg.to_value();
        match v {
            Value::Record(r) => {
                assert_eq!(r.get("name"), Some(&Value::String("data".into())));
                assert_eq!(r.get("type"), Some(&Value::String("bytes".into())));
                assert_eq!(r.get("required"), Some(&Value::Bool(true)));
                assert!(r.contains_key("description"));
            }
            _ => panic!("expected Record"),
        }
    }

    #[test]
    fn t51_crypto_entries_have_correct_args() {
        let entry = schema_for("crypto.aead_encrypt").unwrap();
        assert_eq!(entry.args.len(), 5);
        assert_eq!(entry.args[0].name, "algorithm");
        assert_eq!(entry.args[0].required, true);
    }

    #[test]
    fn t51_zk_entries_have_correct_args() {
        let entry = schema_for("zk.prove_matmul").unwrap();
        assert_eq!(entry.args.len(), 5);
        assert_eq!(entry.args[3].name, "a");
        assert_eq!(entry.args[3].ty, "List<f64>");
    }
}
