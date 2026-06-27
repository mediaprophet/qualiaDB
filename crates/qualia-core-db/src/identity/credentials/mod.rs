//! W3C Verifiable Credentials — issue / hold / present / verify, signed with real
//! ML-DSA-65 (FIPS-204) via [`crate::crypto::fiduciary_crypto`].
//!
//! Consolidated from two stranded worktree branches (`0.0.19-g2-vc` = the runtime,
//! `0.0.19-g3-cbor-ld` = the [`codecs`]) into one module with a **single** `Credential`
//! type — the duplicate `Credential` each branch had defined is unified here.
//!
//! Verification is **fail-closed**: a missing proof, a tampered credential, or a failed
//! signature check all return an error, never a silent `false`/`true`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::crypto::fiduciary_crypto::{FiduciaryCrypto, MlDsaSignature};

pub mod codecs;

/// Error types for Verifiable Credentials.
#[derive(Debug, Clone, PartialEq)]
pub enum VcError {
    CryptoError(String),
    SerializationError(String),
    MissingProof,
    VerificationFailed,
    NotImplemented,
}

/// W3C Verifiable Credential Data Model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    pub id: String,
    #[serde(rename = "type")]
    pub types: Vec<String>,
    pub issuer: String,
    #[serde(rename = "issuanceDate")]
    pub issuance_date: String,
    #[serde(rename = "credentialSubject")]
    pub credential_subject: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<Proof>,
}

/// W3C Verifiable Presentation Data Model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Presentation {
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    #[serde(rename = "type")]
    pub types: Vec<String>,
    #[serde(rename = "verifiableCredential")]
    pub verifiable_credential: Vec<Credential>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<Proof>,
}

/// Proof object (Data Integrity Proof) carrying the ML-DSA signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proof {
    #[serde(rename = "type")]
    pub proof_type: String,
    pub created: String,
    #[serde(rename = "verificationMethod")]
    pub verification_method: String,
    #[serde(rename = "proofPurpose")]
    pub proof_purpose: String,
    #[serde(rename = "proofValue")]
    pub proof_value: MlDsaSignature,
}

/// Selective-disclosure interface (BBS-style); not yet implemented.
pub trait SelectiveDisclosure {
    fn generate_selective_presentation(
        &self,
        credential: &Credential,
        disclosed_fields: &[String],
    ) -> Result<Presentation, VcError>;
    fn verify_selective_presentation(&self, presentation: &Presentation) -> Result<bool, VcError>;
}

/// Zero-knowledge predicate-disclosure interface; not yet implemented.
pub trait ZkDisclosure {
    fn generate_zk_proof(
        &self,
        credential: &Credential,
        predicates: &[String],
    ) -> Result<Presentation, VcError>;
    fn verify_zk_proof(&self, presentation: &Presentation) -> Result<bool, VcError>;
}

/// Credential status (revocation/suspension) interface.
pub trait CredentialStatus {
    fn check_status(&self, credential_id: &str) -> Result<StatusResult, VcError>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatusResult {
    Valid,
    Revoked,
    Suspended,
}

/// Issues, holds, presents, and verifies Verifiable Credentials with ML-DSA-65 proofs.
pub struct VcRuntime {
    crypto: FiduciaryCrypto,
}

impl VcRuntime {
    pub fn new(crypto: FiduciaryCrypto) -> Self {
        Self { crypto }
    }

    /// Issue a Verifiable Credential: sign the proof-less credential and attach the proof.
    pub fn issue(&self, mut credential: Credential, key_id: Option<&str>) -> Result<Credential, VcError> {
        credential.proof = None;

        let serialized =
            serde_json::to_vec(&credential).map_err(|e| VcError::SerializationError(e.to_string()))?;

        let signature = self
            .crypto
            .sign(&serialized, key_id, "vc-dm".to_string(), "assertionMethod".to_string())
            .map_err(|e| VcError::CryptoError(format!("{:?}", e)))?;

        credential.proof = Some(Proof {
            proof_type: "MlDsaSignature2024".to_string(),
            created: "2026-06-24T00:00:00Z".to_string(),
            verification_method: key_id.unwrap_or("default").to_string(),
            proof_purpose: "assertionMethod".to_string(),
            proof_value: signature,
        });
        Ok(credential)
    }

    /// Hold a credential (secure storage is the caller's responsibility for now).
    pub fn hold(&self, _credential: &Credential) -> Result<(), VcError> {
        Ok(())
    }

    /// Present one or more credentials inside a signed Verifiable Presentation.
    pub fn present(&self, credentials: Vec<Credential>, key_id: Option<&str>) -> Result<Presentation, VcError> {
        let mut presentation = Presentation {
            context: vec!["https://www.w3.org/2018/credentials/v1".to_string()],
            types: vec!["VerifiablePresentation".to_string()],
            verifiable_credential: credentials,
            proof: None,
        };

        let serialized = serde_json::to_vec(&presentation)
            .map_err(|e| VcError::SerializationError(e.to_string()))?;

        let signature = self
            .crypto
            .sign(&serialized, key_id, "vc-dm".to_string(), "authentication".to_string())
            .map_err(|e| VcError::CryptoError(format!("{:?}", e)))?;

        presentation.proof = Some(Proof {
            proof_type: "MlDsaSignature2024".to_string(),
            created: "2026-06-24T00:00:00Z".to_string(),
            verification_method: key_id.unwrap_or("default").to_string(),
            proof_purpose: "authentication".to_string(),
            proof_value: signature,
        });
        Ok(presentation)
    }

    /// Verify a credential's proof. **Fail-closed**: missing proof or a failed check errors.
    pub fn verify_credential(&self, credential: &Credential) -> Result<bool, VcError> {
        let proof = credential.proof.as_ref().ok_or(VcError::MissingProof)?;

        let mut bare = credential.clone();
        bare.proof = None;
        let serialized =
            serde_json::to_vec(&bare).map_err(|e| VcError::SerializationError(e.to_string()))?;

        let ok = self
            .crypto
            .verify(
                &serialized,
                &proof.proof_value,
                Some(&proof.verification_method),
                "vc-dm".to_string(),
                proof.proof_purpose.clone(),
            )
            .map_err(|e| VcError::CryptoError(format!("{:?}", e)))?;

        if !ok {
            return Err(VcError::VerificationFailed);
        }
        Ok(true)
    }

    /// Verify a presentation's proof and every credential it carries (all fail-closed).
    pub fn verify_presentation(&self, presentation: &Presentation) -> Result<bool, VcError> {
        let proof = presentation.proof.as_ref().ok_or(VcError::MissingProof)?;

        let mut bare = presentation.clone();
        bare.proof = None;
        let serialized =
            serde_json::to_vec(&bare).map_err(|e| VcError::SerializationError(e.to_string()))?;

        let ok = self
            .crypto
            .verify(
                &serialized,
                &proof.proof_value,
                Some(&proof.verification_method),
                "vc-dm".to_string(),
                proof.proof_purpose.clone(),
            )
            .map_err(|e| VcError::CryptoError(format!("{:?}", e)))?;

        if !ok {
            return Err(VcError::VerificationFailed);
        }

        for cred in &presentation.verifiable_credential {
            if !self.verify_credential(cred)? {
                return Err(VcError::VerificationFailed);
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_credential() -> Credential {
        let mut subject = HashMap::new();
        subject.insert("id".to_string(), "did:example:ebfeb1f712ebc6f1c276e12ec21".to_string());
        subject.insert("degree".to_string(), "Bachelor of Science and Arts".to_string());
        Credential {
            context: vec!["https://www.w3.org/2018/credentials/v1".to_string()],
            id: "http://example.edu/credentials/3732".to_string(),
            types: vec!["VerifiableCredential".to_string(), "UniversityDegreeCredential".to_string()],
            issuer: "https://example.edu/issuers/565049".to_string(),
            issuance_date: "2010-01-01T19:23:24Z".to_string(),
            credential_subject: subject,
            proof: None,
        }
    }

    #[test]
    fn issue_verify_roundtrip() {
        let mut crypto = FiduciaryCrypto::new();
        crypto.generate_key("default".to_string()).unwrap();
        let runtime = VcRuntime::new(crypto);

        let issued = runtime.issue(test_credential(), Some("default")).expect("issue");
        assert!(issued.proof.is_some());
        assert_eq!(runtime.verify_credential(&issued), Ok(true));
    }

    #[test]
    fn tampered_credential_fails_closed() {
        let mut crypto = FiduciaryCrypto::new();
        crypto.generate_key("default".to_string()).unwrap();
        let runtime = VcRuntime::new(crypto);

        let mut issued = runtime.issue(test_credential(), Some("default")).unwrap();
        issued.issuer = "https://hacker.com".to_string();
        assert!(matches!(
            runtime.verify_credential(&issued),
            Err(VcError::VerificationFailed) | Err(VcError::CryptoError(_))
        ));
    }

    #[test]
    fn unsigned_credential_fails_closed() {
        let runtime = VcRuntime::new(FiduciaryCrypto::new());
        assert_eq!(runtime.verify_credential(&test_credential()), Err(VcError::MissingProof));
    }

    #[test]
    fn present_verify_roundtrip() {
        let mut crypto = FiduciaryCrypto::new();
        crypto.generate_key("default".to_string()).unwrap();
        let runtime = VcRuntime::new(crypto);

        let issued = runtime.issue(test_credential(), Some("default")).unwrap();
        let presentation = runtime.present(vec![issued], Some("default")).unwrap();
        assert_eq!(runtime.verify_presentation(&presentation), Ok(true));
    }
}
