//! Cold call preparation (`prepare_extension_call`) resolving canonical IRIs,
//! short aliases, `SemVerReq` constraints, and target profiles into `PreparedCallHandle`s.

use super::registry::ExtensionRegistry;
use vibescript_abi::{
    PreparedCallHandle, SemVerReq, TargetProfileMask, VibeOpCode, VibeTrap,
};

/// Cold-stage resolver (`Stage 2: Prepare`) that binds `(namespace_or_alias, sub_namespace, func_name)`
/// against `version_req` and `target_profile` into a generational `PreparedCallHandle`.
pub fn prepare_extension_call(
    registry: &ExtensionRegistry<'_>,
    namespace_or_alias: &str,
    sub_namespace: &str,
    func_name: &str,
    version_req: SemVerReq,
    target_profile: TargetProfileMask,
) -> Result<PreparedCallHandle, VibeTrap> {
    let (slot, _provider, desc) = registry
        .find_by_namespace_or_alias(namespace_or_alias)
        .ok_or(VibeTrap::NamespaceNotFound)?;

    if !version_req.matches(desc.versions.provider_version) {
        return Err(VibeTrap::VersionRequirementUnsatisfied);
    }

    if !desc.supported_targets.supports(target_profile) {
        return Err(VibeTrap::TargetProfileUnsupported);
    }

    let sub = ExtensionRegistry::find_sub_namespace(desc, sub_namespace)
        .ok_or(VibeTrap::FunctionNotFound)?;

    let func = sub
        .functions
        .iter()
        .find(|f| f.name == func_name)
        .ok_or(VibeTrap::FunctionNotFound)?;

    let opcode = VibeOpCode::pack(slot, func.stable_id);
    Ok(PreparedCallHandle {
        opcode,
        registry_generation: registry.generation(),
        reserved: 0,
        provider_iri_hash: desc.canonical_iri.fnv1a_hash,
        func_iri_hash: func.canonical_iri.fnv1a_hash,
        contract_digest: func.contract.contract_digest,
    })
}
