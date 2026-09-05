//! Frozen `vibe-host-0.1` four-op surface for Webizen Desktop.
//!
//! Same ops as `poet::vibe_host` and `vibe-wasm`: parse, check, diagnose,
//! capability.invoke, plus version stamps. Source is passed as text so a
//! script edit does not require a desktop rebuild.

use serde::Serialize;
use vibe::{
    catalog, check_cell, check_program, diagnose, parse_cell, parse_program, Span, Value,
    HOST_VERSION, LANGUAGE_VERSION,
};

#[derive(Serialize)]
pub struct VibeHostInfo {
    pub language_version: &'static str,
    pub host_version: &'static str,
    pub crate_stamp: &'static str,
}

#[derive(Serialize)]
pub struct VibeOpResult {
    pub ok: bool,
    pub kind: String,
    pub diagnostic_json: Option<String>,
}

#[derive(Serialize)]
pub struct VibeInvokeResult {
    pub ok: bool,
    pub value_debug: Option<String>,
    pub diagnostic_json: Option<String>,
}

const CRATE_STAMP: &str = env!("CARGO_PKG_VERSION");

fn json_to_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::I64(i)
            } else if let Some(u) = n.as_u64() {
                Value::U64(u)
            } else {
                Value::F64(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(items) => Value::List(items.iter().map(json_to_value).collect()),
        serde_json::Value::Object(map) => {
            let mut rec = std::collections::BTreeMap::new();
            for (k, val) in map {
                rec.insert(k.clone(), json_to_value(val));
            }
            Value::Record(rec)
        }
    }
}

#[tauri::command]
pub fn vibe_host_info() -> VibeHostInfo {
    VibeHostInfo {
        language_version: LANGUAGE_VERSION,
        host_version: HOST_VERSION,
        crate_stamp: CRATE_STAMP,
    }
}

#[tauri::command]
pub fn vibe_diagnose(src: String) -> String {
    diagnose(&src).to_json()
}

#[tauri::command]
pub fn vibe_parse(src: String) -> VibeOpResult {
    let trimmed = src.trim_start_matches('\u{feff}').trim_start();
    if trimmed.starts_with('=') {
        match parse_cell(&src) {
            Ok(_) => VibeOpResult {
                ok: true,
                kind: "cell".into(),
                diagnostic_json: None,
            },
            Err(d) => VibeOpResult {
                ok: false,
                kind: "cell".into(),
                diagnostic_json: Some(d.to_json()),
            },
        }
    } else {
        match parse_program(&src) {
            Ok(_) => VibeOpResult {
                ok: true,
                kind: "module".into(),
                diagnostic_json: None,
            },
            Err(d) => VibeOpResult {
                ok: false,
                kind: "module".into(),
                diagnostic_json: Some(d.to_json()),
            },
        }
    }
}

#[tauri::command]
pub fn vibe_check(src: String) -> VibeOpResult {
    let trimmed = src.trim_start_matches('\u{feff}').trim_start();
    if trimmed.starts_with('=') {
        match parse_cell(&src).and_then(|e| check_cell(&e).map(|_| e)) {
            Ok(_) => VibeOpResult {
                ok: true,
                kind: "cell".into(),
                diagnostic_json: None,
            },
            Err(d) => VibeOpResult {
                ok: false,
                kind: "cell".into(),
                diagnostic_json: Some(d.to_json()),
            },
        }
    } else {
        match parse_program(&src).and_then(|p| check_program(&p).map(|_| p)) {
            Ok(_) => VibeOpResult {
                ok: true,
                kind: "module".into(),
                diagnostic_json: None,
            },
            Err(d) => VibeOpResult {
                ok: false,
                kind: "module".into(),
                diagnostic_json: Some(d.to_json()),
            },
        }
    }
}

#[tauri::command]
pub fn vibe_capability_invoke(id: String, args_json: Option<String>) -> VibeInvokeResult {
    let args = args_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .map(|v| json_to_value(&v))
        .unwrap_or(Value::Null);
    match catalog::invoke_local(&id, &args, Span::point(0)) {
        Ok(value) => VibeInvokeResult {
            ok: true,
            value_debug: Some(format!("{value:?}")),
            diagnostic_json: None,
        },
        Err(diag) => VibeInvokeResult {
            ok: false,
            value_debug: None,
            diagnostic_json: Some(diag.to_json()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_match_frozen_host() {
        let info = vibe_host_info();
        assert_eq!(info.language_version, "vibe-0.1");
        assert_eq!(info.host_version, "vibe-host-0.1");
        assert_eq!(info.crate_stamp, "0.0.36-dev");
    }

    #[test]
    fn diagnose_json_lists_errors_on_failure() {
        let json = vibe_diagnose("= pulse.publish(\"t\", 1)".into());
        assert!(json.contains("\"valid\":false"));
        assert!(json.contains("\"errors\":["));
        assert!(json.contains("suggested_fix"));
    }

    #[test]
    fn parse_and_check_good_cell() {
        let parsed = vibe_parse("= math.max(0, 1)".into());
        assert!(parsed.ok);
        assert_eq!(parsed.kind, "cell");
        let checked = vibe_check("= math.max(0, 1)".into());
        assert!(checked.ok);
    }

    #[test]
    fn catalog_invoke_is_live_on_desktop() {
        let args = serde_json::json!({
            "family": "spatial_kinematics",
            "preset": "orbit_spin",
            "t": 0.5
        })
        .to_string();
        let result = vibe_capability_invoke("Animation.evaluate_preset".into(), Some(args));
        assert!(result.ok, "{:?}", result.diagnostic_json);
    }

    #[test]
    fn unknown_invoke_fails_closed() {
        let result = vibe_capability_invoke("Unknown.capability".into(), None);
        assert!(!result.ok);
        let json = result.diagnostic_json.expect("diagnostic");
        assert!(json.contains("E100"));
    }
}
