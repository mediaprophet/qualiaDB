//! Purpose binding for biosense processing (selfhood).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiosensePurpose {
    /// Principal self-monitoring (Wellfair).
    WellfairSelfMonitor = 1,
    /// Opt-in research journal.
    Research = 2,
    /// Security unlock (liveness + 1:1).
    Security = 3,
    /// Explicit multi-party / CCTV policy evaluation only.
    SurveillancePolicy = 4,
}

impl BiosensePurpose {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WellfairSelfMonitor => "wellfair_self_monitor",
            Self::Research => "research",
            Self::Security => "security",
            Self::SurveillancePolicy => "surveillance_policy",
        }
    }
}
