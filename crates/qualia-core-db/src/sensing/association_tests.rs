use super::super::{
    radio_evidence::RadioObservationTime,
    record::{CalibrationDeclaration, MeasurementQuality, ProvenanceDeclaration},
    uncertainty::{EvidenceReference, MethodVersion},
    SemanticId,
};
use super::*;
use crate::{epistemic::EpistemicState, q_hash};
use core::{cell::Cell, num::NonZeroU64};

fn id(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).unwrap()
}
fn citation(value: u64) -> EvidenceReference {
    EvidenceReference::new(value).unwrap()
}

fn hypothesis() -> AssociationHypothesis {
    AssociationHypothesis {
        id: id(1),
        subject: EntityCandidate {
            id: id(10),
            kind: CandidateKind::Entity,
            ontology_type: SemanticId::new(q_hash("urn:test:Dog")).unwrap(),
        },
        relation_id: id(q_hash("urn:test:hasCarer")),
        object: EntityCandidate {
            id: id(20),
            kind: CandidateKind::Agent,
            ontology_type: SemanticId::Unknown,
        },
    }
}

fn request() -> AssociationRequest {
    AssociationRequest {
        context_id: id(30),
        purpose_id: id(31),
        actor_id: None,
        policy_context: None,
    }
}

fn evidence(observation: u64, group: Option<u64>) -> AssociationEvidence<'static> {
    AssociationEvidence {
        observation_id: id(observation),
        source_id: SemanticId::Unknown,
        context_id: SemanticId::Unknown,
        kind: AssociationEvidenceKind::NfcIdentifier,
        value: AssociationValue::Reference(citation(observation)),
        observed_at: RadioObservationTime::Unknown,
        quality: MeasurementQuality::Unknown,
        calibration: CalibrationDeclaration::Missing,
        provenance: ProvenanceDeclaration::Unknown,
        field: None,
        uncertainty: None,
        dependence_group: group.map(id),
    }
}

struct Model {
    prior: Option<f64>,
    likelihoods: Option<BinaryLogLikelihoods>,
    prior_calls: Cell<usize>,
    calls: Cell<usize>,
    group_ids: Cell<[u64; 64]>,
    counts: Cell<[usize; 64]>,
    invalid_hypothesis: Option<NonZeroU64>,
}

fn model(prior: f64) -> Model {
    Model {
        prior: Some(prior),
        likelihoods: Some(BinaryLogLikelihoods {
            given_association: 0.8_f64.ln(),
            given_no_association: 0.2_f64.ln(),
        }),
        prior_calls: Cell::new(0),
        calls: Cell::new(0),
        group_ids: Cell::new([0; 64]),
        counts: Cell::new([0; 64]),
        invalid_hypothesis: None,
    }
}

impl CalibratedAssociationModel for Model {
    fn reference(&self) -> AssociationModelReference {
        AssociationModelReference {
            method: MethodVersion::new(SemanticId::new(40).unwrap(), 2).unwrap(),
            calibration: citation(41),
        }
    }
    fn prior_probability(
        &self,
        _: &AssociationRequest,
        hypothesis: &AssociationHypothesis,
    ) -> Option<f64> {
        self.prior_calls.set(self.prior_calls.get() + 1);
        if self.invalid_hypothesis == Some(hypothesis.id) {
            return Some(f64::NAN);
        }
        self.prior
    }
    fn joint_log_likelihoods(
        &self,
        _: &AssociationRequest,
        _: &AssociationHypothesis,
        group: AssociationEvidenceGroup<'_, '_>,
    ) -> Option<BinaryLogLikelihoods> {
        let index = self.calls.get();
        let mut groups = self.group_ids.get();
        groups[index] = group.group_id().map_or(0, NonZeroU64::get);
        self.group_ids.set(groups);
        let mut counts = self.counts.get();
        counts[index] = group.records().count();
        self.counts.set(counts);
        self.calls.set(index + 1);
        self.likelihoods
    }
}

struct Authority {
    evaluate: AssociationPermission,
    publish: AssociationPermission,
    require_policy: bool,
    calls: usize,
    maximum_probability: Option<f64>,
    seen_results: [Option<AssociationResult>; MAX_ASSOCIATION_HYPOTHESES],
    seen_evaluation: Option<EvidenceReference>,
}

fn authority() -> Authority {
    Authority {
        evaluate: AssociationPermission::Approved {
            decision: citation(50),
        },
        publish: AssociationPermission::Approved {
            decision: citation(51),
        },
        require_policy: false,
        calls: 0,
        maximum_probability: None,
        seen_results: [None; MAX_ASSOCIATION_HYPOTHESES],
        seen_evaluation: None,
    }
}

impl AssociationAuthority for Authority {
    fn authorize(
        &mut self,
        phase: AssociationPhase<'_>,
        request: &AssociationRequest,
        hypotheses: &[AssociationHypothesis],
        _: &[AssociationEvidence<'_>],
        _: AssociationModelReference,
        _: DependenceAssumption,
    ) -> AssociationPermission {
        self.calls += 1;
        if self.require_policy && request.policy_context.is_none() {
            return AssociationPermission::Unknown;
        }
        match phase {
            AssociationPhase::Evaluate => self.evaluate,
            AssociationPhase::Publish {
                results,
                evaluation_decision,
            } => {
                assert_eq!(results.len(), hypotheses.len());
                self.seen_evaluation = Some(evaluation_decision);
                for (slot, result) in self.seen_results.iter_mut().zip(results) {
                    *slot = Some(*result);
                }
                if results.iter().any(|result| {
                    matches!(result,
                        AssociationResult::Estimated { probability }
                            if self.maximum_probability.is_some_and(|limit| *probability > limit)
                    )
                }) {
                    return AssociationPermission::Denied;
                }
                self.publish
            }
        }
    }
}

fn probability(estimate: AssociationEstimate<'_, '_>) -> f64 {
    match estimate.result() {
        AssociationResult::Estimated { probability } => probability,
        _ => panic!("expected assessed hypothesis"),
    }
}

#[test]
fn joint_model_keeps_dog_care_relation_and_all_unknown_metadata() {
    let mut evidence = [evidence(1, Some(3)), evidence(2, Some(3))];
    evidence[1].kind = AssociationEvidenceKind::SocialContext;
    evidence[1].value = AssociationValue::Missing;
    evidence[1].observed_at = RadioObservationTime::WindowsUtc100ns(-7);
    let model = model(0.5);
    let mut output = [None; 1];
    let mut authority = authority();
    assert_eq!(
        estimate_associations(
            request(),
            &[hypothesis()],
            &evidence,
            DependenceAssumption::Joint,
            &model,
            &mut authority,
            &mut output
        ),
        Ok(1)
    );
    let result = output[0].unwrap();
    assert!((probability(result) - 0.8).abs() < 1e-12);
    assert_eq!(model.calls.get(), 1);
    assert_eq!(model.counts.get()[0], 2);
    assert_eq!(result.hypothesis(), hypothesis());
    assert_eq!(result.evidence(), evidence.as_slice());
    assert_eq!(result.epistemic_state(), EpistemicState::InferredBelief);
    assert_eq!(result.evaluation_decision(), citation(50));
    assert_eq!(result.policy_decision(), citation(51));
    assert_eq!(
        authority.seen_evaluation,
        Some(result.evaluation_decision())
    );
    assert_eq!(authority.seen_results[0], Some(result.result()));
    assert_eq!(authority.calls, 2);
}

#[test]
fn explicit_independence_combines_groups_once_in_sorted_order() {
    let evidence = [
        evidence(1, Some(9)),
        evidence(2, Some(3)),
        evidence(3, Some(9)),
    ];
    let model = model(0.5);
    let mut output = [None; 1];
    estimate_associations(
        request(),
        &[hypothesis()],
        &evidence,
        DependenceAssumption::IndependentGroups {
            justification: citation(60),
        },
        &model,
        &mut authority(),
        &mut output,
    )
    .unwrap();
    assert!((probability(output[0].unwrap()) - 16.0 / 17.0).abs() < 1e-12);
    assert_eq!(model.calls.get(), 2);
    assert_eq!(&model.group_ids.get()[..2], &[3, 9]);
    assert_eq!(&model.counts.get()[..2], &[1, 2]);
}

#[test]
fn unknown_dependence_and_missing_groups_produce_no_invented_prior() {
    let evidence = [evidence(1, None)];
    for (assumption, expected) in [
        (
            DependenceAssumption::Unknown,
            UnassessedReason::UnknownDependence,
        ),
        (
            DependenceAssumption::IndependentGroups {
                justification: citation(60),
            },
            UnassessedReason::MissingDependenceGroup,
        ),
    ] {
        let model = model(0.5);
        let mut output = [None; 1];
        estimate_associations(
            request(),
            &[hypothesis()],
            &evidence,
            assumption,
            &model,
            &mut authority(),
            &mut output,
        )
        .unwrap();
        assert_eq!(
            output[0].unwrap().result(),
            AssociationResult::Unassessed(expected)
        );
        assert_eq!(
            output[0].unwrap().epistemic_state(),
            EpistemicState::Unknown
        );
        assert_eq!(model.prior_calls.get(), 0);
        assert_eq!(model.calls.get(), 0);
    }
}

#[test]
fn no_evidence_or_missing_model_coverage_remains_unassessed() {
    let evidence = [evidence(1, None)];
    for no_prior in [false, true] {
        let mut model = model(0.5);
        if no_prior {
            model.prior = None;
        } else {
            model.likelihoods = None;
        }
        let mut output = [None; 1];
        estimate_associations(
            request(),
            &[hypothesis()],
            &evidence,
            DependenceAssumption::Joint,
            &model,
            &mut authority(),
            &mut output,
        )
        .unwrap();
        assert_eq!(
            output[0].unwrap().result(),
            AssociationResult::Unassessed(UnassessedReason::ModelOutsideCoverage)
        );
    }
    let model = model(0.5);
    let mut output = [None; 1];
    estimate_associations(
        request(),
        &[hypothesis()],
        &[],
        DependenceAssumption::Joint,
        &model,
        &mut authority(),
        &mut output,
    )
    .unwrap();
    assert_eq!(
        output[0].unwrap().result(),
        AssociationResult::Unassessed(UnassessedReason::NoEvidence)
    );
    assert_eq!(model.prior_calls.get(), 0);
}

#[test]
fn endpoints_remain_model_probabilities_and_impossible_evidence_errors() {
    let evidence = [evidence(1, None)];
    for prior in [0.0, 1.0] {
        let model = model(prior);
        let mut output = [None; 1];
        estimate_associations(
            request(),
            &[hypothesis()],
            &evidence,
            DependenceAssumption::Joint,
            &model,
            &mut authority(),
            &mut output,
        )
        .unwrap();
        assert_eq!(probability(output[0].unwrap()), prior);
        assert_eq!(
            output[0].unwrap().epistemic_state(),
            EpistemicState::InferredBelief
        );
    }
    let mut model = model(0.0);
    model.likelihoods = Some(BinaryLogLikelihoods {
        given_association: 0.0,
        given_no_association: f64::NEG_INFINITY,
    });
    let mut output = [None; 1];
    assert_eq!(
        estimate_associations(
            request(),
            &[hypothesis()],
            &evidence,
            DependenceAssumption::Joint,
            &model,
            &mut authority(),
            &mut output
        ),
        Err(AssociationError::ImpossibleEvidence)
    );
    assert_eq!(output, [None]);
}

#[test]
fn invalid_prior_or_likelihood_never_publishes_a_result() {
    let evidence = [evidence(1, None)];
    for prior in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        let mut output = [None; 1];
        assert_eq!(
            estimate_associations(
                request(),
                &[hypothesis()],
                &evidence,
                DependenceAssumption::Joint,
                &model(prior),
                &mut authority(),
                &mut output
            ),
            Err(AssociationError::InvalidModelValue)
        );
        assert_eq!(output, [None]);
    }
    for invalid in [f64::NAN, f64::INFINITY] {
        let mut model = model(0.5);
        model.likelihoods.as_mut().unwrap().given_no_association = invalid;
        let mut output = [None; 1];
        assert_eq!(
            estimate_associations(
                request(),
                &[hypothesis()],
                &evidence,
                DependenceAssumption::Joint,
                &model,
                &mut authority(),
                &mut output
            ),
            Err(AssociationError::InvalidModelValue)
        );
        assert_eq!(output, [None]);
    }
}

#[test]
fn extreme_log_likelihoods_avoid_underflow_and_detect_sum_overflow() {
    let evidence = [evidence(1, Some(1)), evidence(2, Some(2))];
    let mut model = model(0.5);
    model.likelihoods = Some(BinaryLogLikelihoods {
        given_association: -1000.0,
        given_no_association: -1001.0,
    });
    let mut output = [None; 1];
    estimate_associations(
        request(),
        &[hypothesis()],
        &evidence,
        DependenceAssumption::Joint,
        &model,
        &mut authority(),
        &mut output,
    )
    .unwrap();
    assert!((probability(output[0].unwrap()) - 1.0 / (1.0 + (-1.0_f64).exp())).abs() < 1e-12);
    let before = output;
    model.likelihoods = Some(BinaryLogLikelihoods {
        given_association: -f64::MAX,
        given_no_association: -f64::MAX,
    });
    assert_eq!(
        estimate_associations(
            request(),
            &[hypothesis()],
            &evidence,
            DependenceAssumption::IndependentGroups {
                justification: citation(60)
            },
            &model,
            &mut authority(),
            &mut output
        ),
        Err(AssociationError::NumericalRange)
    );
    assert_eq!(output, before);
    for (yes, no) in [(-1000.0, 0.0), (0.0, -1000.0)] {
        model.likelihoods = Some(BinaryLogLikelihoods {
            given_association: yes,
            given_no_association: no,
        });
        assert_eq!(
            estimate_associations(
                request(),
                &[hypothesis()],
                &evidence,
                DependenceAssumption::Joint,
                &model,
                &mut authority(),
                &mut output
            ),
            Err(AssociationError::NumericalRange)
        );
        assert_eq!(output, before);
    }
}

#[test]
fn policy_unknown_denied_and_revoked_at_publish_leave_output_unchanged() {
    let evidence = [evidence(1, None)];
    for (permission, expected) in [
        (
            AssociationPermission::Unknown,
            AssociationError::PermissionUnknown,
        ),
        (
            AssociationPermission::Denied,
            AssociationError::PermissionDenied,
        ),
    ] {
        let model = model(0.5);
        let mut authority = authority();
        authority.evaluate = permission;
        let mut output = [None; 1];
        assert_eq!(
            estimate_associations(
                request(),
                &[hypothesis()],
                &evidence,
                DependenceAssumption::Joint,
                &model,
                &mut authority,
                &mut output
            ),
            Err(expected)
        );
        assert_eq!(model.prior_calls.get(), 0);
        assert_eq!(output, [None]);
    }
    let mut authority = authority();
    authority.publish = AssociationPermission::Denied;
    let mut output = [None; 1];
    assert_eq!(
        estimate_associations(
            request(),
            &[hypothesis()],
            &evidence,
            DependenceAssumption::Joint,
            &model(0.5),
            &mut authority,
            &mut output
        ),
        Err(AssociationError::PermissionDenied)
    );
    assert_eq!(output, [None]);
}

#[test]
fn configured_policy_cannot_be_replaced_by_absence_of_actor() {
    let evidence = [evidence(1, None)];
    let mut authority = authority();
    authority.require_policy = true;
    let mut output = [None; 1];
    assert_eq!(
        estimate_associations(
            request(),
            &[hypothesis()],
            &evidence,
            DependenceAssumption::Joint,
            &model(0.5),
            &mut authority,
            &mut output
        ),
        Err(AssociationError::PermissionUnknown)
    );
    let mut request = request();
    request.policy_context = Some(citation(70));
    estimate_associations(
        request,
        &[hypothesis()],
        &evidence,
        DependenceAssumption::Joint,
        &model(0.5),
        &mut authority,
        &mut output,
    )
    .unwrap();
    assert_eq!(output[0].unwrap().request().actor_id, None);
}

#[test]
fn duplicates_capacity_and_late_model_failure_are_atomic() {
    let evidence = [evidence(1, None)];
    let mut output = [None; 2];
    assert_eq!(
        estimate_associations(
            request(),
            &[hypothesis(), hypothesis()],
            &evidence,
            DependenceAssumption::Joint,
            &model(0.5),
            &mut authority(),
            &mut output
        ),
        Err(AssociationError::DuplicateHypothesis)
    );
    assert_eq!(
        estimate_associations(
            request(),
            &[hypothesis()],
            &[evidence[0], evidence[0]],
            DependenceAssumption::Joint,
            &model(0.5),
            &mut authority(),
            &mut [None; 1]
        ),
        Err(AssociationError::DuplicateEvidence)
    );
    assert_eq!(
        estimate_associations(
            request(),
            &[hypothesis()],
            &evidence,
            DependenceAssumption::Joint,
            &model(0.5),
            &mut authority(),
            &mut []
        ),
        Err(AssociationError::OutputTooSmall)
    );
    let mut other = hypothesis();
    other.id = id(2);
    other.relation_id = id(q_hash("urn:test:ownedBy"));
    let mut model = model(0.5);
    model.invalid_hypothesis = Some(id(2));
    assert_eq!(
        estimate_associations(
            request(),
            &[hypothesis(), other],
            &evidence,
            DependenceAssumption::Joint,
            &model,
            &mut authority(),
            &mut output
        ),
        Err(AssociationError::InvalidModelValue)
    );
    assert_eq!(output, [None; 2]);
}

#[test]
fn scientific_entities_and_separate_relations_are_not_normalized_as_competitors() {
    let mut records = [evidence(1, None), evidence(2, None)];
    records[0].kind =
        AssociationEvidenceKind::Measurement(super::super::MeasurementModality::Electromagnetic);
    records[0].value = AssociationValue::Scalar {
        value: 0.42,
        quantity: SemanticId::new(81).unwrap(),
        unit: SemanticId::new(82).unwrap(),
    };
    records[1].kind = AssociationEvidenceKind::EnvironmentalContext;
    let mut first = hypothesis();
    first.subject.ontology_type = SemanticId::new(q_hash("urn:test:FieldSource")).unwrap();
    first.object.kind = CandidateKind::Entity;
    first.object.ontology_type = SemanticId::new(q_hash("urn:test:Region")).unwrap();
    first.relation_id = id(q_hash("urn:test:locatedIn"));
    let mut second = first;
    second.id = id(2);
    second.relation_id = id(q_hash("urn:test:affects"));
    let mut output = [None; 2];
    estimate_associations(
        request(),
        &[first, second],
        &records,
        DependenceAssumption::Joint,
        &model(0.5),
        &mut authority(),
        &mut output,
    )
    .unwrap();
    assert!((probability(output[0].unwrap()) - 0.8).abs() < 1e-12);
    assert!((probability(output[1].unwrap()) - 0.8).abs() < 1e-12);
    assert_eq!(output[0].unwrap().evidence(), records.as_slice());
}

#[test]
fn fixed_input_limits_are_checked_before_authority_or_model_work() {
    let records = [evidence(1, None); MAX_ASSOCIATION_EVIDENCE + 1];
    let model = model(0.5);
    let mut authority = authority();
    let mut output = [None; MAX_ASSOCIATION_HYPOTHESES + 1];
    assert_eq!(
        estimate_associations(
            request(),
            &[hypothesis()],
            &records,
            DependenceAssumption::Joint,
            &model,
            &mut authority,
            &mut output
        ),
        Err(AssociationError::CapacityExceeded)
    );
    assert_eq!(
        estimate_associations(
            request(),
            &[hypothesis(); MAX_ASSOCIATION_HYPOTHESES + 1],
            &records[..1],
            DependenceAssumption::Joint,
            &model,
            &mut authority,
            &mut output
        ),
        Err(AssociationError::CapacityExceeded)
    );
    assert_eq!(authority.calls, 0);
    assert_eq!(model.prior_calls.get(), 0);
    assert!(output.iter().all(Option::is_none));
}

#[test]
fn publication_authority_can_reject_the_exact_posterior_after_permitting_evaluation() {
    let evidence = [evidence(1, None)];
    let model = model(0.5);
    let mut authority = authority();
    authority.maximum_probability = Some(0.7);
    let mut output = [None; 1];
    assert_eq!(
        estimate_associations(
            request(),
            &[hypothesis()],
            &evidence,
            DependenceAssumption::Joint,
            &model,
            &mut authority,
            &mut output
        ),
        Err(AssociationError::PermissionDenied)
    );
    assert_eq!(authority.calls, 2);
    assert_eq!(authority.seen_evaluation, Some(citation(50)));
    assert!(
        matches!(authority.seen_results[0], Some(AssociationResult::Estimated { probability })
        if (probability - 0.8).abs() < 1e-12)
    );
    assert_eq!(model.calls.get(), 1);
    assert_eq!(output, [None]);

    authority.maximum_probability = Some(0.9);
    estimate_associations(
        request(),
        &[hypothesis()],
        &evidence,
        DependenceAssumption::Joint,
        &model,
        &mut authority,
        &mut output,
    )
    .unwrap();
    let published = output[0].unwrap();
    assert_eq!(Some(published.result()), authority.seen_results[0]);
    assert_eq!(
        Some(published.evaluation_decision()),
        authority.seen_evaluation
    );
    assert_eq!(published.policy_decision(), citation(51));
}
