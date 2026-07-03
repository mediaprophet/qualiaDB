//! The **Trigger algebra** (ADR §7.1) — *when* a delegated authority activates.
//!
//! A `Trigger` is a bounded boolean expression over primitive predicates. Authority can activate on
//! **cryptographic / temporal conditions** (a verifiable event, a time window, a deadman/liveness
//! switch) AND / OR on **subjective human consensus** (e.g. *two registered physicians attest
//! incapacity*), composed freely with `All` / `Any` / `Not`.
//!
//! This is deliberately **both, composed** (ADR §7.1): purely-cryptographic predicates cannot capture
//! clinical judgement, and purely-subjective attestation is not accountable. `HumanConsensus` requires
//! *signed* attestations carrying a role/capacity credential, so subjective judgement becomes part of the
//! evidence chain — accountable and contestable — rather than a black box.
//!
//! Load-bearing philosophy (this crate is **supported agency**, not a warden model): a `Trigger` only
//! ever governs the activation of a delegation over *personhood* (socio-legal agency). It amplifies a
//! person's agency (an accountant, a clinical psychologist, a work-peer, or — when someone is isolated —
//! a declared software source of truth). *Selfhood* is inherent to the person and is never delegated, so
//! nothing here can be used to trigger control over it. Higher-stakes / selfhood-adjacent domains simply
//! demand higher `m`-of-`n` thresholds.
//!
//! `evaluate` is a **bounded, deterministic** recursion over the tree: given a `TriggerContext` (the
//! current time, which events have occurred, the collected attestations, and the last liveness ping) it
//! returns whether the trigger has fired. It never allocates unboundedly and has no side effects.

use serde::{Deserialize, Serialize};

/// A composable activation condition for a delegated authority (ADR §7.1).
///
/// Serialized with serde's default externally-tagged representation (a self-describing tree),
/// e.g. `{"all":[{"verifiable_event":{...}},{"human_consensus":{...}}]}`. Internal tagging
/// (`#[serde(tag=...)]`) is deliberately NOT used: this enum is recursive with sequence-wrapping
/// newtype variants (`All(Vec<Trigger>)`, `Not(Box<Trigger>)`), which internal tagging cannot
/// represent and which drives serde's serializer into an unbounded trait-resolution overflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    /// A cryptographically verifiable event/attestation identified by a stable id has occurred
    /// (e.g. `er_admission`, `death_certificate`, an on-chain/ILP receipt).
    VerifiableEvent { event_id: String },

    /// The current time falls in `[from_unix, to_unix]`. An open upper bound (`to_unix: None`) means
    /// "from `from_unix` onward, indefinitely".
    TemporalWindow {
        from_unix: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to_unix: Option<u32>,
    },

    /// Liveness / deadman switch: fires once at least `timeout_secs` have elapsed since the principal
    /// was last seen (`last_seen_unix`). Used for posthumous / incapacity fallbacks.
    DeadmanSwitch { last_seen_unix: u32, timeout_secs: u32 },

    /// `m`-of-`n` **signed** human attestations meeting an optional required capacity/role credential
    /// (e.g. 2-of-N `registered_physician` attesting incapacity). `n` is advisory metadata describing
    /// the expected pool size; only `m` (the threshold) and the required capacity gate activation.
    HumanConsensus {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        required_capacity: Option<String>,
        m: u32,
        n: u32,
    },

    /// Conjunction — fires only if **every** child fires. An empty `All` is vacuously true.
    All(Vec<Trigger>),

    /// Disjunction — fires if **any** child fires. An empty `Any` is vacuously false.
    Any(Vec<Trigger>),

    /// Negation — fires iff the inner trigger does not.
    Not(Box<Trigger>),
}

/// A single human attestation: the role/capacity it was made under (a taxonomy `TermId`, kept as an
/// optional `String` here so the vocabulary stays open) and whether it is cryptographically signed.
///
/// Only *signed* attestations count toward a `HumanConsensus` threshold — an unsigned attestation is not
/// part of the accountable evidence chain, so it cannot activate authority over someone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attestation {
    /// The role/capacity credential the attestation was made under (e.g.
    /// `urn:qualia:agency-domain:welfare:healthcare` or a `registered_physician` capacity term).
    /// `None` = an attestation carrying no specific declared capacity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<String>,
    /// Whether this attestation is cryptographically signed. Unsigned attestations never count.
    pub signed: bool,
}

impl Attestation {
    /// A signed attestation made under a specific capacity/role credential.
    pub fn signed(capacity: impl Into<String>) -> Self {
        Self { capacity: Some(capacity.into()), signed: true }
    }

    /// A signed attestation with no specific declared capacity.
    pub fn signed_anon() -> Self {
        Self { capacity: None, signed: true }
    }

    /// An unsigned attestation (does not count toward consensus).
    pub fn unsigned(capacity: impl Into<String>) -> Self {
        Self { capacity: Some(capacity.into()), signed: false }
    }
}

/// The evaluation context — the observable world state a `Trigger` is tested against. All fields are
/// data the delegation infrastructure already collects (event log, liveness pings, the attestation
/// queue); `evaluate` reads them without mutation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerContext {
    /// Current wall-clock time (unix seconds).
    pub now_unix: u32,
    /// Ids of verifiable events known to have occurred.
    #[serde(default)]
    pub occurred_events: Vec<String>,
    /// Collected human attestations (signed and unsigned).
    #[serde(default)]
    pub attestations: Vec<Attestation>,
    /// The principal's last liveness ping (unix seconds), if any — feeds `DeadmanSwitch`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_liveness_unix: Option<u32>,
}

impl TriggerContext {
    /// A context anchored at `now_unix` with no events, attestations, or liveness signal.
    pub fn at(now_unix: u32) -> Self {
        Self { now_unix, ..Default::default() }
    }

    pub fn with_event(mut self, event_id: impl Into<String>) -> Self {
        self.occurred_events.push(event_id.into());
        self
    }

    pub fn with_attestation(mut self, att: Attestation) -> Self {
        self.attestations.push(att);
        self
    }

    pub fn with_liveness(mut self, last_liveness_unix: u32) -> Self {
        self.last_liveness_unix = Some(last_liveness_unix);
        self
    }
}

/// Count the signed attestations in `ctx` that satisfy an optional required capacity.
///
/// A required capacity of `None` accepts any signed attestation; a required capacity of `Some(cap)`
/// accepts only signed attestations whose `capacity == Some(cap)`.
fn count_matching_attestations(required_capacity: Option<&str>, ctx: &TriggerContext) -> u32 {
    ctx.attestations
        .iter()
        .filter(|att| att.signed)
        .filter(|att| match required_capacity {
            None => true,
            Some(cap) => att.capacity.as_deref() == Some(cap),
        })
        .count()
        // Attestation counts are tiny; saturate rather than risk a (practically impossible) overflow.
        .min(u32::MAX as usize) as u32
}

/// Evaluate a `Trigger` against a `TriggerContext`. Deterministic, side-effect-free, and bounded by the
/// size of the trigger tree (recursion depth = tree depth).
///
/// Semantics (ADR §7.1):
/// - `VerifiableEvent` — the event id is present in `ctx.occurred_events`.
/// - `TemporalWindow` — `now_unix >= from_unix` and (no upper bound, or `now_unix <= to_unix`).
/// - `DeadmanSwitch` — a liveness signal exists and `now_unix - last_liveness >= timeout_secs`
///   (saturating; a missing liveness signal never fires).
/// - `HumanConsensus` — at least `m` signed attestations match the required capacity (`n` is advisory).
/// - `All` — every child fires. `Any` — some child fires. `Not` — the inner trigger does not.
pub fn evaluate(trigger: &Trigger, ctx: &TriggerContext) -> bool {
    match trigger {
        Trigger::VerifiableEvent { event_id } => {
            ctx.occurred_events.iter().any(|e| e == event_id)
        }
        Trigger::TemporalWindow { from_unix, to_unix } => {
            ctx.now_unix >= *from_unix && to_unix.map_or(true, |t| ctx.now_unix <= t)
        }
        Trigger::DeadmanSwitch { last_seen_unix, timeout_secs } => ctx
            .last_liveness_unix
            .map(|ls| ls.max(*last_seen_unix))
            .or(Some(*last_seen_unix))
            .map_or(false, |seen| {
                // Fires only once the elapsed time since last-seen reaches the timeout.
                ctx.now_unix.saturating_sub(seen) >= *timeout_secs
            }),
        Trigger::HumanConsensus { required_capacity, m, .. } => {
            count_matching_attestations(required_capacity.as_deref(), ctx) >= *m
        }
        Trigger::All(children) => children.iter().all(|c| evaluate(c, ctx)),
        Trigger::Any(children) => children.iter().any(|c| evaluate(c, ctx)),
        Trigger::Not(inner) => !evaluate(inner, ctx),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PHYSICIAN: &str = "urn:qualia:capacity:registered-physician";
    const NURSE: &str = "urn:qualia:capacity:registered-nurse";
    const ER_ADMISSION: &str = "er_admission";

    // ---- primitives: VerifiableEvent ----

    #[test]
    fn verifiable_event_true_when_present_false_when_absent() {
        let t = Trigger::VerifiableEvent { event_id: ER_ADMISSION.into() };
        let present = TriggerContext::at(1000).with_event(ER_ADMISSION);
        let absent = TriggerContext::at(1000).with_event("unrelated_event");
        assert!(evaluate(&t, &present));
        assert!(!evaluate(&t, &absent));
        // Empty context also fails.
        assert!(!evaluate(&t, &TriggerContext::at(1000)));
    }

    // ---- primitives: TemporalWindow ----

    #[test]
    fn temporal_window_bounded_range() {
        let t = Trigger::TemporalWindow { from_unix: 100, to_unix: Some(200) };
        assert!(!evaluate(&t, &TriggerContext::at(99))); // before
        assert!(evaluate(&t, &TriggerContext::at(100))); // inclusive lower
        assert!(evaluate(&t, &TriggerContext::at(150))); // inside
        assert!(evaluate(&t, &TriggerContext::at(200))); // inclusive upper
        assert!(!evaluate(&t, &TriggerContext::at(201))); // after
    }

    #[test]
    fn temporal_window_open_upper_bound_is_indefinite() {
        let t = Trigger::TemporalWindow { from_unix: 100, to_unix: None };
        assert!(!evaluate(&t, &TriggerContext::at(99)));
        assert!(evaluate(&t, &TriggerContext::at(100)));
        assert!(evaluate(&t, &TriggerContext::at(u32::MAX)));
    }

    // ---- primitives: DeadmanSwitch ----

    #[test]
    fn deadman_switch_fires_only_after_timeout() {
        let t = Trigger::DeadmanSwitch { last_seen_unix: 1000, timeout_secs: 3600 };
        // No liveness in context: falls back to last_seen_unix baked into the trigger.
        assert!(!evaluate(&t, &TriggerContext::at(1000))); // 0 elapsed
        assert!(!evaluate(&t, &TriggerContext::at(4599))); // 3599 elapsed, just short
        assert!(evaluate(&t, &TriggerContext::at(4600))); // exactly 3600 elapsed
        assert!(evaluate(&t, &TriggerContext::at(10_000))); // well past
    }

    #[test]
    fn deadman_switch_uses_latest_liveness_ping() {
        let t = Trigger::DeadmanSwitch { last_seen_unix: 1000, timeout_secs: 3600 };
        // A fresher liveness ping resets the clock: last seen at 5000, now 6000 → only 1000 elapsed.
        let fresh = TriggerContext::at(6000).with_liveness(5000);
        assert!(!evaluate(&t, &fresh));
        // Old liveness ping older than the baked-in last_seen still uses the newer of the two.
        let stale_ping = TriggerContext::at(4600).with_liveness(500);
        assert!(evaluate(&t, &stale_ping)); // max(1000,500)=1000, 3600 elapsed → fires
    }

    // ---- primitives: HumanConsensus ----

    #[test]
    fn human_consensus_counts_only_signed_matching_capacity() {
        let t = Trigger::HumanConsensus {
            required_capacity: Some(PHYSICIAN.into()),
            m: 2,
            n: 5,
        };
        // Two signed physicians → fires.
        let ok = TriggerContext::at(0)
            .with_attestation(Attestation::signed(PHYSICIAN))
            .with_attestation(Attestation::signed(PHYSICIAN));
        assert!(evaluate(&t, &ok));

        // One physician + one nurse → only one matches → below threshold.
        let wrong_capacity = TriggerContext::at(0)
            .with_attestation(Attestation::signed(PHYSICIAN))
            .with_attestation(Attestation::signed(NURSE));
        assert!(!evaluate(&t, &wrong_capacity));

        // Two physicians but one unsigned → only one counts → fails (accountability requires signature).
        let one_unsigned = TriggerContext::at(0)
            .with_attestation(Attestation::signed(PHYSICIAN))
            .with_attestation(Attestation::unsigned(PHYSICIAN));
        assert!(!evaluate(&t, &one_unsigned));
    }

    #[test]
    fn human_consensus_below_threshold_fails() {
        let t = Trigger::HumanConsensus { required_capacity: Some(PHYSICIAN.into()), m: 3, n: 5 };
        let two = TriggerContext::at(0)
            .with_attestation(Attestation::signed(PHYSICIAN))
            .with_attestation(Attestation::signed(PHYSICIAN));
        assert!(!evaluate(&t, &two)); // 2 < 3
    }

    #[test]
    fn human_consensus_without_required_capacity_accepts_any_signed() {
        let t = Trigger::HumanConsensus { required_capacity: None, m: 2, n: 2 };
        let mixed = TriggerContext::at(0)
            .with_attestation(Attestation::signed(PHYSICIAN))
            .with_attestation(Attestation::signed(NURSE));
        assert!(evaluate(&t, &mixed)); // any two signed count
        // But unsigned still never counts.
        let one_signed = TriggerContext::at(0)
            .with_attestation(Attestation::signed(PHYSICIAN))
            .with_attestation(Attestation::unsigned(NURSE));
        assert!(!evaluate(&t, &one_signed));
    }

    #[test]
    fn human_consensus_n_is_advisory_only() {
        // n smaller than the number present, or than m, must not affect activation — only m gates.
        let t = Trigger::HumanConsensus { required_capacity: None, m: 1, n: 0 };
        let ctx = TriggerContext::at(0).with_attestation(Attestation::signed_anon());
        assert!(evaluate(&t, &ctx));
    }

    // ---- composition: All / Any / Not ----

    #[test]
    fn empty_all_is_true_empty_any_is_false() {
        assert!(evaluate(&Trigger::All(vec![]), &TriggerContext::at(0)));
        assert!(!evaluate(&Trigger::Any(vec![]), &TriggerContext::at(0)));
    }

    #[test]
    fn not_inverts_child() {
        let ev = Trigger::VerifiableEvent { event_id: ER_ADMISSION.into() };
        let not_ev = Trigger::Not(Box::new(ev.clone()));
        let with = TriggerContext::at(0).with_event(ER_ADMISSION);
        let without = TriggerContext::at(0);
        assert!(evaluate(&ev, &with) && !evaluate(&not_ev, &with));
        assert!(!evaluate(&ev, &without) && evaluate(&not_ev, &without));
    }

    #[test]
    fn any_and_all_compose() {
        let a = Trigger::VerifiableEvent { event_id: "a".into() };
        let b = Trigger::VerifiableEvent { event_id: "b".into() };
        let all = Trigger::All(vec![a.clone(), b.clone()]);
        let any = Trigger::Any(vec![a.clone(), b.clone()]);

        let only_a = TriggerContext::at(0).with_event("a");
        let both = TriggerContext::at(0).with_event("a").with_event("b");

        assert!(!evaluate(&all, &only_a)); // All needs both
        assert!(evaluate(&all, &both));
        assert!(evaluate(&any, &only_a)); // Any needs one
        assert!(!evaluate(&any, &TriggerContext::at(0))); // neither
    }

    // ---- the ADR §7.1 crisis composite ----

    #[test]
    fn crisis_composite_verifiable_event_and_two_of_n_physicians() {
        // The canonical ADR example: fire on ER admission AND 2-of-N registered physicians attesting.
        let crisis = Trigger::All(vec![
            Trigger::VerifiableEvent { event_id: ER_ADMISSION.into() },
            Trigger::HumanConsensus { required_capacity: Some(PHYSICIAN.into()), m: 2, n: 5 },
        ]);

        // Full crisis: admitted + two physicians attest → activates.
        let full = TriggerContext::at(1_700_000_000)
            .with_event(ER_ADMISSION)
            .with_attestation(Attestation::signed(PHYSICIAN))
            .with_attestation(Attestation::signed(PHYSICIAN));
        assert!(evaluate(&crisis, &full));

        // Admitted but only one physician has attested → consensus below threshold → does NOT activate.
        let one_doctor = TriggerContext::at(1_700_000_000)
            .with_event(ER_ADMISSION)
            .with_attestation(Attestation::signed(PHYSICIAN));
        assert!(!evaluate(&crisis, &one_doctor));

        // Two physicians attest but no verifiable admission event → does NOT activate.
        let no_event = TriggerContext::at(1_700_000_000)
            .with_attestation(Attestation::signed(PHYSICIAN))
            .with_attestation(Attestation::signed(PHYSICIAN));
        assert!(!evaluate(&crisis, &no_event));
    }

    #[test]
    fn deeply_nested_composite_evaluates() {
        // (window AND (event OR deadman)) AND NOT(revocation event)
        let trigger = Trigger::All(vec![
            Trigger::TemporalWindow { from_unix: 100, to_unix: Some(1000) },
            Trigger::Any(vec![
                Trigger::VerifiableEvent { event_id: "consent".into() },
                Trigger::DeadmanSwitch { last_seen_unix: 100, timeout_secs: 500 },
            ]),
            Trigger::Not(Box::new(Trigger::VerifiableEvent { event_id: "revoked".into() })),
        ]);

        // now=200: in window; consent present; not revoked → fires.
        let ok = TriggerContext::at(200).with_event("consent");
        assert!(evaluate(&trigger, &ok));

        // Revocation present → NOT clause fails the whole All.
        let revoked = TriggerContext::at(200).with_event("consent").with_event("revoked");
        assert!(!evaluate(&trigger, &revoked));

        // now=700: consent absent, but deadman fired (700-100=600 >= 500) → Any still true → fires.
        let deadman = TriggerContext::at(700);
        assert!(evaluate(&trigger, &deadman));

        // now=1001: outside temporal window → whole All fails regardless of the rest.
        let out_of_window = TriggerContext::at(1001).with_event("consent");
        assert!(!evaluate(&trigger, &out_of_window));
    }

    // ---- serde round-trip ----

    #[test]
    fn trigger_serde_round_trips() {
        let trigger = Trigger::All(vec![
            Trigger::VerifiableEvent { event_id: ER_ADMISSION.into() },
            Trigger::HumanConsensus { required_capacity: Some(PHYSICIAN.into()), m: 2, n: 5 },
            Trigger::Not(Box::new(Trigger::TemporalWindow { from_unix: 0, to_unix: None })),
        ]);
        let json = serde_json::to_string(&trigger).expect("serialize");
        // Externally tagged: variant name is the key, e.g. {"all":[{"human_consensus":{...}}]}.
        assert!(json.contains("\"all\""));
        assert!(json.contains("\"human_consensus\""));
        let back: Trigger = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(trigger, back);
    }

    #[test]
    fn context_and_attestation_serde_round_trip() {
        let ctx = TriggerContext::at(42)
            .with_event(ER_ADMISSION)
            .with_attestation(Attestation::signed(PHYSICIAN))
            .with_attestation(Attestation::unsigned(NURSE))
            .with_liveness(40);
        let json = serde_json::to_string(&ctx).expect("serialize");
        let back: TriggerContext = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ctx, back);
    }

    #[test]
    fn attestation_constructors_are_correct() {
        assert_eq!(
            Attestation::signed(PHYSICIAN),
            Attestation { capacity: Some(PHYSICIAN.to_string()), signed: true }
        );
        assert_eq!(
            Attestation::signed_anon(),
            Attestation { capacity: None, signed: true }
        );
        assert_eq!(
            Attestation::unsigned(NURSE),
            Attestation { capacity: Some(NURSE.to_string()), signed: false }
        );
    }
}
