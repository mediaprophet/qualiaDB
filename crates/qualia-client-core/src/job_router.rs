//! Job routing / placement policy — decide **where** a curated job is processed.
//!
//! Given a curated job, this module answers one narrow question: should the work run on
//! the person's **local, in-process inference engine**, or be sent to an **external
//! provider reached over MCP**? It is *transport / placement policy only*. It performs no
//! I/O, no async work, and no cryptography — it is a pure decision function over a small
//! set of inputs so it is trivially testable and auditable.
//!
//! # Privacy-first, fail-closed ordering
//!
//! The directive being implemented (Timothy): *local inference is PREFERRED when the person
//! can run it; costly external MCP services are used only when wanted/needed and consented;
//! sanctuary/private data must never leave the device.*
//!
//! The rules are therefore evaluated in a deliberate order so that the most protective
//! outcome always wins:
//!
//! 1. **Classified / sanctuary data → local only.** Such data must never leave the device,
//!    regardless of consent or policy. If no local engine is available the job is
//!    [`RoutingDecision::Blocked`] rather than sent out.
//! 2. **Policy forbids external → stay local.** If the person's [`RoutingPolicy`] disables
//!    external providers, the job runs locally (or is blocked if no local engine exists).
//! 3. **Capability gap → external needed.** If the job needs a capability the local engine
//!    lacks, an external provider is required — but only *with explicit consent*. Otherwise
//!    the caller is told consent is needed ([`RoutingDecision::NeedsConsent`]).
//! 4. **No local engine → external fallback.** For non-classified data, if no local engine
//!    is available the job goes external *with explicit consent*, else consent is requested.
//! 5. **Cost ceiling.** A remote path that would exceed the policy's cost ceiling requires
//!    consent for the spend. Cost is only meaningful on a remote path — a [`RoutingDecision::Local`]
//!    decision has no metered cost, so the ceiling is never applied to it.
//! 6. **Default → local.** With a local engine available and nothing forcing a remote hop,
//!    the job runs locally.
//!
//! # Relationship to the authority check
//!
//! This is **placement policy, not an authority check.** Whether the requesting agent is
//! *permitted* to run the job at all is decided separately and complementarily by
//! [`qualia_cooperative_core::agency_delegation::delegation_permits`] (a fail-closed ABAC
//! evaluator). The caller runs both: `delegation_permits` answers *"is this allowed?"*, and
//! [`route_job`] answers *"where should it run?"*. This module deliberately does **not**
//! reimplement or second-guess that authority decision.

use serde::{Deserialize, Serialize};
use wellfare_core::record::SensitivityClass;

/// A modest default per-job cost ceiling: **10 US cents** expressed in microcents
/// (1 cent = 1_000_000 microcents, so 10 cents = `10_000_000`).
///
/// Above this, a remote job asks the person to confirm the spend rather than silently
/// incurring the cost. It is only ever compared against a *remote* path — local inference
/// has no metered cost.
pub const DEFAULT_COST_CEILING_MICROCENTS: u64 = 10_000_000;

/// The inputs to a routing decision for a single curated job.
///
/// All fields are supplied by the caller from the job's curation metadata and the current
/// device/consent state; this module treats them as ground truth and does not fetch them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingInputs {
    /// Sensitivity of the data the job will touch. [`SensitivityClass::Classified`] is
    /// sanctuary-grade and is *never* routed off-device.
    pub sensitivity: SensitivityClass,
    /// Whether a local, in-process inference engine is available to run the job.
    pub local_available: bool,
    /// Whether the person has explicitly consented to using an external MCP provider for
    /// this job. Consent is a precondition for every remote path (never assumed).
    pub external_consented: bool,
    /// A capability the job requires (e.g. a specific model or tool), if any. When present
    /// and not satisfied locally, the job needs an external provider.
    pub requires_capability: Option<String>,
    /// Whether the local engine can satisfy [`RoutingInputs::requires_capability`]. Ignored
    /// when no capability is required.
    pub local_has_capability: bool,
    /// The estimated cost of the *remote* execution, in microcents (1 cent = 1_000_000
    /// microcents). Only compared against the ceiling when a remote path is chosen.
    pub estimated_cost_microcents: u64,
}

/// The person's placement policy — the guard rails [`route_job`] evaluates against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingPolicy {
    /// Whether external MCP providers may be used at all. When `false`, every job stays
    /// local (or is blocked if no local engine exists), regardless of consent.
    pub allow_external: bool,
    /// The maximum estimated remote cost (in microcents) that may be incurred without an
    /// extra consent step. A remote path above this becomes [`RoutingDecision::NeedsConsent`].
    pub cost_ceiling_microcents: u64,
}

impl Default for RoutingPolicy {
    /// A sensible default: external providers are permitted, with a modest
    /// [`DEFAULT_COST_CEILING_MICROCENTS`] (10 cents) per-job ceiling above which the person
    /// is asked to confirm the spend.
    fn default() -> Self {
        Self {
            allow_external: true,
            cost_ceiling_microcents: DEFAULT_COST_CEILING_MICROCENTS,
        }
    }
}

/// The placement decision for a curated job.
///
/// Every non-`Local` variant carries a clear, human-readable `reason` so the caller can
/// surface *why* a job was blocked, deferred for consent, or sent out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingDecision {
    /// Run on the person's local, in-process inference engine. No outbound traffic, no cost.
    Local,
    /// Route to an external provider over MCP. Only reached when external use is permitted
    /// by policy, consented by the person, and within the cost ceiling.
    RemoteMcp { reason: String },
    /// A remote path is needed or wanted, but the person's explicit consent (to use an
    /// external provider, or to authorise a spend above the ceiling) has not yet been given.
    NeedsConsent { reason: String },
    /// The job cannot be routed anywhere: classified data with no local engine, or external
    /// providers disabled by policy with no local engine.
    Blocked { reason: String },
}

/// Decide where a curated job should run.
///
/// A pure function of ([`RoutingInputs`], [`RoutingPolicy`]) implementing the privacy-first,
/// fail-closed ordering documented at the module level. See the module docs for the full
/// rule list and its rationale.
pub fn route_job(inputs: &RoutingInputs, policy: &RoutingPolicy) -> RoutingDecision {
    // Rule 1 — Classified / sanctuary data: LOCAL ONLY. Never routed off-device, regardless
    // of consent or policy. This is the strongest, first-evaluated protection.
    if inputs.sensitivity == SensitivityClass::Classified {
        return if inputs.local_available {
            RoutingDecision::Local
        } else {
            RoutingDecision::Blocked {
                reason: "classified/sanctuary data cannot leave the device and no local engine is available"
                    .to_string(),
            }
        };
    }

    // Rule 2 — Policy forbids external providers: the job must stay local (or block). This
    // outranks capability and availability: an explicit "no external" is honoured absolutely.
    if !policy.allow_external {
        return if inputs.local_available {
            RoutingDecision::Local
        } else {
            RoutingDecision::Blocked {
                reason:
                    "external providers are disabled by policy and no local engine is available"
                        .to_string(),
            }
        };
    }

    // From here on: sensitivity is Public or Restricted, and policy allows external use.
    // Restricted data still reaches an external provider ONLY via an explicit-consent branch
    // below (rules 3 and 4); it can never go out unconsented.

    // Rule 3 — Capability gap: the job needs a capability the local engine cannot provide,
    // so an external provider is required. Consent gates the hop.
    if let Some(cap) = inputs.requires_capability.as_deref() {
        if !inputs.local_has_capability {
            if inputs.external_consented {
                return remote_unless_over_ceiling(
                    inputs,
                    policy,
                    format!(
                        "local engine lacks the required capability '{cap}'; routing to an external MCP provider"
                    ),
                );
            }
            return RoutingDecision::NeedsConsent {
                reason: format!(
                    "external capability '{cap}' required — needs consent to use an external provider"
                ),
            };
        }
    }

    // Rule 4 — No local engine available (Public/Restricted): fall back to an external
    // provider, again gated by explicit consent.
    if !inputs.local_available {
        if inputs.external_consented {
            return remote_unless_over_ceiling(
                inputs,
                policy,
                "no local inference engine is available; routing to an external MCP provider"
                    .to_string(),
            );
        }
        return RoutingDecision::NeedsConsent {
            reason: "no local engine available — needs consent to use external provider"
                .to_string(),
        };
    }

    // Rule 6 — Default: a local engine is available and nothing forces a remote hop, so run
    // locally. (Rule 5, the cost ceiling, is applied inside `remote_unless_over_ceiling` on
    // the remote paths above; a local decision has no metered cost.)
    RoutingDecision::Local
}

/// Rule 5 helper: on an otherwise-chosen remote path, downgrade to [`RoutingDecision::NeedsConsent`]
/// when the estimated cost exceeds the policy ceiling; otherwise commit to [`RoutingDecision::RemoteMcp`].
///
/// Kept private: the cost ceiling is only ever meaningful once a remote path has been
/// selected, so it is never applied to a local decision.
fn remote_unless_over_ceiling(
    inputs: &RoutingInputs,
    policy: &RoutingPolicy,
    remote_reason: String,
) -> RoutingDecision {
    if inputs.estimated_cost_microcents > policy.cost_ceiling_microcents {
        RoutingDecision::NeedsConsent {
            reason: format!(
                "estimated cost {} microcents exceeds the policy ceiling of {} microcents — needs consent to authorise the spend",
                inputs.estimated_cost_microcents, policy.cost_ceiling_microcents
            ),
        }
    } else {
        RoutingDecision::RemoteMcp {
            reason: remote_reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Coarse discriminant of a decision, used so the case table can assert the outcome
    /// *shape* without pinning every exact reason string.
    #[derive(Debug, PartialEq, Eq)]
    enum Kind {
        Local,
        Remote,
        NeedsConsent,
        Blocked,
    }

    fn kind_of(d: &RoutingDecision) -> Kind {
        match d {
            RoutingDecision::Local => Kind::Local,
            RoutingDecision::RemoteMcp { .. } => Kind::Remote,
            RoutingDecision::NeedsConsent { .. } => Kind::NeedsConsent,
            RoutingDecision::Blocked { .. } => Kind::Blocked,
        }
    }

    fn reason_of(d: &RoutingDecision) -> Option<&str> {
        match d {
            RoutingDecision::Local => None,
            RoutingDecision::RemoteMcp { reason }
            | RoutingDecision::NeedsConsent { reason }
            | RoutingDecision::Blocked { reason } => Some(reason.as_str()),
        }
    }

    /// A permissive baseline job: Public data, local engine present, nothing else set.
    /// Cases override only the fields they care about via `..base()`.
    fn base() -> RoutingInputs {
        RoutingInputs {
            sensitivity: SensitivityClass::Public,
            local_available: true,
            external_consented: false,
            requires_capability: None,
            local_has_capability: false,
            estimated_cost_microcents: 0,
        }
    }

    fn no_external_policy() -> RoutingPolicy {
        RoutingPolicy {
            allow_external: false,
            ..RoutingPolicy::default()
        }
    }

    struct Case {
        name: &'static str,
        inputs: RoutingInputs,
        policy: RoutingPolicy,
        expect: Kind,
        /// If set, the decision's reason must contain this substring.
        reason_contains: Option<&'static str>,
    }

    #[test]
    fn route_job_table() {
        let over_ceiling = DEFAULT_COST_CEILING_MICROCENTS + 1;

        let cases = vec![
            // --- Rule 1: Classified is local-only, and never external. ---
            Case {
                name: "classified_with_local_runs_local",
                inputs: RoutingInputs {
                    sensitivity: SensitivityClass::Classified,
                    ..base()
                },
                policy: RoutingPolicy::default(),
                expect: Kind::Local,
                reason_contains: None,
            },
            Case {
                name: "classified_without_local_is_blocked",
                inputs: RoutingInputs {
                    sensitivity: SensitivityClass::Classified,
                    local_available: false,
                    ..base()
                },
                policy: RoutingPolicy::default(),
                expect: Kind::Blocked,
                reason_contains: Some("classified"),
            },
            Case {
                // Consent must NOT let classified data leave the device.
                name: "classified_never_external_even_with_consent",
                inputs: RoutingInputs {
                    sensitivity: SensitivityClass::Classified,
                    local_available: false,
                    external_consented: true,
                    requires_capability: Some("vision".into()),
                    estimated_cost_microcents: 5,
                    ..base()
                },
                policy: RoutingPolicy::default(),
                expect: Kind::Blocked,
                reason_contains: Some("cannot leave the device"),
            },
            Case {
                // Classified + local available: stays local; cost is irrelevant on local.
                name: "classified_local_ignores_cost",
                inputs: RoutingInputs {
                    sensitivity: SensitivityClass::Classified,
                    estimated_cost_microcents: over_ceiling,
                    external_consented: true,
                    ..base()
                },
                policy: RoutingPolicy::default(),
                expect: Kind::Local,
                reason_contains: None,
            },
            // --- Rule 2: policy disables external. ---
            Case {
                name: "no_external_policy_runs_local",
                inputs: base(),
                policy: no_external_policy(),
                expect: Kind::Local,
                reason_contains: None,
            },
            Case {
                // No external allowed AND no local engine → blocked, even with consent.
                name: "no_external_policy_without_local_is_blocked",
                inputs: RoutingInputs {
                    local_available: false,
                    external_consented: true,
                    ..base()
                },
                policy: no_external_policy(),
                expect: Kind::Blocked,
                reason_contains: Some("policy"),
            },
            Case {
                // Policy "no external" outranks a capability gap (rule 2 before rule 3).
                name: "no_external_policy_overrides_capability_gap",
                inputs: RoutingInputs {
                    requires_capability: Some("ocr".into()),
                    local_has_capability: false,
                    external_consented: true,
                    ..base()
                },
                policy: no_external_policy(),
                expect: Kind::Local,
                reason_contains: None,
            },
            // --- Rule 3: capability gap. ---
            Case {
                name: "capability_gap_with_consent_goes_remote",
                inputs: RoutingInputs {
                    requires_capability: Some("vision".into()),
                    local_has_capability: false,
                    external_consented: true,
                    ..base()
                },
                policy: RoutingPolicy::default(),
                expect: Kind::Remote,
                reason_contains: Some("capability"),
            },
            Case {
                name: "capability_gap_without_consent_needs_consent",
                inputs: RoutingInputs {
                    requires_capability: Some("vision".into()),
                    local_has_capability: false,
                    external_consented: false,
                    ..base()
                },
                policy: RoutingPolicy::default(),
                expect: Kind::NeedsConsent,
                reason_contains: Some("capability"),
            },
            Case {
                // Restricted data + capability gap: only reaches external WITH consent.
                name: "restricted_capability_gap_goes_external_only_with_consent",
                inputs: RoutingInputs {
                    sensitivity: SensitivityClass::Restricted,
                    requires_capability: Some("vision".into()),
                    local_has_capability: false,
                    external_consented: true,
                    ..base()
                },
                policy: RoutingPolicy::default(),
                expect: Kind::Remote,
                reason_contains: Some("capability"),
            },
            Case {
                name: "restricted_capability_gap_without_consent_needs_consent",
                inputs: RoutingInputs {
                    sensitivity: SensitivityClass::Restricted,
                    requires_capability: Some("vision".into()),
                    local_has_capability: false,
                    external_consented: false,
                    ..base()
                },
                policy: RoutingPolicy::default(),
                expect: Kind::NeedsConsent,
                reason_contains: Some("consent"),
            },
            Case {
                // Capability required but locally satisfied → no gap → local.
                name: "capability_satisfied_locally_runs_local",
                inputs: RoutingInputs {
                    requires_capability: Some("summarise".into()),
                    local_has_capability: true,
                    ..base()
                },
                policy: RoutingPolicy::default(),
                expect: Kind::Local,
                reason_contains: None,
            },
            // --- Rule 4: no local engine (non-classified). ---
            Case {
                name: "no_local_with_consent_goes_remote",
                inputs: RoutingInputs {
                    local_available: false,
                    external_consented: true,
                    ..base()
                },
                policy: RoutingPolicy::default(),
                expect: Kind::Remote,
                reason_contains: Some("MCP"),
            },
            Case {
                name: "no_local_without_consent_needs_consent",
                inputs: RoutingInputs {
                    local_available: false,
                    external_consented: false,
                    ..base()
                },
                policy: RoutingPolicy::default(),
                expect: Kind::NeedsConsent,
                reason_contains: Some("no local engine available"),
            },
            // --- Rule 5: cost ceiling on remote paths. ---
            Case {
                name: "no_local_over_cost_needs_consent",
                inputs: RoutingInputs {
                    local_available: false,
                    external_consented: true,
                    estimated_cost_microcents: over_ceiling,
                    ..base()
                },
                policy: RoutingPolicy::default(),
                expect: Kind::NeedsConsent,
                reason_contains: Some("ceiling"),
            },
            Case {
                name: "capability_gap_over_cost_needs_consent",
                inputs: RoutingInputs {
                    requires_capability: Some("vision".into()),
                    local_has_capability: false,
                    external_consented: true,
                    estimated_cost_microcents: over_ceiling,
                    ..base()
                },
                policy: RoutingPolicy::default(),
                expect: Kind::NeedsConsent,
                reason_contains: Some("cost"),
            },
            Case {
                // Cost equal to the ceiling is allowed (strictly-greater triggers consent).
                name: "cost_equal_to_ceiling_is_allowed_remote",
                inputs: RoutingInputs {
                    local_available: false,
                    external_consented: true,
                    estimated_cost_microcents: DEFAULT_COST_CEILING_MICROCENTS,
                    ..base()
                },
                policy: RoutingPolicy::default(),
                expect: Kind::Remote,
                reason_contains: None,
            },
            Case {
                // Cost never blocks a LOCAL decision: huge cost + local available → local.
                name: "local_path_ignores_cost_ceiling",
                inputs: RoutingInputs {
                    sensitivity: SensitivityClass::Restricted,
                    estimated_cost_microcents: over_ceiling,
                    ..base()
                },
                policy: RoutingPolicy::default(),
                expect: Kind::Local,
                reason_contains: None,
            },
            // --- Rule 6: default. ---
            Case {
                name: "default_public_local_runs_local",
                inputs: base(),
                policy: RoutingPolicy::default(),
                expect: Kind::Local,
                reason_contains: None,
            },
            Case {
                // Restricted data prefers local and does not go external unbidden.
                name: "restricted_prefers_local",
                inputs: RoutingInputs {
                    sensitivity: SensitivityClass::Restricted,
                    ..base()
                },
                policy: RoutingPolicy::default(),
                expect: Kind::Local,
                reason_contains: None,
            },
        ];

        for c in &cases {
            let decision = route_job(&c.inputs, &c.policy);
            assert_eq!(
                kind_of(&decision),
                c.expect,
                "case '{}' expected {:?} but got {:?}",
                c.name,
                c.expect,
                decision
            );
            if let Some(needle) = c.reason_contains {
                let reason = reason_of(&decision).unwrap_or("");
                assert!(
                    reason.contains(needle),
                    "case '{}': reason {:?} did not contain {:?}",
                    c.name,
                    reason,
                    needle
                );
            }
        }
    }

    #[test]
    fn default_policy_is_sensible() {
        let p = RoutingPolicy::default();
        assert!(
            p.allow_external,
            "default policy should permit external use"
        );
        assert_eq!(p.cost_ceiling_microcents, DEFAULT_COST_CEILING_MICROCENTS);
    }

    #[test]
    fn public_types_round_trip_through_serde() {
        let inputs = RoutingInputs {
            sensitivity: SensitivityClass::Restricted,
            local_available: false,
            external_consented: true,
            requires_capability: Some("vision".into()),
            local_has_capability: false,
            estimated_cost_microcents: 42,
        };
        let policy = RoutingPolicy::default();

        let inputs_json = serde_json::to_string(&inputs).expect("serialize inputs");
        let inputs_back: RoutingInputs =
            serde_json::from_str(&inputs_json).expect("deserialize inputs");
        assert_eq!(inputs, inputs_back);

        let policy_json = serde_json::to_string(&policy).expect("serialize policy");
        let policy_back: RoutingPolicy =
            serde_json::from_str(&policy_json).expect("deserialize policy");
        assert_eq!(policy, policy_back);

        let decision = route_job(&inputs, &policy);
        let decision_json = serde_json::to_string(&decision).expect("serialize decision");
        let decision_back: RoutingDecision =
            serde_json::from_str(&decision_json).expect("deserialize decision");
        assert_eq!(decision, decision_back);
    }
}
