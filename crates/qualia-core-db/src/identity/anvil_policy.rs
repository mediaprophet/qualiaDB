//! QualiaDB / Webizen policy over Anvil hardware facts.
//!
//! Anvil records what the silicon can attest (IOMMU, GPU `deviceUUID`, TPM
//! slot) in [`NodeProfile64`]. This module **manages** Sanctuary Mode: Human
//! Agency consent is required in addition to hardware isolation. Anvil does
//! not unlock a vault.
//!
//! N2: a hardware-attested vault claim **fails closed** when the live profile
//! file is missing or invalid. Do not advertise that claim without the file.
//! N3: Etalon overlays Vulkan `deviceUUID` onto that same file. Zeros are not
//! written over a previous bind.
//! N4: F4 overlay copy uses [`hardware_attested_vault_copy`]. The phrase
//! “hardware-attested vault” is emitted only when the live profile on *that*
//! box supports advertising (IOMMU + GPU UUID + tier ≤ 1).

use anvil_core::{
    NodeProfile64, NodeProfileError, PROFILE_HAS_IOMMU, PROFILE_HAS_GPU_UUID,
};

/// Filename Anvil's scraper writes; Qualia looks for it under storage.
pub const NODE_PROFILE_FILE_NAME: &str = "anvil_node_profile.np64";

/// Product phrase F4 may print only when [`may_advertise_hardware_attested_vault`] is true.
pub const HARDWARE_ATTESTED_VAULT_PHRASE: &str = "hardware-attested vault";

/// Empty when the gate is closed. Never a partial claim.
pub fn hardware_attested_vault_copy(may_advertise: bool) -> &'static str {
    if may_advertise {
        HARDWARE_ATTESTED_VAULT_PHRASE
    } else {
        ""
    }
}

/// Hardware isolation facts Anvil supplies. Qualia adds sealed-anchor state.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HardwareIsolationFacts {
    pub iommu_present: bool,
    pub gpu_device_uuid: [u8; 16],
    pub sealed_anchor_matches: bool,
    pub attestation_tier: u8,
}

impl HardwareIsolationFacts {
    /// Die-serial or better (Anvil tiers 0..=1), IOMMU, non-zero GPU UUID, seal.
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

    /// Anvil-sourced bits only (no Qualia seal). Used to refuse advertising.
    pub fn profile_supports_attestation(self) -> bool {
        let mut has_uuid = false;
        let mut i = 0;
        while i < 16 {
            if self.gpu_device_uuid[i] != 0 {
                has_uuid = true;
                break;
            }
            i += 1;
        }
        self.iommu_present && self.attestation_tier <= 1 && has_uuid
    }
}

/// Map a parsed Anvil profile into Qualia facts. Seal is Qualia's after calibration.
pub fn facts_from_profile(profile: &NodeProfile64, sealed_anchor_matches: bool) -> HardwareIsolationFacts {
    HardwareIsolationFacts {
        iommu_present: profile.has_flag(PROFILE_HAS_IOMMU),
        gpu_device_uuid: if profile.has_flag(PROFILE_HAS_GPU_UUID) {
            profile.gpu_device_uuid
        } else {
            [0; 16]
        },
        sealed_anchor_matches,
        attestation_tier: profile.attestation_tier,
    }
}

/// Human Agency consent recorded by QualiaDB (not by Anvil).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HumanAgencyConsent {
    pub granted: bool,
}

/// Why an attested-vault claim cannot proceed.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AttestationClaimError {
    ProfileMissing,
    ProfileUnreadable,
    ProfileInvalid,
}

impl From<NodeProfileError> for AttestationClaimError {
    fn from(_: NodeProfileError) -> Self {
        Self::ProfileInvalid
    }
}

/// Sanctuary Mode is granted only when hardware isolation **and** Human Agency
/// consent are both present.
pub fn grant_sanctuary_mode(facts: HardwareIsolationFacts, consent: HumanAgencyConsent) -> bool {
    facts.hardware_isolated() && consent.granted
}

/// Parse a live profile from bytes. No filesystem.
pub fn parse_node_profile(bytes: &[u8]) -> Result<NodeProfile64, AttestationClaimError> {
    NodeProfile64::parse(bytes).map_err(Into::into)
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use std::path::{Path, PathBuf};

    pub fn node_profile_path_in_storage(storage: &Path) -> PathBuf {
        storage.join(NODE_PROFILE_FILE_NAME)
    }

    /// `ANVIL_NODE_PROFILE`, else `{QUALIA_STORAGE_PATH}/anvil_node_profile.np64`.
    pub fn default_node_profile_path() -> Option<PathBuf> {
        if let Ok(p) = std::env::var("ANVIL_NODE_PROFILE") {
            if !p.is_empty() {
                return Some(PathBuf::from(p));
            }
        }
        if let Ok(p) = std::env::var("QUALIA_STORAGE_PATH") {
            if !p.is_empty() {
                return Some(PathBuf::from(p).join(NODE_PROFILE_FILE_NAME));
            }
        }
        None
    }

    pub fn load_node_profile(path: &Path) -> Result<NodeProfile64, AttestationClaimError> {
        if !path.is_file() {
            return Err(AttestationClaimError::ProfileMissing);
        }
        let bytes = std::fs::read(path).map_err(|_| AttestationClaimError::ProfileUnreadable)?;
        parse_node_profile(&bytes)
    }

    /// Fail closed: missing/invalid profile is an error, not a silent deny.
    pub fn claim_hardware_attested_vault(
        profile_path: &Path,
        consent: HumanAgencyConsent,
        sealed_anchor_matches: bool,
    ) -> Result<bool, AttestationClaimError> {
        let profile = load_node_profile(profile_path)?;
        let facts = facts_from_profile(&profile, sealed_anchor_matches);
        Ok(grant_sanctuary_mode(facts, consent))
    }

    /// Advertising copy: profile file must parse **and** carry IOMMU + GPU UUID + tier ≤ 1.
    pub fn may_advertise_hardware_attested_vault(profile_path: &Path) -> bool {
        match load_node_profile(profile_path) {
            Ok(profile) => profile.supports_attested_vault_advertising(),
            Err(_) => false,
        }
    }

    /// N4: this process. Missing `ANVIL_NODE_PROFILE` / storage path fails closed.
    pub fn advertise_hardware_attested_vault_on_this_host() -> bool {
        match default_node_profile_path() {
            Some(path) => may_advertise_hardware_attested_vault(&path),
            None => false,
        }
    }

    /// F4 overlay line. Empty unless this host may advertise.
    pub fn f4_hardware_attested_vault_line() -> &'static str {
        hardware_attested_vault_copy(advertise_hardware_attested_vault_on_this_host())
    }

    /// Overlay Etalon's live GPU UUID onto the existing profile file.
    ///
    /// Missing/invalid file fails closed. All-zero UUID does not rewrite the
    /// file (a previous bind stays). Returns whether bytes were written.
    /// Obtain `uuid` from `etalon_agent::anvil_bind::live_device_uuid` once
    /// the workspace Etalon pin includes N3.
    pub fn persist_gpu_uuid(
        profile_path: &Path,
        uuid: [u8; 16],
    ) -> Result<bool, AttestationClaimError> {
        let mut nonzero = false;
        let mut i = 0;
        while i < 16 {
            if uuid[i] != 0 {
                nonzero = true;
                break;
            }
            i += 1;
        }
        if !nonzero {
            return Ok(false);
        }
        let mut profile = load_node_profile(profile_path)?;
        profile.bind_gpu_device_uuid(uuid);
        std::fs::write(profile_path, profile.to_bytes())
            .map_err(|_| AttestationClaimError::ProfileUnreadable)?;
        Ok(true)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::{
    advertise_hardware_attested_vault_on_this_host, claim_hardware_attested_vault,
    default_node_profile_path, f4_hardware_attested_vault_line, load_node_profile,
    may_advertise_hardware_attested_vault, node_profile_path_in_storage, persist_gpu_uuid,
};

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_core::{AnvilContext, AttestationTier, ISO_VTD_IOMMU};

    fn isolated_device_001() -> HardwareIsolationFacts {
        HardwareIsolationFacts {
            iommu_present: true,
            gpu_device_uuid: [0xAA; 16],
            sealed_anchor_matches: true,
            attestation_tier: 1,
        }
    }

    fn isolated_profile() -> NodeProfile64 {
        let mut ctx = AnvilContext::empty();
        ctx.capabilities.isolation = ISO_VTD_IOMMU;
        ctx.attestation_tier = AttestationTier::DieSerial;
        ctx.bind_gpu_device_uuid([0xAA; 16]);
        NodeProfile64::from_context(&ctx, 0x10DE, 0x2531, 0x00A1, true)
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

    #[test]
    fn facts_from_anvil_profile_carry_iommu_and_uuid() {
        let p = isolated_profile();
        assert!(p.has_flag(PROFILE_HAS_IOMMU));
        assert!(p.has_flag(PROFILE_HAS_GPU_UUID));
        assert!(p.supports_attested_vault_advertising());
        let facts = facts_from_profile(&p, true);
        assert!(facts.hardware_isolated());
        assert!(facts.profile_supports_attestation());
    }

    #[test]
    fn parse_rejects_truncated() {
        assert_eq!(
            parse_node_profile(&[0x4E, 0x50]),
            Err(AttestationClaimError::ProfileInvalid)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    mod native_files {
        use super::*;
        use std::time::{SystemTime, UNIX_EPOCH};

        fn temp_path(tag: &str) -> std::path::PathBuf {
            let n = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            std::env::temp_dir().join(format!("anvil-n2-{tag}-{n}.np64"))
        }

        #[test]
        fn missing_profile_fails_closed() {
            let path = std::env::temp_dir().join("anvil-n2-missing-does-not-exist.np64");
            let _ = std::fs::remove_file(&path);
            assert_eq!(
                claim_hardware_attested_vault(
                    &path,
                    HumanAgencyConsent { granted: true },
                    true
                ),
                Err(AttestationClaimError::ProfileMissing)
            );
            assert!(!may_advertise_hardware_attested_vault(&path));
        }

        #[test]
        fn invalid_profile_fails_closed() {
            let path = temp_path("bad");
            std::fs::write(&path, b"not-an-np64-file").unwrap();
            let err = claim_hardware_attested_vault(
                &path,
                HumanAgencyConsent { granted: true },
                true,
            );
            let _ = std::fs::remove_file(&path);
            assert_eq!(err, Err(AttestationClaimError::ProfileInvalid));
        }

        #[test]
        fn isolated_profile_file_may_advertise_and_grant() {
            let path = temp_path("ok");
            std::fs::write(&path, isolated_profile().to_bytes()).unwrap();
            assert!(may_advertise_hardware_attested_vault(&path));
            assert_eq!(
                claim_hardware_attested_vault(
                    &path,
                    HumanAgencyConsent { granted: true },
                    true
                ),
                Ok(true)
            );
            assert_eq!(
                claim_hardware_attested_vault(
                    &path,
                    HumanAgencyConsent { granted: false },
                    true
                ),
                Ok(false)
            );
            let _ = std::fs::remove_file(&path);
        }

        #[test]
        fn persist_gpu_uuid_overlays_n1_file() {
            let mut ctx = AnvilContext::empty();
            ctx.attestation_tier = AttestationTier::DieSerial;
            ctx.capabilities.isolation = ISO_VTD_IOMMU;
            let p = NodeProfile64::from_context(&ctx, 0x10DE, 0x2D58, 0, true);
            let path = temp_path("n3");
            std::fs::write(&path, p.to_bytes()).unwrap();
            assert!(!may_advertise_hardware_attested_vault(&path));
            assert_eq!(persist_gpu_uuid(&path, [0xAA; 16]), Ok(true));
            assert!(may_advertise_hardware_attested_vault(&path));
            let loaded = load_node_profile(&path).unwrap();
            assert_eq!(loaded.gpu_device_uuid, [0xAA; 16]);
            assert_eq!(persist_gpu_uuid(&path, [0; 16]), Ok(false));
            let loaded = load_node_profile(&path).unwrap();
            assert_eq!(loaded.gpu_device_uuid, [0xAA; 16]);
            let _ = std::fs::remove_file(&path);
        }

        #[test]
        fn persist_gpu_uuid_missing_file_fails_closed() {
            let path = std::env::temp_dir().join("anvil-n3-missing-does-not-exist.np64");
            let _ = std::fs::remove_file(&path);
            assert_eq!(
                persist_gpu_uuid(&path, [0xAA; 16]),
                Err(AttestationClaimError::ProfileMissing)
            );
        }

        #[test]
        fn n4_copy_omits_phrase_when_gate_closed() {
            assert_eq!(hardware_attested_vault_copy(false), "");
            let path = std::env::temp_dir().join("anvil-n4-missing-does-not-exist.np64");
            let _ = std::fs::remove_file(&path);
            assert!(!may_advertise_hardware_attested_vault(&path));
            assert_eq!(
                hardware_attested_vault_copy(may_advertise_hardware_attested_vault(&path)),
                ""
            );
        }

        #[test]
        fn n4_copy_includes_phrase_only_when_gate_open() {
            let path = temp_path("n4-open");
            std::fs::write(&path, isolated_profile().to_bytes()).unwrap();
            assert!(may_advertise_hardware_attested_vault(&path));
            assert_eq!(
                hardware_attested_vault_copy(true),
                HARDWARE_ATTESTED_VAULT_PHRASE
            );
            assert_eq!(
                hardware_attested_vault_copy(may_advertise_hardware_attested_vault(&path)),
                HARDWARE_ATTESTED_VAULT_PHRASE
            );
            let _ = std::fs::remove_file(&path);
        }

        #[test]
        fn n4_this_box_n1_shape_must_not_advertise() {
            let mut ctx = AnvilContext::empty();
            ctx.attestation_tier = AttestationTier::DieSerial;
            let p = NodeProfile64::from_context(&ctx, 0x10DE, 0x2D58, 0, true);
            let path = temp_path("n4-box");
            std::fs::write(&path, p.to_bytes()).unwrap();
            assert!(!may_advertise_hardware_attested_vault(&path));
            assert_eq!(
                hardware_attested_vault_copy(may_advertise_hardware_attested_vault(&path)),
                ""
            );
            let _ = std::fs::remove_file(&path);
        }

        #[test]
        fn live_n1_profile_without_uuid_must_not_advertise() {
            let mut ctx = AnvilContext::empty();
            ctx.attestation_tier = AttestationTier::DieSerial;
            let p = NodeProfile64::from_context(&ctx, 0x10DE, 0x2D58, 0, true);
            let path = temp_path("n1");
            std::fs::write(&path, p.to_bytes()).unwrap();
            assert!(!may_advertise_hardware_attested_vault(&path));
            assert_eq!(
                claim_hardware_attested_vault(
                    &path,
                    HumanAgencyConsent { granted: true },
                    true
                ),
                Ok(false)
            );
            let _ = std::fs::remove_file(&path);
        }
    }
}
