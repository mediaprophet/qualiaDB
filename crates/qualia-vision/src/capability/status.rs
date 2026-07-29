//! Capability maturity for the vision excellence registry.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityStatus {
    /// Implemented, tested, product-usable at declared honesty level.
    Present,
    /// Usable path exists but incomplete vs excellence bar.
    Partial,
    /// Not implemented.
    Missing,
    /// Not a Qualia product goal.
    NotApplicable,
    /// Qualia has a different/stronger story than classical CV vendors.
    Beyond,
    /// Code path exists; production claim blocked on principal gate.
    CompleteWithGate,
}

impl CapabilityStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Partial => "partial",
            Self::Missing => "missing",
            Self::NotApplicable => "n/a",
            Self::Beyond => "beyond",
            Self::CompleteWithGate => "complete_with_gate",
        }
    }
}
