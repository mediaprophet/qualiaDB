//! Sanctuary audit **DAG** (vault v2, slice A) — the append-only, per-session-branch log a
//! coercer's actions get recorded into.
//!
//! This module sits *on top of* the crypto primitives in [`super::sanctuary_audit`] — it does not
//! re-implement any of them. Each record carries an opaque `sealed` blob (produced by
//! [`super::sanctuary_audit::seal_to`] in the real design) that only the real lane can
//! [`super::sanctuary_audit::open_sealed`]; the DAG stores it verbatim and never inspects it.
//!
//! # What the DAG guarantees
//!
//! Records are content-addressed and hash-chained with
//! [`chain_hash`](super::sanctuary_audit::chain_hash): a record's `id` is
//! `chain_hash(&parent, &canonical_bytes(record))`, and each record's `parent` is the previous
//! record's `id`. Because [`canonical_bytes`] is a deterministic, unambiguous (length-prefixed)
//! encoding of the *content* fields, any of the following becomes detectable by
//! [`verify_chain`]:
//!
//! * **Rewrite** — changing any content field changes the recomputed `id` ⇒ `Tampered`.
//! * **Reorder / broken link** — a record whose `parent` no longer matches its predecessor's `id`
//!   ⇒ `BrokenLink`.
//! * **Drop** — removing a middle record breaks the successor's parent link ⇒ `BrokenLink`.
//!
//! # What the DAG does *not* guarantee — a deliberate honesty note
//!
//! [`derive_sessions`] groups records by `branch_ref` (one branch per duress-unlock entry point).
//! The number of sessions is the number of *distinct entry-point unlocks*. This is a **proxy**, not
//! a verified head-count of attackers: shared credentials (many people, one branch) and one
//! persistent actor opening many sessions (one person, many branches) both fool it. Treat the count
//! as a loose lower/upper-bound signal, never as evidence of "how many people".

use serde::{Deserialize, Serialize};

use super::sanctuary_audit::{chain_hash, GENESIS_PARENT};

/// The kind of action a session recorded. `Other` carries a free-form label for anything not in the
/// fixed set. Serialized in `snake_case` (e.g. `open_session`, `add_note`, `{"other":"..."}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    /// A (possibly duress) session was opened on a branch.
    OpenSession,
    /// A note was created.
    AddNote,
    /// An existing note was edited.
    EditNote,
    /// A note was deleted.
    DeleteNote,
    /// Anything else, tagged with a caller-supplied label.
    Other(String),
}

impl AuditAction {
    /// A stable single-byte discriminant used only inside [`canonical_bytes`]. This is an internal
    /// wire detail for content-addressing; it is **not** the serde representation and must never be
    /// reordered or reused (doing so would silently change every historical `id`).
    fn tag(&self) -> u8 {
        match self {
            AuditAction::OpenSession => 0,
            AuditAction::AddNote => 1,
            AuditAction::EditNote => 2,
            AuditAction::DeleteNote => 3,
            AuditAction::Other(_) => 4,
        }
    }
}

/// How records are routed once they arrive from a duress session. Serialized in `snake_case`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionMode {
    /// Everything is archived automatically (no human triage step).
    AutoArchive,
    /// Everything waits in an inbox for a human to explicitly keep/archive later.
    ManualTriage,
}

impl Default for RetentionMode {
    fn default() -> Self {
        RetentionMode::AutoArchive
    }
}

/// One append-only node in the audit DAG.
///
/// `id` is the content-address: `id == chain_hash(&parent, &canonical_bytes(self))`. The `sealed`
/// field is an opaque blob (a [`super::sanctuary_audit::seal_to`] output in the real lane); the DAG
/// never reads it. Construct with [`AuditRecord::new`] so `id` is always computed consistently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditRecord {
    /// Content-address of this record (`chain_hash(parent, canonical_bytes)`).
    pub id: [u8; 32],
    /// The `id` of the previous record on this branch, or [`GENESIS_PARENT`] for the first.
    pub parent: [u8; 32],
    /// Which branch (duress-unlock session) this record belongs to.
    pub branch_ref: String,
    /// DID of the actor as *asserted* by the session (unauthenticated; a duress session may lie).
    pub actor_did: String,
    /// Optional asserted role.
    pub role: Option<String>,
    /// Optional asserted purpose for the action.
    pub stated_purpose: Option<String>,
    /// What happened.
    pub action: AuditAction,
    /// Unix seconds (u32) when the action was recorded.
    pub unix: u32,
    /// Opaque sealed payload — stored verbatim, never inspected by the DAG.
    pub sealed: Vec<u8>,
}

/// Push a length-prefixed byte string: `u32-LE length ‖ bytes`. Length-prefixing every variable
/// field is what makes the encoding unambiguous — `("ab","c")` and `("a","bc")` cannot collide.
fn push_lp(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
}

/// Push an `Option<&str>` as a 1-byte presence flag followed (if present) by a length-prefixed
/// string. This keeps `None` and `Some("")` distinct.
fn push_opt(out: &mut Vec<u8>, value: Option<&str>) {
    match value {
        None => out.push(0),
        Some(s) => {
            out.push(1);
            push_lp(out, s.as_bytes());
        }
    }
}

/// The deterministic content encoding hashed into a record's `id`. Encodes, in a fixed order and
/// with unambiguous framing, every content field **except `id`**:
/// `branch_ref, actor_did, role, stated_purpose, action, unix, sealed`.
///
/// The encoding is intentionally hand-rolled (not `serde`/CBOR) so the content-address is stable
/// and independent of any serialization crate's version or config. Any change to any content field
/// changes these bytes and therefore the `id`.
pub fn canonical_bytes(record: &AuditRecord) -> Vec<u8> {
    let mut out = Vec::new();
    push_lp(&mut out, record.branch_ref.as_bytes());
    push_lp(&mut out, record.actor_did.as_bytes());
    push_opt(&mut out, record.role.as_deref());
    push_opt(&mut out, record.stated_purpose.as_deref());
    // action: discriminant byte, then (for `Other`) its label length-prefixed.
    out.push(record.action.tag());
    if let AuditAction::Other(label) = &record.action {
        push_lp(&mut out, label.as_bytes());
    }
    out.extend_from_slice(&record.unix.to_le_bytes());
    push_lp(&mut out, &record.sealed);
    out
}

impl AuditRecord {
    /// Build a record and compute its content-address `id` from `parent` and the record's content.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        parent: [u8; 32],
        branch_ref: impl Into<String>,
        actor_did: impl Into<String>,
        role: Option<String>,
        stated_purpose: Option<String>,
        action: AuditAction,
        unix: u32,
        sealed: Vec<u8>,
    ) -> Self {
        let mut record = AuditRecord {
            id: [0u8; 32],
            parent,
            branch_ref: branch_ref.into(),
            actor_did: actor_did.into(),
            role,
            stated_purpose,
            action,
            unix,
            sealed,
        };
        record.id = chain_hash(&record.parent, &canonical_bytes(&record));
        record
    }

    /// Recompute this record's content-address from its own content fields. Equals `id` iff the
    /// record has not been tampered with.
    pub fn recomputed_id(&self) -> [u8; 32] {
        chain_hash(&self.parent, &canonical_bytes(self))
    }
}

/// Result of verifying one branch's chain integrity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChainStatus {
    /// Every record's `id` recomputes and every parent link matches.
    Ok,
    /// The record at `at_index` has a content field that does not match its `id` (rewritten).
    Tampered { at_index: usize },
    /// The record at `at_index` does not link to its predecessor's `id` (reorder / drop / forged
    /// parent). For `at_index == 0` this means the first record's parent is not [`GENESIS_PARENT`].
    BrokenLink { at_index: usize },
}

/// Verify the integrity of a single branch's records, assumed in chain order.
///
/// * index 0's `parent` must be [`GENESIS_PARENT`] (else `BrokenLink { at_index: 0 }`);
/// * each record's `id` must equal its recomputed content-address (else `Tampered`);
/// * each record's `parent` must equal the previous record's `id` (else `BrokenLink`).
///
/// The `Tampered` check runs before the link check at each index, so a rewritten record is reported
/// as tampering rather than as the broken link it would also cause downstream. An empty branch is
/// vacuously [`ChainStatus::Ok`].
pub fn verify_chain(branch: &[AuditRecord]) -> ChainStatus {
    let mut prev_id: Option<[u8; 32]> = None;
    for (i, record) in branch.iter().enumerate() {
        // Content integrity: does the stored id match the content?
        if record.id != record.recomputed_id() {
            return ChainStatus::Tampered { at_index: i };
        }
        // Link integrity: does parent point at the right predecessor?
        match prev_id {
            None => {
                if record.parent != GENESIS_PARENT {
                    return ChainStatus::BrokenLink { at_index: i };
                }
            }
            Some(expected_parent) => {
                if record.parent != expected_parent {
                    return ChainStatus::BrokenLink { at_index: i };
                }
            }
        }
        prev_id = Some(record.id);
    }
    ChainStatus::Ok
}

/// A derived view of one branch as a session. `records` are ordered by chain linkage where the
/// branch is well-formed, falling back to `unix` order otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// The branch this session corresponds to.
    pub branch_ref: String,
    /// Earliest `unix` seen on the branch (the unlock time, in practice).
    pub opened_unix: u32,
    /// The branch's records, ordered.
    pub records: Vec<AuditRecord>,
    /// Number of records / actions on the branch (`== records.len()`).
    pub action_count: usize,
}

/// Order a branch's records by hash linkage: start at the record whose parent is
/// [`GENESIS_PARENT`], then repeatedly follow `parent -> id`. If the branch is not a clean single
/// chain (missing genesis, fork, or dangling parent), fall back to a stable `unix`-then-`id` sort so
/// the function is total and never panics or loops forever.
fn order_branch(mut records: Vec<AuditRecord>) -> Vec<AuditRecord> {
    use std::collections::HashMap;

    // Index by parent so we can walk the chain. A well-formed branch has exactly one record per
    // distinct parent value.
    let mut by_parent: HashMap<[u8; 32], usize> = HashMap::with_capacity(records.len());
    let mut duplicate_parent = false;
    for (i, r) in records.iter().enumerate() {
        if by_parent.insert(r.parent, i).is_some() {
            duplicate_parent = true;
        }
    }

    let can_chain = !duplicate_parent && by_parent.contains_key(&GENESIS_PARENT);
    if can_chain {
        let mut ordered = Vec::with_capacity(records.len());
        let mut visited = vec![false; records.len()];
        let mut cursor = GENESIS_PARENT;
        while let Some(&idx) = by_parent.get(&cursor) {
            if visited[idx] {
                break; // defensive: a cycle — bail to the fallback below.
            }
            visited[idx] = true;
            cursor = records[idx].id;
            ordered.push(idx);
        }
        if ordered.len() == records.len() {
            // Reassemble in linked order. Move out of `records` by taking indices in order.
            let mut slots: Vec<Option<AuditRecord>> = records.into_iter().map(Some).collect();
            return ordered
                .into_iter()
                .map(|i| slots[i].take().expect("each index visited once"))
                .collect();
        }
    }

    // Fallback: stable order by (unix, id).
    records.sort_by(|a, b| a.unix.cmp(&b.unix).then_with(|| a.id.cmp(&b.id)));
    records
}

/// Group records into sessions, one per distinct `branch_ref`.
///
/// Branches are emitted in first-seen order (the order their first record appears in `records`), so
/// the result is deterministic. Each branch's records are ordered by [`order_branch`], `opened_unix`
/// is the minimum `unix` on the branch, and `action_count == records.len()`.
///
/// **Honesty note (read this):** the number of returned sessions is the number of distinct
/// *entry-point unlocks*, which is only a **proxy** for the number of attackers — see the module
/// docs. Shared credentials or one persistent actor across branches both defeat a naive head-count.
pub fn derive_sessions(records: &[AuditRecord]) -> Vec<Session> {
    use std::collections::HashMap;

    // Preserve first-seen branch order for determinism.
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<AuditRecord>> = HashMap::new();
    for r in records {
        if !groups.contains_key(&r.branch_ref) {
            order.push(r.branch_ref.clone());
        }
        groups.entry(r.branch_ref.clone()).or_default().push(r.clone());
    }

    order
        .into_iter()
        .map(|branch_ref| {
            let branch = groups.remove(&branch_ref).unwrap_or_default();
            let ordered = order_branch(branch);
            let opened_unix = ordered.iter().map(|r| r.unix).min().unwrap_or(0);
            let action_count = ordered.len();
            Session {
                branch_ref,
                opened_unix,
                records: ordered,
                action_count,
            }
        })
        .collect()
}

/// The outcome of retention routing: records destined for the archive vs. the human-triage inbox.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Routing {
    /// Records placed straight into the archive.
    pub archived: Vec<AuditRecord>,
    /// Records held for a human to triage (keep/archive later).
    pub inbox: Vec<AuditRecord>,
}

/// Route records according to the retention policy.
///
/// * [`RetentionMode::AutoArchive`] — every record goes to `archived`; `inbox` is empty.
/// * [`RetentionMode::ManualTriage`] — every record goes to `inbox`; nothing is archived until an
///   explicit later keep decision.
///
/// This is a pure partition of the input (every record lands in exactly one bucket), so re-running
/// it on `archived ∪ inbox` with the same mode is idempotent.
pub fn route(records: Vec<AuditRecord>, mode: RetentionMode) -> Routing {
    match mode {
        RetentionMode::AutoArchive => Routing {
            archived: records,
            inbox: Vec::new(),
        },
        RetentionMode::ManualTriage => Routing {
            archived: Vec::new(),
            inbox: records,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::sanctuary_audit::{open_sealed, seal_to, AuditKeypair};

    fn rec(parent: [u8; 32], branch: &str, action: AuditAction, unix: u32) -> AuditRecord {
        AuditRecord::new(
            parent,
            branch,
            "did:example:actor",
            Some("editor".into()),
            Some("logging".into()),
            action,
            unix,
            vec![1, 2, 3],
        )
    }

    /// A well-formed branch of `n` records starting from genesis. Returns them in chain order.
    fn build_branch(branch: &str, n: u32) -> Vec<AuditRecord> {
        let mut out = Vec::new();
        let mut parent = GENESIS_PARENT;
        for i in 0..n {
            let action = if i == 0 {
                AuditAction::OpenSession
            } else {
                AuditAction::AddNote
            };
            let r = rec(parent, branch, action, 1000 + i);
            parent = r.id;
            out.push(r);
        }
        out
    }

    #[test]
    fn new_computes_stable_id() {
        let a = rec(GENESIS_PARENT, "b1", AuditAction::OpenSession, 1000);
        // Recomputing from the record's own content yields the same id.
        assert_eq!(a.id, a.recomputed_id());
        // The genesis record links to GENESIS_PARENT.
        assert_eq!(a.parent, GENESIS_PARENT);
        assert_ne!(a.id, [0u8; 32]);
    }

    #[test]
    fn identical_inputs_yield_identical_id() {
        let a = rec(GENESIS_PARENT, "b1", AuditAction::AddNote, 1234);
        let b = rec(GENESIS_PARENT, "b1", AuditAction::AddNote, 1234);
        assert_eq!(a.id, b.id);
    }

    #[test]
    fn any_field_change_changes_id() {
        let base = rec(GENESIS_PARENT, "b1", AuditAction::AddNote, 1234);

        let diff_branch = rec(GENESIS_PARENT, "b2", AuditAction::AddNote, 1234);
        assert_ne!(base.id, diff_branch.id);

        let diff_action = rec(GENESIS_PARENT, "b1", AuditAction::EditNote, 1234);
        assert_ne!(base.id, diff_action.id);

        let diff_unix = rec(GENESIS_PARENT, "b1", AuditAction::AddNote, 1235);
        assert_ne!(base.id, diff_unix.id);

        let diff_parent = rec([9u8; 32], "b1", AuditAction::AddNote, 1234);
        assert_ne!(base.id, diff_parent.id);

        // role / stated_purpose / actor_did / sealed all feed the address too.
        let diff_role = AuditRecord::new(
            GENESIS_PARENT,
            "b1",
            "did:example:actor",
            Some("viewer".into()),
            Some("logging".into()),
            AuditAction::AddNote,
            1234,
            vec![1, 2, 3],
        );
        assert_ne!(base.id, diff_role.id);

        let diff_sealed = AuditRecord::new(
            GENESIS_PARENT,
            "b1",
            "did:example:actor",
            Some("editor".into()),
            Some("logging".into()),
            AuditAction::AddNote,
            1234,
            vec![9, 9, 9],
        );
        assert_ne!(base.id, diff_sealed.id);
    }

    #[test]
    fn canonical_bytes_is_unambiguous_across_field_boundaries() {
        // "ab"+"c" vs "a"+"bc" for (branch_ref, actor_did) must not collide thanks to length prefixes.
        let x = AuditRecord::new(
            GENESIS_PARENT, "ab", "c", None, None, AuditAction::AddNote, 1, vec![],
        );
        let y = AuditRecord::new(
            GENESIS_PARENT, "a", "bc", None, None, AuditAction::AddNote, 1, vec![],
        );
        assert_ne!(x.id, y.id);
    }

    #[test]
    fn none_and_empty_option_are_distinct() {
        let none = AuditRecord::new(
            GENESIS_PARENT, "b", "a", None, None, AuditAction::AddNote, 1, vec![],
        );
        let empty = AuditRecord::new(
            GENESIS_PARENT, "b", "a", Some(String::new()), None, AuditAction::AddNote, 1, vec![],
        );
        assert_ne!(none.id, empty.id);
    }

    #[test]
    fn well_formed_branch_verifies_ok() {
        let branch = build_branch("session-1", 3);
        assert_eq!(verify_chain(&branch), ChainStatus::Ok);
    }

    #[test]
    fn empty_branch_is_ok() {
        assert_eq!(verify_chain(&[]), ChainStatus::Ok);
    }

    #[test]
    fn first_record_not_from_genesis_is_broken_link() {
        let mut branch = build_branch("s", 2);
        // Forge record 0's parent away from genesis, then repair its id so it isn't flagged as
        // Tampered first — this isolates the genesis-link check.
        branch[0].parent = [5u8; 32];
        branch[0].id = branch[0].recomputed_id();
        assert_eq!(verify_chain(&branch), ChainStatus::BrokenLink { at_index: 0 });
    }

    #[test]
    fn tampering_payload_is_detected_as_tampered() {
        let mut branch = build_branch("session-1", 3);
        // Rewrite record 1's content WITHOUT updating its stored id => content no longer matches.
        branch[1].sealed = vec![0xFF, 0xEE];
        assert_eq!(verify_chain(&branch), ChainStatus::Tampered { at_index: 1 });
    }

    #[test]
    fn tampering_and_resealing_id_surfaces_as_broken_link() {
        // If an attacker rewrites record 1's content AND recomputes its id to hide the tamper, the
        // new id no longer matches record 2's parent — the chain still catches it downstream.
        let mut branch = build_branch("session-1", 3);
        branch[1].sealed = vec![0xFF, 0xEE];
        branch[1].id = branch[1].recomputed_id();
        assert_eq!(verify_chain(&branch), ChainStatus::BrokenLink { at_index: 2 });
    }

    #[test]
    fn swapping_parent_of_record_2_is_broken_link() {
        let mut branch = build_branch("session-1", 3);
        // Point record 2 at the wrong predecessor, repairing its id so the link check (not the
        // tamper check) is what fires.
        branch[2].parent = [7u8; 32];
        branch[2].id = branch[2].recomputed_id();
        assert_eq!(verify_chain(&branch), ChainStatus::BrokenLink { at_index: 2 });
    }

    #[test]
    fn dropping_the_middle_record_is_detected() {
        let branch = build_branch("session-1", 3);
        let dropped = vec![branch[0].clone(), branch[2].clone()];
        // record[2].parent points at record[1].id, which is now absent => break at index 1.
        assert_eq!(verify_chain(&dropped), ChainStatus::BrokenLink { at_index: 1 });
    }

    #[test]
    fn derive_sessions_two_branches_correct_counts() {
        let mut all = build_branch("branch-A", 3);
        all.extend(build_branch("branch-B", 2));

        let sessions = derive_sessions(&all);
        assert_eq!(sessions.len(), 2);

        let a = sessions.iter().find(|s| s.branch_ref == "branch-A").unwrap();
        let b = sessions.iter().find(|s| s.branch_ref == "branch-B").unwrap();
        assert_eq!(a.action_count, 3);
        assert_eq!(a.records.len(), 3);
        assert_eq!(b.action_count, 2);
        assert_eq!(b.opened_unix, 1000);
        // First-seen order preserved (branch-A appeared first).
        assert_eq!(sessions[0].branch_ref, "branch-A");
    }

    #[test]
    fn derive_sessions_single_branch_ordered_by_linkage() {
        // Feed records out of order; order_branch must reassemble the chain.
        let branch = build_branch("only", 4);
        let shuffled = vec![
            branch[2].clone(),
            branch[0].clone(),
            branch[3].clone(),
            branch[1].clone(),
        ];
        let sessions = derive_sessions(&shuffled);
        assert_eq!(sessions.len(), 1);
        let s = &sessions[0];
        assert_eq!(s.action_count, 4);
        // Reassembled in chain order => verify_chain is Ok on the derived records.
        assert_eq!(verify_chain(&s.records), ChainStatus::Ok);
        assert_eq!(s.opened_unix, 1000);
    }

    #[test]
    fn derive_sessions_empty_input() {
        assert!(derive_sessions(&[]).is_empty());
    }

    #[test]
    fn route_auto_archive_puts_all_in_archived() {
        let recs = build_branch("s", 3);
        let routed = route(recs.clone(), RetentionMode::AutoArchive);
        assert_eq!(routed.archived.len(), 3);
        assert!(routed.inbox.is_empty());
        // Idempotent: re-routing the archived+inbox union yields the same partition.
        let again = route(
            routed.archived.iter().chain(routed.inbox.iter()).cloned().collect(),
            RetentionMode::AutoArchive,
        );
        assert_eq!(again, routed);
    }

    #[test]
    fn route_manual_triage_puts_all_in_inbox() {
        let recs = build_branch("s", 3);
        let routed = route(recs.clone(), RetentionMode::ManualTriage);
        assert_eq!(routed.inbox.len(), 3);
        assert!(routed.archived.is_empty());
        // Idempotent.
        let again = route(
            routed.archived.iter().chain(routed.inbox.iter()).cloned().collect(),
            RetentionMode::ManualTriage,
        );
        assert_eq!(again, routed);
    }

    #[test]
    fn default_retention_mode_is_auto_archive() {
        assert_eq!(RetentionMode::default(), RetentionMode::AutoArchive);
    }

    #[test]
    fn action_and_mode_serde_snake_case() {
        // Sanity that serde renders the documented snake_case wire form.
        assert_eq!(
            serde_json::to_string(&AuditAction::OpenSession).unwrap(),
            "\"open_session\""
        );
        assert_eq!(
            serde_json::to_string(&AuditAction::Other("x".into())).unwrap(),
            "{\"other\":\"x\"}"
        );
        assert_eq!(
            serde_json::to_string(&RetentionMode::ManualTriage).unwrap(),
            "\"manual_triage\""
        );
    }

    #[test]
    fn record_round_trips_through_serde() {
        let r = rec(GENESIS_PARENT, "b1", AuditAction::AddNote, 42);
        let json = serde_json::to_string(&r).unwrap();
        let back: AuditRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
        assert_eq!(back.id, back.recomputed_id());
    }

    #[test]
    fn dag_stores_real_sealed_blob_opaquely_and_real_lane_recovers_it() {
        // Integration with the real crypto lane: seal a payload with the audit *public* key (as a
        // decoy session would), store it in a DAG record, and confirm (a) the DAG treats it as
        // opaque bytes yet content-addresses correctly, and (b) the real lane's secret opens it.
        let kp = AuditKeypair::generate().unwrap();
        let aad = b"branch:duress-1";
        let plaintext = b"coercer added note at 12:04 under duress";
        let sealed = seal_to(&kp.public, plaintext, aad).unwrap();

        let record = AuditRecord::new(
            GENESIS_PARENT,
            "duress-1",
            "did:example:coercer",
            Some("guest".into()),
            None,
            AuditAction::AddNote,
            1717171717,
            sealed.clone(),
        );

        // The DAG stored the blob verbatim (never inspected it) and content-addressed over it.
        assert_eq!(record.sealed, sealed);
        assert_eq!(record.id, record.recomputed_id());
        assert_eq!(verify_chain(std::slice::from_ref(&record)), ChainStatus::Ok);

        // The real lane opens the stored blob with the secret and recovers the plaintext.
        let opened = open_sealed(kp.secret_bytes(), &record.sealed, aad).unwrap();
        assert_eq!(opened, plaintext);

        // The public key alone (what a decoy holds) cannot recover it.
        assert!(open_sealed(&kp.public, &record.sealed, aad).is_err());
    }
}
