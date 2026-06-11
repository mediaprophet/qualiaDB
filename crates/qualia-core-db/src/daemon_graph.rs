//! In-process graph backing store for the loopback daemon `/query` route.
//!
//! Phase 1: fixed-capacity `Vec<NQuin>` seeded with Anatomy health demo
//! triples and optionally extended from `{storage_path}/Index/*.q42` headers.

use crate::{q_hash, NQuin};
use std::path::Path;
use std::sync::{OnceLock, RwLock};

/// Bench datasets (Schema.org ~18K quins) must fit for browser/native parity.
const MAX_GRAPH_QUINS: usize = 65_536;

static GRAPH: OnceLock<RwLock<Vec<NQuin>>> = OnceLock::new();

fn graph_lock() -> &'static RwLock<Vec<NQuin>> {
    GRAPH.get_or_init(|| RwLock::new(Vec::new()))
}

#[inline]
fn triple_quin(subject: &str, predicate: &str, object: &str, context: &str) -> NQuin {
    let subject = q_hash(subject);
    let predicate = q_hash(predicate);
    let object = q_hash(object);
    // Keep sensitivity lane public — q_hash may set bits [56..63].
    let context = q_hash(context) & 0x00FF_FFFF_FFFF_FFFF;
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata: 0,
        parity: subject ^ predicate ^ object ^ context,
    }
}

fn push_quin(store: &mut Vec<NQuin>, quin: NQuin) {
    if store.len() < MAX_GRAPH_QUINS {
        store.push(quin);
    }
}

/// Seed representative health-condition triples for Anatomy app development.
fn seed_anatomy_health_graph(store: &mut Vec<NQuin>) {
    const BIO: &str = "https://qualia.anatomy.example/ontology/bio#";
    const ORGAN: &str = "https://qualia.anatomy.example/ontology/organ#";
    const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    const HAS_PRIMARY: &str =
        "https://qualia.anatomy.example/ontology/impact#hasPrimaryImpactSystem";
    const IMPACTS: &str = "https://qualia.anatomy.example/ontology/impact#Impacts";
    const USER_CTX: &str = "did:qualia:user:local-health-graph";

    let seeds: [(&str, &str); 8] = [
        ("Type2Diabetes", "organ:EndocrineSystem"),
        ("Hypertension", "organ:CirculatorySystem"),
        ("ChronicKidneyDisease", "organ:UrinarySystem"),
        ("HeartFailure", "organ:CirculatorySystem"),
        ("COPD", "organ:RespiratorySystem"),
        ("Obesity", "organ:EndocrineSystem"),
        ("AtrialFibrillation", "organ:CirculatorySystem"),
        ("Depression", "organ:NervousSystem"),
    ];

    for (local_name, primary_system) in seeds {
        let condition = format!("{BIO}{local_name}");
        push_quin(
            store,
            triple_quin(&condition, RDF_TYPE, &format!("{BIO}Condition"), USER_CTX),
        );
        push_quin(
            store,
            triple_quin(
                &condition,
                HAS_PRIMARY,
                &format!("{ORGAN}{}", primary_system.trim_start_matches("organ:")),
                USER_CTX,
            ),
        );
        // Secondary impact edge for graph richness (re-use primary system).
        push_quin(
            store,
            triple_quin(
                &condition,
                IMPACTS,
                &format!("{ORGAN}{}", primary_system.trim_start_matches("organ:")),
                USER_CTX,
            ),
        );
    }
}

fn try_load_index_dir(store: &mut Vec<NQuin>, storage_path: &str) {
    let index = Path::new(storage_path).join("Index");
    let Ok(entries) = std::fs::read_dir(&index) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("q42") {
            continue;
        }
        if path
            .file_name()
            .map(|n| n.to_string_lossy().contains(".meta."))
            .unwrap_or(false)
        {
            continue;
        }
        // Compatibility path: some local datasets are still legacy framed
        // compressed artifacts even though canonical raw `.q42` is block-based.
        if let Ok(quins) = crate::q42_reader::read_c_q42_quins(&path) {
            for quin in quins {
                push_quin(store, quin);
            }
        }
    }
}

/// Initialise or refresh the daemon graph from storage path.
pub fn init_daemon_graph(storage_path: &str) {
    let mut store = Vec::with_capacity(64);
    seed_anatomy_health_graph(&mut store);
    try_load_index_dir(&mut store, storage_path);
    let lock = graph_lock();
    if let Ok(mut guard) = lock.write() {
        *guard = store;
    }
}

/// Number of Quins currently available to `/query`.
pub fn graph_quin_count() -> usize {
    graph_lock().read().map(|g| g.len()).unwrap_or(0)
}

/// Read guard over the live graph (lock is process-static via `OnceLock`).
pub fn graph_read_guard() -> std::sync::RwLockReadGuard<'static, Vec<NQuin>> {
    graph_lock().read().expect("daemon graph poisoned")
}

/// Extend the live graph with ontology quins from `qualia-core-db::ontology_loader`.
///
/// Called at daemon startup after `init_daemon_graph`.  Quins are deduplicated by
/// `(subject, predicate, context)` to avoid re-seeding across restarts.
pub fn extend_with_ontology_quins(quins: Vec<crate::NQuin>) {
    if quins.is_empty() { return; }
    let lock = graph_lock();
    if let Ok(mut guard) = lock.write() {
        let existing: std::collections::HashSet<(u64, u64, u64)> = guard
            .iter()
            .map(|q| (q.subject, q.predicate, q.context))
            .collect();
        for q in quins {
            if !existing.contains(&(q.subject, q.predicate, q.context)) {
                guard.push(q);
            }
        }
    }
}

/// Replace the in-memory graph with flat 48-byte NQuin bytes (browser bench_load).
pub fn replace_graph_from_flat_bytes(bytes: &[u8]) -> Result<usize, &'static str> {
    if bytes.is_empty() {
        let lock = graph_lock();
        if let Ok(mut guard) = lock.write() {
            guard.clear();
        }
        return Ok(0);
    }
    if bytes.len() % 48 != 0 {
        return Err("db_bytes length must be a multiple of 48");
    }
    let quin_count = bytes.len() / 48;
    if quin_count > MAX_GRAPH_QUINS {
        return Err("graph exceeds daemon MAX_GRAPH_QUINS");
    }
    let quins: &[NQuin] = bytemuck::cast_slice(bytes);
    let lock = graph_lock();
    let mut guard = lock.write().map_err(|_| "daemon graph poisoned")?;
    guard.clear();
    guard.extend_from_slice(quins);
    Ok(quin_count)
}

/// Known condition subject hashes for Anatomy graph → label mapping.
pub fn condition_label_for_subject_hash(subject: u64) -> Option<&'static str> {
    const BIO: &str = "https://qualia.anatomy.example/ontology/bio#";
    const TABLE: [(&str, &str); 8] = [
        ("Type2Diabetes", "Type 2 Diabetes Mellitus"),
        ("Hypertension", "Hypertension"),
        ("ChronicKidneyDisease", "Chronic Kidney Disease (CKD)"),
        ("HeartFailure", "Heart Failure"),
        ("COPD", "Chronic Obstructive Pulmonary Disease (COPD)"),
        ("Obesity", "Obesity"),
        ("AtrialFibrillation", "Atrial Fibrillation"),
        ("Depression", "Major Depressive Disorder"),
    ];

    for (local, label) in TABLE {
        if q_hash(&format!("{BIO}{local}")) == subject {
            return Some(label);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_graph_has_health_quins() {
        init_daemon_graph("/tmp/qualia-test-graph");
        assert!(graph_quin_count() >= 8);
    }

    #[test]
    fn replace_graph_from_flat_bytes_round_trip() {
        let quin = triple_quin(
            "http://q.test/s/0",
            "http://q.test/p/0",
            "http://q.test/o/0",
            "did:qualia:test",
        );
        let bytes = bytemuck::bytes_of(&quin);
        let count = replace_graph_from_flat_bytes(bytes).expect("load flat quin");
        assert_eq!(count, 1);
        assert_eq!(graph_quin_count(), 1);
    }
}
