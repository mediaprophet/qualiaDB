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

use crate::record::{EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass};

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
}

impl Project {
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
        }
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
}

impl ProjectMembership {
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
        }
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Prior contribution in this contributor's chain, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predecessor_id: Option<String>,
}

fn default_roi() -> f32 { 1.0 }

impl Contribution {
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
            predecessor_id: None,
        }
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

pub fn project_record_id(uuid: &str) -> String {
    format!("urn:wellfair:project:{uuid}")
}

pub fn project_membership_record_id(uuid: &str) -> String {
    format!("urn:wellfair:project_membership:{uuid}")
}

pub fn contribution_record_id(uuid: &str) -> String {
    format!("urn:wellfair:contribution:{uuid}")
}

// ---------------------------------------------------------------------------
// Merge + derivation (money/effort safety — mirror finance.rs discipline)
// ---------------------------------------------------------------------------

/// Merge two contribution sets **add-wins by stable id** (never sum-merge), returning a
/// deterministically ordered union. Idempotent and order-independent, which is what makes
/// replayed/reordered sync frames safe. When the same id appears twice the existing copy
/// is kept (contributions are immutable, so payloads are equal by construction).
pub fn merge_contributions(
    existing: &[Contribution],
    incoming: &[Contribution],
) -> Vec<Contribution> {
    let mut merged: Vec<Contribution> =
        Vec::with_capacity(existing.len() + incoming.len());
    for entry in existing.iter().chain(incoming.iter()) {
        if !merged.iter().any(|e| e.id == entry.id) {
            merged.push(entry.clone());
        }
    }
    // Deterministic order independent of input order: by time, then id.
    merged.sort_by(|a, b| {
        a.occurred_at_unix
            .cmp(&b.occurred_at_unix)
            .then_with(|| a.id.cmp(&b.id))
    });
    merged
}

/// Derive per-(project, contributor) effort obligations purely from the unique-id set.
/// Duplicate ids in the input are collapsed first, so the result is invariant under
/// duplication/reordering. Effort is summed into a `u64` to avoid overflow when many
/// `u32` contributions accumulate.
pub fn derive_obligations(contributions: &[Contribution]) -> Vec<Obligation> {
    // Collapse to unique ids (defensive: callers may pass an un-merged list).
    let unique = merge_contributions(contributions, &[]);
    let mut obligations: Vec<Obligation> = Vec::new();
    for c in &unique {
        match obligations
            .iter_mut()
            .find(|o| o.project_id == c.project_id && o.contributor_did == c.contributor_did)
        {
            Some(ob) => {
                ob.total_effort_minutes += u64::from(c.effort_minutes);
                ob.contribution_count += 1;
            }
            None => obligations.push(Obligation {
                project_id: c.project_id.clone(),
                contributor_did: c.contributor_did.clone(),
                total_effort_minutes: 0,
                total_capital_cents: 0,
                resolved_obligation_score: 0.0,
                contribution_count: 0,
            });
        entry.total_effort_minutes += c.effort_minutes as u64;
        entry.total_capital_cents += c.capital_cents;
        // Base rate logic: effort minutes * rate + capital * roi
        let effort_score = (c.effort_minutes as f64) * c.roi_multiplier as f64;
        let capital_score = (c.capital_cents as f64) * c.roi_multiplier as f64;
        entry.resolved_obligation_score += effort_score + capital_score;
        entry.contribution_count += 1;
    }

    let mut result: Vec<_> = agg.into_values().collect();
    // Sort for determinism
    result.sort_by(|a, b| {
        a.project_id
            .cmp(&b.project_id)
            .then(a.contributor_did.cmp(&b.contributor_did))
    });
    result
}

// ---------------------------------------------------------------------------
// Envelope builders
// ---------------------------------------------------------------------------

/// Shared Restricted envelope for cooperative-project records. `valid_time_start_unix`
/// is set by the caller-facing builders to the record's own event time.
fn project_envelope(
    id: &str,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    valid_start_unix: u32,
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
        valid_time_start_unix: Some(valid_start_unix),
        valid_time_end_unix: None,
        predecessor_id,
        blob_hash,
        tombstone: false,
    }
}

pub fn build_project_envelope(
    project: &Project,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    let id = project_record_id(&project.id);
    project_envelope(
        &id,
        owner_did,
        author_did,
        asserted_unix,
        project.created_at_unix,
        None,
        blob_hash,
    )
}

pub fn build_membership_envelope(
    membership: &ProjectMembership,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    let id = project_membership_record_id(&membership.id);
    project_envelope(
        &id,
        owner_did,
        author_did,
        asserted_unix,
        membership.agreed_at_unix,
        None,
        blob_hash,
    )
}

pub fn build_contribution_envelope(
    contribution: &Contribution,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    let id = contribution_record_id(&contribution.id);
    // Preserve the author chain in the envelope's predecessor link.
    let predecessor_id = contribution
        .predecessor_id
        .as_ref()
        .map(|p| contribution_record_id(p));
    project_envelope(
        &id,
        owner_did,
        author_did,
        asserted_unix,
        contribution.occurred_at_unix,
        predecessor_id,
        blob_hash,
    )
}

// ---------------------------------------------------------------------------
// Summaries (serde_json object strings, as stored on the journal row)
// ---------------------------------------------------------------------------

pub fn project_summary(project: &Project) -> String {
    serde_json::json!({
        "name": project.name,
        "description": project.description,
        "created_at_unix": project.created_at_unix,
    })
    .to_string()
}

pub fn membership_summary(membership: &ProjectMembership) -> String {
    serde_json::json!({
        "project_id": membership.project_id,
        "member_did": membership.member_did,
        "role": membership.role,
        "agreed_at_unix": membership.agreed_at_unix,
    })
    .to_string()
}

pub fn contribution_summary(contribution: &Contribution) -> String {
    serde_json::json!({
        "project_id": contribution.project_id,
        "contributor_did": contribution.contributor_did,
        "description": contribution.description,
        "effort_minutes": contribution.effort_minutes,
        "occurred_at_unix": contribution.occurred_at_unix,
        "predecessor_id": contribution.predecessor_id,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contribution(
        id: &str,
        project: &str,
        contributor: &str,
        minutes: u32,
        at: u32,
    ) -> Contribution {
        Contribution {
            id: id.into(),
            project_id: project.into(),
            contributor_did: contributor.into(),
            description: format!("work-{id}"),
            effort_minutes: minutes,
            occurred_at_unix: at,
            predecessor_id: None,
        }
    }

    #[test]
    fn project_envelope_is_restricted_and_kind_correct() {
        let p = Project::new("Community Garden", "Shared beds", 1_700_000_000);
        let env = build_project_envelope(&p, "did:wf:coop", "did:wf:steward", 10, None);
        assert!(env.id.contains(":project:"));
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);
        assert_eq!(env.valid_time_start_unix, Some(1_700_000_000));
    }

    #[test]
    fn membership_envelope_kind_and_class() {
        let m = ProjectMembership::new("proj-1", "did:wf:alice", ProjectRole::Steward, 42);
        let env = build_membership_envelope(&m, "did:wf:coop", "did:wf:coop", 50, None);
        assert!(env.id.contains(":project_membership:"));
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);
    }

    #[test]
    fn contribution_envelope_kind_and_chain() {
        let first = Contribution::new("proj-1", "did:wf:bob", "dig beds", 60, 100);
        let second = Contribution::new("proj-1", "did:wf:bob", "plant seeds", 30, 200)
            .following(&first);
        let env = build_contribution_envelope(&second, "did:wf:coop", "did:wf:bob", 250, None);
        assert!(env.id.contains(":contribution:"));
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);
        // The author chain is preserved as a namespaced predecessor link.
        assert_eq!(
            env.predecessor_id,
            Some(contribution_record_id(&first.id))
        );
        assert!(env.predecessor_id.unwrap().contains(":contribution:"));
    }

    #[test]
    fn merge_dedupes_by_id_add_wins() {
        let a = vec![
            contribution("c1", "p1", "did:wf:a", 60, 1),
            contribution("c2", "p1", "did:wf:a", 30, 2),
        ];
        let dup = vec![contribution("c1", "p1", "did:wf:a", 60, 1)]; // replayed frame
        let merged = merge_contributions(&a, &dup);
        assert_eq!(merged.len(), 2, "duplicate id must not create a second entry");
    }

    #[test]
    fn obligations_invariant_under_duplication_and_reorder() {
        let base = vec![
            contribution("c1", "p1", "did:wf:a", 60, 3),
            contribution("c2", "p1", "did:wf:a", 30, 1),
            contribution("c3", "p1", "did:wf:b", 45, 2),
            contribution("c4", "p2", "did:wf:a", 15, 4),
        ];
        // Reordered + replayed incoming set.
        let incoming = vec![
            contribution("c3", "p1", "did:wf:b", 45, 2),
            contribution("c1", "p1", "did:wf:a", 60, 3),
            contribution("c4", "p2", "did:wf:a", 15, 4),
            contribution("c2", "p1", "did:wf:a", 30, 1),
            contribution("c2", "p1", "did:wf:a", 30, 1), // duplicate
        ];
        let merged = merge_contributions(&base, &incoming);
        let obligations = derive_obligations(&merged);

        // (p1, a): 60 + 30 = 90 over 2 contributions.
        let p1a = obligations
            .iter()
            .find(|o| o.project_id == "p1" && o.contributor_did == "did:wf:a")
            .unwrap();
        assert_eq!(p1a.total_effort_minutes, 90);
        assert_eq!(p1a.contribution_count, 2);

        // (p1, b): 45 over 1.
        let p1b = obligations
            .iter()
            .find(|o| o.project_id == "p1" && o.contributor_did == "did:wf:b")
            .unwrap();
        assert_eq!(p1b.total_effort_minutes, 45);

        // (p2, a): 15 over 1.
        let p2a = obligations
            .iter()
            .find(|o| o.project_id == "p2" && o.contributor_did == "did:wf:a")
            .unwrap();
        assert_eq!(p2a.total_effort_minutes, 15);

        // Order independence: merging the other way round yields identical obligations.
        let obligations2 = derive_obligations(&merge_contributions(&incoming, &base));
        assert_eq!(obligations, obligations2);
    }

    #[test]
    fn replaying_incoming_twice_does_not_double_effort() {
        let base = vec![contribution("c1", "p1", "did:wf:a", 120, 1)];
        let incoming = vec![contribution("c2", "p1", "did:wf:a", 60, 2)];
        let once = derive_obligations(&merge_contributions(&base, &incoming));
        let twice = derive_obligations(&merge_contributions(
            &merge_contributions(&base, &incoming),
            &incoming,
        ));
        assert_eq!(once, twice);
        assert_eq!(once[0].total_effort_minutes, 180);
    }

    #[test]
    fn derive_obligations_dedupes_raw_unmerged_input() {
        // Passing an un-merged list directly must still collapse duplicates.
        let raw = vec![
            contribution("c1", "p1", "did:wf:a", 60, 1),
            contribution("c1", "p1", "did:wf:a", 60, 1),
            contribution("c1", "p1", "did:wf:a", 60, 1),
        ];
        let obligations = derive_obligations(&raw);
        assert_eq!(obligations.len(), 1);
        assert_eq!(obligations[0].total_effort_minutes, 60);
        assert_eq!(obligations[0].contribution_count, 1);
    }

    #[test]
    fn summaries_include_primary_fields() {
        let p = Project::new("Repair Cafe", "Fix things together", 1_700_000_000);
        let ps = project_summary(&p);
        assert!(ps.contains("Repair Cafe"));
        assert!(ps.contains("Fix things together"));

        let m = ProjectMembership::new("proj-1", "did:wf:carol", ProjectRole::Observer, 99);
        let ms = membership_summary(&m);
        assert!(ms.contains("did:wf:carol"));
        assert!(ms.contains("observer"));

        let c = contribution("c1", "p1", "did:wf:dave", 90, 5);
        let cs = contribution_summary(&c);
        assert!(cs.contains("did:wf:dave"));
        assert!(cs.contains("90"));
    }
}
