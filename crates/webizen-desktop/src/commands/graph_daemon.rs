//! Desktop → local Qualia daemon (`127.0.0.1:4242`) for Catalog · Lexicon.
//!
//! Open pack posts `GraphDatabase.lexicon_manifest` to `/invoke`. No Host widen.
//! Missing daemon or missing pack stays **held / not yet** — never "unavailable".

use super::poet::PoetEvalResult;
use qualia_client_core::api::get_active_daemon_port;
use qualia_core_db::poet_host::{format_value, PoetSnapshot};
use serde::Deserialize;
use std::time::Duration;
use vibe::Value;
use webizen_studio::lexicon_catalog::{
    copy_avoids_unavailable, health_url, invoke_body, invoke_url, resolve_lexicon_path, INVOKE_ID,
    PRIMARY_DAEMON_PORT,
};

const HEALTH_TIMEOUT: Duration = Duration::from_millis(400);
const INVOKE_TIMEOUT: Duration = Duration::from_secs(8);
const CONNECT_WAIT_MS: u64 = 200;
const CONNECT_WAIT_STEPS: u32 = 4; // ~800ms — prefer live daemon, then in-process bind

#[derive(Debug, Deserialize)]
struct DaemonEvalResponse {
    #[serde(default)]
    ok: bool,
    #[serde(default)]
    value: String,
    diagnostic: Option<String>,
    #[serde(default)]
    revision: u64,
    #[serde(default)]
    committed: usize,
    #[serde(default)]
    honesty: String,
}

/// Ports to probe: recorded active port, then Capt/UAT default 4242.
pub fn candidate_ports() -> Vec<u16> {
    let active = get_active_daemon_port();
    let mut ports = Vec::with_capacity(2);
    if active != 0 {
        ports.push(active);
    }
    if !ports.contains(&PRIMARY_DAEMON_PORT) {
        ports.push(PRIMARY_DAEMON_PORT);
    }
    ports
}

pub async fn probe_health(port: u16) -> bool {
    let Ok(client) = reqwest::Client::builder().timeout(HEALTH_TIMEOUT).build() else {
        return false;
    };
    let Ok(res) = client.get(health_url(port)).send().await else {
        return false;
    };
    res.status().is_success()
}

/// Sync probe for `get_desktop_status` — Native Connected is `/health`, not a flag.
/// Runs off the Tauri/tokio worker so we never nest runtimes.
pub fn probe_health_blocking() -> bool {
    std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()?;
        Some(rt.block_on(async {
            for port in candidate_ports() {
                if probe_health(port).await {
                    return true;
                }
            }
            false
        }))
    })
    .join()
    .ok()
    .flatten()
    .unwrap_or(false)
}

async fn wait_for_daemon() -> Option<u16> {
    for _ in 0..CONNECT_WAIT_STEPS {
        for port in candidate_ports() {
            if probe_health(port).await {
                return Some(port);
            }
        }
        tokio::time::sleep(Duration::from_millis(CONNECT_WAIT_MS)).await;
    }
    None
}

async fn post_invoke(port: u16, path: &str) -> Result<DaemonEvalResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(INVOKE_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;
    let body = invoke_body(path);
    let res = client
        .post(invoke_url(port))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = res.status();
    let text = res.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("daemon invoke {status}: {text}"));
    }
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn result_from_daemon(resp: DaemonEvalResponse) -> PoetEvalResult {
    PoetEvalResult {
        ok: resp.ok,
        value: resp.value,
        diagnostic: resp.diagnostic,
        revision: resp.revision,
        committed: resp.committed,
        published: Vec::new(),
        honesty: "live",
        language: vibe::LANGUAGE_VERSION,
        value_cbor_hex: String::new(),
    }
}

fn result_from_snapshot(path: &str) -> PoetEvalResult {
    let mut snap = PoetSnapshot::live();
    let mut rec = std::collections::BTreeMap::new();
    rec.insert("path".into(), Value::String(path.into()));
    match snap.invoke_id(INVOKE_ID, Value::Record(rec)) {
        Ok(v) => PoetEvalResult {
            ok: true,
            value: format_value(&v),
            diagnostic: None,
            revision: snap.revision,
            committed: snap.visible_count(),
            published: Vec::new(),
            honesty: snap.honesty(),
            language: vibe::LANGUAGE_VERSION,
            value_cbor_hex: String::new(),
        },
        Err(e) => PoetEvalResult {
            ok: false,
            value: String::new(),
            diagnostic: Some(e.to_json()),
            revision: snap.revision,
            committed: snap.visible_count(),
            published: Vec::new(),
            honesty: snap.honesty(),
            language: vibe::LANGUAGE_VERSION,
            value_cbor_hex: String::new(),
        },
    }
}

/// Daemon-first Catalog bind. Falls back to in-process `GraphDatabase.lexicon_manifest`.
pub async fn lexicon_manifest_live(path: String) -> PoetEvalResult {
    let resolved = resolve_lexicon_path(&path);
    if let Some(port) = wait_for_daemon().await {
        match post_invoke(port, &resolved).await {
            Ok(resp) => return result_from_daemon(resp),
            Err(_) => { /* honest fallback — same ALL_BOUND bind, never unavailable */ }
        }
    }
    result_from_snapshot(&resolved)
}

pub fn assert_held_copy(text: &str) {
    let folded = text.to_ascii_lowercase();
    assert!(
        copy_avoids_unavailable(text),
        "Catalog copy must never say unavailable: {text}"
    );
    assert!(!folded.contains("broken"), "{text}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use webizen_studio::lexicon_catalog::{interpret_invoke, ManifestOutcome};

    fn fixture_path() -> String {
        resolve_lexicon_path("crates/vibe/fixtures/lexicon/en-core.lexicon.json")
    }

    #[test]
    fn invoke_targets_lexicon_manifest_on_4242() {
        assert_eq!(INVOKE_ID, "GraphDatabase.lexicon_manifest");
        assert!(!INVOKE_ID.contains("qualia."));
        assert_eq!(invoke_url(4242), "http://127.0.0.1:4242/invoke");
        assert_eq!(health_url(4242), "http://127.0.0.1:4242/health");
        let body = invoke_body("crates/vibe/fixtures/lexicon/en-core.lexicon.json");
        assert_eq!(body["id"], INVOKE_ID);
        assert_eq!(
            body["args"]["path"],
            "crates/vibe/fixtures/lexicon/en-core.lexicon.json"
        );
    }

    #[test]
    fn resolve_finds_en_core_fixture_from_relative_pin() {
        let resolved = resolve_lexicon_path("crates/vibe/fixtures/lexicon/en-core.lexicon.json");
        assert!(
            std::path::Path::new(&resolved).is_file(),
            "expected workspace fixture, got {resolved}"
        );
        assert!(resolved.ends_with("en-core.lexicon.json"));
    }

    #[tokio::test]
    async fn empty_path_stays_held_never_unavailable() {
        let result = lexicon_manifest_live(String::new()).await;
        assert!(!result.ok);
        let outcome = interpret_invoke(result.ok, &result.value, result.diagnostic.as_deref());
        match outcome {
            ManifestOutcome::Held { why } => {
                assert_held_copy(&why);
                assert_held_copy(result.diagnostic.as_deref().unwrap_or(""));
            }
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn nonsense_path_stays_held_never_unavailable() {
        let result =
            lexicon_manifest_live("/tmp/does-not-exist-lexicon-pack.lexicon.json".into()).await;
        assert!(!result.ok);
        let outcome = interpret_invoke(result.ok, &result.value, result.diagnostic.as_deref());
        match outcome {
            ManifestOutcome::Held { why } => assert_held_copy(&why),
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn valid_en_core_fixture_opens_pack_when_bind_succeeds() {
        let result = lexicon_manifest_live(fixture_path()).await;
        let outcome = interpret_invoke(result.ok, &result.value, result.diagnostic.as_deref());
        match outcome {
            ManifestOutcome::Open(card) => {
                assert_eq!(card.pack_semver, "0.1.0");
                assert_eq!(card.framing.as_str(), "mixed");
                assert!(result.ok);
            }
            ManifestOutcome::Held { why } => {
                panic!("valid fixture must open when bind succeeds, stayed held: {why}")
            }
        }
    }
}
