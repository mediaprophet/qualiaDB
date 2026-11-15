# QBS-11 verified-storage inventory and proposed contract

2026-11-12. **Read-only inventory and contract proposal; awaiting root alignment before implementation.** No storage source, parent runtime, dependency, status or registry edits; no builds/tests/network actions. Inspected actual implementations below, not capability names alone. This scope does not re-audit unrelated geometry or inference kernels.

## Reusable owners and their actual bounds

Paths in this table are relative to QualiaDB crates/qualia-core-db/src unless Etalon is named.

| Existing owner/export | Inspected behavior / appropriate reuse |
|---|---|
| storage_driver::StorageDriver::{write,append,read,read_range,delete,snapshot,flush}; open_storage | storage_driver.rs:143–172,840. Canonical backend/root selection should remain the owner. A QBS facade must borrow it, not open an independent root or copy quota limits. MmapDriver::read maps then to_vec (259–268); read_range loads the whole value before slicing (272–281); write/append use ordinary fs writes (242–255); flush is currently no-op (308–310). These are real implementations but are not bounded large-range IO or evidence of durable data-before-checkpoint ordering. Driver max_writers is capability metadata, not proof of a held writer-exclusion lease. File is1051 lines: new backend behavior needs a focused child module/owner decomposition, not more mixed lifecycle code. |
| storage::{NativeVfs,OpfsVfs,VirtualFileSystem} | storage.rs:160–303. Existing root/browser plumbing. Native read uses fs::read; write clones input into Vec (193–209). OPFS reads the complete File ArrayBuffer into Vec; write clones the complete input then createWritable/write/close (219–300). Do not call these whole-value surfaces as if they were bounded random-range methods. |
| net::qdnf::replication::{PagedFile,PageWindow,hash_file_pages,verify_file_pages} | replication/stream.rs:18–107,159–206: actual u64 logical length and caller-owned4KiB window; useful paged-read/hash mechanics. Sequential full SHA384, not per-entry QBS proof/verified-bitmap admission. Keep its PAGE_BYTES/global behavior unchanged. |
| net::peer::replication::{CacheTable,cache_key,insert,lookup,evict} | cache.rs:1–5,11,63–150. Canonical16-slot cache keys bind scope/source-generation/policy-generation/ABI. Reuse as a hot lookup front; hit/miss/eviction never establishes full-object completeness or absence. Store full QBS identity in the referenced manifest and compare it after lookup; do not truncate descriptor/root/privacy identity into a u64 scope/ABI tag and treat that as authority. Do not raise MAX_CACHE. |
| peer replication::{commit_verified_block,resume_from_log,stage_paired,commit_pair,recover_pair}; wal_intent::IntentTable | commit_adapter.rs:22–138; pair.rs:1–25,72–207; wal_intent.rs:10–12,133–212. Existing verify-before-effect / intent+effect+receipt pairing and replay separation. Pairing expressly remains an in-memory CORE03 contract, not OS-kill disk proof. MAX_INTENTS32, MAX_OBJECT_BYTES512; a QBS small checkpoint/control record may fit, a16KiB leaf does not. No inspected public retirement/compaction path establishes an unbounded leaf-history journal; keep parent limits and add a qualified paged checkpoint bridge rather than overwrite replay history. |
| peer replication::disk_crash::FilePairStore; qdnf replication::FilePairStore | peer disk_crash.rs:17,38–146 and qdnf sibling:21,57–147. Actual identity/effect/receipt files and sync_all ordering, bounded4KiB effects. Recovery refuses receipt without matching complete effect bytes. Reuse fault fixtures/ordering concepts; neither adapter is an existing large sparse cache or qualified process-kill WAL. Their own qualification functions remain false. |
| peer connectivity::DurableQueue::{persist,recover}; durable::{Delivery,DurableJob} | durable.rs:4–22: eight jobs,256-byte blobs and staged delivery states. durable_store.rs:22–55 writes a temporary file then rename, with no sync_all/data/directory barrier; recover uses unbounded read_to_end before count validation and CRC checks. Keep delivery semantics; do not repurpose this persistence as QBS verified-range durability or treat CRC as authenticated local state. |
| peer replication::custody::{CustodyState,mark_stored,mark_delivered,mark_application_acked} | custody.rs:1–55 separates byte presence, delivery and application acknowledgement. Reuse distinctions. mark_stored's boolean presence contract alone is too weak to mint a QBS proof/durable-custody handle. Transport receipt is not settlement. |
| sensing::local_custody::{publish_bundle,read_bundle}; LocalHeadLease::{acquire,current,compare_and_commit}; LocalChainAuthority | local_custody/mod.rs:1–10; publish.rs:25–80; local_head.rs:81–142 and compare_and_commit. Real protected-root checks, final-component nofollow/reparse checks, held cooperative File::try_lock, TempDir staging, sync_all, non-replacing hard-link publication and transactional head pattern. Local-head Unix directory sync determines Committed vs Indeterminate; non-Unix returns Indeterminate. Root ancestors/locality/rollback/lease coordination remain host responsibilities. Reuse filesystem lifecycle through its owner/extracted generic helper, not by copying another lock/temp/cleanup implementation into QBDL. The sensing receipt/chain format and direct SigningKey path are not a QBDL/PQ checkpoint format or an approved signing facade. |
| sensing::local_custody::quota::{CustodyQuota,from_authenticated_usage,charged_usage,publish_bundle_with_quota} | quota.rs:1–6,49–122,169–214. Existing single-root aggregate admission from authenticated usage; caller must hold the quota lease for its lifetime. Charges survive error/unwind; no unsafe automatic refund or fresh empty ledger after restart. This is cooperative in-process accounting, explicitly not durable cross-process quota/physical allocation enforcement. Current debit is sensing-publication-specific/private; QBS needs an owner-approved generic adapter to the **same** quota authority, not another quota instantiated from the same limits. |
| Etalon etalon-model-io::lease::{ModelLease,LeaseLimits,LeaseIdentity} | lease/mod.rs:1–14; backing.rs:29–47,55–147; platform.rs:77–116. Strong existing complete-byte consumer owner on local Windows NTFS: retained exclusion handle uses share_mode(1), opened-handle backing gate, whole-byte identity and borrowed mapping. No scratch source copy; lifetime must retain lease/Arc<ModelLease>. Non-Windows returns UnsupportedPlatform. Great reuse for completed P64 entry consumption, not a Linux/OPFS range store or PQ authority by SHA256 alone. Recheck outer QBS proof/root on bytes under the retained lease; do not infer SHA384 integrity from its legacy SHA256 identity. |
| Etalon etalon-model-io::snapshot::{Snapshot,SnapshotLimits,CleanupPending} | snapshot/{mod,owner,copy,limits}.rs. Existing explicit caller-parent TempDir snapshot,64KiB copy buffer, byte/read/write-call limits, cancellation checks between calls, sync_all, final excluded lease+rehash; close retains failed cleanup for retry. Source authority/coherence and reader/OS allocations are separate. Windows backing gate only; non-Windows UnsupportedPlatform. Reuse for explicit complete-entry extraction/consumer handoff, no implicit full-source snapshot fallback. |
| wasm_storage::{estimate_browser_storage,write_opfs_block,read_opfs_block,is_opfs_block_cached}; wasm_bridge::device_storage policy/backup APIs | wasm_storage.rs:181–251: block shape is40960 bytes; write clones then createWritable/write/close; read ArrayBuffer is entire cached File; is_cached is existence only. Actual opfs_root uses window(), so this implementation is not a dedicated-worker range path. device_storage.rs:1–11,145–218 plus docs/js/webcivics-device-storage.js provide existing OPFS-root, quota-estimate, backup/manifest lifecycle. Extend that owner/worker bridge; do not create a second origin vault/quota registry or call browser allocation zero-heap. |

Source text search additionally checked explicit StorageQuota/DiskQuota/VerifiedStore/RangeStore/RangeJournal/VerifiedBitmap names and lock/share-mode uses across production core and Etalon model-io. This does not claim algorithms absent from the repository: it identifies the integration contract not supplied by the inspected exports.

## Minimal storage model

QBS storage is a thin verified-range facade over the canonical backend/root/cleanup and quota owners. New proposed files: bundle/stream/storage/{mod,identity,bitmap,verify,checkpoint,resume,read,tests}.rs, plus a root-owned focused bounded-range backend extension under the existing storage owner. No second StorageDriver, directory registry, host ReservationLedger, operation table, lease table or financial ledger.

Bind every record to full descriptor ID, context ID, entry ID, generation, representation/key epoch, TreeSpec length/root, privacy partition, policy generation and storage generation. These are distinct fields; source generation is not storage generation. Any compact lookup index is merely a lookup hint. Caller-selected protected root and opaque fixed canonical object names replace raw entry-key pathname construction.

Payload placement and verified state are separate. A sparse hole, file length, HTTP ACK, CRC, cache hit or OS zero-filled read is never a verified range. Use entry-relative16KiB logical leaves and exact shorter final lengths; no global leaves spanning entries. Bitmap is paged in caller4KiB windows, with descriptor/entry/storage generation per page. A1TiB source can require an8MiB leaf bitmap, so constrained mode must not retain the full bitmap beside its8MiB transfer window. Preserve parent MAX_RANGES8/resume-bitmaps; this is a paged owner above them.

Initial safe fallback: bytes on disk and untrusted restart bitmaps are hints. Re-read into a reserved private buffer and call existing QBS05 verify_leaf against the descriptor-bound TreeSpec/root before consumption. After loss of writer exclusion/untrusted resume, reverify **and rewrite/flush into the current storage generation** before issuing a new durable-present receipt, including legitimate all-zero leaves. A hole is never promoted solely because reading zeroes happens to match a root. Missing retained proof/outboard evidence means Incomplete until it is reacquired; there is no invented verified-zero or digest shortcut.

Public plaintext and verified ciphertext are separate storage classes/privacy partitions. Private authenticated plaintext is not eligible for a shared cache; refuse that class until QBS10 private lease/erase/key-manager integration is accepted. QBS04/07 owns trusted descriptor/current policy binding. A QBS06 CandidateIdentity alone remains a local candidate and cannot authorize protected disclosure or consumption.

## Proposed API boundary (names are new proposals)

    open_verified_store(canonical_backend_session, existing_quota_lease,
        admitted_store_binding, caller_bitmap_scratch) -> VerifiedStore

The backend/session and quota are borrowed actual owners/handles. No constructor accepts numeric caps and constructs a new authoritative ledger/usage snapshot. AdmittedStoreBinding must be compiled by descriptor+policy ownership; plain candidate-only mode may exist explicitly for qualification fixtures and never mint a protected/public-serving grant.

Required bounded backend operations, through the existing storage lifecycle:

    read_exact_at(object, offset:u64, out:&mut[u8])
    write_exact_at(object, offset:u64, bytes:&[u8])
    flush_payload_and_proof()
    append_checkpoint(record:&[u8])
    flush_checkpoint()
    compare_and_promote(expected_storage_generation, manifest_digest)
    close_after_io_drain()

These are explicit capabilities, not wrappers around current MmapDriver::read_range or a no-op flush. Each backend declares actual observed durability/exclusion class and returns Indeterminate on ambiguous publication. Qualified platform behavior is required before DurableVerifiedRange, never inferred from fs::rename returning success. General storage_driver.rs is oversized; root should claim a focused backend child module/decomposition instead of adding mixed behavior there.

Facade operations:

    verify_and_store_leaf(storage_stage, selection_bound_tree, leaf_index,
        bytes, proof, caller_proof_scratch) -> StorageProgress
    read_verified_leaf_into(storage_stage, leaf_index, out, proof_scratch)
        -> borrowed VerifiedLeafView
    checkpoint(storage_stage) -> CheckpointOutcome
    resume(existing_store_session, current_binding, checkpoint, scratch)
        -> ReverifyPlan
    promote_complete_entry(current_binding, complete_verified_closure)
        -> retained consumer snapshot/lease

VerifiedLeafView has a private constructor and borrows actual verified bytes; it is integrity, not authority/consumer-commit. New persistent records remain bounded/caller-output; exact bytes/domains require root review before implementation. Use the corrected central cryptographic-library facade if authenticated local checkpoints are later selected. Do not add raw SigningKeys, HMAC keys or new primitive code in storage. Initial reverify-on-resume mode avoids pretending CRC or an ordinary sensing Ed25519 head is PQ authority.

## Reservation and cancellation composition

QBS08 already borrows canonical ReservationLedger/LeaseTable/OperationTable/FairScheduler and charges network/proof/storage/consumer/scratch before RequestCredit. The store must use that same TransferTicket/CompletionKey/cancel epoch. A derived non-Copy StorageStage guard should borrow the admitted transfer/window, not reserve a second copy of the same physical receive buffer or manufacture a fresh host ledger. Shared network→proof→storage views count once; extra proof/bitmap/journal/worker buffers and storage IO work must be included in the compiled plan. Persistent disk quota is a different resource from resident RAM, and is borrowed from the canonical persistent quota owner rather than silently duplicated.

Existing runtime public state/cancel_epoch validates ticket owner/generation, but a concrete stage/window/physical-buffer binding is not yet exported. Root must align that small facade with QBS09/13: actual buffers/capacity cannot be inferred merely from numeric TransferRequirements. Do not mutate QBS08 parent tables from a separate store. Its8 constrained/30 desktop active limit and control reserve remain unchanged.

Before proof/write and immediately before verified checkpoint promotion, check current policy/storage generation, deadline/cancel epoch and ticket state through the owning runtime/policy lease. Cancellation retains actual file/async IO/window ownership until worker completion. Late completion may reclaim resources but cannot set current bitmap bits, publish a new manifest, commit a consumer or settle an obligation. No store method independently calls parent complete_io while another network/proof/consumer borrower remains alive.

The scoped mutable-borrow limitation from QBS08 remains: do not obtain another mutable owner by constructing copied budgets. QBS09 host delegation/owner handoff must route canonical background work and storage through the same owner.

## Durability order, replay and cleanup

1. Validate exact selection/root/entry/leaf/final length and proof before writing; obtain actual reserved buffers/IO and existing persistent quota admission before growth.
2. Write exact payload and retained proof evidence. Flush according to the qualified backend contract. Physical file contents remain unverified until the journal step.
3. Append a bounded checkpoint delta binding complete identity, exact leaf index/length, writer/storage/policy/cancel generations and data/proof commitment. Flush checkpoint, then expose the bit/receipt. A failure after a visible filesystem change is Indeterminate and must reconcile, not report successful durable storage.
4. Duplicate delivery with identical committed identity is idempotent; it does not allocate a new financial operation or double debit storage. Conflicting bytes/proofs/epochs refuse. Existing CORE03 intent/receipt owner may bind a <=512-byte checkpoint-control record, while payload/proof live in their own files; never stuff a leaf into MAX_OBJECT_BYTES or recycle32 live intents without a persisted, reviewed checkpoint/retirement frontier.
5. On restart re-admit descriptor/authority/current policy, storage/root/writer identity, length/version and checkpoint record. CRC detects corruption only. Without trustworthy retained local authentication, reverify retained actual bytes against the same root, then establish current-generation write/flush evidence before marking durable. Re-opening after lost writer exclusion invalidates prior fast-path verified views.
6. Complete P64 requires every entry byte plus dependencies, then existing retained ModelLease/Snapshot where supported; no sparse holes passed to whole-file readers. Native mapping addresses/alignment and source+mapped+snapshot resource estimates remain QBS13 checks.
7. Pins, custody commitments, evidence holds and active readers block deletion. Reuse owned TempDir/cleanup-retention lifecycle. Cleanup deletes only marker-verified owned runs under the explicitly configured protected parent; no broad root/system-temp recursion. Keep conservative quota charges through failures until the canonical owner reconciles authenticated usage; no refund solely on TTL/drop.

Retaining proof paths per verified leaf is a simple bounded-record first implementation, but its total disk cost must be estimated/admitted. A shared outboard can reduce that cost later using existing QBS05 node geometry; storage must not implement another Merkle split/proof algorithm. Large logical extent is not a promise that its proof/index disk cost fits a device quota.

## Alignment decisions required before code

1. Root storage owner: add a focused caller-buffered u64 range/flush/session extension to the existing backend/root lifecycle, or approve an existing retained-file session bridge after review. Current whole-value/no-op flush APIs are not the adapter.
2. Persistent quota owner: generalize/bridge existing authenticated root usage lease through its owner, or identify another already canonical application quota authority. Do not instantiate another CustodyQuota from copied limits or make a new per-transfer disk ledger.
3. Runtime/policy stage capability: define borrowed window/IO and generation binding above existing QBS08 tickets, plus QBS04/07 StoreBinding. Candidate fixtures may explicitly remain unsigned/public-local; that cannot be marked policy-complete.
4. Checkpoint/restart profile: recommend initial reverify-on-resume plus current-generation rewrite, no new signing primitive. Agree bounded checkpoint/proof record schema and persistent intent retirement before a native large-object claim.
5. Platform qualification: Windows NTFS complete lease/snapshot reuse is concrete; Linux independent-peer store and OPFS dedicated-worker bounded range/durable-manifest paths need their own actual adapters/tests. Existing OPFS quota estimate/existence/close is not verified-range durability; JS queues/allocations are bounded/measured separately.

## Located tests and proposed new qualification

Existing source tests (not run here): connectivity durable_store::tests::persist_drop_recover_and_corrupt_fail_closed; peer replication commit_adapter/pair/disk_crash tests; qdnf replication::stream tests; sensing local_custody::{tests,recovery_tests,local_recovery_tests,local_head_adapter_tests,quota::tests}; Etalon model-io lease/snapshot tests including unsupported-platform and cleanup retry; device_storage policy/backup tests.

New required fixtures: data/proof/checkpoint write/flush faults at every boundary; actual process-kill/reopen and backend-specific directory-publication ambiguity; missing/torn/oversized/trailing checkpoint; forged bitmap/hole/all-zero leaf; source/context/entry/representation/privacy/storage/policy splice; duplicate idempotence/conflicting bytes; cancellation between verification/write/flush/bit publication; late IO after slot reuse; quota refusal before growth, concurrent tabs/processes and conservative recovery; retained writer/reader exclusion and mutation races; bounded large-u64/4KiB bitmap paging with allocator positive control; no entire object allocation via old read_range; pinned/evidence deletion refusal; complete-entry lease/snapshot lifetime and cleanup error/unwind.

This inventory is ready for root contract review. No QBS11 implementation or new test result is claimed.

## Inspected source fingerprints

SHA256 of actual source bytes read for this contract; no clean-checkout or executed-test claim.

| Core src-relative path | SHA256 |
|---|---|
| storage_driver.rs | 92b4cfe6a7de58484684a7fedad4555fbb8c494ff888771591c57a4fc4234421 |
| storage.rs | 0110506947dddf7dd057ddff15883e98ae007991a400fa9ba400e5dbe95bff0e |
| net/peer/connectivity/durable_store.rs | 252ed804eccb0f48c3d74ace0abbe863c90f94d9222ecff3788464da699874d4 |
| net/peer/replication/commit_adapter.rs | eeb837e61180937952dc44e32888242e1040b77f57e5be716af27d55459bbeb8 |
| sensing/local_custody/quota.rs | bc13ef6ad66817c4284d78b9be5d5c4754d915546f4506d57adababc0252b025 |
| sensing/local_custody/local_head.rs | e2e23947d51e7d32305d2c1c0a90907d4b1729afc7c49deece5bd75015e009b0 |
| wasm_storage.rs | 256769ac2b8ca3b1d8e98387557400ced3cc9a2971366a3906674c7e3ebc2d35 |
| bundle/stream/runtime/dispatch.rs | f2d394aabb3e7c2e00ff6cda87eedd32389ebf540fd858a403204fa508327c11 |
