//! Actual signed authority/policy harness for native publication boundaries.
//! The execution snapshot schedules cancellation; it does not qualify a model.
use super::super::{control::DecodeControl, governed as g};
use super::{gate::PublicationGate, receipt::NativeExecutionSnapshot};
use crate::gguf_bridge::QTensorEngine;
use ed25519_dalek::SigningKey;
use g::evidence as e;
use std::sync::atomic::{AtomicUsize, Ordering};

#[path = "../governed_cpu/test_actor_fixture.rs"]
mod actor_fixture;

struct Boundary {
    control: DecodeControl,
    calls: AtomicUsize,
    cancel_after_prepare: bool,
}
impl NativeExecutionSnapshot for Boundary {
    fn live(&self) -> Result<(), g::PolicyError> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 1 && self.cancel_after_prepare {
            self.control.cancel();
        }
        Ok(())
    }
    fn validate(
        &self,
        _: &QTensorEngine,
        _: &str,
        _: u64,
        _: &[u32],
    ) -> Result<(), g::PolicyError> {
        Ok(())
    }
}

pub(in crate::inference::inference_agent) fn with_gate(
    control: &DecodeControl,
    cancel_after_prepare: bool,
    body: impl FnOnce(&mut PublicationGate<'_, '_>),
) {
    let dir = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[61; 32]);
    let (owner, handle, scope) = actor_fixture::actor(&dir.path().join("actor"));
    let mut host = g::NativePolicyHost::open(
        &dir.path().join("policy"),
        &dir.path().join("anchor"),
        g::TrustedPolicyIdentity::from_host_configuration(
            key.verifying_key().to_bytes(),
            123,
            g::INFERENCE_POLICY_AUDIENCE,
            7,
        )
        .unwrap(),
        g::POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap();
    host.install_evidence(
        &key,
        g::PolicyRules {
            actors: g::PolicyList::from_vec(vec![scope]).unwrap(),
            model: g::DigestScope::Exact([22; 32]),
            request: g::DigestScope::Exact([11; 32]),
            graph_scopes: g::PolicyList::from_vec(vec![71]).unwrap(),
            program: g::PolicyList::from_vec(vec![g::ConstraintInstruction::MatchSubject {
                value: scope.subject,
            }])
            .unwrap(),
            norms: Default::default(),
            facts: Default::default(),
            epistemic: Default::default(),
            trace: Default::default(),
            required_claim: 0,
            temporal_invariant: 0,
            non_derogable: true,
        },
        e::ObservationRequirements {
            allowed_sources: 0x7f,
            allowed_artifact_origins: 7,
            max_payload_bytes: 256,
            allow_empty_artifact: false,
            score: e::ScoreRequirement::Optional,
            score_program: Default::default(),
        },
        1,
        4_000_000_000,
        host.status().generation,
        g::POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap();
    let mut bytes = [0; 256];
    let mut storage = e::EvidenceStorage::new(
        &mut bytes,
        e::EvidenceLimits {
            storage_bytes: 256 + std::mem::size_of::<e::EvidenceStorage<'_>>(),
            max_payload_bytes: 256,
            max_artifact_events: 4,
        },
    )
    .unwrap();
    let session = host
        .admit_evidence(
            &key,
            g::InferenceAdmission {
                request_digest: [11; 32],
                model_digest: [22; 32],
                model_generation: 4,
                device_id: 5,
                device_generation: 6,
                graph_scope: 71,
                first_position: 0,
                instrumentation: Default::default(),
            },
            g::POLICY_COLD_BUDGET_BYTES,
            owner.clone(),
            actor_fixture::bind(&owner, &handle),
            &mut storage,
        )
        .unwrap();
    let mut gate = PublicationGate::Governed {
        control: None,
        session,
        request: [11; 32],
        actor: scope.subject,
        graph: 71,
        provenance: [33; 32],
        generation: super::super::sticky_infer::current_residency_generation(),
        next_position: 0,
        token_sequence: 1,
        artifact_sequence: 1,
        speculative: false,
        execution: Box::new(Boundary {
            control: control.clone(),
            calls: AtomicUsize::new(0),
            cancel_after_prepare,
        }),
    };
    gate.bind_control(Some(control.clone()));
    body(&mut gate);
}

pub(in crate::inference::inference_agent) fn counts(
    gate: &PublicationGate<'_, '_>,
) -> (u64, u64, u64) {
    let PublicationGate::Governed {
        next_position,
        token_sequence,
        artifact_sequence,
        ..
    } = gate
    else {
        panic!("signed gate required")
    };
    (*next_position, *token_sequence, *artifact_sequence)
}
