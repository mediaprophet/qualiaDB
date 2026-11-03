use super::*;
use crate::{inference::inference_agent::governed::*, NQuin};
use ed25519_dalek::SigningKey;
use etalon_inference_contract::{Decision, SessionIdentity};
fn key() -> SigningKey {
    let mut seed = zeroize::Zeroizing::new([0; 32]);
    getrandom::fill(&mut *seed).unwrap();
    SigningKey::from_bytes(&seed)
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
fn document() -> EvidencePolicy {
    EvidencePolicy {
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
                program: Default::default(),
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
            allowed_sources: 0x7f,
            allowed_artifact_origins: 0x07,
            max_payload_bytes: 1024,
            allow_empty_artifact: false,
            score: ScoreRequirement::Optional,
            score_program: Default::default(),
        },
    }
}
fn trust(k: &SigningKey) -> TrustedPolicyIdentity {
    TrustedPolicyIdentity::from_host_configuration(k.verifying_key().to_bytes(), 17, AUDIENCE_V3, 3)
        .unwrap()
}
fn proposal<'a>(d: &EvidencePolicy, bytes: &'a [u8]) -> Proposal<'a> {
    Proposal {
        identity: SessionIdentity {
            session_id: 1,
            session_generation: 2,
            request_id: 3,
            request_generation: 4,
            model_id: [5; 32],
            model_generation: 6,
            device_id: 7,
            device_generation: 8,
            policy_id: policy_digest_v3(d),
            policy_generation: 1,
        },
        request_digest: [8; 32],
        sequence: 1,
        source: CandidateSource::ModelToken,
        artifact_origin: None,
        payload: Payload::Token {
            token: 12,
            position: 5,
            piece: bytes,
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
#[test]
fn missing_score_is_absence_and_present_score_uses_real_vm() {
    let k = key();
    let mut d = document();
    d.observations.score = ScoreRequirement::Required(ScoreMeaning::Probability);
    d.observations.score_program = PolicyList::from_vec(vec![
        ConstraintInstruction::GreaterThan {
            vector: 2,
            value: 0.5,
        },
        ConstraintInstruction::HaltIfFalse,
    ])
    .unwrap();
    let signed = SignedEvidencePolicy::sign(d.clone(), &k).unwrap();
    let verified = signed.verify(&trust(&k), 20).unwrap();
    let mut p = proposal(&d, b"x");
    assert_eq!(
        evaluate_constraints(&verified, &BoundProposal::bind(p, 1).unwrap(), actor(), 20),
        Err(PolicyError::InvalidCandidate)
    );
    p.numerical = Some(NumericalObservation {
        meaning: ScoreMeaning::Probability,
        value: 0.75,
    });
    assert_eq!(
        evaluate_constraints(&verified, &BoundProposal::bind(p, 1).unwrap(), actor(), 20),
        Ok(Decision::Allow)
    );
    p.numerical.as_mut().unwrap().value = 0.25;
    assert_eq!(
        evaluate_constraints(&verified, &BoundProposal::bind(p, 1).unwrap(), actor(), 20),
        Ok(Decision::Deny)
    );
    p.numerical = Some(NumericalObservation {
        meaning: ScoreMeaning::Logit,
        value: 0.75,
    });
    assert_eq!(
        evaluate_constraints(&verified, &BoundProposal::bind(p, 1).unwrap(), actor(), 20),
        Err(PolicyError::InvalidCandidate)
    );
}
#[test]
fn source_only_policy_is_explicit_and_does_not_skip_original_vm_or_guard() {
    let k = key();
    let mut d = document();
    d.observations.allowed_sources = CandidateSource::GraphForcedToken.bit();
    d.observations.allowed_artifact_origins = 0;
    d.base.rules.program = PolicyList::from_vec(vec![
        ConstraintInstruction::MatchObject { value: 11 },
        ConstraintInstruction::HaltIfFalse,
    ])
    .unwrap();
    let signed = SignedEvidencePolicy::sign(d.clone(), &k).unwrap();
    let verified = signed.verify(&trust(&k), 20).unwrap();
    let mut p = proposal(&d, b"fact");
    assert_eq!(
        evaluate_constraints(&verified, &BoundProposal::bind(p, 4).unwrap(), actor(), 20),
        Err(PolicyError::Scope)
    );
    p.source = CandidateSource::GraphForcedToken;
    assert_eq!(
        evaluate_constraints(&verified, &BoundProposal::bind(p, 4).unwrap(), actor(), 20),
        Ok(Decision::Allow)
    );
    p.observation.object = 12;
    p.observation.parity = 17 ^ 23 ^ 12 ^ 29;
    assert_eq!(
        evaluate_constraints(&verified, &BoundProposal::bind(p, 4).unwrap(), actor(), 20),
        Ok(Decision::Deny)
    );
    d.base.rules.program = Default::default();
    d.base.rules.required_claim = 12345;
    let signed = SignedEvidencePolicy::sign(d.clone(), &k).unwrap();
    let verified = signed.verify(&trust(&k), 20).unwrap();
    p.identity.policy_id = policy_digest_v3(&d);
    assert!(matches!(
        evaluate_constraints(&verified, &BoundProposal::bind(p, 4).unwrap(), actor(), 20),
        Ok(Decision::Steer { .. }) | Ok(Decision::Deny)
    ));
}
#[test]
fn full_identity_and_payload_domains_are_bound() {
    let d = document();
    let p = proposal(&d, b"abc");
    let digest = BoundProposal::bind(p, 3).unwrap().digest();
    for field in 0..22 {
        let mut q = p;
        match field {
            0 => q.identity.session_id += 1,
            1 => q.identity.session_generation += 1,
            2 => q.identity.request_id += 1,
            3 => q.identity.request_generation += 1,
            4 => q.identity.model_id[0] ^= 1,
            5 => q.identity.model_generation += 1,
            6 => q.identity.device_id += 1,
            7 => q.identity.device_generation += 1,
            8 => q.identity.policy_id[0] ^= 1,
            9 => q.identity.policy_generation += 1,
            10 => q.sequence += 1,
            11 => q.request_digest[0] ^= 1,
            12 => q.provenance_digest[0] ^= 1,
            13 => q.source = CandidateSource::TopologyToken,
            14 => {
                q.payload = Payload::Token {
                    token: 13,
                    position: 5,
                    piece: b"abc",
                }
            }
            15 => {
                q.payload = Payload::Token {
                    token: 12,
                    position: 6,
                    piece: b"abc",
                }
            }
            16 => {
                q.payload = Payload::Token {
                    token: 12,
                    position: 5,
                    piece: b"abd",
                }
            }
            17 => {
                q.payload = Payload::Token {
                    token: 12,
                    position: 5,
                    piece: b"ab",
                }
            }
            18 => {
                q.numerical = Some(NumericalObservation {
                    meaning: ScoreMeaning::Logit,
                    value: 0.0,
                })
            }
            19 => {
                q.numerical = Some(NumericalObservation {
                    meaning: ScoreMeaning::Logit,
                    value: -0.0,
                })
            }
            20 => {
                q.numerical = Some(NumericalObservation {
                    meaning: ScoreMeaning::Probability,
                    value: 0.0,
                })
            }
            21 => {
                q.source = CandidateSource::TextReplacement;
                q.artifact_origin = Some(ArtifactOrigin::RenderedCompletion);
                q.payload = Payload::Artifact {
                    media: ArtifactMedia::Utf8Text,
                    bytes: b"abc",
                };
            }
            _ => unreachable!(),
        }
        assert_ne!(
            BoundProposal::bind(q, 3).unwrap().digest(),
            digest,
            "field {field}"
        );
    }
    let mut q = p;
    q.source = CandidateSource::TextReplacement;
    q.artifact_origin = Some(ArtifactOrigin::RenderedCompletion);
    assert!(BoundProposal::bind(q, 3).is_err());
    q.payload = Payload::Artifact {
        media: ArtifactMedia::Utf8Html,
        bytes: b"abc",
    };
    assert!(BoundProposal::bind(q, 3).is_err());
    let mut score = p;
    score.numerical = Some(NumericalObservation {
        meaning: ScoreMeaning::Logit,
        value: 0.0,
    });
    let plus = BoundProposal::bind(score, 3).unwrap().digest();
    score.numerical.as_mut().unwrap().value = -0.0;
    assert_ne!(plus, BoundProposal::bind(score, 3).unwrap().digest());
}
#[test]
fn all_observation_fields_and_parity_are_bound() {
    let d = document();
    let p = proposal(&d, b"x");
    let original = BoundProposal::bind(p, 1).unwrap().digest();
    for field in 0..5 {
        let mut q = p;
        match field {
            0 => q.observation.subject ^= 1,
            1 => q.observation.predicate ^= 1,
            2 => q.observation.object ^= 1,
            3 => q.observation.context ^= 1,
            4 => q.observation.metadata ^= 1,
            _ => unreachable!(),
        };
        q.observation.parity = q.observation.subject
            ^ q.observation.predicate
            ^ q.observation.object
            ^ q.observation.context;
        assert_ne!(original, BoundProposal::bind(q, 1).unwrap().digest());
    }
    let mut q = p;
    q.observation.parity ^= 1;
    assert!(BoundProposal::bind(q, 1).is_err());
}
#[test]
fn score_validation_and_empty_artifacts_are_explicit() {
    let k = key();
    let d = document();
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.1, 1.1] {
        let mut p = proposal(&d, b"x");
        p.numerical = Some(NumericalObservation {
            meaning: ScoreMeaning::Probability,
            value,
        });
        assert!(BoundProposal::bind(p, 1).is_err());
    }
    for value in [-1000.0, 1000.0] {
        let mut p = proposal(&d, b"x");
        p.numerical = Some(NumericalObservation {
            meaning: ScoreMeaning::Logit,
            value,
        });
        assert!(BoundProposal::bind(p, 1).is_ok());
    }
    let mut p = proposal(&d, b"");
    assert!(BoundProposal::bind(p, 0).is_ok());
    p.source = CandidateSource::TextReplacement;
    p.artifact_origin = Some(ArtifactOrigin::RenderedCompletion);
    p.payload = Payload::Artifact {
        media: ArtifactMedia::Utf8Text,
        bytes: b"",
    };
    let signed = SignedEvidencePolicy::sign(d.clone(), &k).unwrap();
    let v = signed.verify(&trust(&k), 20).unwrap();
    assert_eq!(
        evaluate_constraints(&v, &BoundProposal::bind(p, 0).unwrap(), actor(), 20),
        Err(PolicyError::InvalidCandidate)
    );
    let mut allow = d;
    allow.observations.allow_empty_artifact = true;
    p.identity.policy_id = policy_digest_v3(&allow);
    let signed = SignedEvidencePolicy::sign(allow, &k).unwrap();
    let v = signed.verify(&trust(&k), 20).unwrap();
    assert_eq!(
        evaluate_constraints(&v, &BoundProposal::bind(p, 0).unwrap(), actor(), 20),
        Ok(Decision::Allow)
    );
}
#[test]
fn replacement_media_encoding_and_budget_refuse_before_evaluation() {
    let d = document();
    let mut p = proposal(&d, b"x");
    assert!(BoundProposal::bind(p, 0).is_err());
    p.source = CandidateSource::TextReplacement;
    p.artifact_origin = Some(ArtifactOrigin::RenderedCompletion);
    p.payload = Payload::Artifact {
        media: ArtifactMedia::Utf8Text,
        bytes: &[255],
    };
    assert!(BoundProposal::bind(p, 1).is_err());
    p.source = CandidateSource::SemanticArtifact;
    p.artifact_origin = Some(ArtifactOrigin::SemanticProjection);
    p.payload = Payload::Artifact {
        media: ArtifactMedia::CanonicalQuins,
        bytes: &[0; 48],
    };
    assert!(BoundProposal::bind(p, 48).is_ok());
    let mut invalid = [0u8; 48];
    invalid[40] = 1;
    p.payload = Payload::Artifact {
        media: ArtifactMedia::CanonicalQuins,
        bytes: &invalid,
    };
    assert!(BoundProposal::bind(p, 48).is_err());
    p.payload = Payload::Artifact {
        media: ArtifactMedia::CanonicalQuins,
        bytes: &[0; 47],
    };
    assert!(BoundProposal::bind(p, 48).is_err());
}
#[test]
fn requirements_and_v3_signature_cover_every_new_field() {
    let k = key();
    let d = document();
    let signed = SignedEvidencePolicy::sign(d.clone(), &k).unwrap();
    assert!(signed.verify(&trust(&k), 20).is_ok());
    assert!(signed.verify(&trust(&key()), 20).is_err());
    assert!(signed.verify(&trust(&k), 100).is_err());
    for field in 0..9 {
        let mut x = signed.clone();
        match field {
            0 => x.document.version += 1,
            1 => x.document.audience ^= 1,
            2 => x.document.base.rules.required_claim ^= 1,
            3 => x.document.observations.allowed_sources ^= 1,
            4 => x.document.observations.max_payload_bytes += 1,
            5 => x.document.observations.allow_empty_artifact = true,
            6 => x.document.observations.score = ScoreRequirement::Required(ScoreMeaning::Logit),
            7 => {
                x.document.observations.score =
                    ScoreRequirement::Required(ScoreMeaning::Probability);
                x.document.observations.score_program =
                    PolicyList::from_vec(vec![ConstraintInstruction::MatchObject { value: 0 }])
                        .unwrap();
            }
            8 => x.document.base.rules.non_derogable = true,
            _ => unreachable!(),
        };
        assert!(x.verify(&trust(&k), 20).is_err(), "field {field}");
    }
    for mask in [0, 128, u16::MAX] {
        let mut x = d.clone();
        x.observations.allowed_sources = mask;
        assert!(SignedEvidencePolicy::sign(x, &k).is_err());
    }
    let mut x = d;
    x.observations.score_program =
        PolicyList::from_vec(vec![ConstraintInstruction::HaltIfFalse]).unwrap();
    assert!(SignedEvidencePolicy::sign(x, &k).is_err());
}
#[test]
fn v2_signature_is_not_reinterpreted_and_v2_still_verifies() {
    let k = key();
    let d = document();
    let v2 = SignedPolicy::sign(d.base.clone(), &k).unwrap();
    let t = TrustedPolicyIdentity::from_host_configuration(
        k.verifying_key().to_bytes(),
        17,
        INFERENCE_POLICY_AUDIENCE,
        3,
    )
    .unwrap();
    assert!(v2.verify(&t, 20).is_ok());
    let forged = SignedEvidencePolicy {
        document: d,
        signature: v2.signature,
    };
    assert!(forged.verify(&trust(&k), 20).is_err());
    assert!(
        serde_json::from_slice::<SignedEvidencePolicy>(&serde_json::to_vec(&v2).unwrap()).is_err()
    );
}
#[test]
fn hot_binding_and_actual_vm_checks_allocate_zero_with_positive_control() {
    use crate::specialized_libs::computational_geometry::allocation_counter::{
        assert_zero_alloc, AllocGuard,
    };
    let guard = AllocGuard::begin("positive control", true);
    let allocation = vec![7u8; 32];
    std::hint::black_box(&allocation);
    assert!(guard.check().is_err());
    drop(allocation);
    let k = key();
    let d = document();
    let p = proposal(&d, b"actual bytes");
    let signed = SignedEvidencePolicy::sign(d, &k).unwrap();
    let v = signed.verify(&trust(&k), 20).unwrap();
    assert_zero_alloc("candidate binding and VM", || {
        let b = BoundProposal::bind(p, 64).unwrap();
        std::hint::black_box(b.digest());
        assert_eq!(
            evaluate_constraints(&v, &b, actor(), 20),
            Ok(Decision::Allow)
        );
        assert!(BoundProposal::bind(p, 0).is_err());
    });
}

#[test]
fn score_projection_rejects_every_non_numeric_opcode_and_other_vector() {
    use ConstraintInstruction::*;
    let mut d = document();
    d.observations.score = ScoreRequirement::Required(ScoreMeaning::Logit);
    for instruction in [
        MatchSubject { value: 17 },
        MatchPredicate { value: 23 },
        MatchObject { value: 11 },
        MetadataMask { mask: 0 },
        Bind {
            vector: 2,
            register: 0,
        },
        MatchRegister {
            vector: 2,
            register: 0,
        },
        Confidence { minimum: 0.5 },
        HaltIfFalse,
    ] {
        d.observations.score_program = PolicyList::from_vec(vec![instruction]).unwrap();
        assert!(d.observations.validate().is_err(), "{instruction:?}");
    }
    for vector in 0..=u8::MAX {
        if vector == 2 {
            continue;
        }
        for instruction in [
            LessThan { vector, value: 0.5 },
            GreaterThan { vector, value: 0.5 },
            LessOrEqual { vector, value: 0.5 },
            GreaterOrEqual { vector, value: 0.5 },
        ] {
            d.observations.score_program = PolicyList::from_vec(vec![instruction]).unwrap();
            assert!(d.observations.validate().is_err(), "{instruction:?}");
        }
    }
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for instruction in [
            LessThan { vector: 2, value },
            GreaterThan { vector: 2, value },
            LessOrEqual { vector: 2, value },
            GreaterOrEqual { vector: 2, value },
        ] {
            d.observations.score_program = PolicyList::from_vec(vec![instruction]).unwrap();
            assert!(d.observations.validate().is_err(), "{instruction:?}");
        }
    }
    let mut p = proposal(&document(), b"x");
    p.provenance_digest = [0; 32];
    assert!(matches!(
        BoundProposal::bind(p, 1),
        Err(PolicyError::InvalidCandidate)
    ));
}

#[test]
fn all_four_score_comparisons_use_actual_vm_and_are_zero_heap() {
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;
    use ConstraintInstruction::*;
    let k = key();
    for (instruction, expected) in [
        (
            LessThan {
                vector: 2,
                value: 0.5,
            },
            [true, false, false],
        ),
        (
            GreaterThan {
                vector: 2,
                value: 0.5,
            },
            [false, false, true],
        ),
        (
            LessOrEqual {
                vector: 2,
                value: 0.5,
            },
            [true, true, false],
        ),
        (
            GreaterOrEqual {
                vector: 2,
                value: 0.5,
            },
            [false, true, true],
        ),
    ] {
        let mut d = document();
        d.observations.score = ScoreRequirement::Required(ScoreMeaning::Probability);
        d.observations.score_program =
            PolicyList::from_vec(vec![instruction, HaltIfFalse]).unwrap();
        let signed = SignedEvidencePolicy::sign(d.clone(), &k).unwrap();
        let verified = signed.verify(&trust(&k), 20).unwrap();
        for (value, allow) in [0.25, 0.5, 0.75].into_iter().zip(expected) {
            let mut p = proposal(&d, b"x");
            p.numerical = Some(NumericalObservation {
                meaning: ScoreMeaning::Probability,
                value,
            });
            assert_zero_alloc("real numerical projection", || {
                let b = BoundProposal::bind(p, 1).unwrap();
                assert_eq!(
                    evaluate_constraints(&verified, &b, actor(), 20),
                    Ok(if allow {
                        Decision::Allow
                    } else {
                        Decision::Deny
                    })
                );
                p.numerical = None;
                let absent = BoundProposal::bind(p, 1).unwrap();
                assert_eq!(
                    evaluate_constraints(&verified, &absent, actor(), 20),
                    Err(PolicyError::InvalidCandidate)
                );
            });
        }
    }
}

#[path = "schema_origin_tests.rs"]
mod origin_tests;
