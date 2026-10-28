//! Real Studio.export failure propagation through both Qualia MCP tool routes.
use super::super::{poet_artifact_lifecycle, poet_studio_domain};
use qualia_core_db::poet_host::invoke::studio_store::{get_artifact, upsert_artifact, StudioArtifact};
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn args(value: Value) -> Vec<u8> { serde_json::to_vec(&value).unwrap() }
fn fixture(suffix: &str, value: &str) -> StudioArtifact {
    let id = format!("mcp-export-{suffix}-{}", std::process::id());
    let mut art = StudioArtifact::new(&id, "Sheet", "Example");
    art.nodes.insert("A1".into(), BTreeMap::from([("input".into(), value.into())]));
    upsert_artifact(art.clone());
    art
}

#[test]
fn mcp_studio_export_propagates_office_and_missing_artifact_errors() {
    let art = fixture("office", "42");
    for action in ["export", "Studio.export"] {
        for format in ["xlsx", "docx", "odt", "pptx"] {
            let err = poet_artifact_lifecycle(&args(json!({"action": action, "id": art.id.clone(), "format": format}))).unwrap_err();
            assert!(err.to_string().contains("unavailable"));
        }
    }
    let err = poet_artifact_lifecycle(&args(json!({"action": "export", "id": "mcp-export-missing", "format": "csv"}))).unwrap_err();
    assert!(err.to_string().contains("not found"));
    assert_eq!(get_artifact(&art.id), Some(art));
}

#[test]
fn mcp_studio_export_rejects_unsafe_csv_through_both_tools_and_keeps_simple_csv() {
    let bad = fixture("unsafe", "bad,value");
    let err = poet_artifact_lifecycle(&args(json!({"action": "export", "id": bad.id.clone(), "format": "csv"}))).unwrap_err();
    assert!(err.to_string().contains("cannot preserve"));
    let err = poet_studio_domain(&args(json!({"capability": "Studio.export", "args": {"id": bad.id.clone(), "format": "csv"}}))).unwrap_err();
    assert!(err.to_string().contains("cannot preserve"));
    let err = poet_studio_domain(&args(json!({"capability": "Studio.export", "args": {"id": bad.id.clone(), "format": "pptx"}}))).unwrap_err();
    assert!(err.to_string().contains("unavailable"));
    assert_eq!(get_artifact(&bad.id), Some(bad));
    let good = fixture("safe", "42");
    for output in [
        poet_artifact_lifecycle(&args(json!({"action": "export", "id": good.id.clone(), "format": "csv"}))).unwrap(),
        poet_studio_domain(&args(json!({"capability": "Studio.export", "args": {"id": good.id.clone(), "format": "csv"}}))).unwrap(),
    ] {
        let parsed: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["status"], "ok");
        assert_eq!(parsed["result"]["mime"], "text/csv");
        assert_eq!(parsed["result"]["payload"], "node_id,attribute,value\nA1,input,42\n");
    }
}

#[test]
fn mcp_studio_export_binds_legacy_aliases_and_refuses_conflicts() {
    let art = fixture("aliases", "42");
    for input in [
        json!({"action": "export", "artifact_id": art.id.clone(), "target_format": "xlsx"}),
        json!({"action": "export", "id": art.id.clone(), "target_format": "xlsx"}),
    ] {
        assert!(poet_artifact_lifecycle(&args(input)).unwrap_err().to_string().contains("unavailable"));
    }
    let err = poet_studio_domain(&args(json!({"capability": "Studio.export", "args": {"artifact_id": art.id.clone(), "target_format": "xlsx"}}))).unwrap_err();
    assert!(err.to_string().contains("unavailable"));
    for input in [
        json!({"action": "export", "id": art.id.clone(), "artifact_id": "other", "format": "csv"}),
        json!({"action": "export", "id": art.id.clone(), "format": "csv", "target_format": "xlsx"}),
    ] {
        assert!(poet_artifact_lifecycle(&args(input)).unwrap_err().to_string().contains("conflicting"));
    }
    let conflict = json!({"capability": "Studio.export", "args": {"id": art.id.clone(), "format": "csv", "target_format": "xlsx"}});
    assert!(poet_studio_domain(&args(conflict)).unwrap_err().to_string().contains("conflicting"));
    let out = poet_artifact_lifecycle(&args(json!({"action": "export", "artifact_id": art.id.clone(), "target_format": "csv"}))).unwrap();
    let parsed: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["result"]["mime"], "text/csv");
    assert_eq!(parsed["result"]["payload"], "node_id,attribute,value\nA1,input,42\n");
    let out = poet_studio_domain(&args(json!({"capability": "Studio.export", "artifact_id": art.id.clone(), "target_container": art.id.clone(), "args": {"artifact_id": art.id.clone(), "target_format": "csv"}}))).unwrap();
    let parsed: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["result"]["mime"], "text/csv");
    assert!(parsed.get("target_container").is_none());
    assert!(parsed.get("lamport_clock").is_none());
    assert!(parsed.get("did_q42").is_none());
    assert_eq!(get_artifact(&art.id), Some(art));
}

#[test]
fn mcp_studio_export_rejects_invalid_alias_types_without_defaulting_format() {
    let art = fixture("invalid-alias", "42");
    for field in ["id", "artifact_id", "format", "target_format"] {
        for value in [Value::Null, json!(17)] {
            let mut payload = serde_json::Map::new();
            if field != "id" { payload.insert("id".into(), json!(art.id.clone())); }
            payload.insert(field.into(), value);
            let direct = json!({"action": "export", "args": payload.clone()});
            let domain = json!({"capability": "Studio.export", "args": payload});
            assert!(poet_artifact_lifecycle(&args(direct)).is_err());
            assert!(poet_studio_domain(&args(domain)).is_err());
        }
    }
    let out = poet_artifact_lifecycle(&args(json!({"action": "export", "id": art.id.clone()}))).unwrap();
    let parsed: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["result"]["format"], "q42");
    assert!(parsed["result"]["mime"].as_str().unwrap().contains("quins"));
    assert_eq!(get_artifact(&art.id), Some(art));
}
