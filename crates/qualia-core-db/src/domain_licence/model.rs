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
    /// Natural person, self-hosted.
    Personal = 3,
    /// Time-boxed trial for any org.
    Evaluation = 4,
    /// Commercial organisation — payment evidence expected.
    Commercial = 5,
    /// Issuer's own deployments.
    Internal = 6,
}

impl LicenceClass {
    /// Decode a class tag (1–6); out-of-range values become `Unknown`
    /// (fail-closed).
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

    /// Canonical lowercase token for the class (inverse of `parse`).
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

    /// True when the class is gratis community/personal/humanitarian.
    pub fn is_gratis_non_commercial(self) -> bool {
        matches!(
            self,
            Self::Community | Self::CommunitySteward | Self::Personal
        )
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
    /// Parse an agent-kind token (accepts common synonyms); unrecognised
    /// strings become `Unknown` (fail-closed).
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

    /// Canonical token for the agent kind (inverse of `parse`).
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

/// Represented-principal / beneficiary usage classification (`QUALIADB-IMPLEMENTATION-ROADMAP.md` §13).
///
/// Prevents commercial entities from acting through natural-person or AI agents to claim
/// gratis community/personal licensing tiers.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalCategory {
    #[default]
    NaturalPersonDirect = 1,
    FiduciaryAgentForHumanBeneficiary = 2,
    NonProfitOrHumanitarianCommunity = 3,
    CommercialEntityOrPrincipal = 4,
}

/// Context binding the invoking agent to the represented principal and beneficiary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepresentedPrincipalContext {
    pub invoking_agent_kind: AgentKind,
    pub represented_principal: PrincipalCategory,
    pub beneficiary_is_commercial: bool,
}

impl RepresentedPrincipalContext {
    /// Returns `true` when the represented principal or beneficiary is commercial,
    /// regardless of whether the immediate invoking agent is a natural person or AI agent.
    #[inline]
    pub const fn requires_commercial_licence(&self) -> bool {
        matches!(
            self.represented_principal,
            PrincipalCategory::CommercialEntityOrPrincipal
        ) || self.beneficiary_is_commercial
    }
}

/// Delegated-administration scope for steward licences (WIP §2-A).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delegation {
    #[serde(default)]
    pub covers_subdomains: bool,
    #[serde(default)]
    pub members: Vec<String>,
    #[serde(default)]
    pub steward_agreement: String,
}

/// Payment evidence recorded at issuance (WIP §3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentEvidence {
    pub rail: String,
    pub txid: String,
    #[serde(default)]
    pub amount_sats: u64,
}

/// On-chain anchor carrying the canonical licence digest (WIP §4-B).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainAnchor {
    pub chain: String,
    pub txid: String,
    #[serde(default)]
    pub op_return_hex: String,
    pub digest: String,
}

/// A signed domain licence instrument (WIP §3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainLicence {
    pub format: String,
    pub licence_id: String,
    pub issuer_did: String,
    pub issuer_pubkey_hex: String,
    pub subject_domains: Vec<String>,
    #[serde(default)]
    pub domain_did: String,
    #[serde(default)]
    pub delegation: Option<Delegation>,
    #[serde(default)]
    pub licensee_did: String,
    #[serde(default)]
    pub licensee_agent_type: String,
    #[serde(default)]
    pub licensee_label: String,
    pub licence_class: String,
    #[serde(default)]
    pub capability_scope: Vec<String>,
    #[serde(default)]
    pub not_before_unix: u64,
    #[serde(default)]
    pub expires_unix: u64,
    #[serde(default)]
    pub payment_evidence: Option<PaymentEvidence>,
    #[serde(default)]
    pub chain_anchors: Vec<ChainAnchor>,
    #[serde(default)]
    pub revocation_uri: String,
    #[serde(default)]
    pub terms_uri: String,
    #[serde(default)]
    pub signature_hex: String,
}

/// A signed roster amendment chained to a licence by `licence_id` (WIP §7-13).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LicenceAmendment {
    pub licence_id: String,
    pub seq: u32,
    #[serde(default)]
    pub add_members: Vec<String>,
    #[serde(default)]
    pub remove_members: Vec<String>,
    pub issued_unix: u64,
    pub signature_hex: String,
}
