use super::*;
use crate::{inference::inference_agent::governed::*, NQuin};
use ed25519_dalek::SigningKey;
use etalon_inference_contract::artifact::*;
use etalon_inference_contract::{Decision, ExecutionMode, Instrumentation, SessionIdentity};

fn identity() -> SessionIdentity {
    SessionIdentity {
        session_id: 1,
        session_generation: 2,
        request_id: 3,
        request_generation: 4,
        model_id: [5; 32],
        model_generation: 6,
        device_id: 7,
        device_generation: 8,
        policy_id: [9; 32],
        policy_generation: 1,
    }
}
fn actor() -> ActorPolicyScope {
    ActorPolicyScope {
        subject: 17,
        kind: qualia_cooperative_core::authoring::PrincipalKind::Human,
        controller: [1; 32],
        project_digest: [2; 32],
        selected_identifier_digest: [3; 32],
        association_digest: [4; 32],
        admission_digest: [5; 32],
        admission_sequence: 1,
        authority_generation: 1,
        valid_until: 1000,
    }
}
fn proposal(bytes: &[u8]) -> Proposal<'_> {
    Proposal {
        identity: identity(),
        request_digest: [8; 32],
        sequence: 1,
        source: CandidateSource::TextReplacement,
        artifact_origin: Some(ArtifactOrigin::RenderedCompletion),
        payload: Payload::Artifact {
            media: crate::inference::inference_agent::governed::evidence::ArtifactMedia::Utf8Text,
            bytes,
        },
        numerical: None,
        observation: NQuin {
            subject: 17,
            predicate: 23,
            object: 11,
            context: 29,
            metadata: 0,
            parity: 17 ^ 23 ^ 11 ^ 29,
        },
        provenance_digest: [9; 32],
    }
}
fn parent() -> ParentEventBinding {
    ParentEventBinding {
        domain: ParentEventDomain::Token,
        identity: identity(),
        sequence: 2,
        record_digest: [10; 32],
    }
}
fn bind(p: Proposal<'_>, parent: Option<ParentEventBinding>) -> BoundArtifact<'_> {
    BoundArtifact::bind(p, ExecutionMode::GovernedApplication, parent, 1024).unwrap()
}
fn mutate_identity(i: &mut SessionIdentity, n: usize) {
    match n {
        0 => i.session_id += 1,
        1 => i.session_generation += 1,
        2 => i.request_id += 1,
        3 => i.request_generation += 1,
        4 => i.model_id[0] ^= 1,
        5 => i.model_generation += 1,
        6 => i.device_id += 1,
        7 => i.device_generation += 1,
        8 => i.policy_id[0] ^= 1,
        9 => i.policy_generation += 1,
        _ => unreachable!(),
    }
}
#[test]
fn actual_session_owns_event_sequence_and_record_is_consumed_once() {
    let p = proposal(b"exact final bytes");
    let b = BoundArtifact::bind(p, ExecutionMode::StandaloneDeveloper, None, 100).unwrap();
    let mut session = DeveloperArtifactSession::new(
        ArtifactConfig {
            identity: p.identity,
            instrumentation: Instrumentation::default(),
            max_artifact_bytes: 100,
            allow_empty: false,
            max_events: 2,
        },
        0,
    )
    .unwrap();
    let key = session.propose(b.output(), 1).unwrap();
    b.match_session_key(&key).unwrap();
    let mut accepted = None;
    session.accept_into(key, 2, &mut accepted).unwrap();
    let owned = accepted.take().unwrap();
    assert_eq!(owned.into_key(), key);
    assert!(accepted.is_none());
    let second = session.propose(b.output(), 3).unwrap();
    assert_eq!(second.event_sequence, 2);
    assert!(b.match_session_key(&second).is_err());
    // Required caller behavior on mismatched pending key: terminal discard.
    session.cancel();
    assert_eq!(session.phase(), ArtifactPhase::Stopped);
    assert!(session.accept_into(second, 4, &mut accepted).is_err());
    assert!(accepted.is_none());
}
#[test]
fn every_session_key_field_is_exactly_matched() {
    let p = proposal(b"bytes");
    let b = bind(p, Some(parent()));
    let key = ArtifactKey {
        identity: p.identity,
        mode: ExecutionMode::GovernedApplication,
        event_sequence: 1,
        output: b.output(),
    };
    assert!(b.match_session_key(&key).is_ok());
    for n in 0..21 {
        let mut k = key;
        match n {
            0..=9 => mutate_identity(&mut k.identity, n),
            10 => k.mode = ExecutionMode::IntegratedGovernanceTest,
            11 => k.event_sequence += 1,
            12 => k.output.byte_len += 1,
            13 => k.output.content_digest[0] ^= 1,
            14 => k.output.provenance_digest[0] ^= 1,
            15 => k.output.observation_digest[0] ^= 1,
            16 => k.output.media = etalon_inference_contract::artifact::ArtifactMedia::Utf8Html,
            17 => k.output.source = ArtifactSource::GroundedReplacement,
            18 => k.output.parent = None,
            19 => k.output.parent.as_mut().unwrap().sequence += 1,
            20 => k.output.parent.as_mut().unwrap().domain = ParentEventDomain::Artifact,
            _ => unreachable!(),
        }
        assert!(b.match_session_key(&k).is_err(), "field {n}");
    }
}
#[test]
fn parent_lineage_mode_and_actual_origin_have_distinct_complete_digests() {
    let p = proposal(b"same bytes");
    let original = parent();
    let expected = bind(p, Some(original)).output().observation_digest;
    assert_ne!(bind(p, None).output().observation_digest, expected);
    for n in 0..13 {
        let mut changed = original;
        match n {
            0..=9 => mutate_identity(&mut changed.identity, n),
            10 => changed.sequence += 1,
            11 => changed.record_digest[0] ^= 1,
            12 => changed.domain = ParentEventDomain::Artifact,
            _ => unreachable!(),
        }
        assert_ne!(
            bind(p, Some(changed)).output().observation_digest,
            expected,
            "parent field {n}"
        );
    }
    let grounded = BoundArtifact::bind(
        Proposal {
            artifact_origin: Some(ArtifactOrigin::GroundedReplacement),
            ..p
        },
        ExecutionMode::GovernedApplication,
        Some(original),
        100,
    )
    .unwrap();
    assert_eq!(
        grounded.output().content_digest,
        bind(p, Some(original)).output().content_digest
    );
    assert_ne!(grounded.output().observation_digest, expected);
    for mode in [
        ExecutionMode::StandaloneDeveloper,
        ExecutionMode::SimulatedGovernanceTest,
        ExecutionMode::IntegratedGovernanceTest,
    ] {
        let other = BoundArtifact::bind(p, mode, Some(original), 100).unwrap();
        assert_ne!(other.output().observation_digest, expected);
    }
}
#[test]
fn g3_nonzero_generations_and_parent_validation_do_not_change_generic_api() {
    for field in 0..5 {
        let mut p = proposal(b"x");
        match field {
            0 => p.identity.session_generation = 0,
            1 => p.identity.request_generation = 0,
            2 => p.identity.model_generation = 0,
            3 => p.identity.device_generation = 0,
            4 => p.identity.policy_generation = 0,
            _ => unreachable!(),
        }
        // Published generic metadata deliberately keeps its prior convention.
        assert!(DeveloperArtifactSession::new(
            ArtifactConfig {
                identity: p.identity,
                instrumentation: Instrumentation::default(),
                max_artifact_bytes: 1,
                allow_empty: false,
                max_events: 1
            },
            0
        )
        .is_ok());
        assert!(BoundArtifact::bind(p, ExecutionMode::StandaloneDeveloper, None, 1).is_err());
        let mut link = parent();
        link.identity = p.identity;
        assert!(BoundArtifact::bind(
            proposal(b"x"),
            ExecutionMode::GovernedApplication,
            Some(link),
            1
        )
        .is_err());
    }
    for bad in [
        ParentEventBinding {
            sequence: 0,
            ..parent()
        },
        ParentEventBinding {
            record_digest: [0; 32],
            ..parent()
        },
    ] {
        assert!(BoundArtifact::bind(
            proposal(b"x"),
            ExecutionMode::GovernedApplication,
            Some(bad),
            1
        )
        .is_err());
    }
}
#[test]
fn media_and_origin_are_orthogonal_with_explicit_compatible_matrix() {
    use crate::inference::inference_agent::governed::evidence::ArtifactMedia as M;
    for (media, source, bytes) in [
        (
            M::Utf8Text,
            CandidateSource::TextReplacement,
            b"text".as_slice(),
        ),
        (
            M::Utf8Html,
            CandidateSource::HtmlReplacement,
            b"<p>x</p>".as_slice(),
        ),
        (
            M::CanonicalQuins,
            CandidateSource::SemanticArtifact,
            [0u8; 48].as_slice(),
        ),
    ] {
        let mut p = proposal(bytes);
        p.source = source;
        p.payload = Payload::Artifact { media, bytes };
        for origin in [
            ArtifactOrigin::RenderedCompletion,
            ArtifactOrigin::GroundedReplacement,
            ArtifactOrigin::SemanticProjection,
        ] {
            p.artifact_origin = Some(origin);
            let expected =
                (media == M::CanonicalQuins) == (origin == ArtifactOrigin::SemanticProjection);
            let bound = BoundArtifact::bind(p, ExecutionMode::GovernedApplication, None, 100);
            assert_eq!(bound.is_ok(), expected);
            if let Ok(b) = bound {
                assert_eq!(b.media(), media);
                assert_eq!(b.bytes(), bytes);
                assert_eq!(
                    b.output().source,
                    match origin {
                        ArtifactOrigin::RenderedCompletion => ArtifactSource::RenderedCompletion,
                        ArtifactOrigin::GroundedReplacement => ArtifactSource::GroundedReplacement,
                        ArtifactOrigin::SemanticProjection => ArtifactSource::SemanticProjection,
                    }
                );
            }
        }
        p.artifact_origin = None;
        for mode in [
            ExecutionMode::StandaloneDeveloper,
            ExecutionMode::GovernedApplication,
            ExecutionMode::IntegratedGovernanceTest,
            ExecutionMode::SimulatedGovernanceTest,
        ] {
            assert!(BoundArtifact::bind(p, mode, None, 100).is_err());
        }
    }
    let mut token = proposal(b"x");
    token.source = CandidateSource::ModelToken;
    token.artifact_origin = None;
    token.payload = Payload::Token {
        token: 7,
        position: 1,
        piece: b"x",
    };
    assert!(BoundArtifact::bind(token, ExecutionMode::StandaloneDeveloper, None, 1).is_err());
}
#[test]
fn actual_vm_and_graph_evaluation_are_preserved_with_zero_allocation() {
    use crate::specialized_libs::computational_geometry::allocation_counter::{
        assert_zero_alloc, AllocGuard,
    };
    let control = AllocGuard::begin("positive control", true);
    let allocated = vec![1u8; 32];
    std::hint::black_box(&allocated);
    assert!(control.check().is_err());
    drop(allocated);
    let mut seed = zeroize::Zeroizing::new([0; 32]);
    getrandom::fill(&mut *seed).unwrap();
    let key = SigningKey::from_bytes(&seed);
    let d = EvidencePolicy {
        version: 3,
        audience: AUDIENCE_V3,
        base: PolicyDocument {
            version: POLICY_VERSION,
            audience: INFERENCE_POLICY_AUDIENCE,
            principal: 17,
            controller_generation: 3,
            policy_generation: 1,
            not_before: 10,
            expires_at: 100,
            rules: PolicyRules {
                actors: PolicyList::from_vec(vec![actor()]).unwrap(),
                model: DigestScope::Any,
                request: DigestScope::Any,
                graph_scopes: PolicyList::from_vec(vec![29]).unwrap(),
                program: PolicyList::from_vec(vec![
                    ConstraintInstruction::MatchObject { value: 11 },
                    ConstraintInstruction::HaltIfFalse,
                ])
                .unwrap(),
                norms: Default::default(),
                facts: Default::default(),
                epistemic: Default::default(),
                trace: Default::default(),
                required_claim: 0,
                temporal_invariant: 0,
                non_derogable: false,
            },
        },
        observations: ObservationRequirements {
            allowed_sources: 0x30,
            allowed_artifact_origins: ArtifactOrigin::RenderedCompletion.bit(),
            max_payload_bytes: 100,
            allow_empty_artifact: false,
            score: ScoreRequirement::Optional,
            score_program: Default::default(),
        },
    };
    let mut p = proposal(b"actual final bytes");
    p.identity.policy_id = policy_digest_v3(&d);
    let signed = SignedEvidencePolicy::sign(d, &key).unwrap();
    let trust = TrustedPolicyIdentity::from_host_configuration(
        key.verifying_key().to_bytes(),
        17,
        AUDIENCE_V3,
        3,
    )
    .unwrap();
    let verified = signed.verify(&trust, 20).unwrap();
    assert_zero_alloc("artifact mapping and actual VM", || {
        let b = bind(p, None);
        assert_eq!(b.evaluate(&verified, actor(), 20), Ok(Decision::Allow));
        let grounded = bind(
            Proposal {
                artifact_origin: Some(ArtifactOrigin::GroundedReplacement),
                ..p
            },
            None,
        );
        assert_eq!(grounded.output().content_digest, b.output().content_digest);
        assert_ne!(
            grounded.output().observation_digest,
            b.output().observation_digest
        );
        assert_eq!(
            grounded.evaluate(&verified, actor(), 20),
            Err(PolicyError::Scope)
        );

        p.observation.object = 12;
        p.observation.parity = 17 ^ 23 ^ 12 ^ 29;
        let denied = bind(p, None);
        assert_eq!(denied.evaluate(&verified, actor(), 20), Ok(Decision::Deny));
        assert!(BoundArtifact::bind(p, ExecutionMode::GovernedApplication, None, 0).is_err());
    });
}

#[test]
fn genuine_developer_api_remains_policy_free_outside_policy_bound_bridge() {
    let mut i = identity();
    i.policy_id = [0; 32];
    i.policy_generation = 0;
    let mut session = DeveloperArtifactSession::new(
        ArtifactConfig {
            identity: i,
            instrumentation: Instrumentation::default(),
            max_artifact_bytes: 8,
            allow_empty: false,
            max_events: 1,
        },
        0,
    )
    .unwrap();
    let output = ArtifactOutput {
        media: etalon_inference_contract::artifact::ArtifactMedia::Utf8Text,
        source: ArtifactSource::DeveloperArtifact,
        byte_len: 1,
        content_digest: [1; 32],
        provenance_digest: [2; 32],
        observation_digest: [3; 32],
        parent: None,
    };
    let key = session.propose(output, 1).unwrap();
    let mut slot = None;
    session.accept_into(key, 2, &mut slot).unwrap();
    assert_eq!(slot.take().unwrap().into_key(), key);
    assert!(BoundArtifact::bind(
        Proposal {
            identity: i,
            ..proposal(b"x")
        },
        ExecutionMode::StandaloneDeveloper,
        None,
        1
    )
    .is_err());
}
