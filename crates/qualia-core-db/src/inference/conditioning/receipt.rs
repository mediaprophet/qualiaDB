//! Requirement outcome disposition and execution receipt.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequirementDisposition {
    Applied,
    Enforced,
    Degraded,
    Rejected,
    NotApplicable,
}

#[derive(Debug, Clone, Copy)]
pub struct RequirementOutcome<'a> {
    pub id: &'a str,
    pub disposition: RequirementDisposition,
    pub reason: Option<&'a str>,
}

impl<'a> RequirementOutcome<'a> {
    /// `RequirementOutcome::applied`.
    pub const fn applied(id: &'a str) -> Self {
        Self {
            id,
            disposition: RequirementDisposition::Applied,
            reason: None,
        }
    }

    /// `RequirementOutcome::enforced`.
    pub const fn enforced(id: &'a str) -> Self {
        Self {
            id,
            disposition: RequirementDisposition::Enforced,
            reason: None,
        }
    }

    /// `RequirementOutcome::degraded`.
    pub const fn degraded(id: &'a str, reason: &'a str) -> Self {
        Self {
            id,
            disposition: RequirementDisposition::Degraded,
            reason: Some(reason),
        }
    }

    /// `RequirementOutcome::rejected`.
    pub const fn rejected(id: &'a str, reason: &'a str) -> Self {
        Self {
            id,
            disposition: RequirementDisposition::Rejected,
            reason: Some(reason),
        }
    }
}
