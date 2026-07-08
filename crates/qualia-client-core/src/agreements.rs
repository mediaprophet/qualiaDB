//! First-class peer-agreement store — the terms of a relationship between parties.
//!
//! This is **P1** of `docs/plans/rights-aware-peer-agreement-addressbook.md`, the counterpart to the
//! `directory.rs` addressbook (**P0**). Where the directory hosts the *Parties* (the addressbook, joined by
//! pairwise DID) this module hosts the *Agreements* that govern each relationship.
//!
//! An [`Agreement`] is the recorded terms of a relationship between parties, **grounded in
//! values-credentials** ([`Agreement::values_anchors`]) — e.g. UDHR articles — rather than asserted from
//! nowhere. Each term is an [`Undertaking`] (a right, obligation, prohibition, or permission) that may cite
//! the values-credential it derives from ([`Undertaking::source`]). Formation is staged
//! ([`FormationStage`]) and each party records its own [`ConsentState`] — the agreement is only fully
//! consented once every party has [`ConsentState::Granted`] (see [`all_granted`]).
//!
//! Following the crate's store conventions (`directory.rs`, `social_peers.rs`), the pure list/consent
//! helpers ([`set_consent`], [`all_granted`]) carry the logic and are unit-tested in isolation, while the
//! `*_agreement(s)` functions layer a thin pretty-JSON persistence step on top (a `Vec<Agreement>` at
//! `app_meta_dir()/agreements.json`).

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::state::app_meta_dir;

/// Where an agreement is in its formation lifecycle.
///
/// `Draft` → `Offered` (put to the other parties) → `Agreed` (all parties consented) → `Ratified` (finalised
/// / signed). The stage is descriptive record-keeping; [`all_granted`] is the authoritative consent check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FormationStage {
    Draft,
    Offered,
    Agreed,
    Ratified,
}

/// The deontic character of a single term within an agreement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UndertakingKind {
    /// A right held by a party.
    Right,
    /// A duty a party owes.
    Obligation,
    /// Something a party must not do.
    Prohibition,
    /// Something a party is permitted to do.
    Permission,
}

/// One term of an agreement: a right, obligation, prohibition, or permission, optionally grounded in a
/// values-credential.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Undertaking {
    /// The deontic character of this term.
    pub kind: UndertakingKind,
    /// Human-readable statement of the term.
    pub text: String,
    /// The values-credential this term derives from (e.g. a UDHR article id), if any. `None` = ungrounded /
    /// self-asserted.
    pub source: Option<String>,
}

/// A party's consent to an agreement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConsentState {
    /// Consent has not yet been given (nor refused).
    Pending,
    /// The party consents.
    Granted,
    /// The party has withdrawn consent it previously gave.
    Withdrawn,
}

/// One party's consent record within an agreement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartyConsent {
    /// The consenting party's DID.
    pub did: String,
    /// This party's current consent state.
    pub consent: ConsentState,
    /// Detached signature over the agreed terms, hex-encoded, once the party has signed. `None` until signed.
    pub signature_hex: Option<String>,
}

/// The recorded terms of a relationship between parties, grounded in values-credentials.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Agreement {
    /// Stable identifier for this agreement (the store key).
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// The DID of the relationship this agreement governs (joins to the directory).
    pub relationship_did: String,
    /// The parties to this agreement, by DID.
    pub parties: Vec<String>,
    /// The values-credentials this agreement is anchored in (e.g. UDHR article ids).
    pub values_anchors: Vec<String>,
    /// The choice of law / jurisdiction governing this agreement (e.g. urn:jurisdiction:AU).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jurisdiction: Option<String>,
    /// The intended purposes of the agreement (e.g. urn:intent:public-good).
    #[serde(default)]
    pub intents: Vec<String>,
    /// The contextual nature of the artifact being produced or licensed (e.g. urn:context:humanitarian-ict).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_context: Option<String>,
    /// The terms of the agreement.
    pub undertakings: Vec<Undertaking>,
    /// Per-party consent records.
    pub consents: Vec<PartyConsent>,
    /// Where the agreement is in its formation lifecycle.
    pub stage: FormationStage,
    /// Unix seconds at which the agreement was created.
    pub created_at: u64,
    /// Unix seconds at which the agreement was last updated.
    pub updated_at: u64,
}

// ---------------------------------------------------------------------------
// Pure helpers (unit-tested; no filesystem)
// ---------------------------------------------------------------------------

/// Set party `did`'s consent to `state`, in place.
///
/// If a [`PartyConsent`] for `did` already exists its `consent` is replaced (its `signature_hex` is kept);
/// otherwise a new `PartyConsent { did, consent: state, signature_hex: None }` is appended.
pub fn set_consent(a: &mut Agreement, did: &str, state: ConsentState) {
    if let Some(slot) = a.consents.iter_mut().find(|c| c.did == did) {
        slot.consent = state;
    } else {
        a.consents.push(PartyConsent {
            did: did.to_string(),
            consent: state,
            signature_hex: None,
        });
    }
}

/// Is every party consented?
///
/// True **iff** `a.parties` is non-empty AND every party DID has a [`PartyConsent`] whose `consent` is
/// [`ConsentState::Granted`]. An agreement with no parties is never "all granted".
pub fn all_granted(a: &Agreement) -> bool {
    !a.parties.is_empty()
        && a.parties.iter().all(|did| {
            a.consents
                .iter()
                .any(|c| c.did == *did && c.consent == ConsentState::Granted)
        })
}

// ---------------------------------------------------------------------------
// Persistence (filesystem)
// ---------------------------------------------------------------------------

fn agreements_path() -> PathBuf {
    app_meta_dir().join("agreements.json")
}

fn save_agreements(agreements: &[Agreement]) -> Result<(), String> {
    let path = agreements_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(agreements).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

/// Load every stored agreement. Returns `vec![]` if the store file is absent or unreadable.
pub fn list_agreements() -> Vec<Agreement> {
    fs::read_to_string(agreements_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Insert-or-update `a` (keyed by [`Agreement::id`]), then persist the store.
///
/// If an agreement with the same `id` already exists it is replaced in place (preserving its position);
/// otherwise `a` is appended.
pub fn upsert_agreement(a: Agreement) -> Result<(), String> {
    let mut agreements = list_agreements();
    if let Some(slot) = agreements.iter_mut().find(|x| x.id == a.id) {
        *slot = a;
    } else {
        agreements.push(a);
    }
    save_agreements(&agreements)
}

/// Every agreement involving `did` — as a party ([`Agreement::parties`]) OR as the relationship the
/// agreement governs ([`Agreement::relationship_did`]).
pub fn agreements_for(did: &str) -> Vec<Agreement> {
    list_agreements()
        .into_iter()
        .filter(|a| a.relationship_did == did || a.parties.iter().any(|p| p == did))
        .collect()
}

// ---------------------------------------------------------------------------
// Tests — PURE ONLY. These build `Agreement` values in memory and exercise the
// pure helpers; they never touch the real filesystem / app dir.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A two-party agreement with no consent records yet.
    fn agreement(parties: &[&str]) -> Agreement {
        Agreement {
            id: "ag-1".to_string(),
            title: "Care relationship".to_string(),
            relationship_did: "did:wf:rel".to_string(),
            parties: parties.iter().map(|s| s.to_string()).collect(),
            values_anchors: vec!["udhr:art-12".to_string()],
            undertakings: vec![Undertaking {
                kind: UndertakingKind::Right,
                text: "The person controls their own health data.".to_string(),
                source: Some("udhr:art-12".to_string()),
            }],
            consents: vec![],
            stage: FormationStage::Draft,
            created_at: 1_700_000_000,
            updated_at: 1_700_000_000,
        }
    }

    #[test]
    fn set_consent_on_new_did_appends_granted() {
        let mut a = agreement(&["did:wf:alice", "did:wf:bob"]);
        assert!(a.consents.is_empty());

        set_consent(&mut a, "did:wf:alice", ConsentState::Granted);

        assert_eq!(a.consents.len(), 1);
        assert_eq!(a.consents[0].did, "did:wf:alice");
        assert_eq!(a.consents[0].consent, ConsentState::Granted);
        assert_eq!(a.consents[0].signature_hex, None);
    }

    #[test]
    fn set_consent_on_existing_did_updates_in_place() {
        let mut a = agreement(&["did:wf:alice", "did:wf:bob"]);
        set_consent(&mut a, "did:wf:alice", ConsentState::Granted);
        // Give the existing record a signature to prove it survives the update.
        a.consents[0].signature_hex = Some("deadbeef".to_string());

        set_consent(&mut a, "did:wf:alice", ConsentState::Withdrawn);

        // Still one record (updated, not appended); signature preserved; consent changed.
        assert_eq!(a.consents.len(), 1);
        assert_eq!(a.consents[0].did, "did:wf:alice");
        assert_eq!(a.consents[0].consent, ConsentState::Withdrawn);
        assert_eq!(a.consents[0].signature_hex, Some("deadbeef".to_string()));
    }

    #[test]
    fn all_granted_false_while_any_pending_true_when_all_granted() {
        let mut a = agreement(&["did:wf:alice", "did:wf:bob"]);

        // No consents at all → not all granted.
        assert!(!all_granted(&a));

        // One granted, the other still (implicitly) pending → not all granted.
        set_consent(&mut a, "did:wf:alice", ConsentState::Granted);
        assert!(!all_granted(&a));

        // Bob explicitly Pending → still not all granted.
        set_consent(&mut a, "did:wf:bob", ConsentState::Pending);
        assert!(!all_granted(&a));

        // Bob grants → now every party is granted.
        set_consent(&mut a, "did:wf:bob", ConsentState::Granted);
        assert!(all_granted(&a));
    }

    #[test]
    fn all_granted_false_for_empty_parties() {
        let mut a = agreement(&[]);
        // Even a stray granted consent doesn't make a party-less agreement "all granted".
        set_consent(&mut a, "did:wf:ghost", ConsentState::Granted);
        assert!(!all_granted(&a));
    }
}
