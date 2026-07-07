//! **Dead-man switch** — post-death (or believed-death) disposition of the principal's data, under
//! **gamified validation rules**, enacted by the person's **chosen friends who hold the dataset**.
//!
//! The consideration (Timothy, 2026-07-06): if the principal (the person) is *considered dead*, their
//! information may be made public, or become subject to other rules — the erasure-prevention / right-to-truth
//! stance (a murdered person's record must not simply vanish), governed by the person's **prior
//! self-definition** and kept **reversible** (see the post-death-continuity work).
//!
//! The trigger must **not** be one abusable "declare dead" button — that would let an attacker (or a
//! betrayer) fire it falsely. So validation is **gamified**: a *rule set* of independent conditions that must
//! all hold — a **liveness lapse** (no update / no "still here" signal from the principal for X time) **and**
//! an **attestation threshold** (a quorum of the participating parties attesting no-contact / believed-dead /
//! abandonment). And it is enacted by the **friends who store the encrypted dataset** (the
//! `EncryptedCommonsPayload` storers): they hold the ciphertext *and* validate the trigger, so no single
//! party — and no outside actor — can enact it alone.
//!
//! **Reversibility is central.** The principal showing up alive (`principal_alive`) resets the liveness
//! signal and un-fires a not-yet-irreversible switch. (Honesty caveat, §9: once a [`Disposition::MakePublic`]
//! has actually released keys to a durable commons it cannot be un-published — so *which* dispositions are
//! reversible, and the grace/limits, are values calls for Timothy.)
//!
//! Domain model + invariants only; the actual **key-release** on enactment (publish the data key / issue
//! [`ConsentCredential`](crate::consent_credential::ConsentCredential)s to the disposition parties) and the
//! storage are the crypto/commons composition (coordinate).

use serde::{Deserialize, Serialize};

use crate::consent_credential::PayloadCommitment;

/// A **liveness heartbeat** — the principal periodically signals "still here". Its lapse (no update for
/// `lapse_after_secs`) is one of the trigger conditions; touching it (the person is alive) is the reversibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heartbeat {
    pub last_seen_unix: u64,
    /// Grace period (seconds): with no update for at least this long, the liveness signal is *lapsed*.
    pub lapse_after_secs: u64,
}

impl Heartbeat {
    pub fn new(last_seen_unix: u64, lapse_after_secs: u64) -> Self {
        Self { last_seen_unix, lapse_after_secs }
    }
    /// Has the liveness signal lapsed at `now`?
    pub fn is_lapsed(&self, now_unix: u64) -> bool {
        now_unix.saturating_sub(self.last_seen_unix) >= self.lapse_after_secs
    }
    /// The principal is alive — reset the signal (reversibility).
    pub fn touch(&mut self, now_unix: u64) {
        self.last_seen_unix = now_unix;
    }
}

/// What a party attests toward the trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttestationKind {
    /// Has had no contact with the principal.
    NoContact,
    /// Believes the principal is dead.
    BelievedDead,
    /// Releases/abandons their hold (e.g. "the last lets go" — the abandonment condition).
    Abandon,
}

/// One participating party's attestation toward enacting the switch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyAttestation {
    pub party_did: String,
    pub kind: AttestationKind,
    pub time_unix: u64,
}

/// What happens to the data if the switch fires — the person's **prior self-definition** governs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    /// Make the payload public (release the data key widely). **Irreversible in effect** once released.
    MakePublic,
    /// Release access to specific parties (trustees, next-of-kin, a chosen representative) — reversible
    /// (their credentials can be revoked).
    ReleaseTo { parties: Vec<String> },
    /// Enact other self-defined post-death rules (e.g. the digital-vellum representation), grounded-or-refused.
    SelfDefinedRules { rules_ref: String },
}

/// The **gamified trigger rule** — the independent conditions that must *all* hold to fire, resisting a false
/// trigger by any single party or an outside actor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerRule {
    /// Require the liveness heartbeat to have lapsed (no "still here" for the grace period).
    pub require_heartbeat_lapsed: bool,
    /// At least this many **distinct participating parties** must attest.
    pub attestation_threshold: usize,
    /// The participating parties — the friends/stewards who hold the dataset and validate the trigger.
    pub parties: Vec<String>,
}

impl TriggerRule {
    /// Is the rule satisfied at `now`, given the heartbeat + the attestations collected? Requires (heartbeat
    /// lapsed, if required) AND (≥ `attestation_threshold` distinct *participating* parties have attested).
    pub fn is_satisfied(
        &self,
        heartbeat: &Heartbeat,
        attestations: &[PartyAttestation],
        now_unix: u64,
    ) -> bool {
        if self.require_heartbeat_lapsed && !heartbeat.is_lapsed(now_unix) {
            return false;
        }
        let is_party = |did: &str| self.parties.iter().any(|p| p == did);
        let attesters: std::collections::BTreeSet<&str> = attestations
            .iter()
            .map(|a| a.party_did.as_str())
            .filter(|d| is_party(d))
            .collect();
        attesters.len() >= self.attestation_threshold
    }
}

/// A dead-man switch over one commons payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeadMansSwitch {
    /// The dataset this governs (by commitment) — held by the friends (the commons storers).
    pub payload_commitment: PayloadCommitment,
    pub heartbeat: Heartbeat,
    pub trigger: TriggerRule,
    pub disposition: Disposition,
    /// When the switch fired (enacted), if it has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fired_unix: Option<u64>,
}

impl DeadMansSwitch {
    /// Is the switch *currently triggerable* — the gamified rule satisfied and not already fired?
    pub fn is_triggered(&self, attestations: &[PartyAttestation], now_unix: u64) -> bool {
        self.fired_unix.is_none()
            && self.trigger.is_satisfied(&self.heartbeat, attestations, now_unix)
    }

    /// The principal is alive — reset the liveness signal **and un-fire** a not-yet-enacted switch. This is
    /// the reversibility: a person showing up defeats a premature or malicious trigger.
    pub fn principal_alive(&mut self, now_unix: u64) {
        self.heartbeat.touch(now_unix);
        self.fired_unix = None;
    }

    /// **Enact** the switch if triggerable: record it fired and return the [`Disposition`] to apply (which
    /// the caller carries out — publishing the key / issuing credentials to the disposition parties). Returns
    /// `None` if the rule is not satisfied (or it already fired).
    pub fn enact(&mut self, attestations: &[PartyAttestation], now_unix: u64) -> Option<&Disposition> {
        if self.is_triggered(attestations, now_unix) {
            self.fired_unix = Some(now_unix);
            Some(&self.disposition)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const C: PayloadCommitment = [11u8; 32];
    const DAY: u64 = 24 * 60 * 60;

    fn attest(did: &str, kind: AttestationKind) -> PartyAttestation {
        PartyAttestation { party_did: did.into(), kind, time_unix: 0 }
    }

    /// A switch requiring: heartbeat lapsed after 90 days AND 2-of-3 friends attesting.
    fn switch() -> DeadMansSwitch {
        DeadMansSwitch {
            payload_commitment: C,
            heartbeat: Heartbeat::new(1_000_000, 90 * DAY),
            trigger: TriggerRule {
                require_heartbeat_lapsed: true,
                attestation_threshold: 2,
                parties: vec!["did:wf:alice".into(), "did:wf:bob".into(), "did:wf:carol".into()],
            },
            disposition: Disposition::MakePublic,
            fired_unix: None,
        }
    }

    #[test]
    fn does_not_fire_while_the_principal_is_alive_even_if_friends_attest() {
        let s = switch();
        let now = 1_000_000 + 10 * DAY; // heartbeat fresh (10 days < 90)
        let attests = vec![attest("did:wf:alice", AttestationKind::BelievedDead), attest("did:wf:bob", AttestationKind::NoContact)];
        assert!(!s.is_triggered(&attests, now), "fresh liveness signal defeats the trigger");
    }

    #[test]
    fn does_not_fire_on_one_partys_say_so_below_threshold() {
        let s = switch();
        let now = 1_000_000 + 200 * DAY; // heartbeat lapsed
        let one = vec![attest("did:wf:alice", AttestationKind::BelievedDead)];
        assert!(!s.is_triggered(&one, now), "a single party cannot enact it (resists false trigger)");
    }

    #[test]
    fn fires_only_when_liveness_lapsed_and_the_quorum_attests() {
        let mut s = switch();
        let now = 1_000_000 + 200 * DAY; // lapsed
        let quorum = vec![
            attest("did:wf:alice", AttestationKind::BelievedDead),
            attest("did:wf:bob", AttestationKind::NoContact),
        ];
        assert!(s.is_triggered(&quorum, now));
        // Enact → returns the disposition, records fired.
        assert_eq!(s.enact(&quorum, now), Some(&Disposition::MakePublic));
        assert!(s.fired_unix.is_some());
        // Idempotent — does not re-fire.
        assert!(s.enact(&quorum, now + DAY).is_none());
    }

    #[test]
    fn non_party_attestations_do_not_count_toward_the_quorum() {
        let s = switch();
        let now = 1_000_000 + 200 * DAY;
        // An outsider + one real party = still below the 2-party threshold.
        let mixed = vec![
            attest("did:wf:stranger", AttestationKind::BelievedDead),
            attest("did:wf:alice", AttestationKind::BelievedDead),
        ];
        assert!(!s.is_triggered(&mixed, now), "only participating parties count");
    }

    #[test]
    fn reversibility_the_principal_returning_alive_defeats_and_unfires_it() {
        let mut s = switch();
        let now = 1_000_000 + 200 * DAY;
        let quorum = vec![
            attest("did:wf:alice", AttestationKind::BelievedDead),
            attest("did:wf:bob", AttestationKind::NoContact),
        ];
        assert!(s.enact(&quorum, now).is_some(), "fired");
        // The person shows up alive → reset + un-fire (reversibility).
        s.principal_alive(now + DAY);
        assert!(s.fired_unix.is_none(), "un-fired");
        assert!(!s.is_triggered(&quorum, now + 2 * DAY), "fresh liveness defeats the same attestations");
    }

    #[test]
    fn abandonment_by_all_parties_is_a_configurable_rule() {
        // A rule where the condition is that ALL parties abandon (attestation_threshold == parties.len()),
        // heartbeat not required — "when the last lets go".
        let mut s = switch();
        s.trigger.require_heartbeat_lapsed = false;
        s.trigger.attestation_threshold = 3; // all three
        s.disposition = Disposition::SelfDefinedRules { rules_ref: "vellum:self-defined".into() };
        let all = vec![
            attest("did:wf:alice", AttestationKind::Abandon),
            attest("did:wf:bob", AttestationKind::Abandon),
            attest("did:wf:carol", AttestationKind::Abandon),
        ];
        assert!(s.is_triggered(&all, 0), "all parties abandoning satisfies this rule");
        // Two-of-three abandoning does not.
        assert!(!s.is_triggered(&all[..2], 0));
    }

    #[test]
    fn serde_round_trips() {
        let s = switch();
        let back: DeadMansSwitch = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(s, back);
    }
}
