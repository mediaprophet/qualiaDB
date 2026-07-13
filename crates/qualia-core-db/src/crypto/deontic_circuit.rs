// ── Deontic Access Circuit (arkworks) ──────────────────────────────────────
/// Groth16 circuit for zero-knowledge proof verification.

#[cfg(feature = "zk-culling")]
use ark_bls12_381::{Bls12_381, Fr};
#[cfg(feature = "zk-culling")]
use ark_groth16::{Groth16, Proof, ProvingKey, VerifyingKey};
#[cfg(feature = "zk-culling")]
use ark_relations::gr1cs::{
    ConstraintSynthesizer, ConstraintSystemRef, LinearCombination, SynthesisError, Variable,
};
#[cfg(feature = "zk-culling")]
use ark_serialize::CanonicalDeserialize;
#[cfg(feature = "zk-culling")]
use ark_snark::SNARK;

#[cfg(feature = "zk-culling")]
#[derive(Clone)]
pub struct DeonticAccessCircuit {
    pub user_did_commitment: Option<Fr>,
    pub role_id: Option<Fr>,
    pub action_permission: Option<Fr>,
    pub policy_root: Option<Fr>,
    pub temporal_constraint: Option<Fr>,
}

#[cfg(feature = "zk-culling")]
impl ConstraintSynthesizer<Fr> for DeonticAccessCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let did_var = cs.new_witness_variable(|| {
            self.user_did_commitment
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let role_var =
            cs.new_witness_variable(|| self.role_id.ok_or(SynthesisError::AssignmentMissing))?;
        let action_var = cs.new_witness_variable(|| {
            self.action_permission
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let root_var =
            cs.new_input_variable(|| self.policy_root.ok_or(SynthesisError::AssignmentMissing))?;
        let time_var = cs.new_input_variable(|| {
            self.temporal_constraint
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Access constraint: the prover's secret (did, role, action) must sum to the
        // public `policy_root` — i.e. they hold a credential authorised under the
        // committed policy. (Simplified additive commitment; Merkle-set membership is
        // a future hardening, not faked here.)
        cs.enforce_r1cs_constraint(
            || LinearCombination::from(did_var) + role_var + action_var,
            || LinearCombination::from(Variable::One),
            || LinearCombination::from(root_var),
        )?;
        // `temporal_constraint` is bound as a public input (the verifier checks the
        // proof was generated for this exact timestamp), but range enforcement
        // (notBefore <= t <= notAfter) is a documented PendingImplementation. We do
        // NOT fake it with a tautological `t * 1 = t` constraint that enforces nothing.
        let _ = time_var;
        Ok(())
    }
}

#[cfg(feature = "zk-culling")]
pub struct ZkAccessVerifier {
    verifying_key: Option<VerifyingKey<Bls12_381>>,
}

#[cfg(feature = "zk-culling")]
impl ZkAccessVerifier {
    pub fn new() -> Self {
        Self {
            verifying_key: None,
        }
    }

    pub fn verify_access(
        &self,
        proof: &[u8],
        public_inputs: &[Fr],
    ) -> Result<bool, crate::fiduciary_crypto::MlDsaError> {
        if let Some(ref vk) = self.verifying_key {
            let parsed_proof =
                Proof::<Bls12_381>::deserialize_uncompressed(&mut &*proof).map_err(|e| {
                    crate::fiduciary_crypto::MlDsaError::SignatureVerificationFailed(e.to_string())
                })?;
            Groth16::<Bls12_381>::verify(vk, public_inputs, &parsed_proof).map_err(|e| {
                crate::fiduciary_crypto::MlDsaError::SignatureVerificationFailed(e.to_string())
            })
        } else {
            Err(
                crate::fiduciary_crypto::MlDsaError::SignatureVerificationFailed(
                    "Verifying key not loaded".to_string(),
                ),
            )
        }
    }
}

#[cfg(feature = "zk-culling")]
impl Default for ZkAccessVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "zk-culling")]
pub fn generate_deontic_crs() -> Result<(ProvingKey<Bls12_381>, VerifyingKey<Bls12_381>), String> {
    let circuit = DeonticAccessCircuit {
        user_did_commitment: None,
        role_id: None,
        action_permission: None,
        policy_root: None,
        temporal_constraint: None,
    };
    let mut rng = crate::zk_proofs::zk_secure_rng();
    Groth16::<Bls12_381>::circuit_specific_setup(circuit, &mut rng)
        .map_err(|e| format!("Failed to generate parameters: {e}"))
}

#[cfg(test)]
#[cfg(feature = "zk-culling")]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_setup() {
        assert!(generate_deontic_crs().is_ok());
    }

    #[test]
    fn test_deontic_proof_roundtrip_and_soundness() {
        use ark_ff::UniformRand;
        let mut rng = crate::zk_proofs::zk_secure_rng();
        let (pk, vk) = generate_deontic_crs().unwrap();

        // A satisfying credential: did + role + action == policy_root.
        let did = Fr::rand(&mut rng);
        let role = Fr::rand(&mut rng);
        let action = Fr::rand(&mut rng);
        let policy_root = did + role + action;
        let temporal = Fr::from(1_700_000_000u64);

        let circuit = DeonticAccessCircuit {
            user_did_commitment: Some(did),
            role_id: Some(role),
            action_permission: Some(action),
            policy_root: Some(policy_root),
            temporal_constraint: Some(temporal),
        };
        let proof = Groth16::<Bls12_381>::prove(&pk, circuit, &mut rng).unwrap();

        // Valid: the public inputs match the proven relation.
        assert!(
            Groth16::<Bls12_381>::verify(&vk, &[policy_root, temporal], &proof).unwrap(),
            "a satisfying deontic access proof must verify"
        );

        // Soundness: a falsified policy_root must be rejected.
        assert!(
            !Groth16::<Bls12_381>::verify(&vk, &[policy_root + Fr::from(1u64), temporal], &proof)
                .unwrap(),
            "the proof must NOT verify against a falsified policy_root"
        );
    }
}
