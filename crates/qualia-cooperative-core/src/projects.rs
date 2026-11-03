//! Cooperative projects (audit COP-01..19) — Restricted records for shared work,
//! membership, contributions, and the obligations **derived** from them.
//!
//! Money/effort safety mirrors `finance.rs`: contributions merge **add-wins by stable
//! entry id** and never by raw sum, and obligations are a **pure derivation over the
//! unique-id set** keyed by `(project_id, contributor_did)`. This makes duplicate,
//! reordered, or replayed sync frames safe — you can never double-count effort.
//!
//! `Contribution` records form an author chain via `predecessor_id`, so a project's
//! effort history is a verifiable append-only sequence per contributor.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::record::{
    EpistemicStatus, EvidenceType, InstantBridge, RecordEnvelope, SensitivityClass,
};

/// Role a member holds within a cooperative project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectRole {
    /// Coordinates the project and can admit/settle on behalf of members.
    Steward,
    /// Does the work and accrues effort obligations.
    Contributor,
    /// Read-only participant; accrues no effort.
    Observer,
}

impl Default for ProjectRole {
    fn default() -> Self {
        Self::Contributor
    }
}

/// A cooperative project — a named unit of shared work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Semantic URIs defining the licensing constraints (e.g. human rights values, trade rules).
    #[serde(default)]
    pub licensing_ontologies: Vec<String>,
    pub created_at_unix: u32,
    /// High-resolution instant (T71 bridge). Preferred over `created_at_unix`
    /// when present; the u32 field is kept for backward-compatible deserialization.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub created_at_instant: Option<InstantBridge>,
}

impl Project {
    /// Creates a new instance.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        licensing_ontologies: Vec<String>,
        created_at_unix: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: description.into(),
            licensing_ontologies,
            created_at_unix,
            created_at_instant: Some(InstantBridge::from_coarse(created_at_unix)),
        }
    }

    /// Resolve the created-at instant, preferring the high-resolution
    /// `InstantBridge` field when present (T71 bridge).
    pub fn created_at(&self) -> InstantBridge {
        self.created_at_instant
            .unwrap_or_else(|| InstantBridge::from_coarse(self.created_at_unix))
    }
}

/// Membership of a DID in a project, with an agreed role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectMembership {
    pub id: String,
    pub project_id: String,
    pub member_did: String,
    #[serde(default)]
    pub role: ProjectRole,
    pub agreed_at_unix: u32,
    /// High-resolution instant (T71 bridge). Preferred over `agreed_at_unix`
    /// when present; the u32 field is kept for backward-compatible deserialization.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub agreed_at_instant: Option<InstantBridge>,
}

impl ProjectMembership {
    /// Creates a new instance.
    pub fn new(
        project_id: impl Into<String>,
        member_did: impl Into<String>,
        role: ProjectRole,
        agreed_at_unix: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            project_id: project_id.into(),
            member_did: member_did.into(),
            role,
            agreed_at_unix,
            agreed_at_instant: Some(InstantBridge::from_coarse(agreed_at_unix)),
        }
    }

    /// Resolve the agreed-at instant, preferring the high-resolution
    /// `InstantBridge` field when present (T71 bridge).
    pub fn agreed_at(&self) -> InstantBridge {
        self.agreed_at_instant
            .unwrap_or_else(|| InstantBridge::from_coarse(self.agreed_at_unix))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContributionPrivacy {
    Public,
    Permissive,
    Private,
}

impl Default for ContributionPrivacy {
    fn default() -> Self {
        Self::Public
    }
}

/// A single immutable unit of contributed effort. Never mutated; a correction is a
/// new entry linked via `predecessor_id`, so `id` is a stable content anchor for dedup
/// and the entries form an append-only author chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contribution {
    pub id: String,
    pub project_id: String,
    pub contributor_did: String,
    pub description: String,
    /// Effort in whole minutes. Non-negative by construction.
    pub effort_minutes: u32,
    /// Capital injected into the project in cents (e.g. $10.00 = 1000).
    #[serde(default)]
    pub capital_cents: u64,
    /// ROI Multiplier to apply to the obligation ledger (e.g. 1.0, 1.5, 2.0).
    #[serde(default = "default_roi")]
    pub roi_multiplier: f32,
    /// Privacy scope of this contributor's identity.
    #[serde(default)]
    pub privacy_level: ContributionPrivacy,
    pub occurred_at_unix: u32,
    /// High-resolution instant (T71 bridge). Preferred over `occurred_at_unix`
    /// when present; the u32 field is kept for backward-compatible deserialization.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub occurred_at_instant: Option<InstantBridge>,
    /// Prior contribution in this contributor's chain, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predecessor_id: Option<String>,
}

fn default_roi() -> f32 {
    1.0
}

impl Contribution {
    /// Creates a new instance.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: impl Into<String>,
        contributor_did: impl Into<String>,
        description: impl Into<String>,
        effort_minutes: u32,
        capital_cents: u64,
        roi_multiplier: f32,
        privacy_level: ContributionPrivacy,
        occurred_at_unix: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            project_id: project_id.into(),
            contributor_did: contributor_did.into(),
            description: description.into(),
            effort_minutes,
            capital_cents,
            roi_multiplier,
            privacy_level,
            occurred_at_unix,
            occurred_at_instant: Some(InstantBridge::from_coarse(occurred_at_unix)),
            predecessor_id: None,
        }
    }

    /// Resolve the occurred-at instant, preferring the high-resolution
    /// `InstantBridge` field when present (T71 bridge).
    pub fn occurred_at(&self) -> InstantBridge {
        self.occurred_at_instant
            .unwrap_or_else(|| InstantBridge::from_coarse(self.occurred_at_unix))
    }

    /// Link this contribution as the successor of `predecessor`, forming the author chain.
    pub fn following(mut self, predecessor: &Contribution) -> Self {
        self.predecessor_id = Some(predecessor.id.clone());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Obligation {
    pub project_id: String,
    pub contributor_did: String,
    /// Total effort minutes across the unique contributions for this pair.
    pub total_effort_minutes: u64,
    /// Total capital invested.
    #[serde(default)]
    pub total_capital_cents: u64,
    /// The mathematically resolved ROI obligation value.
    #[serde(default)]
    pub resolved_obligation_score: f64,
    /// Number of unique contributions aggregated.
    pub contribution_count: usize,
}

// ---------------------------------------------------------------------------
// Record-id helpers
// ---------------------------------------------------------------------------

/// `project-record` ID.
pub fn project_record_id(uuid: &str) -> String {
    format!("urn:wellfair:project:{uuid}")
}

/// `project-membership-record` ID.
pub fn project_membership_record_id(uuid: &str) -> String {
    format!("urn:wellfair:project_membership:{uuid}")
}

/// `contribution-record` ID.
pub fn contribution_record_id(uuid: &str) -> String {
    format!("urn:wellfair:contribution:{uuid}")
}

// ---------------------------------------------------------------------------
// Merge + derivation (money/effort safety — mirror finance.rs discipline)
// ---------------------------------------------------------------------------

/// Merge two contribution sets **add-wins by stable id** (never sum-merge), returning a
/// deterministically ordered union. Idempotent and order-independent.
pub fn merge_contributions(
    existing: &[Contribution],
    incoming: &[Contribution],
) -> Vec<Contribution> {
    let mut merged: Vec<Contribution> = Vec::with_capacity(existing.len() + incoming.len());
    for entry in existing.iter().chain(incoming.iter()) {
        if !merged.iter().any(|e| e.id == entry.id) {
            merged.push(entry.clone());
        }
    }
    merged.sort_by(|a, b| {
        a.occurred_at_unix
            .cmp(&b.occurred_at_unix)
            .then_with(|| a.id.cmp(&b.id))
    });
    merged
}

/// Derive per-(project, contributor) effort obligations purely from the unique-id set.
pub fn derive_obligations(contributions: &[Contribution]) -> Vec<Obligation> {
    let unique = merge_contributions(contributions, &[]);
    let mut obligations: Vec<Obligation> = Vec::new();
    for c in &unique {
        let score = (c.effort_minutes as f64 + c.capital_cents as f64) * c.roi_multiplier as f64;
        match obligations
            .iter_mut()
            .find(|o| o.project_id == c.project_id && o.contributor_did == c.contributor_did)
        {
            Some(ob) => {
                ob.total_effort_minutes += u64::from(c.effort_minutes);
                ob.total_capital_cents += c.capital_cents;
                ob.resolved_obligation_score += score;
                ob.contribution_count += 1;
            }
            None => obligations.push(Obligation {
                project_id: c.project_id.clone(),
                contributor_did: c.contributor_did.clone(),
                total_effort_minutes: u64::from(c.effort_minutes),
                total_capital_cents: c.capital_cents,
                resolved_obligation_score: score,
                contribution_count: 1,
            }),
        }
    }
    obligations.sort_by(|a, b| {
        a.project_id
            .cmp(&b.project_id)
            .then_with(|| a.contributor_did.cmp(&b.contributor_did))
    });
    obligations
}

// ---------------------------------------------------------------------------
// Envelope builders
// ---------------------------------------------------------------------------

fn project_envelope(
    id: &str,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    valid_start_unix: u32,
    valid_start_instant: Option<InstantBridge>,
    predecessor_id: Option<String>,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    RecordEnvelope {
        id: id.to_string(),
        owner_did: owner_did.to_string(),
        author_did: author_did.to_string(),
        proxy_did: None,
        epistemic_status: EpistemicStatus::Asserted,
        evidence_type: EvidenceType::SelfReported,
        sensitivity: SensitivityClass::Restricted,
        asserted_time_unix: asserted_unix,
        asserted_instant: None,
        valid_time_start_unix: Some(valid_start_unix),
        valid_time_start_instant: valid_start_instant,
        valid_time_end_unix: None,
        valid_time_end_instant: None,
        predecessor_id,
        blob_hash,
        tombstone: false,
    }
}

/// Builds project-envelope.
pub fn build_project_envelope(
    project: &Project,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    let id = project_record_id(&project.id);
    project_envelope(&id, owner_did, author_did, asserted_unix, project.created_at_unix, project.created_at_instant, None, blob_hash)
}

/// Builds membership-envelope.
pub fn build_membership_envelope(
    membership: &ProjectMembership,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    let id = project_membership_record_id(&membership.id);
    project_envelope(&id, owner_did, author_did, asserted_unix, membership.agreed_at_unix, membership.agreed_at_instant, None, blob_hash)
}

/// Builds contribution-envelope.
pub fn build_contribution_envelope(
    contribution: &Contribution,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    let id = contribution_record_id(&contribution.id);
    let predecessor_id = contribution.predecessor_id.as_ref().map(|p| contribution_record_id(p));
    project_envelope(&id, owner_did, author_did, asserted_unix, contribution.occurred_at_unix, contribution.occurred_at_instant, predecessor_id, blob_hash)
}

// ---------------------------------------------------------------------------
// Summaries (lossless serde_json object strings, as stored on the journal row)
// ---------------------------------------------------------------------------

/// Project summary.
pub fn project_summary(project: &Project) -> String {
    serde_json::json!({
        "id": project.id,
        "name": project.name,
        "description": project.description,
        "licensing_ontologies": project.licensing_ontologies,
        "created_at_unix": project.created_at_unix,
        "created_at_instant": project.created_at_instant,
    })
    .to_string()
}

/// Membership summary.
pub fn membership_summary(membership: &ProjectMembership) -> String {
    serde_json::json!({
        "id": membership.id,
        "project_id": membership.project_id,
        "member_did": membership.member_did,
        "role": membership.role,
        "agreed_at_unix": membership.agreed_at_unix,
        "agreed_at_instant": membership.agreed_at_instant,
    })
    .to_string()
}

/// Lossless contribution summary including `capital_cents`, `roi_multiplier`,
/// `privacy_level`, `occurred_at_instant`, and `predecessor_id`.
pub fn contribution_summary(contribution: &Contribution) -> String {
    serde_json::json!({
        "id": contribution.id,
        "project_id": contribution.project_id,
        "contributor_did": contribution.contributor_did,
        "description": contribution.description,
        "effort_minutes": contribution.effort_minutes,
        "capital_cents": contribution.capital_cents,
        "roi_multiplier": contribution.roi_multiplier,
        "privacy_level": contribution.privacy_level,
        "occurred_at_unix": contribution.occurred_at_unix,
        "occurred_at_instant": contribution.occurred_at_instant,
        "predecessor_id": contribution.predecessor_id,
    })
    .to_string()
}

/// Reconstruct a `Contribution` losslessly from a stored/transmitted summary JSON.
pub fn parse_contribution_summary(
    id: impl Into<String>,
    summary: &str,
    fallback_occurred_at_unix: u32,
) -> Option<Contribution> {
    let v: serde_json::Value = serde_json::from_str(summary).ok()?;
    if !v.is_object() {
        return None;
    }
    let id_str = id.into();
    let resolved_id = if id_str.is_empty() {
        v.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string()
    } else {
        id_str
    };
    let privacy_level = v.get("privacy_level").and_then(|x| serde_json::from_value(x.clone()).ok()).unwrap_or_default();
    let occurred_at_unix = v.get("occurred_at_unix").and_then(|x| x.as_u64()).map(|t| t as u32).unwrap_or(fallback_occurred_at_unix);
    let occurred_at_instant = v.get("occurred_at_instant").and_then(|x| serde_json::from_value(x.clone()).ok());
    Some(Contribution {
        id: resolved_id,
        project_id: v.get("project_id").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        contributor_did: v.get("contributor_did").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        description: v.get("description").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        effort_minutes: v.get("effort_minutes").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
        capital_cents: v.get("capital_cents").and_then(|x| x.as_u64()).unwrap_or(0),
        roi_multiplier: v.get("roi_multiplier").and_then(|x| x.as_f64()).map(|f| f as f32).unwrap_or(1.0),
        privacy_level,
        occurred_at_unix,
        occurred_at_instant,
        predecessor_id: v.get("predecessor_id").and_then(|x| x.as_str()).map(|s| s.to_string()),
    })
}

#[cfg(test)]
#[path = "projects_tests.rs"]
mod tests;
