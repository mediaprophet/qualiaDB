use super::*;
use crate::{
    q42_volume::{UnifiedVolumeBuilder, MAX_COMPRESSED_SUPERBLOCK_SIZE, SUPERBLOCK_SIZE},
    query::resolver::pack_typed_literal,
};

fn make_quin(s: u64, p: u64, o: u64, context: u64) -> NQuin {
    let mut fact = NQuin { subject: s, predicate: p, object: o, context, ..NQuin::default() };
    fact.recalculate_parity();
    fact
}

#[test]
fn executes_sealed_n3_macro_rule_after_eont_and_indexed_guard_admission() {
    use crate::{
        bundle::semantic_model::SemanticModelBuilder,
        container_10d::{encode_symbolic_rdf_cbor_ld, MultilingualLexicalTerm, SymbolicRdfCborLdPayload},
    };
    let action_iri = "urn:test:motion:action";
    let selected = "urn:test:motion:ready-frame";
    let filtered = "urn:test:motion:withheld-frame";
    let target = "urn:test:motion:morph";
    let context_iri = "urn:test:motion:context";
    let context = q_hash(context_iri);
    let selector = "https://test.example/motion#candidate";
    let ready = "https://test.example/motion#ready";
    let approval = "urn:test:motion:admitted";
    let rule = "{ ?action <https://test.example/motion#candidate> ?frame . ?frame <https://test.example/motion#ready> <urn:test:motion:admitted> . } => { ?action <https://qualiadb.org/ns/kinematics#keyframe> ?frame . }.";
    let mut payload = SymbolicRdfCborLdPayload {
        context_iri: context_iri.into(),
        lexicon: [action_iri, selected, filtered, target, context_iri, selector, ready, approval,
            "https://qualiadb.org/ns/kinematics#keyframe"]
            .into_iter().map(|iri| MultilingualLexicalTerm {
                term_hash: q_hash(iri), iri_utf8: iri.into(), lang_bcp47: None, label_utf8: String::new(),
            }).collect(),
        n3_logic_rules_utf8: vec![rule.into()],
        ..Default::default()
    };
    for (s, p, o) in [
        (q_hash(action_iri), q_hash(selector), q_hash(selected)),
        (q_hash(action_iri), q_hash(selector), q_hash(filtered)),
        (q_hash(selected), q_hash(ready), q_hash(approval)),
        (q_hash(selected), TARGET, q_hash(target)),
        (q_hash(selected), CHANNEL, MORPH),
        (q_hash(selected), SECONDS, pack_typed_literal("0", "xsd:decimal").unwrap()),
        (q_hash(selected), VALUE, pack_typed_literal("0.5", "xsd:decimal").unwrap()),
    ] {
        payload.push_nquin(&make_quin(s, p, o, context));
    }
    let cbld = encode_symbolic_rdf_cbor_ld(&payload).unwrap();
    let bytes = SemanticModelBuilder::new(1024 * 1024).merge_cbld(&cbld).unwrap().finish().unwrap();
    let model = WasmSliceQ42Reader::open(&bytes).unwrap();
    let mut compressed = vec![0; MAX_COMPRESSED_SUPERBLOCK_SIZE];
    let mut decoded = vec![0; SUPERBLOCK_SIZE];
    let mut lex_page = vec![0; 1024 * 1024];
    let mut lex_value = [0; 4096];
    let mut graph = MotionGraph {
        model: &model, context, max_model_blocks: 4, queries_left: 100,
        compressed: &mut compressed, decoded: &mut decoded, lex_page: &mut lex_page, lex_value: &mut lex_value,
    };
    let grant = q_hash("urn:test:motion:capability");
    let mut record = <AffordanceBinaryRecord as bytemuck::Zeroable>::zeroed();
    record.action_id = q_hash(action_iri);
    record.required_capability_hash = grant;
    let records = [record];
    let mut action = SemanticAction {
        iri: action_iri, affordances: &records, capability_ids: &[],
        observer_status: 0, energy_permill: 0, damage_permill: 0, hydration_permill: 0, wear_permill: 0,
    };
    let mut keys = [MacroKeyframe::default(); 2];
    assert!(matches!(prepare_q42_rule_macro(&mut graph, &action, rule, &mut keys), Err(Error::InvalidCapability)));
    let grants = [grant];
    action.capability_ids = &grants;
    assert!(matches!(prepare_q42_semantic_macro(&mut graph, &action, &mut keys), Err(Error::MissingFact)));
    assert!(matches!(prepare_q42_rule_macro(&mut graph, &action, "unsealed", &mut keys), Err(Error::UnboundTarget)));
    let count = prepare_q42_rule_macro(&mut graph, &action, rule, &mut keys).unwrap();
    assert_eq!(count, 1);
    let mut staged = [None];
    let mut out = [None];
    assert_eq!(sample_prepared_macro(&keys[..count], 0.0, &mut staged, &mut out).unwrap(), 1);
    let sample = out[0].unwrap();
    assert_eq!(sample.channel, MacroChannel::MorphWeight);
    assert_eq!(sample.semantic_id, q_hash(target));
    assert_eq!(sample.value, 0.5);
}

fn fitted(target: &str) -> FittedRotation {
    FittedRotation {
        semantic_id: q_hash(target),
        local_axis: [1.0, 0.0, 0.0],
        local_pivot_m: [0.0; 3],
        rom_radians: [-0.5, 0.5],
        contact_safe_radians: [-0.25, 0.25],
    }
}

#[test]
fn samples_q42_macro_and_refuses_ungranted_action_without_publishing() {
    let action_iri = "macro:test";
    let first = "urn:test:first";
    let second = "urn:test:second";
    let joint = "urn:test:joint";
    let receipt = "urn:test:fit-receipt";
    let context = q_hash("urn:test:graph");
    let mut quins = Vec::new();
    for (iri, time, value) in [(first, "0", "0"), (second, "1", "1")] {
        let frame = q_hash(iri);
        quins.extend([
            make_quin(q_hash(action_iri), KEY, frame, context),
            make_quin(frame, TARGET, q_hash(joint), context),
            make_quin(frame, CHANNEL, ROTATION, context),
            make_quin(frame, SECONDS, pack_typed_literal(time, "xsd:decimal").unwrap(), context),
            make_quin(frame, VALUE, pack_typed_literal(value, "xsd:decimal").unwrap(), context),
        ]);
    }
    let receipt_id = q_hash(receipt);
    quins.extend([
        make_quin(q_hash(joint), FIT_RECEIPT, receipt_id, context),
        make_quin(receipt_id, TARGET, q_hash(joint), context),
        make_quin(receipt_id, ENGINEERING_STATUS, q_hash(VERIFIED), context),
    ]);
    for (predicate, value) in AXIS.into_iter().chain(PIVOT).chain(ROM).chain(CONTACT)
        .zip(["1", "0", "0", "0", "0", "0", "-0.5", "0.5", "-0.25", "0.25"])
    {
        quins.push(make_quin(receipt_id, predicate, pack_typed_literal(value, "xsd:decimal").unwrap(), context));
    }
    let lex = [action_iri, first, second, joint, receipt, VERIFIED].into_iter()
        .map(|iri| (q_hash(iri), iri.to_owned())).collect();
    quins.sort_unstable_by_key(|quin| quin.object);
    let mut builder = UnifiedVolumeBuilder::with_lex_map(&lex)
        .unwrap().with_fixed_timestamp_ms(0);
    builder.push_block(0, &quins).unwrap();
    let bytes = builder.finish_to_bytes();
    let model = WasmSliceQ42Reader::open(&bytes).unwrap();
    let mut compressed = vec![0; MAX_COMPRESSED_SUPERBLOCK_SIZE];
    let mut decoded = vec![0; SUPERBLOCK_SIZE];
    let mut lex_page = vec![0; 1024 * 1024];
    let mut lex_value = [0; 128];
    let mut graph = MotionGraph {
        model: &model, context, max_model_blocks: 1, queries_left: 100,
        compressed: &mut compressed, decoded: &mut decoded, lex_page: &mut lex_page,
        lex_value: &mut lex_value,
    };
    let grant = q_hash("urn:test:capability");
    let mut record = <AffordanceBinaryRecord as bytemuck::Zeroable>::zeroed();
    record.action_id = q_hash(action_iri);
    record.required_capability_hash = grant;
    let records = [record];
    let mut action = SemanticAction {
        iri: action_iri, affordances: &records, capability_ids: &[],
        observer_status: 0, energy_permill: 0, damage_permill: 0, hydration_permill: 0, wear_permill: 0,
    };
    let mut keys = [MacroKeyframe::default(); 2];
    let mut staged = [None];
    let mut out = [None];
    assert!(matches!(
        stage_semantic_macro(&mut graph, &action, &[fitted(joint)], 0.5, &mut keys, &mut staged, &mut out),
        Err(Error::InvalidCapability),
    ));
    assert!(out[0].is_none());
    let grants = [grant];
    action.capability_ids = &grants;
    assert_eq!(
        stage_semantic_macro(&mut graph, &action, &[fitted(joint)], 0.5, &mut keys, &mut staged, &mut out)
            .unwrap(),
        1,
    );
    let sample = out[0].unwrap();
    assert_eq!(sample.semantic_id, q_hash(joint));
    assert_eq!(sample.rotation.unwrap().angle_radians, 0.25);

    // A resident player can then sample admitted joint fits without invoking a
    // graph operation on its frame update path.
    let key_count = prepare_q42_semantic_macro(&mut graph, &action, &mut keys).unwrap();
    assert_eq!(sample_prepared_macro(&keys[..key_count], 0.125, &mut staged, &mut out).unwrap(), 1);
    assert_eq!(out[0].unwrap().rotation.unwrap().angle_radians, 0.125);
}

#[test]
fn feedback_is_torque_limited_and_cannot_cross_contact_limit() {
    let fit = fitted("urn:test:joint");
    let mut angle = 0.0;
    let mut velocity = 0.0;
    for _ in 0..1000 {
        let result = feedback_step(
            &fit,
            ControllerFeedback {
                angle_rad: angle, angular_velocity_rad_s: velocity, target_rad: 2.0, dt_s: 0.005,
            },
            1.0, 4.0, 1.0, 0.3,
        ).unwrap();
        assert!(result.applied_torque_nm.abs() <= 0.3);
        assert!(result.rotation.angle_radians <= 0.25);
        angle = result.rotation.angle_radians;
        velocity = result.angular_velocity_rad_s;
    }
    assert!((angle - 0.25).abs() < 0.01);
}
