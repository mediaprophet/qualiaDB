//! Canonical contract validation and test suite for `qualia-app-contract`.

use qualia_app_contract::*;

#[test]
fn test_command_id_parsing_valid() {
    let id = CommandId::parse("gridcraft::edit.set_cell_value").expect("valid command id");
    assert_eq!(id.app_id(), "gridcraft");
    assert_eq!(id.upstream_id(), "edit.set_cell_value");
    assert_eq!(id.as_str(), "gridcraft::edit.set_cell_value");
}

#[test]
fn test_command_id_parsing_rejects_empty_and_malformed() {
    assert_eq!(CommandId::parse(""), Err(ContractError::EmptyCommandId));
    assert_eq!(CommandId::parse("   "), Err(ContractError::EmptyCommandId));
    assert_eq!(
        CommandId::parse("gridcraft"),
        Err(ContractError::InvalidCommandIdFormat("gridcraft".into()))
    );
    assert_eq!(
        CommandId::parse("gridcraft:::cmd"),
        Err(ContractError::InvalidCommandIdFormat("gridcraft:::cmd".into()))
    );
    assert_eq!(
        CommandId::parse("gridcraft::"),
        Err(ContractError::InvalidCommandIdFormat("gridcraft::".into()))
    );
    assert_eq!(
        CommandId::parse("::cmd"),
        Err(ContractError::InvalidCommandIdFormat("::cmd".into()))
    );
    assert_eq!(
        CommandId::parse("bad id::cmd"),
        Err(ContractError::InvalidCommandIdFormat("bad id::cmd".into()))
    );
}

#[test]
fn test_app_id_validation() {
    let app = AppId::new("wordcraft").expect("valid app id");
    assert_eq!(app.as_str(), "wordcraft");

    assert_eq!(AppId::new(""), Err(ContractError::EmptyAppId));
    assert_eq!(AppId::new("   "), Err(ContractError::EmptyAppId));
    assert_eq!(
        AppId::new("invalid app id"),
        Err(ContractError::InvalidCommandIdFormat("invalid app id".into()))
    );
}

#[test]
fn test_decimal_u64_exact_string_serialization() {
    let max_u64 = DecimalU64::new(u64::MAX);
    let json = serde_json::to_string(&max_u64).unwrap();
    assert_eq!(json, "\"18446744073709551615\"");

    let back: DecimalU64 = serde_json::from_str(&json).unwrap();
    assert_eq!(back.get(), u64::MAX);

    let zero = DecimalU64::new(0);
    assert_eq!(serde_json::to_string(&zero).unwrap(), "\"0\"");
}

#[test]
fn test_decimal_u64_rejects_malformed_string() {
    assert!(serde_json::from_str::<DecimalU64>("\"not_a_number\"").is_err());
    assert!(serde_json::from_str::<DecimalU64>("\"-42\"").is_err());
    assert!(serde_json::from_str::<DecimalU64>("\"3.14\"").is_err());
    assert!(serde_json::from_str::<DecimalU64>("\"18446744073709551616\"").is_err()); // overflows u64
}

#[test]
fn test_decimal_u64_js_unsafe_numeric_conversion() {
    // Safe numbers <= 2^53 - 1 can be deserialized directly from numeric literals
    let safe_json = "9007199254740991";
    let safe: DecimalU64 = serde_json::from_str(safe_json).expect("safe JS integer");
    assert_eq!(safe.get(), 9_007_199_254_740_991);

    // Unsafe raw numeric literal > 2^53 - 1 must be rejected to prevent precision loss in JS hosts
    let unsafe_json = "9007199254740992";
    let err = serde_json::from_str::<DecimalU64>(unsafe_json).unwrap_err();
    assert!(err.to_string().contains("JS-unsafe numeric conversion"));
}

#[test]
fn test_coordinate_validation_rejects_nan_and_infinity() {
    assert!(validate_finite_f64(0.0, "x").is_ok());
    assert!(validate_finite_f64(128.5, "y").is_ok());
    assert!(validate_finite_f64(f64::NAN, "x").is_err());
    assert!(validate_finite_f64(f64::INFINITY, "y").is_err());
    assert!(validate_finite_f64(f64::NEG_INFINITY, "y").is_err());

    let hit = PointerHit::new(10.0, 20.0, "canvas_01").expect("valid pointer hit");
    assert_eq!(hit.x, 10.0);
    assert_eq!(hit.y, 20.0);

    assert!(PointerHit::new(f64::NAN, 20.0, "canvas").is_err());
    assert!(PointerHit::new(10.0, f64::INFINITY, "canvas").is_err());

    let hit_with_pressure = hit.with_pressure(0.75).expect("valid pressure");
    assert_eq!(hit_with_pressure.pressure, Some(0.75));

    // Invalid pressure (> 1.0 or < 0.0)
    assert!(hit_with_pressure.clone().with_pressure(1.5).is_err());
    assert!(hit_with_pressure.with_pressure(-0.1).is_err());
}

#[test]
fn test_effects_classification_and_set() {
    let mut effects = EffectSet::empty();
    effects.insert(CommandEffect::Read);
    assert!(effects.is_read_only());

    effects.insert(CommandEffect::DocumentMutation);
    assert!(!effects.is_read_only());
    assert!(effects.contains(CommandEffect::DocumentMutation));
    assert!(effects.contains(CommandEffect::Read));

    let json = serde_json::to_string(&effects).unwrap();
    let back: EffectSet = serde_json::from_str(&json).unwrap();
    assert_eq!(back, effects);

    assert_eq!(
        CommandEffect::from_str_strict("network_disclosure"),
        Ok(CommandEffect::NetworkDisclosure)
    );
    assert!(CommandEffect::from_str_strict("arbitrary_effect").is_err());
}

#[test]
fn test_command_descriptor_validation() {
    let app_id = AppId::new("gridcraft").unwrap();
    let cmd_id = CommandId::from_parts(&app_id, "format.bold").unwrap();

    let desc = CommandDescriptor {
        schema_version: 1,
        command_id: cmd_id,
        command_version: "1.0.0".into(),
        app_id,
        upstream_id: "format.bold".into(),
        label_key: "cmd.grid.bold.label".into(),
        description_key: "cmd.grid.bold.desc".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": { "active": { "type": "boolean" } }
        }),
        output_schema: serde_json::json!({ "type": "null" }),
        target_kinds: vec![TargetKind::CellRange],
        effects: EffectSet::single(CommandEffect::DocumentMutation),
        required_capabilities: vec!["sheet.format".into()],
        undo_policy: UndoPolicy::ReversibleDocument,
        interaction: InteractionKind::Discrete,
        menu: Some("format".into()),
        toolbox: Some("home".into()),
        tool_chain: None,
        orb_group: Some("text_styling".into()),
        shortcuts: vec!["Ctrl+B".into()],
    };

    assert!(desc.validate().is_ok());

    // Bad schema version
    let mut bad_version = desc.clone();
    bad_version.schema_version = 2;
    assert_eq!(bad_version.validate(), Err(ContractError::UnknownVersion(2)));

    // Bad empty label
    let mut bad_label = desc.clone();
    bad_label.label_key = "   ".into();
    assert!(bad_label.validate().is_err());

    // Mismatched app prefix
    let mut bad_prefix = desc.clone();
    bad_prefix.command_id = CommandId::parse("wordcraft::format.bold").unwrap();
    assert!(bad_prefix.validate().is_err());
}

#[test]
fn test_round_trip_cbor_and_json_determinism() {
    let target = CommandTarget {
        session_id: "session_001".into(),
        document_id: "doc_alpha".into(),
        document_revision: DecimalU64::new(42),
        object_ids: vec!["cell_A1".into(), "cell_A2".into()],
        selection: SelectionKind::Multiple(vec!["cell_A1".into(), "cell_A2".into()]),
        subtarget: None,
    };

    let request = CommandRequest {
        request_id: "req_99".into(),
        command_id: CommandId::parse("gridcraft::cell.set_value").unwrap(),
        command_version: "1.0.0".into(),
        target: target.clone(),
        params: serde_json::json!({ "value": 128.5 }),
        expected_revision: DecimalU64::new(42),
        actor_id: "did:q42:actor:principal_1".into(),
        delegation_id: None,
        approval_id: None,
        idempotency_key: "idem_uuid_000".into(),
    };

    // JSON round-trip
    let json = to_json_string(&request).expect("json serialize");
    let json_back: CommandRequest = from_json_str(&json).expect("json deserialize");
    assert_eq!(json_back, request);

    // CBOR round-trip
    let cbor = to_cbor_bytes(&request).expect("cbor serialize");
    let cbor_back: CommandRequest = from_cbor_slice(&cbor).expect("cbor deserialize");
    assert_eq!(cbor_back, request);

    // CommandReceipt round-trip
    let receipt = CommandReceipt {
        request_id: "req_99".into(),
        actor_id: "did:q42:actor:principal_1".into(),
        app_id: AppId::new("gridcraft").unwrap(),
        command_id: CommandId::parse("gridcraft::cell.set_value").unwrap(),
        target_id: "cell_A1".into(),
        before_revision: DecimalU64::new(42),
        after_revision: DecimalU64::new(43),
        status: CommandStatus::Success,
        effects: EffectSet::single(CommandEffect::DocumentMutation),
        result: serde_json::json!({ "updated_cells": 1 }),
        job_handle: None,
        warnings: vec![],
        provenance_ref: Some("hash:blake3:abc123".into()),
        authority_decision_ref: None,
        cost_cents: None,
    };

    let receipt_json = to_json_string(&receipt).unwrap();
    let receipt_back: CommandReceipt = from_json_str(&receipt_json).unwrap();
    assert_eq!(receipt_back, receipt);
}

#[test]
fn test_compatibility_with_portable_app_manifest() {
    use qualia_core_db::q42::app_manifest::*;

    // Validate that we can read and project a PortableAppManifest without ABI divergence
    let manifest = PortableAppManifest {
        identity: AppIdentity {
            app_id: "did:q42:app:gridcraft.proof".into(),
            version: "1.0.0".into(),
            author: AppAuthor {
                name: "Webizen".into(),
                did: "did:q42:webizen".into(),
            },
        },
        entries: vec![EntryProjection {
            projection: ProjectionKind::Container,
            entry_id: "gridcraft".into(),
            relative_path: "apps/poet-wasm/pkg/gridcraft.wasm".into(),
        }],
        required_capabilities: vec![RequiredCapability {
            id: "sheet".into(),
            min_version: "0.1.0".into(),
        }],
        required_assets: vec![],
        state_schema: StateSchema {
            schema_id: "did:q42:schema:sheet".into(),
            schema_version: "1.0.0".into(),
        },
        permission_intents: vec![],
        presentation_hints: vec![],
        compatibility: Compatibility {
            min_engine_version: "0.0.40".into(),
            max_engine_version: String::new(),
            required_features: vec![],
        },
        integrity: Integrity {
            package_sha256: [1u8; 32],
        },
        update_channel: UpdateChannel {
            channel_id: "stable".into(),
            relative_feed: String::new(),
        },
    };

    assert!(manifest.validate().is_ok());

    // Verify mapping from PortableAppManifest to AppSessionDescriptor
    let app_id = AppId::new("gridcraft").unwrap();
    let package_digest = manifest
        .integrity
        .package_sha256
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    let session_desc = AppSessionDescriptor {
        app_version: manifest.identity.version.clone(),
        package_digest,
        engine_version: "0.0.40".into(),
        schema_version: 1,
        session_id: "sess_100".into(),
        document_id: "sheet_01".into(),
        open_mode: "read_write".into(),
        revision: DecimalU64::new(1),
        is_dirty: false,
        is_durable: true,
        execution_source: "worker_wasm".into(),
        capabilities: manifest
            .required_capabilities
            .iter()
            .map(|c| c.id.clone())
            .collect(),
        payload_refs: vec![],
    };

    assert_eq!(session_desc.app_version, "1.0.0");
    assert_eq!(session_desc.capabilities, vec!["sheet"]);
    assert_eq!(app_id.as_str(), "gridcraft");
}

#[test]
fn test_command_catalog_registration_and_alias_resolution() {
    let mut catalog = CommandCatalog::new();
    let app_id = AppId::new("wordcraft").unwrap();
    let cmd_id = CommandId::from_parts(&app_id, "format.italic").unwrap();

    let desc = CommandDescriptor {
        schema_version: 1,
        command_id: cmd_id.clone(),
        command_version: "1.0.0".into(),
        app_id,
        upstream_id: "format.italic".into(),
        label_key: "cmd.word.italic.label".into(),
        description_key: "cmd.word.italic.desc".into(),
        input_schema: serde_json::json!({ "type": "null" }),
        output_schema: serde_json::json!({ "type": "null" }),
        target_kinds: vec![TargetKind::TextSelection],
        effects: EffectSet::single(CommandEffect::DocumentMutation),
        required_capabilities: vec![],
        undo_policy: UndoPolicy::ReversibleDocument,
        interaction: InteractionKind::Discrete,
        menu: Some("format".into()),
        toolbox: None,
        tool_chain: None,
        orb_group: None,
        shortcuts: vec!["Ctrl+I".into()],
    };

    catalog.register(desc.clone()).expect("register success");
    assert_eq!(catalog.len(), 1);

    // Duplicate registration rejected
    assert_eq!(
        catalog.register(desc.clone()),
        Err(ContractError::DuplicateCommandId("wordcraft::format.italic".into()))
    );

    // Register alias
    catalog.register_alias("italic", &cmd_id).expect("alias success");
    assert_eq!(
        catalog.register_alias("italic", &cmd_id),
        Err(ContractError::DuplicateAlias("italic".into()))
    );

    // Resolve via alias and direct ID
    assert_eq!(catalog.resolve("italic"), Some(&desc));
    assert_eq!(catalog.resolve("wordcraft::format.italic"), Some(&desc));
    assert_eq!(catalog.resolve("unknown"), None);
}

#[test]
fn test_gesture_session_lifecycle() {
    let point1 = PointerHit::new(10.0, 15.0, "surface_01").unwrap();
    let mut session = GestureSession::new("gest_01", "sess_01", "pen_brush", DecimalU64::new(1), point1.clone());

    let point2 = PointerHit::new(12.0, 18.0, "surface_01").unwrap().with_pressure(0.8).unwrap();
    session.push_sample(point2.clone()).expect("push sample");
    assert_eq!(session.samples.len(), 1);

    session.cancel();
    assert!(session.is_cancelled);
    assert!(session.push_sample(point2).is_err());
}

