//! **Duty of inquiry** — expectations that *define* negligence.
//!
//! The consideration (Timothy, 2026-07-06): staff in mental-health (and other welfare) facilities are rarely
//! experts, and often *cannot* understand complex, specialised, international, or secrecy-bound work. A real
//! specialist's real work can look like grandiosity to someone who cannot verify it. Fairness therefore
//! **cannot require them to understand** — but it *can* require them to **check the means when means are
//! available**. This sets an **expectation**, and the expectation is what **defines negligence**:
//!
//! > *Failure to check, even given the means to do so, then acts that cause further injury* = **negligence**.
//!
//! And it keeps negligence **fair** by distinguishing it from the neighbours:
//! - **No fault** — the means were genuinely *not accessible*; the actor could not reasonably have known.
//! - **Negligent** — accessible means were *not checked*, and a harmful act followed.
//! - (**Malfeasance** — checked / knew and harmed anyway, or wilfully avoided checking to keep deniability —
//!   is *beyond* this inquiry primitive; it is the intent case in the accountability spectrum.)
//!
//! The "means to check" are exactly what the rest of the fabric provides: verifiable credentials, the durable
//! disclosure/conduct records, and a person's [`TransparencyInvocation`](crate::incapacity_switch::TransparencyInvocation)
//! (offering their prior-events record to be checked). This module is the pure classifier; it composes with
//! the social-worker accountability spectrum (`docs/plans/social-worker-support-and-accountability.md` §3).

use serde::{Deserialize, Serialize};

/// A means by which an actor could verify a relevant fact before acting — a verifiable credential, a durable
/// record, a transparency invocation the person offered. Carries whether it was **reasonably accessible** to
/// the actor at the relevant time (given to them, or checkable with the means they had). If it was **not**
/// accessible, not-checking it is *not* negligence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeansToCheck {
    pub id: String,
    /// What it would verify, in plain terms (e.g. "a credential attesting the person's specialist role").
    pub description: String,
    /// Was this reasonably accessible / checkable by the actor at the relevant time?
    pub accessible: bool,
}

/// The **duty**: a consequential act is expected to be preceded by checking the relevant means. Sets the
/// expectation against which negligence is measured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DutyOfInquiry {
    /// The consequential act (e.g. "diagnose", "medicate", "restrain", "record as unreliable").
    pub act: String,
    /// The means the actor was expected to check before that act.
    pub expected_means: Vec<MeansToCheck>,
}

/// What actually happened, against the duty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConductAgainstDuty {
    pub actor_did: String,
    /// Ids of the means the actor actually checked.
    pub checked_means_ids: Vec<String>,
    /// Did the actor take the consequential act?
    pub acted: bool,
    /// Did the act cause (further) injury to the person?
    pub caused_further_injury: bool,
}

/// The fair classification of a shortfall — the locus, tying the accountability spectrum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InquiryVerdict {
    /// Every **accessible** means was checked before acting — diligent as to inquiry.
    Diligent,
    /// The means were **not accessible** — the actor could not reasonably have known; no fault.
    NoFault,
    /// Accessible means were **not checked**, but **no harmful act** followed — a procedural shortfall, not
    /// (yet) actionable negligence. Recorded honestly rather than inflated to negligence.
    UncheckedNoHarm,
    /// Accessible means were **not checked** and a **harmful act followed** — **negligence** ("failure to
    /// check, given the means, then acts that cause further injury").
    Negligent,
}

/// Classify conduct against a duty of inquiry. Deterministic; the criteria are the definition above.
pub fn assess(duty: &DutyOfInquiry, conduct: &ConductAgainstDuty) -> InquiryVerdict {
    let any_accessible = duty.expected_means.iter().any(|m| m.accessible);
    if !any_accessible {
        // Nothing the actor could reasonably have checked → they could not have known.
        return InquiryVerdict::NoFault;
    }
    let unchecked_accessible = duty
        .expected_means
        .iter()
        .any(|m| m.accessible && !conduct.checked_means_ids.iter().any(|id| id == &m.id));
    if !unchecked_accessible {
        // Everything accessible was checked.
        return InquiryVerdict::Diligent;
    }
    if conduct.acted && conduct.caused_further_injury {
        InquiryVerdict::Negligent
    } else {
        InquiryVerdict::UncheckedNoHarm
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn means(id: &str, accessible: bool) -> MeansToCheck {
        MeansToCheck {
            id: id.into(),
            description: format!("means {id}"),
            accessible,
        }
    }

    fn duty(means: Vec<MeansToCheck>) -> DutyOfInquiry {
        DutyOfInquiry {
            act: "record the person as unreliable / medicate".into(),
            expected_means: means,
        }
    }

    fn conduct(checked: &[&str], acted: bool, injury: bool) -> ConductAgainstDuty {
        ConductAgainstDuty {
            actor_did: "did:wf:facility-staff".into(),
            checked_means_ids: checked.iter().map(|s| s.to_string()).collect(),
            acted,
            caused_further_injury: injury,
        }
    }

    #[test]
    fn no_fault_when_the_means_were_not_accessible() {
        // The person's specialist credential existed but was NOT accessible to the staff (secrecy / no route
        // to verify) — they could not reasonably have known. Not negligence.
        let d = duty(vec![means("cred:specialist", false)]);
        assert_eq!(
            assess(&d, &conduct(&[], true, true)),
            InquiryVerdict::NoFault
        );
    }

    #[test]
    fn diligent_when_the_accessible_means_were_checked() {
        let d = duty(vec![
            means("cred:specialist", true),
            means("record:timeline", true),
        ]);
        // Both accessible means checked before acting.
        assert_eq!(
            assess(
                &d,
                &conduct(&["cred:specialist", "record:timeline"], true, false)
            ),
            InquiryVerdict::Diligent
        );
    }

    #[test]
    fn negligent_when_accessible_means_unchecked_and_a_harmful_act_follows() {
        // The person offered a transparency invocation + a verifiable credential (accessible); staff did NOT
        // check, then acted in a way that caused further injury. This is the definition of negligence.
        let d = duty(vec![
            means("transparency:timeline", true),
            means("cred:specialist", true),
        ]);
        assert_eq!(
            assess(&d, &conduct(&[], true, true)),
            InquiryVerdict::Negligent
        );
        // Even checking ONE of two accessible means, if the unchecked one was material and harm followed:
        assert_eq!(
            assess(&d, &conduct(&["cred:specialist"], true, true)),
            InquiryVerdict::Negligent
        );
    }

    #[test]
    fn unchecked_but_no_harm_is_a_shortfall_not_inflated_to_negligence() {
        // Accessible means unchecked, but no harmful act followed — honestly a gap, not (yet) negligence.
        let d = duty(vec![means("cred:specialist", true)]);
        assert_eq!(
            assess(&d, &conduct(&[], false, false)),
            InquiryVerdict::UncheckedNoHarm
        );
        // Acted, but no further injury → still not negligence.
        assert_eq!(
            assess(&d, &conduct(&[], true, false)),
            InquiryVerdict::UncheckedNoHarm
        );
    }

    #[test]
    fn serde_round_trips() {
        let d = duty(vec![means("m", true)]);
        let back: DutyOfInquiry =
            serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!(d, back);
        assert_eq!(
            serde_json::from_str::<InquiryVerdict>(
                &serde_json::to_string(&InquiryVerdict::Negligent).unwrap()
            )
            .unwrap(),
            InquiryVerdict::Negligent
        );
    }
}
