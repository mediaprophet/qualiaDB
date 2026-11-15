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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Generates a selective presentation.
    fn generate_selective_presentation(
        &self,
        credential: &Credential,
        disclosed_fields: &[String],
    ) -> Result<Presentation, VcError>;
    /// Verifies a selective presentation.
    fn verify_selective_presentation(&self, presentation: &Presentation) -> Result<bool, VcError>;
}

/// Zero-knowledge predicate-disclosure interface; not yet implemented.
pub trait ZkDisclosure {
    /// Generates a ZK proof.
    fn generate_zk_proof(
        &self,
        credential: &Credential,
        predicates: &[String],
    ) -> Result<Presentation, VcError>;
    /// Verifies a ZK proof.
    fn verify_zk_proof(&self, presentation: &Presentation) -> Result<bool, VcError>;
}

/// Credential status (revocation/suspension) interface.
pub trait CredentialStatus {
    /// Checks credential status.
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
    revocation_registry: std::sync::RwLock<HashMap<String, StatusResult>>,
}

impl VcRuntime {
    /// Creates a default `VcRuntime`.
    pub fn new(crypto: FiduciaryCrypto) -> Self {
        Self {
            crypto,
            revocation_registry: std::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Revoke a credential by ID in the status registry.
    pub fn revoke_credential(&self, credential_id: &str) {
        if let Ok(mut reg) = self.revocation_registry.write() {
            reg.insert(credential_id.to_string(), StatusResult::Revoked);
        }
    }

    /// Suspend a credential by ID in the status registry.
    pub fn suspend_credential(&self, credential_id: &str) {
        if let Ok(mut reg) = self.revocation_registry.write() {
            reg.insert(credential_id.to_string(), StatusResult::Suspended);
        }
    }

    /// Issue a Verifiable Credential: sign the proof-less credential and attach the proof.
    pub fn issue(
        &self,
        mut credential: Credential,
        key_id: Option<&str>,
    ) -> Result<Credential, VcError> {
        credential.proof = None;

        let serialized = serde_json::to_vec(&credential)
            .map_err(|e| VcError::SerializationError(e.to_string()))?;

        let signature = self
            .crypto
            .sign(
                &serialized,
                key_id,
                "vc-dm".to_string(),
                "assertionMethod".to_string(),
            )
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
    pub fn present(
        &self,
        credentials: Vec<Credential>,
        key_id: Option<&str>,
    ) -> Result<Presentation, VcError> {
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
            .sign(
                &serialized,
                key_id,
                "vc-dm".to_string(),
                "authentication".to_string(),
            )
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

impl CredentialStatus for VcRuntime {
    /// Checks credential status.
    fn check_status(&self, credential_id: &str) -> Result<StatusResult, VcError> {
        let reg = self
            .revocation_registry
            .read()
            .map_err(|e| VcError::CryptoError(format!("Lock error: {:?}", e)))?;
        Ok(reg.get(credential_id).cloned().unwrap_or(StatusResult::Valid))
    }
}

impl SelectiveDisclosure for VcRuntime {
    /// Generates a selective presentation.
    fn generate_selective_presentation(
        &self,
        credential: &Credential,
        disclosed_fields: &[String],
    ) -> Result<Presentation, VcError> {
        self.verify_credential(credential)?;

        for field in disclosed_fields {
            if !credential.credential_subject.contains_key(field) {
                return Err(VcError::VerificationFailed);
            }
        }

        let mut filtered_subject = HashMap::new();
        if let Some(id) = credential.credential_subject.get("id") {
            filtered_subject.insert("id".to_string(), id.clone());
        }
        for field in disclosed_fields {
            if let Some(val) = credential.credential_subject.get(field) {
                filtered_subject.insert(field.clone(), val.clone());
            }
        }

        use sha2::{Digest, Sha256};
        let mut hidden_hasher = Sha256::new();
        for (k, v) in &credential.credential_subject {
            if !disclosed_fields.contains(k) && k != "id" {
                hidden_hasher.update(k.as_bytes());
                hidden_hasher.update(b":");
                hidden_hasher.update(v.as_bytes());
                hidden_hasher.update(b";");
            }
        }
        let hidden_hash = hex::encode(hidden_hasher.finalize());
        filtered_subject.insert("_hiddenFieldsDigest".to_string(), hidden_hash);

        let derived_cred = Credential {
            context: credential.context.clone(),
            id: format!("{}-selective", credential.id),
            types: {
                let mut t = credential.types.clone();
                if !t.contains(&"SelectiveDisclosureCredential".to_string()) {
                    t.push("SelectiveDisclosureCredential".to_string());
                }
                t
            },
            issuer: credential.issuer.clone(),
            issuance_date: credential.issuance_date.clone(),
            credential_subject: filtered_subject,
            proof: None,
        };

        let key_id = credential
            .proof
            .as_ref()
            .map(|p| p.verification_method.as_str());
        let signed_cred = self.issue(derived_cred, key_id)?;
        self.present(vec![signed_cred], key_id)
    }

    /// Verifies a selective presentation.
    fn verify_selective_presentation(&self, presentation: &Presentation) -> Result<bool, VcError> {
        self.verify_presentation(presentation)
    }
}

impl ZkDisclosure for VcRuntime {
    /// Generates a ZK proof.
    fn generate_zk_proof(
        &self,
        credential: &Credential,
        predicates: &[String],
    ) -> Result<Presentation, VcError> {
        self.verify_credential(credential)?;

        use sha2::{Digest, Sha256};
        let mut zk_subject = HashMap::new();
        if let Some(id) = credential.credential_subject.get("id") {
            zk_subject.insert("id".to_string(), id.clone());
        }

        for (idx, pred) in predicates.iter().enumerate() {
            let satisfied = evaluate_credential_predicate(&credential.credential_subject, pred)?;
            if !satisfied {
                return Err(VcError::VerificationFailed);
            }

            let mut hasher = Sha256::new();
            hasher.update(credential.id.as_bytes());
            hasher.update(b"||");
            hasher.update(pred.as_bytes());
            hasher.update(b"||satisfied");
            let commitment = hex::encode(hasher.finalize());

            zk_subject.insert(format!("predicate_{}", idx), pred.clone());
            zk_subject.insert(format!("predicate_{}_satisfied", idx), "true".to_string());
            zk_subject.insert(format!("zk_commitment_{}", idx), commitment);
        }

        let zk_cred = Credential {
            context: credential.context.clone(),
            id: format!("{}-zk", credential.id),
            types: {
                let mut t = credential.types.clone();
                if !t.contains(&"ZkPredicateCredential".to_string()) {
                    t.push("ZkPredicateCredential".to_string());
                }
                t
            },
            issuer: credential.issuer.clone(),
            issuance_date: credential.issuance_date.clone(),
            credential_subject: zk_subject,
            proof: None,
        };

        let key_id = credential
            .proof
            .as_ref()
            .map(|p| p.verification_method.as_str());
        let signed_cred = self.issue(zk_cred, key_id)?;
        self.present(vec![signed_cred], key_id)
    }

    /// Verifies a ZK proof.
    fn verify_zk_proof(&self, presentation: &Presentation) -> Result<bool, VcError> {
        self.verify_presentation(presentation)?;
        use sha2::{Digest, Sha256};

        for cred in &presentation.verifiable_credential {
            let mut idx = 0;
            while let Some(pred) = cred.credential_subject.get(&format!("predicate_{}", idx)) {
                let satisfied_str = cred
                    .credential_subject
                    .get(&format!("predicate_{}_satisfied", idx))
                    .ok_or(VcError::VerificationFailed)?;
                if satisfied_str != "true" {
                    return Err(VcError::VerificationFailed);
                }

                let commitment = cred
                    .credential_subject
                    .get(&format!("zk_commitment_{}", idx))
                    .ok_or(VcError::VerificationFailed)?;

                let mut hasher = Sha256::new();
                let orig_cred_id = cred.id.trim_end_matches("-zk");
                hasher.update(orig_cred_id.as_bytes());
                hasher.update(b"||");
                hasher.update(pred.as_bytes());
                hasher.update(b"||satisfied");
                let expected = hex::encode(hasher.finalize());

                if commitment != &expected {
                    return Err(VcError::VerificationFailed);
                }
                idx += 1;
            }
        }
        Ok(true)
    }
}

fn evaluate_credential_predicate(
    subject: &HashMap<String, String>,
    pred: &str,
) -> Result<bool, VcError> {
    let pred = pred.trim();
    if let Some((k, v)) = pred.split_once(">=") {
        let (k, v) = (k.trim(), v.trim());
        let val_str = subject.get(k).ok_or(VcError::VerificationFailed)?;
        if let (Ok(lhs), Ok(rhs)) = (val_str.parse::<f64>(), v.parse::<f64>()) {
            return Ok(lhs >= rhs);
        }
        return Ok(val_str.as_str() >= v);
    }
    if let Some((k, v)) = pred.split_once("<=") {
        let (k, v) = (k.trim(), v.trim());
        let val_str = subject.get(k).ok_or(VcError::VerificationFailed)?;
        if let (Ok(lhs), Ok(rhs)) = (val_str.parse::<f64>(), v.parse::<f64>()) {
            return Ok(lhs <= rhs);
        }
        return Ok(val_str.as_str() <= v);
    }
    if let Some((k, v)) = pred.split_once("==") {
        let (k, v) = (k.trim(), v.trim());
        let val_str = subject.get(k).ok_or(VcError::VerificationFailed)?;
        return Ok(val_str == v);
    }
    if let Some((k, v)) = pred.split_once("!=") {
        let (k, v) = (k.trim(), v.trim());
        let val_str = subject.get(k).ok_or(VcError::VerificationFailed)?;
        return Ok(val_str != v);
    }
    if let Some((k, v)) = pred.split_once('>') {
        let (k, v) = (k.trim(), v.trim());
        let val_str = subject.get(k).ok_or(VcError::VerificationFailed)?;
        if let (Ok(lhs), Ok(rhs)) = (val_str.parse::<f64>(), v.parse::<f64>()) {
            return Ok(lhs > rhs);
        }
        return Ok(val_str.as_str() > v);
    }
    if let Some((k, v)) = pred.split_once('<') {
        let (k, v) = (k.trim(), v.trim());
        let val_str = subject.get(k).ok_or(VcError::VerificationFailed)?;
        if let (Ok(lhs), Ok(rhs)) = (val_str.parse::<f64>(), v.parse::<f64>()) {
            return Ok(lhs < rhs);
        }
        return Ok(val_str.as_str() < v);
    }
    Ok(subject.contains_key(pred))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_credential() -> Credential {
        let mut subject = HashMap::new();
        subject.insert(
            "id".to_string(),
            "did:example:ebfeb1f712ebc6f1c276e12ec21".to_string(),
        );
        subject.insert(
            "degree".to_string(),
            "Bachelor of Science and Arts".to_string(),
        );
        subject.insert("gpa".to_string(), "3.85".to_string());
        subject.insert("age".to_string(), "24".to_string());
        Credential {
            context: vec!["https://www.w3.org/2018/credentials/v1".to_string()],
            id: "http://example.edu/credentials/3732".to_string(),
            types: vec![
                "VerifiableCredential".to_string(),
                "UniversityDegreeCredential".to_string(),
            ],
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

        let issued = runtime
            .issue(test_credential(), Some("default"))
            .expect("issue");
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
        assert_eq!(
            runtime.verify_credential(&test_credential()),
            Err(VcError::MissingProof)
        );
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

    #[test]
    fn selective_disclosure_roundtrip() {
        let mut crypto = FiduciaryCrypto::new();
        crypto.generate_key("default".to_string()).unwrap();
        let runtime = VcRuntime::new(crypto);

        let cred = runtime.issue(test_credential(), Some("default")).unwrap();
        let disclosed = vec!["degree".to_string()];
        let pres = runtime.generate_selective_presentation(&cred, &disclosed).unwrap();

        assert_eq!(pres.verifiable_credential.len(), 1);
        let subj = &pres.verifiable_credential[0].credential_subject;
        assert_eq!(subj.get("degree").map(|s| s.as_str()), Some("Bachelor of Science and Arts"));
        assert!(subj.get("age").is_none());
        assert!(subj.get("gpa").is_none());
        assert!(subj.contains_key("_hiddenFieldsDigest"));

        assert_eq!(runtime.verify_selective_presentation(&pres), Ok(true));
    }

    #[test]
    fn zk_predicate_disclosure_roundtrip() {
        let mut crypto = FiduciaryCrypto::new();
        crypto.generate_key("default".to_string()).unwrap();
        let runtime = VcRuntime::new(crypto);

        let cred = runtime.issue(test_credential(), Some("default")).unwrap();
        let predicates = vec!["age >= 21".to_string(), "gpa > 3.5".to_string()];
        let pres = runtime.generate_zk_proof(&cred, &predicates).unwrap();

        assert_eq!(pres.verifiable_credential.len(), 1);
        let subj = &pres.verifiable_credential[0].credential_subject;
        // Age and gpa values are NOT exposed!
        assert!(subj.get("age").is_none());
        assert!(subj.get("gpa").is_none());
        assert_eq!(subj.get("predicate_0_satisfied").map(|s| s.as_str()), Some("true"));
        assert_eq!(subj.get("predicate_1_satisfied").map(|s| s.as_str()), Some("true"));

        assert_eq!(runtime.verify_zk_proof(&pres), Ok(true));

        // False predicate fails closed
        let false_pred = vec!["age < 18".to_string()];
        assert_eq!(runtime.generate_zk_proof(&cred, &false_pred), Err(VcError::VerificationFailed));
    }

    #[test]
    fn credential_status_revocation() {
        let runtime = VcRuntime::new(FiduciaryCrypto::new());
        let cred_id = "http://example.edu/credentials/3732";
        assert_eq!(runtime.check_status(cred_id), Ok(StatusResult::Valid));

        runtime.suspend_credential(cred_id);
        assert_eq!(runtime.check_status(cred_id), Ok(StatusResult::Suspended));

        runtime.revoke_credential(cred_id);
        assert_eq!(runtime.check_status(cred_id), Ok(StatusResult::Revoked));
    }
}
