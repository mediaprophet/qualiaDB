//! In-process graph backing store for the loopback daemon `/query` route.
//!
//! The live daemon graph is a fixed-capacity, zero-heap store backed by a
//! caller-invisible `[NQuin; MAX_GRAPH_QUINS]` buffer. Cold-path ontology and
//! file ingestion may still allocate while parsing, but the resident graph used
//! by `/query` no longer relies on `Vec` or `HashSet`.

use crate::{q_hash, NQuin};
use std::ops::Index;
use std::path::Path;
use std::sync::RwLock;

/// Bench datasets (Schema.org ~18K quins) must fit for browser/native parity.
pub const MAX_GRAPH_QUINS: usize = 65_536;

#[derive(Debug)]
pub struct DaemonGraphStore {
    quins: [NQuin; MAX_GRAPH_QUINS],
    len: usize,
}

impl DaemonGraphStore {
    pub const fn new() -> Self {
        Self {
            quins: [NQuin {
                subject: 0,
                predicate: 0,
                object: 0,
                context: 0,
                metadata: 0,
                parity: 0,
            }; MAX_GRAPH_QUINS],
            len: 0,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn as_slice(&self) -> &[NQuin] {
        &self.quins[..self.len]
    }

    #[inline]
    pub fn clear(&mut self) {
        for quin in &mut self.quins[..self.len] {
            *quin = NQuin::default();
        }
        self.len = 0;
    }

    #[inline]
    pub fn push(&mut self, quin: NQuin) -> bool {
        if self.len >= MAX_GRAPH_QUINS {
            return false;
        }
        self.quins[self.len] = quin;
        self.len += 1;
        true
    }

    #[inline]
    pub fn extend_from_slice(&mut self, quins: &[NQuin]) -> usize {
        let remaining = MAX_GRAPH_QUINS.saturating_sub(self.len);
        let to_copy = quins.len().min(remaining);
        if to_copy == 0 {
            return 0;
        }
        self.quins[self.len..self.len + to_copy].copy_from_slice(&quins[..to_copy]);
        self.len += to_copy;
        to_copy
    }

    #[inline]
    fn contains_subject_predicate_context(&self, subject: u64, predicate: u64, context: u64) -> bool {
        self.as_slice().iter().any(|q| {
            q.subject == subject && q.predicate == predicate && q.context == context
        })
    }

    fn push_unique(&mut self, quin: NQuin) -> bool {
        if self.contains_subject_predicate_context(quin.subject, quin.predicate, quin.context) {
            return false;
        }
        self.push(quin)
    }
}

impl Default for DaemonGraphStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Index<usize> for DaemonGraphStore {
    type Output = NQuin;

    fn index(&self, index: usize) -> &Self::Output {
        &self.as_slice()[index]
    }
}

static GRAPH: RwLock<DaemonGraphStore> = RwLock::new(DaemonGraphStore::new());

fn graph_lock() -> &'static RwLock<DaemonGraphStore> {
    &GRAPH
}

#[inline]
fn triple_quin(subject: &str, predicate: &str, object: &str, context: &str) -> NQuin {
    let subject = q_hash(subject);
    let predicate = q_hash(predicate);
    let object = q_hash(object);
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

fn push_quin(store: &mut DaemonGraphStore, quin: NQuin) {
    let _ = store.push(quin);
}

/// Seed representative health-condition triples for Anatomy app development.
fn seed_anatomy_health_graph(store: &mut DaemonGraphStore) {
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

fn try_load_index_dir(store: &mut DaemonGraphStore, storage_path: &str) {
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
        if let Ok(quins) = crate::q42_reader::read_c_q42_quins(&path) {
            store.extend_from_slice(&quins);
        }
    }
}

/// Initialise or refresh the daemon graph from storage path.
pub fn init_daemon_graph(storage_path: &str) {
    let lock = graph_lock();
    if let Ok(mut guard) = lock.write() {
        guard.clear();
        seed_anatomy_health_graph(&mut guard);
        try_load_index_dir(&mut guard, storage_path);
    }
}

/// Number of Quins currently available to `/query`.
pub fn graph_quin_count() -> usize {
    graph_lock().read().map(|g| g.len()).unwrap_or(0)
}

/// Read guard over the live graph (lock is process-static via `OnceLock`).
pub fn graph_read_guard() -> std::sync::RwLockReadGuard<'static, DaemonGraphStore> {
    graph_lock().read().expect("daemon graph poisoned")
}

/// Extend the live graph with ontology quins from `qualia-core-db::ontology_loader`.
pub fn extend_with_ontology_quins(quins: Vec<crate::NQuin>) {
    extend_with_ontology_quins_slice(&quins);
}

/// Zero-heap resident update path for ontology insertion.
pub fn extend_with_ontology_quins_slice(quins: &[crate::NQuin]) {
    if quins.is_empty() {
        return;
    }
    let lock = graph_lock();
    if let Ok(mut guard) = lock.write() {
        for &q in quins {
            let _ = guard.push_unique(q);
        }
    }
}

/// Replace the in-memory graph with flat 48-byte NQuin bytes (browser bench_load).
pub fn replace_graph_from_flat_bytes(bytes: &[u8]) -> Result<usize, &'static str> {
    let lock = graph_lock();
    let mut guard = lock.write().map_err(|_| "daemon graph poisoned")?;
    if bytes.is_empty() {
        guard.clear();
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
    guard.clear();
    guard.extend_from_slice(quins);
    Ok(quin_count)
}

/// Known condition subject hashes for Anatomy graph -> label mapping.
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
    use serial_test::serial;

    fn reset_graph_for_test() {
        let lock = graph_lock();
        let mut guard = lock.write().expect("daemon graph poisoned");
        guard.clear();
    }

    #[test]
    #[serial]
    fn seed_graph_has_health_quins() {
        reset_graph_for_test();
        init_daemon_graph("/tmp/qualia-test-graph");
        assert!(graph_quin_count() >= 8);
        reset_graph_for_test();
    }

    #[test]
    #[serial]
    fn replace_graph_from_flat_bytes_round_trip() {
        reset_graph_for_test();
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
        reset_graph_for_test();
    }

    #[test]
    #[serial]
    fn extend_with_ontology_quins_deduplicates_within_single_batch() {
        reset_graph_for_test();
        let quin = triple_quin(
            "http://q.test/s/duplicate",
            "http://q.test/p/duplicate",
            "http://q.test/o/first",
            "did:qualia:test",
        );

        extend_with_ontology_quins_slice(&[quin, quin]);

        let guard = graph_read_guard();
        assert_eq!(guard.len(), 1);
        assert_eq!(guard[0], quin);
        drop(guard);
        reset_graph_for_test();
    }
}
