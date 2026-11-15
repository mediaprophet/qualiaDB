use etalon_inference_contract::artifact::{ArtifactSource, ParentEventBinding, ParentEventDomain};
use etalon_inference_contract::{ExecutionMode, SessionIdentity};
use sha2::{Digest, Sha256};

pub(super) fn content(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"qualia:g3:artifact-content:v1\0");
    h.update((bytes.len() as u64).to_le_bytes());
    h.update(bytes);
    h.finalize().into()
}
fn identity(h: &mut Sha256, i: SessionIdentity) {
    for v in [
        i.session_id,
        i.session_generation,
        i.request_id,
        i.request_generation,
    ] {
        h.update(v.to_le_bytes());
    }
    h.update(i.model_id);
    for v in [i.model_generation, i.device_id, i.device_generation] {
        h.update(v.to_le_bytes());
    }
    h.update(i.policy_id);
    h.update(i.policy_generation.to_le_bytes());
}
pub(super) fn complete(
    v3_digest: [u8; 32],
    mode: ExecutionMode,
    origin: ArtifactSource,
    parent: Option<ParentEventBinding>,
) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"qualia:g3:artifact-complete-evidence:v1\0");
    h.update(v3_digest);
    h.update([match mode {
        ExecutionMode::StandaloneDeveloper => 1,
        ExecutionMode::SimulatedGovernanceTest => 2,
        ExecutionMode::GovernedApplication => 3,
        ExecutionMode::IntegratedGovernanceTest => 4,
    }]);
    h.update([match origin {
        ArtifactSource::RenderedCompletion => 1,
        ArtifactSource::GroundedReplacement => 2,
        ArtifactSource::SemanticProjection => 3,
        ArtifactSource::DeveloperArtifact => 4,
    }]);
    match parent {
        None => h.update([0]),
        Some(p) => {
            h.update([
                1,
                match p.domain {
                    ParentEventDomain::Token => 1,
                    ParentEventDomain::Artifact => 2,
                },
            ]);
            identity(&mut h, p.identity);
            h.update(p.sequence.to_le_bytes());
            h.update(p.record_digest);
        }
    }
    h.finalize().into()
}
