//! **Incapacity switch** — involuntary psychiatric admission / serious injury (more common than death), and
//! the **discrediting-counter** that goes with it.
//!
//! The consideration (Timothy, 2026-07-06): the outcome for a person seeking protection is *more often* an
//! **involuntary psychiatric admission** or a **serious injury** than death. Both incapacitate — but,
//! unlike death, are **reversible** (the person recovers). Two mechanisms are needed:
//!
//! 1. **Advocacy during incapacity.** A pre-designated advocate/trustee is activated to act on the person's
//!    behalf, under a **gamified, corroborated** trigger (a quorum of participating parties attest, optionally
//!    plus an official instrument — a committal order / medical record), and **reverses** on recovery.
//!
//! 2. **A counter to weaponised discrediting.** The sharp part: an involuntary psychiatric committal is
//!    frequently *weaponised* — the intent is to ensure **no-one believes anything the person says** — and
//!    that discrediting is **leveraged off privacy** (the committal taints; privacy then hides the context
//!    that would exonerate). The counter is that the person (or their advocate) can **choose to make prior
//!    events transparent** — the durable, un-erasable disclosure/conduct/`cc` record (e.g. "reported to the
//!    MP, then committed" → retaliation, not madness). Crucially this is:
//!    - **the person's *choice*** — privacy is never forcibly lifted (autonomy);
//!    - **honest-contingent** — the system *enables* truthful transparency; it cannot compel honesty, and it
//!      cannot make a dishonest record true. "If the person is willing to be honest — which isn't always the
//!      case." What keeps even a *selective* disclosure bounded is that the underlying records are **durable +
//!      tamper-evident** (from the commons + disclosure-trace layers): the person can choose *what* to reveal,
//!      but cannot delete what they don't, and the invocation itself is recorded.
//!
//! Domain model + invariants; the key-release/advocacy wiring + the storage compose from
//! [`crate::consent_credential`] / [`crate::disclosure_trace`] and the vault (coordinate).

use serde::{Deserialize, Serialize};

use crate::consent_credential::PayloadCommitment;

/// The kind of incapacity a switch covers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncapacityKind {
    /// Involuntary psychiatric admission (the weaponised-discrediting case).
    InvoluntaryPsychiatric,
    /// A serious injury leaving the person unable to manage their affairs.
    SeriousInjury,
    Other(String),
}

/// The **gamified, corroborated** trigger for activating advocacy: a quorum of participating parties attest
/// the incapacity, optionally **also** requiring an independent official instrument (a committal order /
/// medical record). Resists a false trigger by any single party.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncapacityTrigger {
    /// The advocates/friends who may attest the incapacity.
    pub parties: Vec<String>,
    /// At least this many **distinct participating** parties must attest.
    pub attestation_threshold: usize,
    /// If true, an official instrument (committal order / medical record) is **also** required — a second,
    /// independent corroboration, so party-attestation alone cannot activate it.
    pub require_official_instrument: bool,
}

impl IncapacityTrigger {
    /// Satisfied iff (official instrument present, if required) AND ≥ `attestation_threshold` distinct
    /// *participating* parties have attested.
    pub fn is_satisfied(&self, attesting_parties: &[String], official_instrument: Option<&str>) -> bool {
        if self.require_official_instrument && official_instrument.is_none() {
            return false;
        }
        let distinct: std::collections::BTreeSet<&str> = attesting_parties
            .iter()
            .map(|s| s.as_str())
            .filter(|d| self.parties.iter().any(|p| p == d))
            .collect();
        distinct.len() >= self.attestation_threshold
    }
}

/// A reversible incapacity switch: activates a pre-designated advocate under the trigger, reverses on
/// recovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncapacitySwitch {
    pub principal_did: String,
    pub kind: IncapacityKind,
    pub trigger: IncapacityTrigger,
    /// The advocate/trustee pre-designated to act during incapacity (scoped, revocable — a care/steward role,
    /// never custody of the person's fabric).
    pub advocate_did: String,
    /// `Some(t)` while the advocate is active (incapacitated since `t`); `None` when the person has capacity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_since_unix: Option<u64>,
}

impl IncapacitySwitch {
    /// Is the advocate currently acting (the person incapacitated)?
    pub fn advocate_active(&self) -> bool {
        self.active_since_unix.is_some()
    }

    /// Whether the switch can be activated now (trigger satisfied and not already active).
    pub fn can_activate(&self, attesting_parties: &[String], official_instrument: Option<&str>) -> bool {
        !self.advocate_active() && self.trigger.is_satisfied(attesting_parties, official_instrument)
    }

    /// Activate advocacy if the trigger is satisfied. Returns whether it activated.
    pub fn activate(
        &mut self,
        attesting_parties: &[String],
        official_instrument: Option<&str>,
        now_unix: u64,
    ) -> bool {
        if self.can_activate(attesting_parties, official_instrument) {
            self.active_since_unix = Some(now_unix);
            true
        } else {
            false
        }
    }

    /// The person **regained capacity** — reverse: the advocate stands down, control reverts to the
    /// principal. The reversibility that distinguishes incapacity from death.
    pub fn regain_capacity(&mut self, _now_unix: u64) {
        self.active_since_unix = None;
    }
}

/// A **transparency invocation** — the person (or, during incapacity, their advocate) *chooses* to make a
/// **scoped** set of prior-events records transparent, to **counter discrediting** by showing context. The
/// counter to privacy-weaponisation: the person's own durable, un-erasable record, disclosed on **their**
/// terms.
///
/// It is the person's **choice** (privacy is not forcibly lifted) and its value is **honest-contingent** —
/// the system enables truthful transparency; it neither compels honesty nor can make a dishonest record true.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyInvocation {
    /// Who invoked it — the person, or their advocate during a validated incapacity.
    pub invoker_did: String,
    /// Whose record it is (the person).
    pub subject_did: String,
    /// The prior-events records (by commitment) being made transparent — a **scoped** selection the invoker
    /// chooses.
    pub disclosed_commitments: Vec<PayloadCommitment>,
    /// Why (e.g. "counter discrediting after involuntary committal — show the retaliation timeline").
    pub purpose: String,
    pub invoked_unix: u64,
}

impl TransparencyInvocation {
    /// Whether an advocate may invoke this on the subject's behalf — only during a *validated active*
    /// incapacity for that subject (otherwise only the subject themselves may). Enforces "the advocate acts
    /// only while the person cannot".
    pub fn advocate_may_invoke(&self, switch: &IncapacitySwitch) -> bool {
        self.invoker_did == self.subject_did
            || (self.invoker_did == switch.advocate_did
                && switch.principal_did == self.subject_did
                && switch.advocate_active())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn switch() -> IncapacitySwitch {
        IncapacitySwitch {
            principal_did: "did:wf:person".into(),
            kind: IncapacityKind::InvoluntaryPsychiatric,
            trigger: IncapacityTrigger {
                parties: vec!["did:wf:alice".into(), "did:wf:bob".into(), "did:wf:carol".into()],
                attestation_threshold: 2,
                require_official_instrument: true,
            },
            advocate_did: "did:wf:advocate".into(),
            active_since_unix: None,
        }
    }

    #[test]
    fn requires_quorum_and_the_official_instrument_to_activate() {
        let mut s = switch();
        let quorum = vec!["did:wf:alice".to_string(), "did:wf:bob".to_string()];

        // Quorum but NO official instrument → not satisfied (needs corroboration).
        assert!(!s.can_activate(&quorum, None));
        // Official instrument but only one party → below threshold.
        assert!(!s.can_activate(&["did:wf:alice".to_string()], Some("committal-order:7")));
        // Both → activates.
        assert!(s.activate(&quorum, Some("committal-order:7"), 1_000));
        assert!(s.advocate_active());
    }

    #[test]
    fn non_party_attestations_do_not_count() {
        let s = switch();
        let mixed = vec!["did:wf:stranger".to_string(), "did:wf:alice".to_string()];
        assert!(!s.can_activate(&mixed, Some("order")), "only participating parties count toward quorum");
    }

    #[test]
    fn incapacity_is_reversible_the_person_recovers_and_reclaims_control() {
        let mut s = switch();
        let quorum = vec!["did:wf:alice".to_string(), "did:wf:bob".to_string()];
        assert!(s.activate(&quorum, Some("order"), 1_000));
        assert!(s.advocate_active());
        // The person recovers → advocate stands down, control reverts.
        s.regain_capacity(2_000);
        assert!(!s.advocate_active(), "reversible — not death");
        // Can re-activate if incapacitated again later.
        assert!(s.activate(&quorum, Some("order-2"), 3_000));
        assert!(s.advocate_active());
    }

    #[test]
    fn the_person_can_always_invoke_transparency_to_counter_discrediting() {
        let s = switch(); // not active — the person has capacity
        let inv = TransparencyInvocation {
            invoker_did: "did:wf:person".into(),
            subject_did: "did:wf:person".into(),
            disclosed_commitments: vec![[1u8; 32], [2u8; 32]], // the reported-to-MP timeline, e.g.
            purpose: "counter discrediting after involuntary committal".into(),
            invoked_unix: 1_500,
        };
        // The person can always make their OWN prior events transparent, on their terms.
        assert!(inv.advocate_may_invoke(&s));
        assert_eq!(inv.disclosed_commitments.len(), 2, "a scoped selection they choose");
    }

    #[test]
    fn an_advocate_may_invoke_only_during_a_validated_active_incapacity() {
        let mut s = switch();
        let inv = TransparencyInvocation {
            invoker_did: "did:wf:advocate".into(),
            subject_did: "did:wf:person".into(),
            disclosed_commitments: vec![[1u8; 32]],
            purpose: "counter discrediting while the person is committed".into(),
            invoked_unix: 1_500,
        };
        // Not active yet → the advocate may NOT invoke on the person's behalf.
        assert!(!inv.advocate_may_invoke(&s));
        // Once a validated incapacity is active, the advocate may.
        s.activate(&["did:wf:alice".to_string(), "did:wf:bob".to_string()], Some("order"), 1_000);
        assert!(inv.advocate_may_invoke(&s));
        // And once the person recovers, the advocate may not again.
        s.regain_capacity(2_000);
        assert!(!inv.advocate_may_invoke(&s), "advocate acts only while the person cannot");
    }

    #[test]
    fn serde_round_trips() {
        let mut s = switch();
        s.activate(&["did:wf:alice".to_string(), "did:wf:bob".to_string()], Some("order"), 1_000);
        let back: IncapacitySwitch = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(s, back);
    }
}
