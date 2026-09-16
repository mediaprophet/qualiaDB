//! Webizen Desktop — collect/inspect/run semantic instruments (SI-10 thin).
//! Collect is not activate. Demo stays labelled. No Host IDs.

use qualia_core_db::semantic_instruments::resolve::DependencyKind;
use qualia_core_db::semantic_instruments::{
    build_demo, demo_catalog, open_collectable, plan_resolution, preflight, reference_catalog,
    run_entry, seed_by_slug, CatalogRecord, DependencyLock, DependencyReq, InstalledRelease,
    InstrumentError, LocalRegistry, RunOutcome, RunRequest,
};
use std::collections::BTreeSet;
use std::sync::{Mutex, MutexGuard};
use tauri::command;

/// Registry stores bytes as closed so later `preflight` can succeed.
/// Host `active` is the collect≠activate gate (core-db has no `set_closed`).
struct HostState {
    registry: LocalRegistry,
    active: BTreeSet<String>,
    suspended: BTreeSet<String>,
    revoked: BTreeSet<String>,
    receipts: Vec<StoredReceipt>,
    run_permitted: bool,
}

#[derive(Clone)]
struct StoredReceipt {
    release_id: String,
    entry_point: String,
    outcome: String,
}

impl HostState {
    fn new() -> Self {
        Self {
            registry: LocalRegistry::new(),
            active: BTreeSet::new(),
            suspended: BTreeSet::new(),
            revoked: BTreeSet::new(),
            receipts: Vec::new(),
            run_permitted: true,
        }
    }
}

static HOST: Mutex<Option<HostState>> = Mutex::new(None);

fn host() -> MutexGuard<'static, Option<HostState>> {
    HOST.lock().unwrap_or_else(|p| p.into_inner())
}

#[derive(serde::Serialize)]
pub struct LibraryCard {
    pub slug: String,
    pub name: String,
    pub release_id: String,
    pub category: String,
    pub entry_point: String,
}

#[command]
pub fn si_list_demos() -> Vec<LibraryCard> {
    demo_catalog()
        .iter()
        .map(|s| LibraryCard {
            slug: s.slug.into(),
            name: s.name.into(),
            release_id: s.release_id.into(),
            category: "demo".into(),
            entry_point: s.entry_point.into(),
        })
        .collect()
}

#[command]
pub fn si_list_references() -> Vec<LibraryCard> {
    reference_catalog()
        .iter()
        .map(|s| LibraryCard {
            slug: s.slug.into(),
            name: s.name.into(),
            release_id: s.release_id.into(),
            category: "reference".into(),
            entry_point: s.entry_point.into(),
        })
        .collect()
}

#[command]
pub fn si_collect_demo(slug: String) -> Result<String, String> {
    let bytes = build_demo(&slug).map_err(|e| e.to_string())?;
    let opened = open_collectable(&bytes).map_err(|e| e.to_string())?;
    let mut guard = host();
    let state = guard.get_or_insert_with(HostState::new);
    state
        .registry
        .install(InstalledRelease {
            release_id: opened.manifest.release_id.clone(),
            content_digest: opened.manifest.content_digest.clone(),
            lock: DependencyLock {
                release_id: opened.manifest.release_id.clone(),
                entries: Vec::new(),
            },
            closed: true,
        })
        .map_err(|e| e.to_string())?;
    Ok(opened.manifest.release_id)
}

#[command]
pub fn si_activate_demo(slug: String) -> Result<String, String> {
    let seed = seed_by_slug(&slug).ok_or_else(|| "unknown demo".to_string())?;
    let mut guard = host();
    let state = guard.get_or_insert_with(HostState::new);
    if state.revoked.contains(seed.release_id) {
        return Err("held / not yet — revoked version cannot start a new run".into());
    }
    let allow = {
        let installed = state
            .registry
            .lookup(seed.release_id)
            .ok_or_else(|| "collect first".to_string())?;
        if state.active.contains(seed.release_id) {
            return Ok(seed.release_id.to_string());
        }
        installed.lock.entries.is_empty()
    };
    if !allow {
        return Err(InstrumentError::NotClosed.to_string());
    }
    state.suspended.remove(seed.release_id);
    state.active.insert(seed.release_id.to_string());
    Ok(seed.release_id.to_string())
}

#[command]
pub fn si_inspect_demo(slug: String) -> Result<serde_json::Value, String> {
    // Local demo bytes only — never a network fetch (SI-10 inspect-before-fetch).
    let bytes = build_demo(&slug).map_err(|e| e.to_string())?;
    let opened = open_collectable(&bytes).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "release_id": opened.manifest.release_id,
        "name": opened.manifest.name,
        "category": "demo",
        "licence": opened.manifest.licence,
        "byte_length": bytes.len(),
        "honesty": opened.manifest.honesty_notice,
        "endorsed": false,
        "accessible_text": opened.manifest.accessible_text,
        "network_fetch": false,
    }))
}

#[command]
pub fn si_set_run_permitted(permitted: bool) -> Result<bool, String> {
    let mut guard = host();
    let state = guard.get_or_insert_with(HostState::new);
    state.run_permitted = permitted;
    Ok(permitted)
}

#[command]
pub fn si_run_demo(slug: String, entry_point: String, input: String) -> Result<serde_json::Value, String> {
    if entry_point.contains("Host.") {
        return Err("unknown entry point (not a Host ID)".into());
    }
    let seed = seed_by_slug(&slug).ok_or_else(|| "unknown demo".to_string())?;
    let mut guard = host();
    let state = guard.get_or_insert_with(HostState::new);
    if !state.run_permitted {
        return Err(
            "held / not yet — permission denial is usable; grant was refused".into(),
        );
    }
    if state.revoked.contains(seed.release_id) {
        return Err("held / not yet — revoked version cannot start a new run".into());
    }
    if state.registry.lookup(seed.release_id).is_none() {
        return Err("collect first".into());
    }
    if state.suspended.contains(seed.release_id) {
        return Err("held / not yet — suspended pack cannot start a new run".into());
    }
    if !state.active.contains(seed.release_id) {
        return Err(InstrumentError::NotClosed.to_string());
    }
    let req = RunRequest {
        release_id: seed.release_id,
        entry_point: seed.entry_point,
        input: input.as_bytes(),
        now: 0,
        cancelled: false,
        max_input_bytes: 65_536,
        max_steps: 8,
        parent_receipt: None,
    };
    let installed = preflight(&state.registry, &req).map_err(|e| e.to_string())?;
    match run_entry(&state.registry, installed, &req) {
        RunOutcome::Completed { receipt } | RunOutcome::Held { receipt } => {
            state.receipts.push(StoredReceipt {
                release_id: receipt.release_id.clone(),
                entry_point: receipt.entry_point.clone(),
                outcome: receipt.outcome.clone(),
            });
            Ok(serde_json::json!({
                "outcome": receipt.outcome,
                "entry_point": receipt.entry_point,
                "lock_digest": receipt.lock_digest,
            }))
        }
        RunOutcome::Refused { error } => Err(error.to_string()),
    }
}

#[command]
pub fn si_list_receipts(slug: String) -> Result<Vec<serde_json::Value>, String> {
    let seed = seed_by_slug(&slug).ok_or_else(|| "unknown demo".to_string())?;
    let guard = host();
    let state = guard.as_ref().ok_or_else(|| "collect first".to_string())?;
    Ok(state
        .receipts
        .iter()
        .filter(|r| r.release_id == seed.release_id)
        .map(|r| {
            serde_json::json!({
                "release_id": r.release_id,
                "entry_point": r.entry_point,
                "outcome": r.outcome,
            })
        })
        .collect())
}

#[command]
pub fn si_revoke_demo(slug: String) -> Result<String, String> {
    let seed = seed_by_slug(&slug).ok_or_else(|| "unknown demo".to_string())?;
    let mut guard = host();
    let state = guard.get_or_insert_with(HostState::new);
    if state.registry.lookup(seed.release_id).is_none() && state.receipts.is_empty() {
        return Err("collect first".into());
    }
    state.active.remove(seed.release_id);
    state.revoked.insert(seed.release_id.to_string());
    Ok(seed.release_id.to_string())
}

#[command]
pub fn si_suspend_demo(slug: String) -> Result<String, String> {
    let seed = seed_by_slug(&slug).ok_or_else(|| "unknown demo".to_string())?;
    let mut guard = host();
    let state = guard.get_or_insert_with(HostState::new);
    if state.registry.lookup(seed.release_id).is_none() {
        return Err("collect first".into());
    }
    state.active.remove(seed.release_id);
    state.suspended.insert(seed.release_id.to_string());
    Ok(seed.release_id.to_string())
}

#[command]
pub fn si_remove_demo(slug: String) -> Result<String, String> {
    let seed = seed_by_slug(&slug).ok_or_else(|| "unknown demo".to_string())?;
    let mut guard = host();
    let state = guard.get_or_insert_with(HostState::new);
    state.active.remove(seed.release_id);
    state.suspended.remove(seed.release_id);
    state.registry.uninstall(seed.release_id);
    Ok(seed.release_id.to_string())
}

#[command]
pub fn si_cancel_run(slug: String) -> Result<String, String> {
    if slug.contains("Host.") {
        return Err("unknown entry point (not a Host ID)".into());
    }
    let _ = seed_by_slug(&slug).ok_or_else(|| "unknown demo".to_string())?;
    Ok("cancel run".into())
}

/// Offline-resolved dependency plan: required ontology available_offline.
pub fn offline_resolved_unit_convert() -> Result<(), InstrumentError> {
    let bytes = build_demo("unit-convert")?;
    let opened = open_collectable(&bytes)?;
    let dep_id = "https://ns.webizen.org/demo/concepts/units";
    let catalog = [CatalogRecord {
        id: dep_id.into(),
        version: "1.0.0".into(),
        digest: "sha256:offline-units".into(),
        locator: "local:units".into(),
        revoked: false,
        available_offline: true,
    }];
    let reqs = [DependencyReq {
        id: dep_id.into(),
        kind: DependencyKind::Ontology,
        version_constraint: "1.0.0".into(),
        expected_digest: String::new(),
        required: true,
        purpose: "offline-run".into(),
    }];
    let lock = plan_resolution(&opened.manifest.release_id, &reqs, &catalog)?;
    if lock.entries.is_empty() {
        return Err(InstrumentError::OfflineHeld);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_fresh_host<R>(f: impl FnOnce() -> R) -> R {
        static SERIAL: Mutex<()> = Mutex::new(());
        let _serial = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        {
            let mut g = host();
            *g = Some(HostState::new());
        }
        f()
    }

    #[test]
    fn list_demos_is_labelled() {
        let cards = si_list_demos();
        assert_eq!(cards.len(), 3);
        assert!(cards.iter().all(|c| c.category == "demo"));
        let refs = si_list_references();
        assert_eq!(refs.len(), 3);
        assert!(refs.iter().all(|c| c.category == "reference"));
    }

    #[test]
    fn inspect_is_local_and_unendorsed() {
        let v = si_inspect_demo("unit-convert".into()).expect("inspect");
        assert_eq!(v["endorsed"], false);
        assert_eq!(v["network_fetch"], false);
        assert!(v["byte_length"].as_u64().unwrap() > 0);
        assert!(v["licence"].as_str().unwrap().len() > 0);
        assert_eq!(v["category"], "demo");
    }

    #[test]
    fn collect_does_not_activate() {
        with_fresh_host(|| {
            let id = si_collect_demo("unit-convert".into()).expect("collect");
            assert!(id.contains("unit-convert"));
            let err = si_run_demo("unit-convert".into(), "assess".into(), "1m".into())
                .expect_err("run must not succeed before activate");
            assert_eq!(err, InstrumentError::NotClosed.to_string());
        });
    }

    #[test]
    fn activate_after_collect_then_run() {
        with_fresh_host(|| {
            si_collect_demo("unit-convert".into()).expect("collect");
            let id = si_activate_demo("unit-convert".into()).expect("activate");
            assert!(id.contains("unit-convert"));
            let out = si_run_demo("unit-convert".into(), "assess".into(), "1m".into())
                .expect("run after activate");
            assert_eq!(out["entry_point"], "assess");
        });
    }

    #[test]
    fn run_without_activate_fails() {
        with_fresh_host(|| {
            let missing = si_run_demo("unit-convert".into(), "assess".into(), "1m".into())
                .expect_err("not collected");
            assert_eq!(missing, "collect first");
            assert_eq!(
                si_activate_demo("unit-convert".into()).expect_err("activate without collect"),
                "collect first"
            );
            si_collect_demo("unit-convert".into()).expect("collect");
            let not_closed = si_run_demo("unit-convert".into(), "assess".into(), "1m".into())
                .expect_err("collected but not activated");
            assert_eq!(not_closed, InstrumentError::NotClosed.to_string());
        });
    }

    #[test]
    fn host_entry_point_is_refused() {
        with_fresh_host(|| {
            si_collect_demo("unit-convert".into()).expect("collect");
            si_activate_demo("unit-convert".into()).expect("activate");
            let err = si_run_demo("unit-convert".into(), "Host.assess".into(), "1m".into())
                .expect_err("Host. must be refused");
            assert_eq!(err, "unknown entry point (not a Host ID)");
        });
    }

    #[test]
    fn suspend_blocks_run_remove_drops_collect() {
        with_fresh_host(|| {
            si_collect_demo("unit-convert".into()).expect("collect");
            si_activate_demo("unit-convert".into()).expect("activate");
            si_suspend_demo("unit-convert".into()).expect("suspend");
            let err = si_run_demo("unit-convert".into(), "assess".into(), "1m".into())
                .expect_err("suspended");
            assert!(err.contains("suspended"));
            assert_eq!(si_cancel_run("unit-convert".into()).unwrap(), "cancel run");
            si_remove_demo("unit-convert".into()).expect("remove");
            let missing = si_run_demo("unit-convert".into(), "assess".into(), "1m".into())
                .expect_err("removed");
            assert_eq!(missing, "collect first");
        });
    }

    #[test]
    fn permission_denial_is_usable() {
        with_fresh_host(|| {
            si_collect_demo("unit-convert".into()).expect("collect");
            si_activate_demo("unit-convert".into()).expect("activate");
            si_set_run_permitted(false).unwrap();
            let err = si_run_demo("unit-convert".into(), "assess".into(), "1m".into())
                .expect_err("denied");
            assert!(err.contains("permission denial"));
            si_set_run_permitted(true).unwrap();
            si_run_demo("unit-convert".into(), "assess".into(), "1m".into()).expect("granted");
        });
    }

    #[test]
    fn revoke_blocks_run_keeps_receipts() {
        with_fresh_host(|| {
            si_collect_demo("unit-convert".into()).expect("collect");
            si_activate_demo("unit-convert".into()).expect("activate");
            si_run_demo("unit-convert".into(), "assess".into(), "1m".into()).expect("run");
            let before = si_list_receipts("unit-convert".into()).expect("receipts");
            assert!(!before.is_empty());
            si_revoke_demo("unit-convert".into()).expect("revoke");
            let err = si_run_demo("unit-convert".into(), "assess".into(), "1m".into())
                .expect_err("revoked");
            assert!(err.contains("revoked"));
            let after = si_list_receipts("unit-convert".into()).expect("receipts remain");
            assert_eq!(after.len(), before.len());
        });
    }

    #[test]
    fn scripted_si10_walkthrough() {
        with_fresh_host(|| {
            let insp = si_inspect_demo("unit-convert".into()).unwrap();
            assert_eq!(insp["network_fetch"], false);
            si_collect_demo("unit-convert".into()).unwrap();
            si_activate_demo("unit-convert".into()).unwrap();
            offline_resolved_unit_convert().expect("offline plan");
            si_run_demo("unit-convert".into(), "assess".into(), "1m".into()).unwrap();
            assert!(!si_list_receipts("unit-convert".into()).unwrap().is_empty());
            si_set_run_permitted(false).unwrap();
            assert!(si_run_demo("unit-convert".into(), "assess".into(), "x".into()).is_err());
            si_set_run_permitted(true).unwrap();
            si_revoke_demo("unit-convert".into()).unwrap();
            assert!(si_run_demo("unit-convert".into(), "assess".into(), "x".into()).is_err());
            assert!(!si_list_receipts("unit-convert".into()).unwrap().is_empty());
        });
    }

    #[test]
    fn fixture_seed_slugs_match_demo_catalog() {
        let cards = si_list_demos();
        for seed in demo_catalog() {
            assert!(
                cards.iter().any(|c| c.slug == seed.slug && c.entry_point == seed.entry_point),
                "missing {}",
                seed.slug
            );
            assert!(!seed.entry_point.contains("Host."));
        }
    }
}
