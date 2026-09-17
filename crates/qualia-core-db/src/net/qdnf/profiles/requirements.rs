//! Required control set from label confidentiality, task, sender policy and
//! environment. Cost preference is not an input and cannot reduce protection.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::policy_labels::Confidentiality;

use super::catalog::{
    catalog_entry, ControlId, ControlPredicate, ControlSet, ProtectionProfile, MAX_CONTROLS,
};

/// Application task. Raises the *minimum* named profile; catalog predicates
/// still have to be present at negotiation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskClass {
    PublicDiscovery = 0,
    KnownPeerMessage = 1,
    ProfessionalRecord = 2,
    HostileEnvironmentCare = 3,
    CompartmentedMission = 4,
}

/// Assessed network/endpoint environment.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssessedEnvironment {
    Friendly = 0,
    Observed = 1,
    Hostile = 2,
}

/// Sender constraints that may only add controls or raise the floor.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SenderPolicy {
    pub extra_protection: bool,
    pub require_isolated_bearer: bool,
    pub require_cover_traffic: bool,
    pub require_short_capabilities: bool,
}

impl SenderPolicy {
    pub const DEFAULT: Self = Self {
        extra_protection: false,
        require_isolated_bearer: false,
        require_cover_traffic: false,
        require_short_capabilities: false,
    };
}

/// Inputs for [`required_control_set`]. No cost/paid-tier field exists.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequirementContext {
    pub confidentiality: Confidentiality,
    pub compartment_count: u8,
    pub extra_controls: [ControlId; MAX_CONTROLS],
    pub extra_control_count: u8,
    pub task: TaskClass,
    pub environment: AssessedEnvironment,
    pub sender: SenderPolicy,
}

impl RequirementContext {
    pub fn new(
        confidentiality: Confidentiality,
        task: TaskClass,
        environment: AssessedEnvironment,
    ) -> Self {
        Self {
            confidentiality,
            compartment_count: 0,
            extra_controls: [ControlId::ZERO; MAX_CONTROLS],
            extra_control_count: 0,
            task,
            environment,
            sender: SenderPolicy::DEFAULT,
        }
    }

    pub fn extra_controls(&self) -> &[ControlId] {
        &self.extra_controls[..self.extra_control_count as usize]
    }

    pub fn push_extra(&mut self, id: ControlId) -> Result<(), QdnfError> {
        let mut i = 0usize;
        while i < self.extra_control_count as usize {
            if self.extra_controls[i] == id {
                return Ok(());
            }
            i += 1;
        }
        if self.extra_control_count as usize >= MAX_CONTROLS {
            return Err(QdnfError::Capacity);
        }
        self.extra_controls[self.extra_control_count as usize] = id;
        self.extra_control_count += 1;
        Ok(())
    }
}

pub fn min_profile_for_confidentiality(
    confidentiality: Confidentiality,
    compartment_count: u8,
) -> Result<ProtectionProfile, QdnfError> {
    match confidentiality {
        Confidentiality::C0Public => Ok(ProtectionProfile::P0),
        Confidentiality::C1Private => Ok(ProtectionProfile::P1),
        Confidentiality::C2Sensitive => Ok(ProtectionProfile::P2),
        Confidentiality::C3Compartmented => {
            if compartment_count > 0 {
                Ok(ProtectionProfile::P4)
            } else {
                Ok(ProtectionProfile::P3)
            }
        }
        Confidentiality::Unknown => Err(QdnfError::Conflict),
    }
}

fn task_floor(task: TaskClass) -> ProtectionProfile {
    match task {
        TaskClass::PublicDiscovery => ProtectionProfile::P0,
        TaskClass::KnownPeerMessage => ProtectionProfile::P1,
        TaskClass::ProfessionalRecord => ProtectionProfile::P2,
        TaskClass::HostileEnvironmentCare => ProtectionProfile::P3,
        TaskClass::CompartmentedMission => ProtectionProfile::P4,
    }
}

fn environment_floor(
    environment: AssessedEnvironment,
    confidentiality: Confidentiality,
) -> ProtectionProfile {
    match environment {
        AssessedEnvironment::Friendly => ProtectionProfile::P0,
        AssessedEnvironment::Observed | AssessedEnvironment::Hostile => {
            if matches!(confidentiality, Confidentiality::C0Public) {
                ProtectionProfile::P0
            } else {
                ProtectionProfile::P3
            }
        }
    }
}

pub fn required_control_set(ctx: &RequirementContext) -> Result<ControlSet, QdnfError> {
    let conf_floor = min_profile_for_confidentiality(ctx.confidentiality, ctx.compartment_count)?;
    let mut floor = conf_floor
        .raise_floor(task_floor(ctx.task))
        .raise_floor(environment_floor(ctx.environment, ctx.confidentiality));
    if ctx.sender.require_isolated_bearer {
        floor = floor.raise_floor(ProtectionProfile::P4);
    }
    if ctx.sender.extra_protection {
        floor = floor.one_stronger();
    }
    let mut set = catalog_entry(floor);
    if ctx.sender.require_isolated_bearer {
        set.try_push_predicate(ControlPredicate::IsolatedBearer)?;
    }
    if ctx.sender.require_cover_traffic {
        set.try_push_predicate(ControlPredicate::CoverTraffic)?;
    }
    if ctx.sender.require_short_capabilities {
        set.try_push_predicate(ControlPredicate::ShortCapabilities)?;
    }
    let mut i = 0usize;
    while i < ctx.extra_control_count as usize {
        set.try_push(ctx.extra_controls[i])?;
        i += 1;
    }
    Ok(set)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::profiles::catalog::catalog_entry;

    #[test]
    fn unknown_confidentiality_never_maps_to_p0() {
        assert_eq!(
            min_profile_for_confidentiality(Confidentiality::Unknown, 0),
            Err(QdnfError::Conflict)
        );
        let ctx = RequirementContext::new(
            Confidentiality::Unknown,
            TaskClass::PublicDiscovery,
            AssessedEnvironment::Friendly,
        );
        assert_eq!(required_control_set(&ctx), Err(QdnfError::Conflict));
        assert_ne!(Confidentiality::Unknown, Confidentiality::C0Public);
        let p0 = required_control_set(&RequirementContext::new(
            Confidentiality::C0Public,
            TaskClass::PublicDiscovery,
            AssessedEnvironment::Friendly,
        ))
        .unwrap();
        assert_eq!(p0.profile, ProtectionProfile::P0);
    }

    #[test]
    fn confidentiality_floors() {
        assert_eq!(
            min_profile_for_confidentiality(Confidentiality::C0Public, 0).unwrap(),
            ProtectionProfile::P0
        );
        assert_eq!(
            min_profile_for_confidentiality(Confidentiality::C1Private, 0).unwrap(),
            ProtectionProfile::P1
        );
        assert_eq!(
            min_profile_for_confidentiality(Confidentiality::C2Sensitive, 0).unwrap(),
            ProtectionProfile::P2
        );
        assert_eq!(
            min_profile_for_confidentiality(Confidentiality::C3Compartmented, 0).unwrap(),
            ProtectionProfile::P3
        );
        assert_eq!(
            min_profile_for_confidentiality(Confidentiality::C3Compartmented, 1).unwrap(),
            ProtectionProfile::P4
        );
    }

    #[test]
    fn c2_requires_p2_catalog() {
        let ctx = RequirementContext::new(
            Confidentiality::C2Sensitive,
            TaskClass::KnownPeerMessage,
            AssessedEnvironment::Friendly,
        );
        let set = required_control_set(&ctx).unwrap();
        assert_eq!(set.profile, ProtectionProfile::P2);
        assert!(set.contains_predicate(ControlPredicate::ProtectedEndpoint));
        assert_eq!(set, catalog_entry(ProtectionProfile::P2));
    }

    #[test]
    fn extra_label_controls_must_be_present() {
        let mut ctx = RequirementContext::new(
            Confidentiality::C1Private,
            TaskClass::KnownPeerMessage,
            AssessedEnvironment::Friendly,
        );
        ctx.push_extra(ControlId::from_predicate(ControlPredicate::ApprovedRelay))
            .unwrap();
        let set = required_control_set(&ctx).unwrap();
        assert_eq!(set.profile, ProtectionProfile::P1);
        assert!(set.contains_predicate(ControlPredicate::ApprovedRelay));
        assert!(set.contains_all(&catalog_entry(ProtectionProfile::P1)));
    }

    #[test]
    fn extra_protection_may_raise_floor() {
        let mut ctx = RequirementContext::new(
            Confidentiality::C1Private,
            TaskClass::KnownPeerMessage,
            AssessedEnvironment::Friendly,
        );
        ctx.sender.extra_protection = true;
        let set = required_control_set(&ctx).unwrap();
        assert_eq!(set.profile, ProtectionProfile::P2);
    }

    #[test]
    fn hostile_environment_does_not_raise_public_c0() {
        let ctx = RequirementContext::new(
            Confidentiality::C0Public,
            TaskClass::PublicDiscovery,
            AssessedEnvironment::Hostile,
        );
        let set = required_control_set(&ctx).unwrap();
        assert_eq!(set.profile, ProtectionProfile::P0);
    }

    #[test]
    fn hostile_environment_raises_private_to_p3() {
        let ctx = RequirementContext::new(
            Confidentiality::C1Private,
            TaskClass::KnownPeerMessage,
            AssessedEnvironment::Hostile,
        );
        let set = required_control_set(&ctx).unwrap();
        assert_eq!(set.profile, ProtectionProfile::P3);
        assert!(set.contains_predicate(ControlPredicate::ApprovedRelay));
    }
}
