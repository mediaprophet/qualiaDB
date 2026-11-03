use super::super::{
    association::{
        estimate_associations, AssociationAuthority, AssociationEvidenceKind,
        AssociationPermission, AssociationPhase, AssociationResult, CandidateKind, EntityCandidate,
        UnassessedReason,
    },
    radio_evidence::RadioObservationTime,
    record::{CalibrationDeclaration, MeasurementQuality, ProvenanceDeclaration},
    uncertainty::{EvidenceReference, MethodVersion},
    SemanticId,
};
use super::*;

fn id(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).unwrap()
}
fn citation(value: u64) -> EvidenceReference {
    EvidenceReference::new(value).unwrap()
}
fn request() -> AssociationRequest {
    AssociationRequest {
        context_id: id(1),
        purpose_id: id(2),
        actor_id: None,
        policy_context: None,
    }
}
fn reference() -> AssociationModelReference {
    AssociationModelReference {
        method: MethodVersion::new(SemanticId::new(3).unwrap(), 4).unwrap(),
        calibration: citation(5),
    }
}
fn hypothesis() -> AssociationHypothesis {
    AssociationHypothesis {
        id: id(6),
        subject: EntityCandidate {
            id: id(7),
            kind: CandidateKind::Entity,
            ontology_type: SemanticId::Unknown,
        },
        relation_id: id(8),
        object: EntityCandidate {
            id: id(9),
            kind: CandidateKind::Unknown,
            ontology_type: SemanticId::Unknown,
        },
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
fn prior(probability: f64) -> AssociationPriorRow {
    AssociationPriorRow {
        hypothesis: hypothesis(),
        probability,
    }
}
fn likelihood(group: Option<u64>) -> AssociationLikelihoodRow {
    AssociationLikelihoodRow {
        hypothesis_id: hypothesis().id,
        group_id: group.map(id),
        likelihoods: BinaryLogLikelihoods {
            given_association: 0.8_f64.ln(),
            given_no_association: 0.2_f64.ln(),
        },
    }
}

struct Authority;
impl AssociationAuthority for Authority {
    fn authorize(
        &mut self,
        phase: AssociationPhase<'_>,
        _: &AssociationRequest,
        hypotheses: &[AssociationHypothesis],
        _: &[AssociationEvidence<'_>],
        _: AssociationModelReference,
        _: DependenceAssumption,
    ) -> AssociationPermission {
        if let AssociationPhase::Publish { results, .. } = phase {
            assert_eq!(results.len(), hypotheses.len());
        }
        AssociationPermission::Approved {
            decision: citation(10),
        }
    }
}

fn assess(
    table: &TableAssociationModel<'_, '_>,
    request: AssociationRequest,
    hypothesis: AssociationHypothesis,
    evidence: &[AssociationEvidence<'_>],
) -> AssociationResult {
    let mut output = [None; 1];
    estimate_associations(
        request,
        &[hypothesis],
        evidence,
        table.dependence(),
        table,
        &mut Authority,
        &mut output,
    )
    .unwrap();
    assert_eq!(output[0].unwrap().model(), table.reference());
    output[0].unwrap().result()
}

#[test]
fn supplied_prior_and_joint_likelihood_are_used_without_training_or_defaults() {
    let records = [evidence(11, None), evidence(12, None)];
    let priors = [prior(0.2)];
    let likelihoods = [likelihood(None)];
    let table = TableAssociationModel::new(
        reference(),
        request(),
        DependenceAssumption::Joint,
        &records,
        &priors,
        &likelihoods,
    )
    .unwrap();
    assert!(matches!(assess(&table, request(), hypothesis(), &records),
        AssociationResult::Estimated { probability } if (probability - 0.5).abs() < 1e-12));
    assert_eq!(table.request(), request());
    assert_eq!(table.evidence(), records.as_slice());
    assert_eq!(table.priors(), priors.as_slice());
    assert_eq!(table.likelihoods(), likelihoods.as_slice());
}

#[test]
fn explicit_group_rows_are_joint_and_require_complete_declared_coverage() {
    let records = [
        evidence(11, Some(1)),
        evidence(12, Some(1)),
        evidence(13, Some(2)),
    ];
    let priors = [prior(0.5)];
    let likelihoods = [likelihood(Some(1)), likelihood(Some(2))];
    let dependence = DependenceAssumption::IndependentGroups {
        justification: citation(14),
    };
    assert_eq!(
        TableAssociationModel::new(
            reference(),
            request(),
            dependence,
            &records,
            &priors,
            &likelihoods[..1]
        )
        .err(),
        Some(AssociationTableError::MissingLikelihood)
    );
    let table = TableAssociationModel::new(
        reference(),
        request(),
        dependence,
        &records,
        &priors,
        &likelihoods,
    )
    .unwrap();
    assert!(matches!(assess(&table, request(), hypothesis(), &records),
        AssociationResult::Estimated { probability } if (probability - 16.0 / 17.0).abs() < 1e-12));
    let mut changed = records;
    changed[1].value = AssociationValue::Reference(citation(15));
    assert_eq!(
        assess(&table, request(), hypothesis(), &changed),
        AssociationResult::Unassessed(UnassessedReason::ModelOutsideCoverage)
    );
}

#[test]
fn lookup_binds_the_complete_request_and_hypothesis_not_just_ids() {
    let records = [evidence(11, None)];
    let priors = [prior(0.5)];
    let likelihoods = [likelihood(None)];
    let table = TableAssociationModel::new(
        reference(),
        request(),
        DependenceAssumption::Joint,
        &records,
        &priors,
        &likelihoods,
    )
    .unwrap();
    let mut changed_request = request();
    changed_request.policy_context = Some(citation(16));
    assert_eq!(
        table.prior_probability(&changed_request, &hypothesis()),
        None
    );
    assert_eq!(
        assess(&table, changed_request, hypothesis(), &records),
        AssociationResult::Unassessed(UnassessedReason::ModelOutsideCoverage)
    );
    let mut changed_hypothesis = hypothesis();
    changed_hypothesis.object.id = id(17);
    assert_eq!(
        table.prior_probability(&request(), &changed_hypothesis),
        None
    );
    assert_eq!(
        assess(&table, request(), changed_hypothesis, &records),
        AssociationResult::Unassessed(UnassessedReason::ModelOutsideCoverage)
    );
}

#[test]
fn timestamps_sources_citations_order_and_missing_rows_cannot_reuse_a_likelihood() {
    let records = [evidence(11, None), evidence(12, None)];
    let priors = [prior(0.5)];
    let likelihoods = [likelihood(None)];
    let table = TableAssociationModel::new(
        reference(),
        request(),
        DependenceAssumption::Joint,
        &records,
        &priors,
        &likelihoods,
    )
    .unwrap();
    for field in 0..5 {
        let mut changed = records;
        match field {
            0 => changed[0].observed_at = RadioObservationTime::WindowsUtc100ns(1),
            1 => changed[0].source_id = SemanticId::new(18).unwrap(),
            2 => changed[0].value = AssociationValue::Reference(citation(19)),
            3 => changed[0].value = AssociationValue::Missing,
            _ => changed.swap(0, 1),
        }
        assert_eq!(
            assess(&table, request(), hypothesis(), &changed),
            AssociationResult::Unassessed(UnassessedReason::ModelOutsideCoverage)
        );
    }
    assert_eq!(
        assess(&table, request(), hypothesis(), &records[..1]),
        AssociationResult::Unassessed(UnassessedReason::ModelOutsideCoverage)
    );
}

#[test]
fn signed_zero_scalar_content_is_matched_exactly() {
    let mut records = [evidence(11, None)];
    records[0].value = AssociationValue::Scalar {
        value: 0.0,
        quantity: SemanticId::Unknown,
        unit: SemanticId::Unknown,
    };
    let priors = [prior(0.5)];
    let likelihoods = [likelihood(None)];
    let table = TableAssociationModel::new(
        reference(),
        request(),
        DependenceAssumption::Joint,
        &records,
        &priors,
        &likelihoods,
    )
    .unwrap();
    let mut changed = records;
    changed[0].value = AssociationValue::Scalar {
        value: -0.0,
        quantity: SemanticId::Unknown,
        unit: SemanticId::Unknown,
    };
    assert_eq!(
        assess(&table, request(), hypothesis(), &changed),
        AssociationResult::Unassessed(UnassessedReason::ModelOutsideCoverage)
    );
}

#[test]
fn duplicates_or_orphaned_rows_are_rejected() {
    let records = [evidence(11, None)];
    let priors = [prior(0.5)];
    let likelihoods = [likelihood(None)];
    assert_eq!(
        TableAssociationModel::new(
            reference(),
            request(),
            DependenceAssumption::Joint,
            &records,
            &[priors[0]; 2],
            &likelihoods
        )
        .err(),
        Some(AssociationTableError::DuplicatePrior)
    );
    assert_eq!(
        TableAssociationModel::new(
            reference(),
            request(),
            DependenceAssumption::Joint,
            &records,
            &priors,
            &[likelihoods[0]; 2]
        )
        .err(),
        Some(AssociationTableError::DuplicateLikelihood)
    );
    assert_eq!(
        TableAssociationModel::new(
            reference(),
            request(),
            DependenceAssumption::Joint,
            &[records[0]; 2],
            &priors,
            &likelihoods
        )
        .err(),
        Some(AssociationTableError::DuplicateEvidence)
    );
    let mut orphan = likelihood(None);
    orphan.hypothesis_id = id(20);
    assert_eq!(
        TableAssociationModel::new(
            reference(),
            request(),
            DependenceAssumption::Joint,
            &records,
            &priors,
            &[orphan]
        )
        .err(),
        Some(AssociationTableError::UnknownHypothesis)
    );
    assert_eq!(
        TableAssociationModel::new(
            reference(),
            request(),
            DependenceAssumption::Joint,
            &records,
            &priors,
            &[likelihood(Some(1))]
        )
        .err(),
        Some(AssociationTableError::UnknownGroup)
    );
}

#[test]
fn invalid_priors_likelihoods_and_observations_are_not_accepted() {
    let records = [evidence(11, None)];
    let priors = [prior(0.5)];
    let likelihoods = [likelihood(None)];
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.1, 1.1] {
        assert_eq!(
            TableAssociationModel::new(
                reference(),
                request(),
                DependenceAssumption::Joint,
                &records,
                &[prior(value)],
                &likelihoods
            )
            .err(),
            Some(AssociationTableError::InvalidPrior)
        );
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut invalid = likelihood(None);
        invalid.likelihoods.given_association = value;
        assert_eq!(
            TableAssociationModel::new(
                reference(),
                request(),
                DependenceAssumption::Joint,
                &records,
                &priors,
                &[invalid]
            )
            .err(),
            Some(AssociationTableError::InvalidLikelihood)
        );
        let mut invalid = records;
        invalid[0].value = AssociationValue::Scalar {
            value,
            quantity: SemanticId::Unknown,
            unit: SemanticId::Unknown,
        };
        assert_eq!(
            TableAssociationModel::new(
                reference(),
                request(),
                DependenceAssumption::Joint,
                &invalid,
                &priors,
                &likelihoods
            )
            .err(),
            Some(AssociationTableError::InvalidEvidence)
        );
    }
}

#[test]
fn missing_inputs_unknown_dependence_and_capacity_fail_closed() {
    let records = [evidence(11, None)];
    let priors = [prior(0.5)];
    let likelihoods = [likelihood(None)];
    for (rows, priors) in [(&records[..0], &priors[..]), (&records[..], &priors[..0])] {
        assert_eq!(
            TableAssociationModel::new(
                reference(),
                request(),
                DependenceAssumption::Joint,
                rows,
                priors,
                &likelihoods
            )
            .err(),
            Some(AssociationTableError::MissingInputs)
        );
    }
    assert_eq!(
        TableAssociationModel::new(
            reference(),
            request(),
            DependenceAssumption::Joint,
            &records,
            &priors,
            &[]
        )
        .err(),
        Some(AssociationTableError::MissingLikelihood)
    );
    for dependence in [
        DependenceAssumption::Unknown,
        DependenceAssumption::IndependentGroups {
            justification: citation(21),
        },
    ] {
        assert_eq!(
            TableAssociationModel::new(
                reference(),
                request(),
                dependence,
                &records,
                &priors,
                &likelihoods
            )
            .err(),
            Some(AssociationTableError::InvalidDependence)
        );
    }
    assert_eq!(
        TableAssociationModel::new(
            reference(),
            request(),
            DependenceAssumption::Joint,
            &[records[0]; MAX_ASSOCIATION_EVIDENCE + 1],
            &priors,
            &likelihoods
        )
        .err(),
        Some(AssociationTableError::CapacityExceeded)
    );
}
