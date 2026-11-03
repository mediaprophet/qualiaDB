//! Explicit actor login through real native key possession and signed admission.
use super::{configuration, registry, store};
use qualia_core_db::{
    key_vault::KeyVault,
    llm_agent::governed::ActorPolicyScope,
    poet_host::native_authority::{
        enrollment::NativeSigner,
        inference::{AuthenticatedInferenceAssociation, IdentifierSelection},
        NativeAuthority, SessionHandle,
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, sync::Arc};
use zeroize::Zeroizing;

/// Selection data is not a credential. Exact association must exist in the
/// configured signed admission, independent of subject kind or controller key.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedInferenceIdentifier {
    pub value: String,
    pub scheme: String,
    pub issuer: String,
    pub project_id: String,
    pub evidence_ref: String,
}
impl SelectedInferenceIdentifier {
    fn selection(&self) -> IdentifierSelection<'_> {
        IdentifierSelection {
            value: &self.value,
            scheme: &self.scheme,
            issuer: &self.issuer,
            project_id: &self.project_id,
            evidence_ref: &self.evidence_ref,
        }
    }
    fn validate(&self) -> Result<(), String> {
        if [
            &self.value,
            &self.scheme,
            &self.issuer,
            &self.project_id,
            &self.evidence_ref,
        ]
        .iter()
        .any(|s| s.is_empty() || s.len() > 4096)
        {
            return Err("Identifier selection exceeds bounds".into());
        }
        Ok(())
    }
}
/// Opaque authenticated client handle, not serializable or constructible from
/// selected identifier strings. The native owner retains all liveness checks.
pub struct InferenceActorSession {
    owner: Arc<NativeAuthority>,
    session: SessionHandle,
    selected: SelectedInferenceIdentifier,
    epoch: u64,
}
impl InferenceActorSession {
    pub(super) fn owner(&self) -> &Arc<NativeAuthority> {
        &self.owner
    }
    pub(super) fn bind(&self) -> Result<AuthenticatedInferenceAssociation, String> {
        self.current()?;
        self.owner
            .bind_inference_association(&self.session, self.selected.selection())
            .map_err(|e| format!("Inference actor association: {e:?}"))
    }
    /// Policy-authoring metadata only; copying it cannot create a live session.
    pub fn policy_scope(&self) -> Result<ActorPolicyScope, String> {
        let association = self.bind()?;
        self.owner
            .with_current_inference_association(&association, ActorPolicyScope::from_evidence)
            .map_err(|e| format!("Inference actor expired: {e:?}"))
    }
    pub fn logout(&self) -> Result<(), String> {
        self.current()?;
        let result = self.owner.disconnect(&self.session);
        registry::forget(&self.owner, &self.session);
        result
    }
    fn current(&self) -> Result<(), String> {
        if registry::epoch()? != self.epoch {
            Err("Inference actor requires fresh login after controller loss".into())
        } else {
            Ok(())
        }
    }
}
fn signer_path(label: &str) -> Result<PathBuf, String> {
    if label.trim().is_empty() || label.len() > 1024 {
        return Err("Explicit bounded signer label required".into());
    }
    let root = store::meta_root()?;
    let directory = store::child_directory(&root, "inference-actor-keys")?;
    Ok(directory.join(format!(
        "{}.sealed",
        hex::encode(Sha256::digest(label.as_bytes()))
    )))
}
fn wrapping(vault: &KeyVault, label: &str) -> Zeroizing<[u8; 32]> {
    Zeroizing::new(
        vault
            .derive_key(&format!(
                "qualia.inference.signer.wrap.v1:{}:{label}",
                label.len()
            ))
            .to_bytes(),
    )
}
fn open_signer(vault: &KeyVault, label: &str) -> Result<NativeSigner, String> {
    let path = signer_path(label)?;
    NativeSigner::open_sealed(&path, &wrapping(vault, label))
}
/// Explicit key creation, NOT identity enrollment or admission. Never called by
/// login or inference. Existing keys are not overwritten, even on a racing create.
pub fn create_inference_actor_signer(label: &str, budget: usize) -> Result<[u8; 32], String> {
    configuration::budget(budget)?;
    store::with_vault(|vault| {
        let path = signer_path(label)?;
        if path.exists() {
            return Err("Inference actor signer already exists".into());
        }
        let scratch = tempfile::Builder::new()
            .prefix("inference-signer-")
            .tempdir_in(path.parent().expect("child path"))
            .map_err(|e| e.to_string())?;
        let temporary = scratch.path().join("controller.sealed");
        let signer = NativeSigner::create_sealed(&temporary, &wrapping(vault, label))?;
        if std::fs::metadata(&temporary)
            .map_err(|e| e.to_string())?
            .len()
            != 76
        {
            return Err("Unexpected sealed signer size".into());
        }
        // Same-filesystem create-new link is atomic and never replaces a winner.
        // RAII removes the temporary name on success/error/unwind.
        std::fs::hard_link(&temporary, &path).map_err(|e| e.to_string())?;
        Ok(signer.public_key())
    })
}
/// Actual signed challenge, not identifier matching or a boolean authentication
/// assertion. No signer is created and no missing admission is synthesized.
pub fn login_inference_actor(
    subject: &str,
    selected: SelectedInferenceIdentifier,
    signer_label: &str,
    lifetime_seconds: u64,
    budget: usize,
) -> Result<InferenceActorSession, String> {
    configuration::budget(budget)?;
    selected.validate()?;
    if subject.is_empty() || subject.len() > 4096 {
        return Err("Actor subject exceeds bounds".into());
    }
    store::with_host(|host, _, vault| {
        let epoch = registry::epoch()?;
        let signer = open_signer(vault, signer_label)?;
        let challenge = host.owner.begin_session(subject, lifetime_seconds)?;
        let proof = signer.prove_session(&challenge)?;
        let session = host.owner.complete_session(challenge, &proof)?;
        let actor = InferenceActorSession {
            owner: Arc::clone(&host.owner),
            session,
            selected,
            epoch,
        };
        let association = match actor.bind() {
            Ok(a) => a,
            Err(error) => {
                let _ = host.owner.disconnect(&actor.session);
                return Err(error);
            }
        };
        if let Err(error) = registry::track(&host.owner, &actor.session, association) {
            let _ = host.owner.disconnect(&actor.session);
            return Err(error);
        }
        Ok(actor)
    })
}
