use super::super::{
    admission::{live, strong_key, BoundedBytes},
    AdmissionSnapshot, PrincipalKind,
};
use crate::crypto::network::ed25519;
use qualia_cooperative_core::authoring::delegation::{
    check_parent_attenuation, validate_delegation, AuthoringDelegation, SignedAuthoringDelegation,
    MAX_AUTHORING_DELEGATION_BYTES, MAX_AUTHORING_DELEGATION_DEPTH,
};
use sha2::{Digest, Sha256};

pub const AUTHORING_DELEGATION_CONTEXT: &[u8] = b"qualia.native-authoring-delegation.v1";

pub fn authoring_delegation_digest(record: &AuthoringDelegation) -> Result<[u8; 32], String> {
    let mut writer = BoundedBytes {
        bytes: Vec::new(),
        limit: MAX_AUTHORING_DELEGATION_BYTES,
    };
    serde_json::to_writer(&mut writer, record).map_err(|e| e.to_string())?;
    Ok(Sha256::digest(&writer.bytes).into())
}

/// This root comes only from the current NativeAuthority lock plus its live
/// authenticated Human-capacity session. It is project/controller admission,
/// not independent personhood evidence, domain acceptance or an adopted grant.
pub(super) struct ControllerRoot<'a> {
    pub subject: &'a str,
    pub controller: &'a [u8; 32],
    pub admission_digest: [u8; 32],
}

pub(super) struct CheckedChain {
    pub path: [[u8; 32]; MAX_AUTHORING_DELEGATION_DEPTH],
    pub count: usize,
}

fn admitted<'a>(
    snapshot: &'a AdmissionSnapshot,
    subject: &str,
    key: &[u8; 32],
    now: u64,
) -> Result<&'a super::super::SubjectAdmission, String> {
    let s = snapshot
        .subjects
        .iter()
        .find(|s| s.subject == subject)
        .ok_or("delegation subject has no current signed admission")?;
    if s.controller_public_key != *key
        || !strong_key(key)
        || s.revoked
        || !live(now, s.valid_from_unix, s.valid_until_unix)
        || s.membership.project_id != snapshot.project.id
        || u64::try_from(s.membership.agreed_at().to_unix_secs()).map_or(true, |time| time > now)
        || !s.identifiers.iter().any(|id| {
            id.value == s.membership.member_did
                && id.project_id == snapshot.project.id
                && live(now, id.valid_from_unix, id.valid_until_unix)
        })
    {
        return Err(
            "delegation controller/membership association expired/revoked/mismatched".into(),
        );
    }
    Ok(s)
}

/// Validate every supplied edge against the current host admission and exact
/// parent digest. The owner must additionally retain adoption, current lifecycle,
/// shared budget consumption and live leaf authentication before dispatch.
pub(super) fn verify_chain(
    root: ControllerRoot<'_>,
    snapshot: &AdmissionSnapshot,
    chain: &[SignedAuthoringDelegation],
    now: u64,
    cancelled: &[[u8; 32]],
) -> Result<CheckedChain, String> {
    if chain.is_empty()
        || chain.len() > MAX_AUTHORING_DELEGATION_DEPTH
        || cancelled.len() > 256
        || snapshot.revoked
        || !live(now, snapshot.valid_from_unix, snapshot.valid_until_unix)
    {
        return Err("delegation chain/admission bounds or lifecycle denied".into());
    }
    let principal = admitted(snapshot, root.subject, root.controller, now)?;
    if principal.kind != PrincipalKind::Human {
        return Err(
            "root requires current project-admitted Human capacity; this does not prove personhood"
                .into(),
        );
    }
    let mut checked = CheckedChain {
        path: [[0; 32]; MAX_AUTHORING_DELEGATION_DEPTH],
        count: chain.len(),
    };
    for (index, signed) in chain.iter().enumerate() {
        let d = &signed.delegation;
        validate_delegation(d, now).map_err(|e| format!("delegation policy: {e:?}"))?;
        if d.root_subject != root.subject
            || d.root_adoption_digest != root.admission_digest
            || d.project_id != snapshot.project.id
            || d.agency.principal_did != principal.membership.member_did
            || d.remaining_depth as usize + index >= MAX_AUTHORING_DELEGATION_DEPTH
        {
            return Err("delegation root/project/admission binding mismatch".into());
        }
        let grantor = admitted(snapshot, &d.grantor_subject, &d.grantor_controller, now)?;
        let grantee = admitted(snapshot, &d.grantee_subject, &d.grantee_controller, now)?;
        let end = d
            .agency
            .valid_to_unix
            .ok_or("bounded delegation expiry required")? as u64;
        if [principal, grantor, grantee].iter().any(|s| {
            (d.agency.valid_from_unix as u64) < s.valid_from_unix || end > s.valid_until_unix
        }) || (d.agency.valid_from_unix as u64) < snapshot.valid_from_unix
            || end > snapshot.valid_until_unix
        {
            return Err("delegation exceeds admitted controller validity".into());
        }
        if grantee.kind != PrincipalKind::SoftwareAgent
            || d.agency.agent_dids[0] != grantee.membership.member_did
        {
            return Err("delegation recipient is not the admitted agent association".into());
        }
        // Repeating any controller or subject in a linear path is a cycle,
        // including a return to the root under another identifier.
        if d.grantee_subject == root.subject
            || d.grantee_controller == *root.controller
            || chain[..index].iter().any(|p| {
                p.delegation.grantee_subject == d.grantee_subject
                    || p.delegation.grantee_controller == d.grantee_controller
            })
        {
            return Err("delegation cycle refused".into());
        }
        if chain[..index]
            .iter()
            .any(|p| p.delegation.task.delegation_id == d.task.delegation_id)
        {
            return Err("delegation identifier replay refused".into());
        }
        if index == 0 {
            if d.parent_digest.is_some()
                || d.grantor_subject != root.subject
                || d.grantor_controller != *root.controller
            {
                return Err(
                    "root consent must be signed by exact admitted principal controller".into(),
                );
            }
        } else {
            if d.parent_digest != Some(checked.path[index - 1]) {
                return Err("delegation parent digest mismatch".into());
            }
            check_parent_attenuation(&chain[index - 1].delegation, d)
                .map_err(|e| format!("delegation attenuation: {e:?}"))?;
        }
        let digest = authoring_delegation_digest(d)?;
        if checked.path[..index].contains(&digest) || cancelled.contains(&digest) {
            return Err("delegation repeated/cancelled/revoked".into());
        }
        let signature: [u8; 64] = signed
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| "delegation signature must contain 64 bytes")?;
        ed25519::verify_with_context(
            &d.grantor_controller,
            &digest,
            AUTHORING_DELEGATION_CONTEXT,
            &signature,
        )
        .map_err(|_| "delegation controller consent signature invalid")?;
        checked.path[index] = digest;
    }
    Ok(checked)
}
