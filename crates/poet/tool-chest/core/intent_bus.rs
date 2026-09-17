//! Intent Bus: VibeScript payload routing for the QualiaDB Tool-Chest UI.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)
//!
//! This module defines the foundational types and trait for emitting
//! structured VibeScript intents from native UI components and routing
//! them to the QualiaDB backend via a central [`IntentBus`].
//!
//! # Architectural invariants
//!
//! - **No web legacy.** There is no DOM, HTML, or JavaScript. UI components
//!   are native Rust. Interactions produce structured data, not events.
//! - **Vibe Script payloads.** Every user interaction (button press, cell
//!   edit, annotation drag) is encoded as a [`VibeScriptPayload`] that
//!   serialises to CBOR-LD for wire transmission and persistence.
//! - **Strict decoupling.** UI components never touch database logic. They
//!   construct a payload and push it to the [`IntentBus`]. The bus is
//!   responsible for capability gating, provenance stamping, and routing
//!   to the QualiaDB microkernel.
//! - **Provenance.** Every payload carries the DID of the emitting agent
//!   or component and a monotonically increasing intent counter so that
//!   downstream nquins can anchor byte-exact provenance.
//!
//! # WASM compatibility
//!
//! All types are `#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]`
//! with no platform-specific dependencies, ensuring `wasm32-unknown-unknown`
//! compatibility.

use core::fmt;

use ActionType::*;
use TargetKind::*;

// ---------------------------------------------------------------------------
// ActionType
// ---------------------------------------------------------------------------

/// Top-level classification of a VibeScript intent.
///
/// This is the first dispatch key the [`IntentBus`] uses to route a
/// payload. It is deliberately coarse-grained; fine-grained semantics
/// live inside the generic `parameters` field of [`VibeScriptPayload`].
///
/// The variants align with QualiaDB capability namespaces (`graph:`,
/// `pulse:`, `aura:`, `vibe:`) so that the bus can perform capability
/// gating without inspecting the payload body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// Read-only graph query (SPARQL-star, pattern match, nquin lookup).
    Query,
    /// Mutate the Q42 graph (insert, retract, transaction commit).
    Mutate,
    /// Publish a pulse event on a topic channel.
    Publish,
    /// Validate or apply an aura (SHACL) shape constraint.
    Validate,
    /// Navigate the UI to a different document, section, or asset.
    Navigate,
    /// Create or modify a context annotation on a text span.
    Annotate,
    /// Invoke a registered capability via `capability.invoke("…")`.
    Invoke,
    /// Cancel a previously dispatched intent (best-effort).
    Cancel,
}

impl fmt::Display for ActionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Query => f.write_str("query"),
            Mutate => f.write_str("mutate"),
            Publish => f.write_str("publish"),
            Validate => f.write_str("validate"),
            Navigate => f.write_str("navigate"),
            Annotate => f.write_str("annotate"),
            Invoke => f.write_str("invoke"),
            Cancel => f.write_str("cancel"),
        }
    }
}

// ---------------------------------------------------------------------------
// TargetIdentifier
// ---------------------------------------------------------------------------

/// The kind of identifier carried in [`TargetIdentifier`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    /// A resolved IRI (W3C RDF 1.2 node).
    Iri,
    /// A decentralised identifier — an enumerated state involving the use
    /// of multiple cryptography-supported identifiers and related datasets
    /// of an agent & entity-centric basis.
    Did,
    /// A reference to a 48-byte Qualia nquin in the Q42 graph.
    NquinRef,
    /// An RDF blank node.
    BlankNode,
    /// A local UI-internal component id (not a graph node).
    ComponentId,
}

/// Strongly typed target for a VibeScript intent.
///
/// Prevents accidental mixing of raw strings with cryptographically
/// verified DIDs or RDF nodes. The `kind` discriminant lets the
/// [`IntentBus`] perform fast structural validation before deserialising
/// the payload body.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TargetIdentifier {
    /// Discriminant telling the bus how to interpret `value`.
    pub kind: TargetKind,
    /// The identifier value as a compact string.
    ///
    /// For [`Iri`] this is the full IRI string.
    /// For [`Did`] this is a `did:qualia:…` string.
    /// For [`NquinRef`] this is a hex-encoded 48-byte nquin hash.
    /// For [`BlankNode`] this is the `_:label` string.
    /// For [`ComponentId`] this is a UI-internal path.
    pub value: String,
}

impl TargetIdentifier {
    /// Construct an IRI target.
    pub fn iri(value: impl Into<String>) -> Self {
        Self { kind: Iri, value: value.into() }
    }

    /// Construct a DID target.
    pub fn did(value: impl Into<String>) -> Self {
        Self { kind: Did, value: value.into() }
    }

    /// Construct a nquin reference from a hex-encoded 48-byte hash.
    pub fn nquin_ref(value: impl Into<String>) -> Self {
        Self { kind: NquinRef, value: value.into() }
    }

    /// Construct a blank-node target.
    pub fn blank_node(value: impl Into<String>) -> Self {
        Self { kind: BlankNode, value: value.into() }
    }

    /// Construct a UI-internal component-id target.
    pub fn component_id(value: impl Into<String>) -> Self {
        Self { kind: ComponentId, value: value.into() }
    }
}

// ---------------------------------------------------------------------------
// Provenance
// ---------------------------------------------------------------------------

/// Provenance metadata attached to every emitted intent.
///
/// The bus stamps this at dispatch time. Downstream nquin construction
/// in QualiaDB reads these fields to anchor cryptographic provenance
/// back to the originating UI component.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Provenance {
    /// DID of the agent or component that emitted the intent.
    ///
    /// In demo contexts this is `did:qualia:timothy_charles_holborn`.
    pub emitter_did: String,
    /// Human-readable label of the originating tool-chest component.
    pub component_label: String,
    /// Monotonically increasing intent counter scoped to the emitter.
    pub intent_counter: u64,
    /// Optional capability scope required to execute this intent.
    ///
    /// E.g. `"graph:read"`, `"aura:validate"`, `"pulse:publish"`.
    pub capability_scope: Option<String>,
}

// ---------------------------------------------------------------------------
// VibeScriptPayload
// ---------------------------------------------------------------------------

/// The base VibeScript payload emitted by native UI components.
///
/// Serialises to CBOR-LD for wire transmission and Q42 persistence.
/// The generic parameter `P` allows each tool-chest module to define
/// its own strongly-typed parameter struct while sharing the common
/// envelope.
///
/// # CBOR-LD context
///
/// The `context` field carries the CBOR-LD term-dictionary IRI (or
/// embedded compact context) so that receivers can expand compact
/// keys without out-of-band schema negotiation. This is the same
/// mechanism used by HCF documents (see `FileFormat.md` §3).
///
/// # Example
///
/// ```
/// use tool_chest_core::intent_bus::*;
///
/// let payload = VibeScriptPayload::new(
///     ActionType::Query,
///     TargetIdentifier::iri("https://qualiadb.org/graph/clinical"),
///     MyQueryParams { limit: 10 },
/// )
/// .with_context("https://qualiadb.org/schema/vibe#");
/// ```
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct VibeScriptPayload<P> {
    /// CBOR-LD context: either an IRI string or an embedded context map.
    ///
    /// Defaults to the Vibe schema IRI. Receivers use this to expand
    /// compact keys during CBOR-LD deserialisation.
    #[serde(rename = "@context", default)]
    pub context: ContextRef,

    /// Coarse-grained action classification for first-level routing.
    pub action_type: ActionType,

    /// Strongly typed target of the intent (IRI, DID, nquin ref, etc.).
    pub target_identifier: TargetIdentifier,

    /// Module-specific parameters.
    ///
    /// Each tool-chest module defines its own `P` struct. The bus
    /// routes based on `action_type` and `target_identifier` before
    /// deserialising `parameters`.
    pub parameters: P,

    /// Provenance metadata stamped at dispatch time.
    ///
    /// May be `None` if the payload has not yet been submitted to the
    /// bus. The bus fills this in before forwarding to QualiaDB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

/// CBOR-LD context reference: either a remote IRI or an embedded map.
///
/// In the common case this is a single IRI string pointing at the
/// Vibe schema. For modules that need custom term compaction, the
/// embedded variant carries a `Vec<(String, String)>` of term→IRI
/// pairs that the CBOR-LD encoder will fold into the term dictionary.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ContextRef {
    /// A single context IRI (most common).
    Iri(String),
    /// An embedded context: ordered list of `(term, iri)` pairs.
    Embedded(Vec<(String, String)>),
}

impl Default for ContextRef {
    fn default() -> Self {
        Self::Iri("https://qualiadb.org/schema/vibe#".to_string())
    }
}

impl<P> VibeScriptPayload<P> {
    /// Create a new payload with default context and no provenance.
    pub fn new(
        action_type: ActionType,
        target_identifier: TargetIdentifier,
        parameters: P,
    ) -> Self {
        Self {
            context: ContextRef::default(),
            action_type,
            target_identifier,
            parameters,
            provenance: None,
        }
    }

    /// Set the CBOR-LD context reference.
    pub fn with_context(mut self, context: impl Into<ContextRef>) -> Self {
        self.context = context.into();
        self
    }

    /// Attach provenance metadata.
    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = Some(provenance);
        self
    }
}

impl From<String> for ContextRef {
    fn from(s: String) -> Self {
        ContextRef::Iri(s)
    }
}

impl From<&str> for ContextRef {
    fn from(s: &str) -> Self {
        ContextRef::Iri(s.to_string())
    }
}

// ---------------------------------------------------------------------------
// IntentReceipt
// ---------------------------------------------------------------------------

/// Status returned by the [`IntentBus`] after dispatch.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentStatus {
    /// The intent was accepted and queued for processing.
    Accepted,
    /// The intent was rejected (capability gate, malformed payload, etc.).
    Rejected(String),
    /// The intent was routed but the result is pending (async backend).
    Pending,
    /// The intent was cancelled before execution.
    Cancelled,
}

/// Acknowledgment returned by the [`IntentBus::dispatch`] call.
///
/// Carries the stamped provenance (if the bus filled it in) and a
/// status. For async backends the status will be [`IntentStatus::Pending`]
/// and the caller may poll or subscribe for completion.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct IntentReceipt {
    /// Monotonic dispatch id assigned by the bus.
    pub dispatch_id: u64,
    /// Final or interim status.
    pub status: IntentStatus,
    /// Provenance stamped by the bus (if accepted).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

// ---------------------------------------------------------------------------
// IntentBus trait
// ---------------------------------------------------------------------------

/// Central routing interface for VibeScript intents.
///
/// UI components construct a [`VibeScriptPayload`] and push it to the
/// bus. The bus is responsible for:
///
/// 1. **Capability gating** — verifying the emitter holds the
///    capability scope declared in the payload.
/// 2. **Provenance stamping** — filling in [`Provenance`] with the
///    emitter DID, intent counter, and timestamp.
/// 3. **Routing** — forwarding to the appropriate QualiaDB backend
///    handler based on `action_type` and `target_identifier`.
/// 4. **Receipt** — returning an [`IntentReceipt`] so the UI can
///    track dispatch status.
///
/// Implementors should **not** embed database logic in this trait.
/// The bus is a router and gatekeeper; actual graph mutations happen
/// in the QualiaDB microkernel.
///
/// # Async
///
/// The trait uses native `async fn` (stabilised in Rust 1.75). For
/// `wasm32-unknown-unknown` targets the future is single-threaded
/// and non-blocking, compatible with `wasm-bindgen` async bridges.
pub trait IntentBus: Send + Sync {
    /// The error type for dispatch failures (transport, serialisation).
    type Error: fmt::Debug + Send;

    /// Dispatch a VibeScript payload to the QualiaDB backend.
    ///
    /// The bus stamps provenance, performs capability gating, and
    /// routes the payload. Returns an [`IntentReceipt`] indicating
    /// whether the intent was accepted, rejected, or is pending.
    ///
    /// The generic parameter `P` must be `Serialize` so the bus can
    /// encode it to CBOR-LD before forwarding.
    async fn dispatch<P>(
        &self,
        payload: VibeScriptPayload<P>,
    ) -> Result<IntentReceipt, Self::Error>
    where
        P: serde::Serialize + Send + Sync;

    /// Cancel a previously dispatched intent by its dispatch id.
    ///
    /// Best-effort: if the intent has already been committed to the
    /// Q42 graph, cancellation may not be possible.
    async fn cancel(
        &self,
        dispatch_id: u64,
    ) -> Result<IntentReceipt, Self::Error>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct TestParams {
        limit: u32,
        filter: String,
    }

    #[test]
    fn payload_construction() {
        let p = VibeScriptPayload::new(
            ActionType::Query,
            TargetIdentifier::iri("https://example.org/graph"),
            TestParams { limit: 10, filter: "active".into() },
        );

        assert_eq!(p.action_type, ActionType::Query);
        assert_eq!(p.target_identifier.kind, TargetKind::Iri);
        assert_eq!(p.parameters.limit, 10);
        assert!(p.provenance.is_none());
    }

    #[test]
    fn payload_with_provenance() {
        let p = VibeScriptPayload::new(
            ActionType::Annotate,
            TargetIdentifier::nquin_ref("a1b2c3"),
            TestParams { limit: 1, filter: "test".into() },
        )
        .with_provenance(Provenance {
            emitter_did: "did:qualia:timothy_charles_holborn".into(),
            component_label: "annotation_tool".into(),
            intent_counter: 42,
            capability_scope: Some("graph:read".into()),
        });

        let prov = p.provenance.as_ref().unwrap();
        assert_eq!(prov.emitter_did, "did:qualia:timothy_charles_holborn");
        assert_eq!(prov.intent_counter, 42);
    }

    #[test]
    fn action_type_display() {
        assert_eq!(ActionType::Query.to_string(), "query");
        assert_eq!(ActionType::Mutate.to_string(), "mutate");
        assert_eq!(ActionType::Publish.to_string(), "publish");
    }

    #[test]
    fn target_identifier_constructors() {
        let iri = TargetIdentifier::iri("https://example.org/foo");
        assert_eq!(iri.kind, TargetKind::Iri);

        let did = TargetIdentifier::did("did:qualia:alice");
        assert_eq!(did.kind, TargetKind::Did);

        let nquin = TargetIdentifier::nquin_ref("deadbeef");
        assert_eq!(nquin.kind, TargetKind::NquinRef);

        let bn = TargetIdentifier::blank_node("_:b1");
        assert_eq!(bn.kind, TargetKind::BlankNode);

        let cid = TargetIdentifier::component_id("tool:annotation:1");
        assert_eq!(cid.kind, TargetKind::ComponentId);
    }

    #[test]
    fn context_ref_default() {
        match ContextRef::default() {
            ContextRef::Iri(s) => assert_eq!(s, "https://qualiadb.org/schema/vibe#"),
            _ => panic!("default should be Iri variant"),
        }
    }

    #[test]
    fn cbor_ld_serialisation_roundtrip() {
        let p = VibeScriptPayload::new(
            ActionType::Query,
            TargetIdentifier::iri("https://example.org/g"),
            TestParams { limit: 5, filter: "x".into() },
        )
        .with_provenance(Provenance {
            emitter_did: "did:qualia:timothy_charles_holborn".into(),
            component_label: "test".into(),
            intent_counter: 1,
            capability_scope: None,
        });

        // Serialise to CBOR (CBOR-LD uses CBOR as the binary encoding).
        let cbor_bytes = ciborium::to_vec(&p).expect("cbor encode");
        assert!(!cbor_bytes.is_empty());

        // Deserialise back.
        let decoded: VibeScriptPayload<TestParams> =
            ciborium::from_reader(&cbor_bytes[..]).expect("cbor decode");

        assert_eq!(decoded.action_type, ActionType::Query);
        assert_eq!(decoded.parameters, p.parameters);
        assert_eq!(
            decoded.provenance.unwrap().emitter_did,
            "did:qualia:timothy_charles_holborn"
        );
    }
}
