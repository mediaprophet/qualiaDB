//! **Disclosure traceability** — so a *betrayal is knowable and attributable*.
//!
//! The high-stakes case (Timothy, 2026-07-06): a person seeking protection from serious crime "**cc**"s a
//! **transparency credential** to an oversight authority — their local member of parliament, a minister — to
//! get help / put them on notice. But the perpetrator may be a **PEP** (politically-exposed person) or a
//! political **donor** with influence over that very authority. If the authority — **or their staff** — leaks
//! the disclosure to the perpetrator, threats follow. The system must make that **knowable**: *who* accessed
//! or was told *what*, *when*, under *which* credential, and — crucially — by *whom* (including a **delegate**
//! such as an MP's staffer acting under the MP's credential), with a per-recipient **fingerprint** so a
//! leaked copy (or leaked knowledge) traces back to its source.
//!
//! This is the anti-corruption / anti-retaliation substrate for the **knowledge economy** around protection —
//! and it is *particularly* load-bearing for **UN / World-Bank development-funding** and **human-rights
//! support** use-cases, where beneficiaries and whistle-blowers face capture and reprisal, and where "who
//! knew, and who told" must survive powerful actors' attempts to hide it.
//!
//! It composes with [`crate::consent_credential`]: disclosures are of the durable, un-deletable
//! `EncryptedCommonsPayload` (so the *trace itself cannot be erased* by a betrayer), each carrying the
//! payload's commitment. The real per-recipient watermark / traitor-tracing scheme and the tamper-evident
//! store (signed WAL + commons) are the crypto/storage composition (coordinate); this is the domain model +
//! the invariants: **the trace makes the leak knowable, and attributable to a specific actor.**

use serde::{Deserialize, Serialize};

use crate::consent_credential::PayloadCommitment;

/// A per-recipient **tracing fingerprint** — a unique tag bound to *one disclosure to one party*, so a
/// leaked copy (or knowledge recovered from a leak) can be traced to whose disclosure it came from. The real
/// mechanism is per-recipient watermarking / traitor-tracing; here it is the tag the trace keys on.
pub type DisclosureFingerprint = [u8; 16];

/// What kind of disclosure an event records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisclosureKind {
    /// The recipient (or their delegate) accessed the payload directly under their credential.
    DirectAccess,
    /// The recipient (or their delegate) **shared it onward** to another party — recorded, so the chain of
    /// who-told-whom is traced (this is how an onward leak becomes visible).
    OnwardShare { to_did: String },
}

/// One traced access / disclosure event — **durable and tamper-evident** (of the commons payload; a person
/// revoking access, or a betrayer, cannot erase it). Says *who* was given/took access to *what*, *when*,
/// under *which* credential, and *by whom* (including a delegate).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisclosureEvent {
    pub id: String,
    /// The commons payload disclosed (by commitment).
    pub payload_commitment: PayloadCommitment,
    /// Under which consent / transparency credential the disclosure occurred.
    pub credential_id: String,
    /// The party the disclosure was **to** / who holds the credential (e.g. the MP / minister).
    pub recipient_did: String,
    /// If a **delegate** actually acted under the credential (e.g. the MP's staffer), their identifier.
    /// `None` = the recipient themselves acted. **This is what makes a staff leak attributable.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acting_delegate_did: Option<String>,
    pub time_unix: u64,
    /// The per-recipient tracing fingerprint bound to this disclosure.
    pub fingerprint: DisclosureFingerprint,
    pub kind: DisclosureKind,
}

impl DisclosureEvent {
    /// The **actor accountable** for this disclosure: the acting delegate (a staffer) if one acted, else the
    /// recipient (the authority) themselves. A leak traced to this event is attributable to this actor.
    pub fn accountable_actor(&self) -> &str {
        self.acting_delegate_did.as_deref().unwrap_or(&self.recipient_did)
    }
}

/// A "**cc**" / transparency credential note — the record that the person **informed an oversight authority**
/// (MP / minister) for transparency / protection. The record is itself protective and **durable**: "I
/// informed them on date X for purpose Y" is provable, so if the authority betrays or fails to act, that is
/// knowable *against this record*.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyCc {
    /// The credential that carries the disclosure (links to the access + the trace).
    pub credential_id: String,
    /// The authority informed (the MP / minister).
    pub informed_authority_did: String,
    /// Why they were informed (e.g. "protection from serious crime").
    pub purpose: String,
    pub informed_unix: u64,
}

/// Every disclosure of a given payload, in order — the **audit chain**: who could have leaked this, and by
/// which route (direct access vs onward share). Tamper-evident; survives revocation.
pub fn disclosure_chain<'a>(
    events: &'a [DisclosureEvent],
    commitment: &PayloadCommitment,
) -> Vec<&'a DisclosureEvent> {
    events.iter().filter(|e| &e.payload_commitment == commitment).collect()
}

/// The distinct actors who had access to a payload (recipients + any acting delegates) — the set the leak
/// **must** be within. If the perpetrator demonstrably knows something disclosed only here, the leak is one
/// of these.
pub fn actors_with_access<'a>(
    events: &'a [DisclosureEvent],
    commitment: &PayloadCommitment,
) -> Vec<&'a str> {
    let mut out: Vec<&str> = Vec::new();
    for e in events.iter().filter(|e| &e.payload_commitment == commitment) {
        for actor in [Some(e.recipient_did.as_str()), e.acting_delegate_did.as_deref()]
            .into_iter()
            .flatten()
        {
            if !out.contains(&actor) {
                out.push(actor);
            }
        }
    }
    out
}

/// Trace a leak by its fingerprint — the disclosure it came from — making the betrayal **knowable**. A
/// leaked copy (or knowledge recovered from a leak) carrying `leaked` is matched to the exact disclosure, and
/// thence to the [`accountable_actor`](DisclosureEvent::accountable_actor) (the authority, or their staffer).
pub fn trace_leak<'a>(
    events: &'a [DisclosureEvent],
    leaked: &DisclosureFingerprint,
) -> Option<&'a DisclosureEvent> {
    events.iter().find(|e| &e.fingerprint == leaked)
}

#[cfg(test)]
mod tests {
    use super::*;

    const C: PayloadCommitment = [9u8; 32];
    const FP_MP: DisclosureFingerprint = [1u8; 16];
    const FP_STAFF: DisclosureFingerprint = [2u8; 16];
    const FP_MINISTER: DisclosureFingerprint = [3u8; 16];

    fn ev(
        id: &str,
        recipient: &str,
        delegate: Option<&str>,
        fp: DisclosureFingerprint,
        kind: DisclosureKind,
    ) -> DisclosureEvent {
        DisclosureEvent {
            id: id.into(),
            payload_commitment: C,
            credential_id: "cc-transparency".into(),
            recipient_did: recipient.into(),
            acting_delegate_did: delegate.map(|d| d.into()),
            time_unix: 1_000,
            fingerprint: fp,
            kind,
        }
    }

    #[test]
    fn a_transparency_cc_records_that_the_authority_was_informed() {
        let cc = TransparencyCc {
            credential_id: "cc-transparency".into(),
            informed_authority_did: "did:wf:mp-smith".into(),
            purpose: "protection from serious crime".into(),
            informed_unix: 1_000,
        };
        // "I informed them on date X for purpose Y" is provable — the protective record.
        assert_eq!(cc.informed_authority_did, "did:wf:mp-smith");
        assert_eq!(cc.purpose, "protection from serious crime");
    }

    #[test]
    fn a_leak_is_traceable_to_its_source_and_attributable_to_the_actor() {
        // The person cc's the MP; the MP's disclosure and the minister's each carry a distinct fingerprint.
        let events = vec![
            ev("d1", "did:wf:mp-smith", None, FP_MP, DisclosureKind::DirectAccess),
            ev("d2", "did:wf:minister", None, FP_MINISTER, DisclosureKind::DirectAccess),
        ];
        // A leaked copy carrying the MP's fingerprint surfaces near the perpetrator → traced to the MP.
        let src = trace_leak(&events, &FP_MP).expect("leak traced");
        assert_eq!(src.recipient_did, "did:wf:mp-smith");
        assert_eq!(src.accountable_actor(), "did:wf:mp-smith", "attributable to the MP");
    }

    #[test]
    fn a_staff_leak_is_attributed_to_the_staffer_not_only_the_authority() {
        // The MP's STAFFER accessed under the MP's credential and leaked. The trace attributes it to the
        // staffer (the accountable actor) — the "or their staff" case.
        let events = vec![ev(
            "d1",
            "did:wf:mp-smith",
            Some("did:wf:mp-staffer-jones"),
            FP_STAFF,
            DisclosureKind::DirectAccess,
        )];
        let src = trace_leak(&events, &FP_STAFF).unwrap();
        assert_eq!(src.recipient_did, "did:wf:mp-smith", "under the MP's credential");
        assert_eq!(
            src.accountable_actor(),
            "did:wf:mp-staffer-jones",
            "attributable to the specific staffer who acted"
        );
    }

    #[test]
    fn onward_sharing_is_recorded_so_the_chain_is_traced() {
        // The staffer shares onward to the perpetrator — recorded, so the route is visible.
        let events = vec![
            ev("d1", "did:wf:mp-smith", Some("did:wf:mp-staffer-jones"), FP_STAFF, DisclosureKind::DirectAccess),
            ev(
                "d2",
                "did:wf:mp-smith",
                Some("did:wf:mp-staffer-jones"),
                [4u8; 16],
                DisclosureKind::OnwardShare { to_did: "did:wf:perpetrator-pep".into() },
            ),
        ];
        let chain = disclosure_chain(&events, &C);
        assert_eq!(chain.len(), 2);
        // The onward share to the perpetrator is on the record, attributed to the staffer.
        let onward = chain
            .iter()
            .find(|e| matches!(&e.kind, DisclosureKind::OnwardShare { to_did } if to_did == "did:wf:perpetrator-pep"))
            .expect("onward share recorded");
        assert_eq!(onward.accountable_actor(), "did:wf:mp-staffer-jones");
    }

    #[test]
    fn the_leak_set_is_bounded_to_who_had_access() {
        // If the perpetrator knows something disclosed only to these actors, the leak is one of them.
        let events = vec![
            ev("d1", "did:wf:mp-smith", Some("did:wf:mp-staffer-jones"), FP_STAFF, DisclosureKind::DirectAccess),
            ev("d2", "did:wf:minister", None, FP_MINISTER, DisclosureKind::DirectAccess),
        ];
        let actors = actors_with_access(&events, &C);
        assert!(actors.contains(&"did:wf:mp-smith"));
        assert!(actors.contains(&"did:wf:mp-staffer-jones"));
        assert!(actors.contains(&"did:wf:minister"));
        assert_eq!(actors.len(), 3, "the bounded set the leak must be within");
    }

    #[test]
    fn serde_round_trips() {
        let e = ev("d1", "did:wf:mp", Some("did:wf:staff"), FP_STAFF, DisclosureKind::DirectAccess);
        let back: DisclosureEvent = serde_json::from_str(&serde_json::to_string(&e).unwrap()).unwrap();
        assert_eq!(e, back);
    }
}
