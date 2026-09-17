//! Desktop ↔ local QualiaDB daemon (`127.0.0.1:4242`) probe and HTTP invoke.
//!
//! Catalog · Lexicon open-pack must exercise live `GraphDatabase.lexicon_manifest`
//! on the graph daemon — the same `/health` + `/invoke` path Poet WASM uses.
//! In-process `PoetSnapshot` stays a fallback when the bind is reachable locally
//! but HTTP is not yet up. Wait-honest chrome says **held / not yet**, never
//! unavailable / broken.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::time::Duration;

/// Primary loopback HTTP port. Extra ports only if the process already moved.
pub const GRAPH_DAEMON_PORT: u16 = 4242;
const HEALTH_TIMEOUT_MS: u64 = 800;
const INVOKE_TIMEOUT_MS: u64 = 8_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaemonProbe {
    pub reachable: bool,
    pub url: String,
    pub port: u16,
    pub engine: Option<String>,
    pub version: Option<String>,
    pub graph_quin_count: Option<usize>,
    /// `live` when `/health` answered with engine; otherwise `held`.
    pub honesty: String,
    pub label: String,
}

impl DaemonProbe {
    pub fn held(port: u16) -> Self {
        Self {
            reachable: false,
            url: format!("http://127.0.0.1:{port}"),
            port,
            engine: None,
            version: None,
            graph_quin_count: None,
            honesty: "held".into(),
            label: format!("held / not yet — local daemon 127.0.0.1:{port}"),
        }
    }

    pub fn live(
        port: u16,
        engine: Option<String>,
        version: Option<String>,
        graph_quin_count: Option<usize>,
    ) -> Self {
        Self {
            reachable: true,
            url: format!("http://127.0.0.1:{port}"),
            port,
            engine,
            version,
            graph_quin_count,
            honesty: "live".into(),
            label: format!("Native Connected to 127.0.0.1:{port}"),
        }
    }
}

/// Ports to try: canonical 4242 first, then any active bumped port.
/// Preferring 4242 stops Catalog from following a Desktop spawn on :4243 when
/// a healthy Qualia daemon is already on the contract port.
pub fn candidate_ports() -> Vec<u16> {
    let active = qualia_client_core::api::get_active_daemon_port();
    let mut ports = vec![GRAPH_DAEMON_PORT];
    if active != 0 && active != GRAPH_DAEMON_PORT && !ports.contains(&active) {
        ports.push(active);
    }
    ports
}

fn parse_health_json(v: &JsonValue) -> Option<(Option<String>, Option<String>, Option<usize>)> {
    let engine = v
        .get("engine")
        .and_then(|x| x.as_str())
        .map(str::to_string);
    if engine.is_none() && v.get("engine_version").is_none() {
        return None;
    }
    let version = v
        .get("version")
        .or_else(|| v.get("engine_version"))
        .and_then(|x| x.as_str())
        .map(str::to_string);
    let quins = v.get("graph_quin_count").and_then(|x| x.as_u64()).map(|n| n as usize);
    Some((engine, version, quins))
}

fn blocking_client(timeout_ms: u64) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
        .map_err(|e| e.to_string())
}

/// HTTP GET `/health` on one port — Qualia engine only (must carry `engine`).
pub fn probe_daemon_port(port: u16) -> DaemonProbe {
    let Ok(client) = blocking_client(HEALTH_TIMEOUT_MS) else {
        return DaemonProbe::held(port);
    };
    let url = format!("http://127.0.0.1:{port}/health");
    let Ok(res) = client.get(&url).send() else {
        return DaemonProbe::held(port);
    };
    if !res.status().is_success() {
        return DaemonProbe::held(port);
    }
    let Ok(v) = res.json::<JsonValue>() else {
        return DaemonProbe::held(port);
    };
    if let Some((engine, version, quins)) = parse_health_json(&v) {
        return DaemonProbe::live(port, engine, version, quins);
    }
    DaemonProbe::held(port)
}

/// HTTP GET `/health` — Qualia engine only (must carry `engine`).
pub fn probe_local_daemon() -> DaemonProbe {
    for port in candidate_ports() {
        let p = probe_daemon_port(port);
        if p.reachable {
            return p;
        }
    }
    DaemonProbe::held(GRAPH_DAEMON_PORT)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpInvokeResult {
    pub ok: bool,
    pub value: String,
    pub diagnostic: Option<String>,
    pub revision: u64,
    pub committed: usize,
    pub honesty: String,
}

/// POST `/invoke` on the connected daemon. `None` if the daemon is not live.
pub fn http_invoke(id: &str, args: JsonValue) -> Option<HttpInvokeResult> {
    let probe = probe_local_daemon();
    if !probe.reachable {
        return None;
    }
    let client = blocking_client(INVOKE_TIMEOUT_MS).ok()?;
    let url = format!("{}/invoke", probe.url);
    let body = serde_json::json!({ "id": id, "args": args });
    let res = client.post(&url).json(&body).send().ok()?;
    if !res.status().is_success() {
        return None;
    }
    let v: JsonValue = res.json().ok()?;
    let value = match v.get("value") {
        Some(JsonValue::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    };
    Some(HttpInvokeResult {
        ok: v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false),
        value,
        diagnostic: v
            .get("diagnostic")
            .and_then(|x| x.as_str())
            .map(str::to_string),
        revision: v.get("revision").and_then(|x| x.as_u64()).unwrap_or(0),
        committed: v.get("committed").and_then(|x| x.as_u64()).unwrap_or(0) as usize,
        honesty: v
            .get("honesty")
            .and_then(|x| x.as_str())
            .unwrap_or("live")
            .to_string(),
    })
}

#[tauri::command]
pub fn poet_daemon_probe() -> DaemonProbe {
    probe_local_daemon()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_probe_never_says_unavailable_or_broken() {
        let p = DaemonProbe::held(4242);
        let folded = format!("{} {} {}", p.label, p.honesty, p.url).to_ascii_lowercase();
        assert!(!folded.contains("unavailable"));
        assert!(!folded.contains("broken"));
        assert!(p.label.contains("held / not yet"));
        assert_eq!(p.url, "http://127.0.0.1:4242");
        assert!(!p.reachable);
    }

    #[test]
    fn live_probe_says_native_connected() {
        let p = DaemonProbe::live(4242, Some("qualia-core-db".into()), Some("0.0.38".into()), Some(12));
        assert!(p.reachable);
        assert_eq!(p.label, "Native Connected to 127.0.0.1:4242");
        assert_eq!(p.honesty, "live");
        assert!(!p.label.to_ascii_lowercase().contains("unavailable"));
    }

    #[test]
    fn candidate_ports_always_include_4242() {
        let ports = candidate_ports();
        assert!(ports.contains(&GRAPH_DAEMON_PORT));
        assert_eq!(ports[0], GRAPH_DAEMON_PORT);
    }

    #[test]
    fn health_json_requires_engine() {
        let empty = serde_json::json!({ "status": "ok" });
        assert!(parse_health_json(&empty).is_none());
        let ok = serde_json::json!({
            "engine": "qualia-core-db",
            "version": "0.0.38",
            "graph_quin_count": 3
        });
        let parsed = parse_health_json(&ok).expect("engine health");
        assert_eq!(parsed.0.as_deref(), Some("qualia-core-db"));
        assert_eq!(parsed.2, Some(3));
    }

    #[test]
    fn loopback_stub_connected_opens_en_core_via_http_invoke() {
        use axum::{routing::{get, post}, Json, Router};
        use qualia_core_db::poet_host::{format_value, PoetSnapshot};
        use std::sync::Arc;
        use vibe::Value;

        let bind = std::net::TcpListener::bind("127.0.0.1:4242");
        let Ok(std_listener) = bind else {
            eprintln!("skip: :4242 already bound");
            return;
        };
        drop(std_listener);

        let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/vibe/fixtures/lexicon/en-core.lexicon.json")
            .canonicalize()
            .expect("en-core fixture");
        let fixture_s = fixture.display().to_string();

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("rt");
        let fixture_for_server = fixture_s.clone();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<()>();
        rt.spawn(async move {
            async fn health() -> Json<serde_json::Value> {
                Json(serde_json::json!({
                    "status": "active",
                    "engine": "qualia-core-db",
                    "version": "test-stub",
                    "graph_quin_count": 1
                }))
            }
            let snap = Arc::new(std::sync::Mutex::new(PoetSnapshot::from_daemon()));
            let invoke = {
                let snap = snap.clone();
                move |Json(body): Json<serde_json::Value>| {
                    let snap = snap.clone();
                    async move {
                        let id = body.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        let path = body
                            .get("args")
                            .and_then(|a| a.get("path"))
                            .and_then(|p| p.as_str())
                            .unwrap_or("")
                            .to_string();
                        let mut rec = std::collections::BTreeMap::new();
                        rec.insert("path".into(), Value::String(path));
                        let mut guard = snap.lock().expect("snap");
                        let result = guard.invoke_id(id, Value::Record(rec));
                        Json(match result {
                            Ok(v) => serde_json::json!({
                                "ok": true,
                                "value": format_value(&v),
                                "honesty": "live",
                                "revision": 1,
                                "committed": 0
                            }),
                            Err(e) => serde_json::json!({
                                "ok": false,
                                "value": "",
                                "diagnostic": e.to_json(),
                                "honesty": "held",
                                "revision": 1,
                                "committed": 0
                            }),
                        })
                    }
                }
            };
            let app = Router::new()
                .route("/health", get(health))
                .route("/invoke", post(invoke));
            let listener = tokio::net::TcpListener::bind("127.0.0.1:4242")
                .await
                .expect("bind stub");
            let _ = ready_tx.send(());
            let _ = fixture_for_server;
            axum::serve(listener, app).await.ok();
        });
        ready_rx.recv_timeout(std::time::Duration::from_secs(2)).expect("stub ready");

        let probe = probe_local_daemon();
        assert!(probe.reachable, "{probe:?}");
        assert_eq!(probe.label, "Native Connected to 127.0.0.1:4242");
        assert!(!probe.label.to_ascii_lowercase().contains("unavailable"));

        let result = http_invoke(
            "GraphDatabase.lexicon_manifest",
            serde_json::json!({ "path": fixture_s }),
        )
        .expect("HTTP invoke");
        assert!(result.ok, "{result:?}");
        assert!(result.value.contains("0.1.0"), "{}", result.value);
        match webizen_studio::lexicon_catalog::interpret_invoke(true, &result.value, None) {
            webizen_studio::lexicon_catalog::ManifestOutcome::Open(card) => {
                assert_eq!(card.pack_semver, "0.1.0");
            }
            other => panic!("{other:?}"),
        }
    }
}
