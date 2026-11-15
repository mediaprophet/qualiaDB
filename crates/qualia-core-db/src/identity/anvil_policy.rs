//! QualiaDB / Webizen policy over Anvil hardware facts.
//!
//! Anvil records what the silicon can attest (IOMMU, GPU `deviceUUID`, TPM
//! slot). This module **manages** Sanctuary Mode: Human Agency consent is
//! required in addition to hardware isolation. Anvil does not unlock a vault.

/// Hardware isolation facts Anvil (or a probe) supplies. QualiaDB does not
/// depend on `anvil-core` yet; the field names match Anvil's slots.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HardwareIsolationFacts {
    pub iommu_present: bool,
    pub gpu_device_uuid: [u8; 16],
    pub sealed_anchor_matches: bool,
    pub attestation_tier: u8,
}

impl HardwareIsolationFacts {
    /// Die-serial or better (Anvil tiers 0..=1).
    pub fn hardware_isolated(self) -> bool {
        let mut has_uuid = false;
        let mut i = 0;
        while i < 16 {
            if self.gpu_device_uuid[i] != 0 {
                has_uuid = true;
                break;
            }
            i += 1;
        }
        self.iommu_present && self.sealed_anchor_matches && self.attestation_tier <= 1 && has_uuid
    }
}

/// Human Agency consent recorded by QualiaDB (not by Anvil).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HumanAgencyConsent {
    pub granted: bool,
}

/// Sanctuary Mode is granted only when hardware isolation **and** Human Agency
/// consent are both present.
pub fn grant_sanctuary_mode(facts: HardwareIsolationFacts, consent: HumanAgencyConsent) -> bool {
    facts.hardware_isolated() && consent.granted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn isolated_device_001() -> HardwareIsolationFacts {
        HardwareIsolationFacts {
            iommu_present: true,
            gpu_device_uuid: [0xAA; 16],
            sealed_anchor_matches: true,
            attestation_tier: 1,
        }
    }

    #[test]
    fn hardware_alone_does_not_unlock() {
        assert!(!grant_sanctuary_mode(
            isolated_device_001(),
            HumanAgencyConsent { granted: false }
        ));
    }

    #[test]
    fn hardware_and_agency_unlock() {
        assert!(grant_sanctuary_mode(
            isolated_device_001(),
            HumanAgencyConsent { granted: true }
        ));
    }

    #[test]
    fn spoofed_empty_uuid_stays_locked() {
        let mut facts = isolated_device_001();
        facts.gpu_device_uuid = [0; 16];
        assert!(!grant_sanctuary_mode(
            facts,
            HumanAgencyConsent { granted: true }
        ));
    }
}
