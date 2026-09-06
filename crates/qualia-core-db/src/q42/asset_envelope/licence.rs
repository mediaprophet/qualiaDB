//! Licence / use / redistribution policy for governed Q42 assets.
//!
//! Unknown licences fail closed: they never allow redistribution and cannot be
//! accepted into a validated envelope. Derived assets inherit the union of
//! upstream obligations (most restrictive wins).

use super::error::AssetEnvelopeError;

/// Recognised upstream licence classes for health/dataset assets.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LicenceClass {
    /// Provenance unknown — never redistributable.
    Unknown = 0,
    /// CC0 / public-domain-equivalent.
    Cc0 = 1,
    /// CC BY 4.0 (attribution).
    CcBy = 2,
    /// CC BY-SA 4.0 (attribution + share-alike).
    CcBySa = 3,
    /// CC BY-NC 4.0 (non-commercial).
    CcByNc = 4,
    /// Explicit proprietary grant recorded on the envelope (still may forbid redistribution).
    ProprietaryPermitted = 5,
}

impl LicenceClass {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Cc0,
            2 => Self::CcBy,
            3 => Self::CcBySa,
            4 => Self::CcByNc,
            5 => Self::ProprietaryPermitted,
            _ => Self::Unknown,
        }
    }

    /// Parse a short licence tag. Unrecognised strings become [`LicenceClass::Unknown`].
    pub fn parse(tag: &str) -> Self {
        let normalised = tag.trim().to_ascii_uppercase().replace(' ', "-");
        match normalised.as_str() {
            "CC0" | "CC0-1.0" | "PUBLIC-DOMAIN" => Self::Cc0,
            "CC-BY" | "CC-BY-4.0" | "CCBY" => Self::CcBy,
            "CC-BY-SA" | "CC-BY-SA-4.0" | "CCBYSA" => Self::CcBySa,
            "CC-BY-NC" | "CC-BY-NC-4.0" | "CCBYNC" => Self::CcByNc,
            "PROPRIETARY-PERMITTED" | "PROPRIETARY" => Self::ProprietaryPermitted,
            _ => Self::Unknown,
        }
    }
}

/// Intended use class recorded on the envelope.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UseClass {
    Unknown = 0,
    Research = 1,
    NonCommercial = 2,
    Commercial = 3,
    Internal = 4,
}

impl UseClass {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Research,
            2 => Self::NonCommercial,
            3 => Self::Commercial,
            4 => Self::Internal,
            _ => Self::Unknown,
        }
    }

    fn restrictiveness(self) -> u8 {
        match self {
            Self::Commercial => 1,
            Self::Research => 2,
            Self::NonCommercial => 3,
            Self::Internal => 4,
            Self::Unknown => 5,
        }
    }

    fn stricter(self, other: Self) -> Self {
        if self.restrictiveness() >= other.restrictiveness() {
            self
        } else {
            other
        }
    }
}

/// Redistribution posture for the asset and its derivatives.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RedistributionClass {
    Unknown = 0,
    FreelyRedistributable = 1,
    AttributionRequired = 2,
    NonCommercialOnly = 3,
    NoRedistribution = 4,
}

impl RedistributionClass {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::FreelyRedistributable,
            2 => Self::AttributionRequired,
            3 => Self::NonCommercialOnly,
            4 => Self::NoRedistribution,
            _ => Self::Unknown,
        }
    }

    fn restrictiveness(self) -> u8 {
        match self {
            Self::FreelyRedistributable => 1,
            Self::AttributionRequired => 2,
            Self::NonCommercialOnly => 3,
            Self::NoRedistribution => 4,
            Self::Unknown => 5,
        }
    }

    fn stricter(self, other: Self) -> Self {
        if self.restrictiveness() >= other.restrictiveness() {
            self
        } else {
            other
        }
    }
}

/// Obligation flags that propagate to derived assets (bitfield).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LicenceObligations(pub u16);

impl LicenceObligations {
    pub const ATTRIBUTION: u16 = 1 << 0;
    pub const SHARE_ALIKE: u16 = 1 << 1;
    pub const NON_COMMERCIAL: u16 = 1 << 2;
    pub const NO_REDISTRIBUTION: u16 = 1 << 3;
    pub const NOTICE: u16 = 1 << 4;

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn contains(self, flag: u16) -> bool {
        self.0 & flag != 0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn from_licence(class: LicenceClass) -> Self {
        match class {
            LicenceClass::Cc0 => Self::empty(),
            LicenceClass::CcBy => Self(Self::ATTRIBUTION | Self::NOTICE),
            LicenceClass::CcBySa => Self(Self::ATTRIBUTION | Self::SHARE_ALIKE | Self::NOTICE),
            LicenceClass::CcByNc => Self(Self::ATTRIBUTION | Self::NON_COMMERCIAL | Self::NOTICE),
            LicenceClass::ProprietaryPermitted => Self(Self::NOTICE | Self::NO_REDISTRIBUTION),
            LicenceClass::Unknown => Self(Self::NO_REDISTRIBUTION | Self::NOTICE),
        }
    }
}

/// Full licence policy recorded on a Q42 asset envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicencePolicy {
    pub class: LicenceClass,
    pub use_class: UseClass,
    pub redistribution: RedistributionClass,
    pub obligations: LicenceObligations,
    /// Upstream terms URL (may be empty only for CC0 when explicitly recorded).
    pub terms_url: String,
    pub attribution: String,
}

impl LicencePolicy {
    /// Build a policy from explicit classes. Unknown licence fails closed.
    pub fn try_new(
        class: LicenceClass,
        use_class: UseClass,
        redistribution: RedistributionClass,
        terms_url: impl Into<String>,
        attribution: impl Into<String>,
    ) -> Result<Self, AssetEnvelopeError> {
        if class == LicenceClass::Unknown {
            return Err(AssetEnvelopeError::UnknownLicence);
        }
        if redistribution == RedistributionClass::Unknown {
            return Err(AssetEnvelopeError::UnknownLicence);
        }
        if use_class == UseClass::Unknown {
            return Err(AssetEnvelopeError::UnknownLicence);
        }
        let terms_url = terms_url.into();
        let attribution = attribution.into();
        if class != LicenceClass::Cc0 && terms_url.trim().is_empty() {
            return Err(AssetEnvelopeError::MissingTermsUrl);
        }
        let mut obligations = LicenceObligations::from_licence(class);
        if matches!(
            redistribution,
            RedistributionClass::NoRedistribution | RedistributionClass::NonCommercialOnly
        ) {
            obligations = obligations.union(LicenceObligations(LicenceObligations::NO_REDISTRIBUTION));
        }
        if redistribution == RedistributionClass::NonCommercialOnly {
            obligations = obligations.union(LicenceObligations(LicenceObligations::NON_COMMERCIAL));
        }
        if redistribution == RedistributionClass::AttributionRequired {
            obligations = obligations.union(LicenceObligations(LicenceObligations::ATTRIBUTION));
        }
        Ok(Self {
            class,
            use_class,
            redistribution,
            obligations,
            terms_url,
            attribution,
        })
    }

    /// Parse from a short licence tag plus recorded terms. Unknown tags fail closed.
    pub fn from_tag(
        tag: &str,
        terms_url: impl Into<String>,
        attribution: impl Into<String>,
    ) -> Result<Self, AssetEnvelopeError> {
        let class = LicenceClass::parse(tag);
        let (use_class, redistribution) = match class {
            LicenceClass::Cc0 => (UseClass::Commercial, RedistributionClass::FreelyRedistributable),
            LicenceClass::CcBy | LicenceClass::CcBySa => {
                (UseClass::Commercial, RedistributionClass::AttributionRequired)
            }
            LicenceClass::CcByNc => (UseClass::NonCommercial, RedistributionClass::NonCommercialOnly),
            LicenceClass::ProprietaryPermitted => {
                (UseClass::Internal, RedistributionClass::NoRedistribution)
            }
            LicenceClass::Unknown => {
                return Err(AssetEnvelopeError::UnknownLicence);
            }
        };
        Self::try_new(class, use_class, redistribution, terms_url, attribution)
    }

    /// May this asset (or a derived bundle) be redistributed outside the local node?
    pub fn allows_redistribution(&self) -> bool {
        !self
            .obligations
            .contains(LicenceObligations::NO_REDISTRIBUTION)
            && matches!(
                self.redistribution,
                RedistributionClass::FreelyRedistributable
                    | RedistributionClass::AttributionRequired
            )
            && self.class != LicenceClass::Unknown
    }

    /// Most-restrictive union of two policies for a derived asset.
    pub fn union_obligations(&self, other: &Self) -> Self {
        let class = if self.class.restrictiveness() >= other.class.restrictiveness() {
            self.class
        } else {
            other.class
        };
        let use_class = self.use_class.stricter(other.use_class);
        let redistribution = self.redistribution.stricter(other.redistribution);
        let obligations = self.obligations.union(other.obligations);
        let terms_url = if other.terms_url.len() > self.terms_url.len() {
            other.terms_url.clone()
        } else {
            self.terms_url.clone()
        };
        let attribution = if self.attribution.is_empty() {
            other.attribution.clone()
        } else if other.attribution.is_empty() || self.attribution == other.attribution {
            self.attribution.clone()
        } else {
            format!("{}; {}", self.attribution, other.attribution)
        };
        Self {
            class,
            use_class,
            redistribution,
            obligations,
            terms_url,
            attribution,
        }
    }
}

impl LicenceClass {
    fn restrictiveness(self) -> u8 {
        match self {
            Self::Cc0 => 1,
            Self::CcBy => 2,
            Self::CcBySa => 3,
            Self::CcByNc => 4,
            Self::ProprietaryPermitted => 5,
            Self::Unknown => 6,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_licence_tag_fails_closed() {
        assert!(matches!(
            LicencePolicy::from_tag("totally-made-up", "https://example.test", "x"),
            Err(AssetEnvelopeError::UnknownLicence)
        ));
    }

    #[test]
    fn cc_by_allows_redistribution_with_attribution() {
        let policy = LicencePolicy::from_tag(
            "CC-BY-4.0",
            "https://creativecommons.org/licenses/by/4.0/",
            "ChEBI",
        )
        .unwrap();
        assert!(policy.allows_redistribution());
        assert!(policy.obligations.contains(LicenceObligations::ATTRIBUTION));
    }

    #[test]
    fn obligation_union_takes_stricter_side() {
        let a = LicencePolicy::from_tag(
            "CC-BY-4.0",
            "https://creativecommons.org/licenses/by/4.0/",
            "A",
        )
        .unwrap();
        let b = LicencePolicy::from_tag(
            "CC-BY-NC-4.0",
            "https://creativecommons.org/licenses/by-nc/4.0/",
            "B",
        )
        .unwrap();
        let u = a.union_obligations(&b);
        assert_eq!(u.class, LicenceClass::CcByNc);
        assert_eq!(u.redistribution, RedistributionClass::NonCommercialOnly);
        assert!(u.obligations.contains(LicenceObligations::NON_COMMERCIAL));
        assert!(!u.allows_redistribution());
    }
}
