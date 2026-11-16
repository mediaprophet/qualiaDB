use super::*;

fn id(bytes: &[u8]) -> ContentId {
    ContentId::sha256(bytes)
}
fn external(kind: ResourceKind, bytes: &[u8]) -> ResourceRef {
    ResourceRef {
        id: id(bytes),
        kind,
        encoding: ResourceEncoding::ExternalTyped {
            schema: id(b"fixture-schema/v1"),
        },
        byte_len: bytes.len() as u64,
        element_count: Some(1),
    }
}
fn arithmetic() -> ArithmeticProfile {
    ArithmeticProfile {
        storage: Precision::F32,
        arithmetic: Precision::F32,
        accumulation: Precision::F32,
        reduction: ReductionOrder::FixedSerial,
        exceptional_values: ExceptionalValues::RejectNonFinite,
        minimum_magnitude: Some(0.0),
        maximum_magnitude: Some(100.0),
        scaling: "identity".into(),
    }
}
fn quality() -> QualityProfile {
    QualityProfile {
        name: "fixture/v1".into(),
        reference_scale: 1.0,
        criteria: vec![ErrorCriterion {
            name: "position".into(),
            units: "m".into(),
            threshold: 0.001,
            class: MeasurementClass::EnclosingBound,
            method: "analytic".into(),
            required: true,
        }],
        domains: vec![],
        assurance_label: "A1".into(),
        fidelity_label: "F1".into(),
        arithmetic: arithmetic(),
        resources: ResourceBounds {
            cold_bytes: 1024,
            work_items: 64,
            output_elements: 64,
        },
        expected_rejections: vec![ReasonCode::InvalidInput],
    }
}
fn document(record: ContractRecord) -> ContractDocument {
    ContractDocument {
        schema: SchemaVersion::V1,
        record,
    }
}
fn variant() -> ExecutionVariant {
    ExecutionVariant {
        kernel: id(b"kernel"),
        schedule: ScheduleContract {
            workgroup: [64, 1, 1],
            items_per_invocation: 2,
            vector_width: 4,
        },
        abi: external(ResourceKind::Evidence, b"ABI"),
        emitted_source: ResourceRef {
            id: id(b"WGSL"),
            kind: ResourceKind::ShaderSource,
            encoding: ResourceEncoding::WgslUtf8,
            byte_len: 4,
            element_count: None,
        },
        precision: arithmetic(),
        adapter: AdapterScope {
            backend: "dx12".into(),
            adapter: id(b"adapter"),
            driver: "driver1".into(),
            compiler: "compiler1".into(),
            features: vec![],
            limits: external(ResourceKind::Evidence, b"limits"),
        },
        toolchain: id(b"toolchain"),
        dependencies: vec![],
        forge_manifest: None,
    }
}
fn transfer() -> TransferContract {
    TransferContract {
        source: id(b"source"),
        target: id(b"target"),
        direction: TransferDirection::SourceToTarget,
        source_dofs: 3,
        target_dofs: 3,
        construction_method: "identity sparse operator".into(),
        domain: EntityDomain::Vertex,
        semantics: FieldSemantics::Intensive,
        units: "m".into(),
        source_frame: id(b"frame"),
        target_frame: id(b"frame"),
        rule: TransferRule::Interpolation,
        support: external(ResourceKind::SparseMap, b"identity operator"),
        maximum_nonzeros: 3,
        quantisation: "none-f64".into(),
        domain_restrictions: vec![],
        validation: vec![],
        errors: vec![],
    }
}
fn receipt(value: MetricValue) -> ValidationReceipt {
    ValidationReceipt {
        targets: vec![id(b"target")],
        stage: ValidationStage::OracleVerified,
        fixtures: vec![],
        seeds: vec![42],
        methods: vec!["analytic".into()],
        metrics: vec![NamedMetric {
            name: "position".into(),
            value,
        }],
        status: ResultStatus::Passed,
        reason: None,
        explanation: None,
        timing_source: TimingSource::NotMeasured,
        resources: vec![],
        provenance: vec![],
        etalon_receipt: None,
    }
}

#[test]
fn unknown_version_and_normative_fields_are_rejected() {
    let bytes = canonical_bytes(
        &document(ContractRecord::Quality(quality())),
        ContractLimits::default(),
    )
    .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["schema"] = serde_json::json!("q42.production-contracts/v2");
    assert_eq!(
        parse_document(
            &serde_json::to_vec(&value).unwrap(),
            ContractLimits::default()
        )
        .unwrap_err()
        .code,
        ReasonCode::InvalidSchema
    );
    value["schema"] = serde_json::json!("q42.production-contracts/v1");
    value["record"]["value"]["arithmetic"]["fast_math_override"] = serde_json::json!(true);
    assert!(parse_document(
        &serde_json::to_vec(&value).unwrap(),
        ContractLimits::default()
    )
    .is_err());
    value["record"]["value"]["arithmetic"]
        .as_object_mut()
        .unwrap()
        .remove("fast_math_override");
    value["unchecked_extension"] = serde_json::json!(true);
    assert!(parse_document(
        &serde_json::to_vec(&value).unwrap(),
        ContractLimits::default()
    )
    .is_err());
}

#[test]
fn canonical_roundtrip_ignores_object_input_order() {
    let original = document(ContractRecord::Quality(quality()));
    let canonical = canonical_bytes(&original, ContractLimits::default()).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&canonical).unwrap();
    let reordered = serde_json::to_vec_pretty(&value).unwrap();
    let parsed = parse_document(&reordered, ContractLimits::default()).unwrap();
    assert_eq!(
        canonical,
        canonical_bytes(&parsed, ContractLimits::default()).unwrap()
    );
    assert_eq!(
        document_identity(&original, ContractLimits::default()).unwrap(),
        id(&canonical)
    );
}

#[test]
fn precision_abi_schedule_and_semantics_invalidate_identity() {
    let original = variant();
    let base = document_identity(
        &document(ContractRecord::Variant(original.clone())),
        ContractLimits::default(),
    )
    .unwrap();
    let mut candidates = vec![];
    let mut changed = original.clone();
    changed.precision.accumulation = Precision::F64;
    candidates.push(changed);
    let mut changed = original.clone();
    changed.abi.id = id(b"changed ABI");
    candidates.push(changed);
    let mut changed = original.clone();
    changed.schedule.workgroup[0] = 32;
    candidates.push(changed);
    let mut changed = original;
    changed.kernel = id(b"changed semantics");
    candidates.push(changed);
    for changed in candidates {
        assert_ne!(
            base,
            document_identity(
                &document(ContractRecord::Variant(changed)),
                ContractLimits::default()
            )
            .unwrap()
        );
    }
}

#[test]
fn absence_never_serializes_as_numeric_zero() {
    for metric in [
        MetricValue::NotMeasured {
            reason: "not observed".into(),
        },
        MetricValue::NotApplicable {
            reason: "outside declared profile".into(),
        },
    ] {
        let value = serde_json::to_value(&metric).unwrap();
        assert!(value.get("value").is_none());
        assert!(value["reason"].is_string());
        assert!(
            validate_receipt_against_profile(&receipt(metric), &quality(), id(b"target")).is_err()
        );
    }
}

#[test]
fn nonfinite_and_duplicate_measurements_cannot_pass() {
    let mut invalid_profile = quality();
    invalid_profile.criteria[0].threshold = f64::NAN;
    assert!(canonical_bytes(
        &document(ContractRecord::Quality(invalid_profile)),
        ContractLimits::default()
    )
    .is_err());
    let metric = MetricValue::Available {
        value: 0.0001,
        units: "m".into(),
        class: MeasurementClass::EnclosingBound,
        method: "analytic".into(),
    };
    let mut r = receipt(metric);
    assert!(validate_receipt_against_profile(&r, &quality(), id(b"target")).is_ok());
    r.metrics.push(r.metrics[0].clone());
    assert!(validate_document(
        &document(ContractRecord::Receipt(r)),
        ContractLimits::default()
    )
    .is_err());
}

#[test]
fn transfer_endpoints_and_semantics_are_checked() {
    let mut t = transfer();
    assert!(validate_document(
        &document(ContractRecord::Transfer(t.clone())),
        ContractLimits::default()
    )
    .is_ok());
    assert!(validate_transfer_endpoints(&t, id(b"source"), id(b"target")).is_ok());
    assert_eq!(
        validate_transfer_endpoints(&t, id(b"target"), id(b"source"))
            .unwrap_err()
            .code,
        ReasonCode::StaleIdentity
    );
    t.support.kind = ResourceKind::IndexRemap;
    assert!(validate_transfer_semantics(&t).is_err());
    t = transfer();
    t.semantics = FieldSemantics::Categorical;
    assert!(validate_transfer_semantics(&t).is_err());
    t = transfer();
    t.semantics = FieldSemantics::CornerUv;
    t.domain = EntityDomain::Corner;
    assert!(validate_transfer_semantics(&t).is_err());
}

#[test]
fn reference_type_and_byte_identity_are_independent_gates() {
    let resource = external(ResourceKind::AttributeData, b"actual payload");
    assert!(resource.verify(b"actual payload").is_ok());
    assert!(resource.verify(b"edited payload").is_err());
    let mut v = variant();
    v.emitted_source.encoding = ResourceEncoding::FemcV1;
    assert!(validate_document(
        &document(ContractRecord::Variant(v)),
        ContractLimits::default()
    )
    .is_err());
    assert!(verify_existing_identity(id(b"a"), b"a", b"b").is_err());
}

#[test]
fn byte_and_count_caps_reject_before_canonical_output() {
    let doc = document(ContractRecord::Quality(quality()));
    let tiny = ContractLimits {
        envelope_bytes: 1,
        ..ContractLimits::default()
    };
    assert_eq!(
        parse_document(b"{}", tiny).unwrap_err().code,
        ReasonCode::CapacityExhausted
    );
    assert_eq!(
        canonical_bytes(&doc, tiny).unwrap_err().code,
        ReasonCode::CapacityExhausted
    );
    let mut q = quality();
    q.expected_rejections = vec![ReasonCode::InvalidInput; 129];
    assert!(canonical_bytes(
        &document(ContractRecord::Quality(q)),
        ContractLimits::default()
    )
    .is_err());
}

#[test]
fn dependency_resolver_rejects_stale_and_missing_inputs() {
    let q = document(ContractRecord::Quality(quality()));
    let qid = document_identity(&q, ContractLimits::default()).unwrap();
    assert!(validate_dependency_graph(&[(qid, &q)], &[], ContractLimits::default()).is_ok());
    let v = document(ContractRecord::Variant(variant()));
    let vid = document_identity(&v, ContractLimits::default()).unwrap();
    assert_eq!(
        validate_dependency_graph(&[(vid, &v)], &[], ContractLimits::default())
            .unwrap_err()
            .code,
        ReasonCode::StaleIdentity
    );
    assert!(
        validate_dependency_graph(&[(id(b"wrong"), &q)], &[], ContractLimits::default()).is_err()
    );
}

#[test]
fn journal_reservation_abort_does_not_publish_an_event() {
    let mut events = [None];
    let mut journal = CollapseJournal::new(&mut events);
    {
        let _reservation = journal.reserve().unwrap();
    }
    assert!(journal.committed().is_empty());
    let proposal = CollapseProposal {
        run: id(b"run"),
        source: id(b"source"),
        policy: id(b"policy"),
        sequence: 0,
        topology_generation: 0,
        removed_vertex: 0,
        kept_vertex: 1,
        stable_endpoint_ids: [10, 20],
        endpoint_generations: [0, 0],
        target: super::super::primitives::Point3::new(0.0, 0.0, 0.0),
        t_from_removed_to_kept: 0.5,
    };
    proposal.validate().unwrap();
    journal.reserve().unwrap().commit(CollapseEvent {
        proposal,
        transfer: id(b"actual prepared support"),
    });
    assert_eq!(journal.committed().len(), 1);
    assert_eq!(
        journal.reserve().unwrap_err().code,
        ReasonCode::CapacityExhausted
    );
}
