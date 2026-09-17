//! Requirement classes and refs.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequirementClass {
    Enforced,
    EvidenceObligation,
    Guidance,
}

#[derive(Debug, Clone, Copy)]
pub struct RequirementRef<'a> {
    pub id: &'a str,
    pub class: RequirementClass,
    pub rule: &'a str,
    pub validator: Option<&'a str>,
    pub required: bool,
    pub priority: u8,
}
