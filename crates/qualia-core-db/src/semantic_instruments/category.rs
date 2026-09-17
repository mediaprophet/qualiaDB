//! Content category: demo seed vs reference vs operational.

use super::errors::InstrumentError;

/// How a collectable may be presented when populating an environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContentCategory {
    /// Demonstration and development seed. Must stay labelled.
    Demo,
    /// Conformance/test reference. Must stay labelled.
    Reference,
    /// Intended for real employment under honesty/applicability/capacity.
    Operational,
}

impl ContentCategory {
    pub const fn as_iri(self) -> &'static str {
        match self {
            Self::Demo => "si:Demo",
            Self::Reference => "si:Reference",
            Self::Operational => "si:Operational",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, InstrumentError> {
        match raw.trim() {
            "si:Demo" | "demo" | "Demo" => Ok(Self::Demo),
            "si:Reference" | "reference" | "Reference" => Ok(Self::Reference),
            "si:Operational" | "operational" | "Operational" => Ok(Self::Operational),
            _ => Err(InstrumentError::InvalidCategory),
        }
    }

    pub const fn requires_seed_label(self) -> bool {
        matches!(self, Self::Demo | Self::Reference)
    }
}

/// A Demo/Reference notice must say it is demonstration, development, labelled,
/// fixture, or reference — never silent operational chrome.
pub fn seed_notice_is_labelled(notice: &str) -> bool {
    let n = notice.to_ascii_lowercase();
    n.contains("demo")
        || n.contains("demonstration")
        || n.contains("development")
        || n.contains("labelled fixture")
        || n.contains("labeled fixture")
        || n.contains("reference pack")
        || n.contains("not operational")
}
