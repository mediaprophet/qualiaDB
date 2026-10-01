//! Data model for domain licence instruments.

use serde::{Deserialize, Serialize};

/// Maximum subject domains per licence.
pub const MAX_DOMAINS: usize = 64;
/// Maximum delegated members in a steward licence.
pub const MAX_MEMBERS: usize = 256;
/// Maximum capability scopes per licence.
pub const MAX_SCOPES: usize = 16;
/// Maximum chain anchors per licence.
pub const MAX_ANCHORS: usize = 8;
/// Maximum roster amendments chained to one licence.
pub const MAX_AMENDMENT_DELTA: usize = 256;

/// Recognised licence classes (WIP §2). Unknown fails closed.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LicenceClass {
    #[default]
    Unknown = 0,
    /// Non-profit / community / humanitarian — gratis.
    Community = 1,
    /// Local custodian administering delegated members (the "inn" model).
    CommunitySteward = 2,
    /// Natural person, sovereign self-hosting.
    Personal = 3,
    /// Time-boxed trial for any org.
    Evaluation = 4,
    /// Commercial organisation — payment evidence expected.
    Commercial = 5,
    /// Issuer's own deployments.
    Internal = 6,
}

impl LicenceClass {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Community,
            2 => Self::CommunitySteward,
            3 => Self::Personal,
            4 => Self::Evaluation,
            5 => Self::Commercial,
            6 => Self::Internal,
            _ => Self::Unknown,
        }
    }

    /// Parse a class token. Unrecognised strings become `Unknown` (fail-closed).
    pub fn parse(tag: &str) -> Self {
        match tag.trim().to_ascii_lowercase().as_str() {
            "community" => Self::Community,
            "community-steward" | "community_steward" | "steward" => Self::CommunitySteward,
            "personal" => Self::Personal,
            "evaluation" | "eval" | "trial" => Self::Evaluation,
            "commercial" => Self::Commercial,
            "internal" => Self::Internal,
            _ => Self::Unknown,
        }
    }

    pub fn token(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Community => "community",
            Self::CommunitySteward => "community-steward",
            Self::Personal => "personal",
            Self::Evaluation => "evaluation",
            Self::Commercial => "commercial",
            Self::Internal => "internal",
        }
    }

    /// Does this class permit delegated administration of member subdomains?
    pub fn permits_delegation(self) -> bool {
        matches!(self, Self::CommunitySteward | Self::Internal)
    }

    /// Does this class normally require payment evidence at issuance?
    pub fn expects_payment(self) -> bool {
        matches!(self, Self::Commercial)
    }
}

/// Licensee agent kind — mirrors the QDP `AgentType` token set in
/// `client-core/domains.rs`. Unknown fails closed.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentKind {
    Unknown = 0,
    NaturalPerson = 1,
    Organization = 2,
    AiAgent = 3,
    HumanitarianService = 4,
    ContentProvider = 5,
    Group = 6,
}

impl AgentKind {
    pub fn parse(tag: &str) -> Self {
        match tag.trim().to_ascii_lowercase().as_str() {
            "person" | "natural-person" => Self::NaturalPerson,
            "org" | "organization" | "organisation" => Self::Organization,
            "ai" | "agent" => Self::AiAgent,
            "service" | "humanitarian" => Self::HumanitarianService,
            "content" => Self::ContentProvider,
            "group" => Self::Group,
            _ => Self::Unknown,
        }
    }

    pub fn token(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::NaturalPerson => "person",
            Self::Organization => "org",
            Self::AiAgent => "ai",
            Self::HumanitarianService => "service",
            Self::ContentProvider => "content",
            Self::Group => "group",
        }
    }
}

/// Delegated-administration scope for steward licences (WIP §2-A).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delegation {
    /// Whether any subdomain of a subject domain is covered (wildcard policy).
    /// Default false — prefer the explicit `members` roster (fail-closed).
    #[serde(default)]
    pub covers_subdomains: bool,
    /// Explicit member roster (subdomain or address names under a subject domain).
    #[serde(default)]
    pub members: Vec<String>,
    /// DID of the steward's M:N operating agreement, when the steward is a group.
    #[serde(default)]
    pub steward_agreement: String,
}

/// Payment evidence recorded at issuance (WIP §3). Verified by the issuer at
/// issuance; the runtime verifier trusts the signature, not the rail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentEvidence {
    /// Rail identifier: "xec" first; extensible ("cashu", "ilp", …).
    pub rail: String,
    /// Transaction id or token reference proving payment.
    pub txid: String,
    /// Amount in the rail's base unit (sats for xec).
    #[serde(default)]
    pub amount_sats: u64,
}

/// On-chain anchor carrying the canonical licence digest (WIP §4-B).
/// Outside the signed bytes — may be appended post-issuance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainAnchor {
    /// Nominated chain identifier (e.g. "xec").
    pub chain: String,
    /// Anchor transaction id (may equal the payment tx when combined).
    pub txid: String,
    /// OP_RETURN payload hex committed on-chain (e.g. "QUALIA-LIC" + digest).
    #[serde(default)]
    pub op_return_hex: String,
    /// Canonical licence digest this anchor commits ("sha256:<hex>").
    pub digest: String,
}

/// A signed domain licence instrument (WIP §3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainLicence {
    /// Must equal [`crate::domain_licence::LICENCE_FORMAT`].
    pub format: String,
    /// `did:q42:licence:<hash>` — derived at issuance from a provisional digest.
    pub licence_id: String,
    pub issuer_did: String,
    /// Ed25519 verifying key of the issuer (lowercase hex).
    pub issuer_pubkey_hex: String,
    /// Domains this licence covers.
    pub subject_domains: Vec<String>,
    /// Optional QDP agent binding (`did:web:…`) for the primary domain.
    #[serde(default)]
    pub domain_did: String,
    /// Delegated-administration scope (steward classes only).
    #[serde(default)]
    pub delegation: Option<Delegation>,
    #[serde(default)]
    pub licensee_did: String,
    /// Token from [`AgentKind::token`].
    #[serde(default)]
    pub licensee_agent_type: String,
    #[serde(default)]
    pub licensee_label: String,
    /// Token from [`LicenceClass::token`].
    pub licence_class: String,
    /// Capability profiles this licence unlocks (e.g. "wasm-ontology").
    #[serde(default)]
    pub capability_scope: Vec<String>,
    #[serde(default)]
    pub not_before_unix: u64,
    /// 0 = no expiry.
    #[serde(default)]
    pub expires_unix: u64,
    #[serde(default)]
    pub payment_evidence: Option<PaymentEvidence>,
    /// On-chain anchors — NOT covered by the signature (WIP §7-8).
    #[serde(default)]
    pub chain_anchors: Vec<ChainAnchor>,
    #[serde(default)]
    pub revocation_uri: String,
    #[serde(default)]
    pub terms_uri: String,
    /// Ed25519 signature over canonical bytes (lowercase hex).
    #[serde(default)]
    pub signature_hex: String,
}

/// A signed roster amendment chained to a licence by `licence_id` (WIP §7-13).
/// Only the issuer key may sign; members join/leave without re-issuing the base.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LicenceAmendment {
    /// Must match the base licence's `licence_id`.
    pub licence_id: String,
    /// Monotonic sequence; amendments must apply in order.
    pub seq: u32,
    #[serde(default)]
    pub add_members: Vec<String>,
    #[serde(default)]
    pub remove_members: Vec<String>,
    pub issued_unix: u64,
    /// Ed25519 signature over the amendment's canonical bytes.
    pub signature_hex: String,
}
