//! In-process graph backing store for the loopback daemon `/query` route.
//!
//! The live daemon graph is a fixed-capacity, zero-heap store backed by a
//! caller-invisible `[NQuin; MAX_GRAPH_QUINS]` buffer. Cold-path ontology and
//! file ingestion may still allocate while parsing, but the resident graph used
//! by `/query` no longer relies on `Vec` or `HashSet`.

use crate::{q_hash, NQuin};
use std::ops::Index;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{OnceLock, RwLock};
use tokio::sync::broadcast;

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
    fn contains_subject_predicate_context(
        &self,
        subject: u64,
        predicate: u64,
        context: u64,
    ) -> bool {
        self.as_slice()
            .iter()
            .any(|q| q.subject == subject && q.predicate == predicate && q.context == context)
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
static GRAPH_REVISION: AtomicU64 = AtomicU64::new(0);
static REVISION_TX: OnceLock<broadcast::Sender<u64>> = OnceLock::new();

fn graph_lock() -> &'static RwLock<DaemonGraphStore> {
    &GRAPH
}

fn revision_tx() -> &'static broadcast::Sender<u64> {
    REVISION_TX.get_or_init(|| {
        let (tx, _) = broadcast::channel(64);
        tx
    })
}

/// Monotonic Lamport-style revision counter for daemon graph mutations.
#[inline]
pub fn graph_revision() -> u64 {
    GRAPH_REVISION.load(Ordering::Acquire)
}

/// Subscribe to graph revision bumps (used by `GET /tensor/events` SSE).
pub fn subscribe_graph_revisions() -> broadcast::Receiver<u64> {
    revision_tx().subscribe()
}

/// Increment revision and notify SSE subscribers (Release ordering).
pub fn bump_graph_revision() -> u64 {
    let rev = GRAPH_REVISION.fetch_add(1, Ordering::Release) + 1;
    let _ = revision_tx().send(rev);
    rev
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

/// Controls which resident graph layers are seeded at daemon boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InitGraphOptions {
    /// Seed the built-in anatomy/health demo triples.
    pub seed_defaults: bool,
    /// Load `.q42` volumes from `{storage_path}/Index`.
    pub load_index: bool,
}

impl Default for InitGraphOptions {
    fn default() -> Self {
        Self {
            seed_defaults: true,
            load_index: true,
        }
    }
}

/// Initialise or refresh the daemon graph from storage path.
pub fn init_daemon_graph(storage_path: &str) {
    init_daemon_graph_with_options(storage_path, InitGraphOptions::default());
}

/// Initialise or refresh the daemon graph with explicit seeding policy.
pub fn init_daemon_graph_with_options(storage_path: &str, opts: InitGraphOptions) {
    let lock = graph_lock();
    if let Ok(mut guard) = lock.write() {
        guard.clear();
        if opts.seed_defaults {
            seed_anatomy_health_graph(&mut guard);
        }
        if opts.load_index {
            try_load_index_dir(&mut guard, storage_path);
        }
    }
    bump_graph_revision();
}

/// Number of Quins currently available to `/query`.
pub fn graph_quin_count() -> usize {
    graph_lock().read().map(|g| g.len()).unwrap_or(0)
}

/// Read guard over the live graph (lock is process-static via `OnceLock`).
pub fn graph_read_guard() -> std::sync::RwLockReadGuard<'static, DaemonGraphStore> {
    graph_lock().read().expect("daemon graph poisoned")
}

/// Outcome of applying a SPARQL Update to the daemon graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateOutcome {
    /// Number of quins added to the graph.
    pub inserted: u64,
    /// Number of quins removed from the graph.
    pub deleted: u64,
    /// Whether the change was signed and persisted to the WAL (true only when a
    /// real signer callback was supplied). `false` = ephemeral, in-memory only.
    pub persisted: bool,
}

/// Apply a parsed SPARQL Update to the resident daemon graph.
///
/// The mutation is applied under a single write guard (copy → run
/// `UpdateExecutor` → write back), then the graph revision is bumped so
/// subscribers (e.g. WebSocket sessions) are notified.
///
/// `on_change`, if supplied, receives the `(inserted, deleted)` quin sets so the
/// caller can sign and persist them to the WAL with a **real** key (see
/// `wal::commit_semantic_mutation`); `persisted` is then `true`. If it is
/// `None`, the change is applied in memory only (ephemeral) and `persisted` is
/// `false`. A signature is **never fabricated here** — durable, signed mutation
/// requires the caller to supply a real signer, so an irreversible delete can
/// never be committed under a placeholder key.
pub fn apply_sparql_update(
    op: &crate::sparql_library::sparql_update::UpdateOperation,
    ctx: &crate::sparql_ast::SparqlQueryContext,
    on_change: Option<&mut dyn FnMut(&[NQuin], &[NQuin]) -> Result<(), String>>,
) -> Result<UpdateOutcome, String> {
    use crate::sparql_library::sparql_update::UpdateExecutor;

    let lock = graph_lock();
    let mut guard = lock.write().map_err(|_| "daemon graph poisoned".to_string())?;

    let before: Vec<NQuin> = guard.as_slice().to_vec();
    let mut working = before.clone();
    UpdateExecutor::new(&mut working).execute(op, ctx)?;

    // Delta by semantic identity (subject/predicate/object/context), ignoring
    // parity/metadata noise.
    let same = |a: &NQuin, b: &NQuin| {
        a.subject == b.subject
            && a.predicate == b.predicate
            && a.object == b.object
            && a.context == b.context
    };
    let inserted: Vec<NQuin> = working
        .iter()
        .filter(|w| !before.iter().any(|b| same(w, b)))
        .copied()
        .collect();
    let deleted: Vec<NQuin> = before
        .iter()
        .filter(|b| !working.iter().any(|w| same(w, b)))
        .copied()
        .collect();

    guard.clear();
    guard.extend_from_slice(&working);
    drop(guard);
    bump_graph_revision();

    let persisted = if let Some(cb) = on_change {
        // The graph is already updated; the callback makes it durable. If it
        // fails, the in-memory change stands but is reported as not persisted.
        cb(&inserted, &deleted).map_err(|e| format!("persist failed: {e}"))?;
        true
    } else {
        false
    };

    Ok(UpdateOutcome {
        inserted: inserted.len() as u64,
        deleted: deleted.len() as u64,
        persisted,
    })
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
        let before = guard.len();
        for &q in quins {
            let _ = guard.push_unique(q);
        }
        if guard.len() > before {
            bump_graph_revision();
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
    bump_graph_revision();
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

    #[test]
    #[serial]
    fn init_daemon_graph_bumps_revision() {
        reset_graph_for_test();
        let before = graph_revision();
        init_daemon_graph("/tmp/qualia-test-graph");
        assert!(graph_revision() > before);
        reset_graph_for_test();
    }

    #[test]
    #[serial]
    fn extend_with_ontology_quins_bumps_revision_only_when_added() {
        reset_graph_for_test();
        let quin = triple_quin(
            "http://q.test/s/rev",
            "http://q.test/p/rev",
            "http://q.test/o/rev",
            "did:qualia:test",
        );
        let before = graph_revision();
        extend_with_ontology_quins_slice(&[quin]);
        assert!(graph_revision() > before);

        let unchanged = graph_revision();
        extend_with_ontology_quins_slice(&[quin]);
        assert_eq!(graph_revision(), unchanged);
        reset_graph_for_test();
    }

    #[test]
    #[serial]
    fn replace_graph_from_flat_bytes_bumps_revision() {
        reset_graph_for_test();
        let quin = triple_quin(
            "http://q.test/s/replace",
            "http://q.test/p/replace",
            "http://q.test/o/replace",
            "did:qualia:test",
        );
        let before = graph_revision();
        let bytes = bytemuck::bytes_of(&quin);
        replace_graph_from_flat_bytes(bytes).expect("load flat quin");
        assert!(graph_revision() > before);
        reset_graph_for_test();
    }

    fn parse_upd(src: &str) -> (crate::sparql_ast::SparqlQueryContext, crate::sparql_library::sparql_update::UpdateOperation) {
        let mut ctx = crate::sparql_ast::SparqlQueryContext::new();
        let op = crate::sparql_library::sparql_grammar::parse_update(
            src,
            &mut ctx,
            &std::collections::HashMap::new(),
        )
        .unwrap();
        (ctx, op)
    }

    #[test]
    #[serial]
    fn apply_update_insert_grows_graph_ephemeral() {
        reset_graph_for_test();
        let (ctx, op) =
            parse_upd("INSERT DATA { <http://q.test/s1> <http://q.test/p1> <http://q.test/o1> }");
        let before = graph_quin_count();
        let rev_before = graph_revision();
        let out = apply_sparql_update(&op, &ctx, None).unwrap();
        assert_eq!(out.inserted, 1);
        assert!(!out.persisted, "no signer callback → ephemeral, not persisted");
        assert_eq!(graph_quin_count(), before + 1);
        assert!(graph_revision() > rev_before, "revision bumped for subscribers");
        reset_graph_for_test();
    }

    #[test]
    #[serial]
    fn apply_update_delete_removes_and_signer_sees_delta() {
        reset_graph_for_test();
        let (ictx, iop) =
            parse_upd("INSERT DATA { <http://q.test/s2> <http://q.test/p2> <http://q.test/o2> }");
        apply_sparql_update(&iop, &ictx, None).unwrap();
        let seeded = graph_quin_count();

        let (dctx, dop) =
            parse_upd("DELETE DATA { <http://q.test/s2> <http://q.test/p2> <http://q.test/o2> }");
        let mut captured_deleted = 0usize;
        let mut cb = |_ins: &[NQuin], del: &[NQuin]| -> Result<(), String> {
            captured_deleted = del.len();
            Ok(())
        };
        let out = apply_sparql_update(&dop, &dctx, Some(&mut cb)).unwrap();
        assert_eq!(out.deleted, 1);
        assert!(out.persisted, "signer callback supplied → persisted");
        assert_eq!(captured_deleted, 1, "callback received the deleted quin");
        assert_eq!(graph_quin_count(), seeded - 1);
        reset_graph_for_test();
    }
}
