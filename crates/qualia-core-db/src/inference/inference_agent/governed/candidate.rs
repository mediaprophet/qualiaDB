use super::PolicyError;
use crate::NQuin;
use etalon_inference_contract::CandidateOutput;
use sha2::{Digest, Sha256};

#[allow(dead_code)]
pub const TOKEN_EVIDENCE_BYTES: usize = 68;

/// Actual proposal data supplied by the numerical/caller adapter. This is not a
/// semantic classifier: `observation` must be the adapter's actual candidate
/// observation, not a fabricated "safe" fact. Its source is part of qualification.
#[derive(Clone, Copy, Debug)]
pub struct TokenProposal {
    pub position: u64,
    pub token: u32,
    pub logit: f32,
    pub observation: NQuin,
}
#[allow(dead_code)]
impl TokenProposal {
    pub(crate) fn encode(self) -> Result<[u8; TOKEN_EVIDENCE_BYTES], PolicyError> {
        let q = self.observation;
        if !self.logit.is_finite() || q.parity != q.subject ^ q.predicate ^ q.object ^ q.context {
            return Err(PolicyError::InvalidCandidate);
        }
        let mut out = [0; TOKEN_EVIDENCE_BYTES];
        out[..4].copy_from_slice(&1u32.to_le_bytes());
        out[4..12].copy_from_slice(&self.position.to_le_bytes());
        out[12..16].copy_from_slice(&self.token.to_le_bytes());
        out[16..20].copy_from_slice(&self.logit.to_bits().to_le_bytes());
        for (chunk, value) in out[20..].chunks_exact_mut(8).zip([
            q.subject,
            q.predicate,
            q.object,
            q.context,
            q.metadata,
            q.parity,
        ]) {
            chunk.copy_from_slice(&value.to_le_bytes());
        }
        Ok(out)
    }
    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, PolicyError> {
        if bytes.len() != TOKEN_EVIDENCE_BYTES || bytes[..4] != 1u32.to_le_bytes() {
            return Err(PolicyError::InvalidCandidate);
        }
        let u64_at =
            |i| u64::from_le_bytes(bytes[i..i + 8].try_into().expect("checked evidence length"));
        let proposal = Self {
            position: u64_at(4),
            token: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
            logit: f32::from_bits(u32::from_le_bytes(bytes[16..20].try_into().unwrap())),
            observation: NQuin {
                subject: u64_at(20),
                predicate: u64_at(28),
                object: u64_at(36),
                context: u64_at(44),
                metadata: u64_at(52),
                parity: u64_at(60),
            },
        };
        proposal.encode()?;
        Ok(proposal)
    }
    pub(crate) fn output(self) -> Result<CandidateOutput, PolicyError> {
        let bytes = self.encode()?;
        let mut h = Sha256::new();
        h.update(b"qualia:token-output:v1\0");
        h.update(&bytes[..20]);
        let output_digest = h.finalize().into();
        let mut h = Sha256::new();
        h.update(b"qualia:token-observation:v1\0");
        h.update(bytes);
        Ok(CandidateOutput {
            output_digest,
            observation_digest: h.finalize().into(),
            token_count: 1,
        })
    }
}

/// Admission values supplied by the authenticated host, then frozen for the
/// request. Model identity must identify actual immutable model bytes/revision.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct InferenceAdmission {
    pub request_digest: [u8; 32],
    pub model_digest: [u8; 32],
    pub model_generation: u64,
    pub device_id: u64,
    pub device_generation: u64,
    pub graph_scope: u64,
    pub first_position: u64,
    pub instrumentation: etalon_inference_contract::Instrumentation,
}
