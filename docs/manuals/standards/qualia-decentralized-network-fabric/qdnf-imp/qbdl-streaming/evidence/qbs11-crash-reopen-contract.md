# QBS11 phase D — crash-reopen contract

Status: proposed, source-inspected only. No parent implementation, subprocess run or power-loss result is claimed. Root must agree scope before code. Phase C remains live-session-only.

## Reused owners and inspected evidence

Core src-relative APIs actually read:

- storage_driver/ranges/session.rs: real new TempDir/create_new staging, retained file handles, bounded u64 I/O, actual sync_all, exact64-byte marker, guarded close/retry/Drop and scoped marker restoration after verified-start cleanup failure. Creation is not adoption/reopen of an old run.
- sensing/local_custody/quota.rs:23–83: QuotaSnapshot is opaque authenticated ID plus aggregate bytes/entries. Existing QuotaSnapshotAuthority admits it into the one CustodyQuota. Host freshness, protected root and aggregate-quota lease remain required; usage includes committed/orphan/uncertain charges.
- quota/staging.rs: actual non-Copy full-allowance charge before staging mutation, surviving every failure/drop.
- local_custody/recovery.rs:79 and local_recovery.rs:43: real bounded non-mutating sensing checkpoint/head recovery under current host policy/held lease. Unknown membership is not committed membership; no scan/repair/deletion permission follows.
- local_custody/open_checked.rs:9,22: actual Unix O_NOFOLLOW/O_NONBLOCK/O_CLOEXEC and Windows OPEN_REPARSE_POINT/post-open regular/nonreparse checks. Extend/reuse narrowly for retained exclusive reads.
- local_custody/local_head.rs: actual cooperative-lock/signed sensing-head owner, with independent host rollback policy and legacy direct SigningKey. Do not copy its crypto/format into PQ QBS quota/recovery.
- net/peer/connectivity/durable_store.rs:40: real recovery for its own QDUR1 queue uses whole-file read_to_end; it is not the bounded range-source adapter.
- bundle/stream/selection, storage/native and runtime/storage_stage: exact signed plaintext entry selection, actual canonical gates, QBS05 proof verification, retained workspace incarnation and real ordered file barriers. CP400/bitmap remain unkeyed hints.

These are inspected integration boundaries, not filename-based claims that other repository algorithms do not exist.

## Recovery rule

Old runs are UNTRUSTED READ-ONLY MATERIAL. Marker/checkpoint bytes never reconstruct current authority, a lost runtime/workspace ticket, old quota, verified progress or serving/durable permission.

Create a NEW destination via the same actual CustodyQuota, fresh attempt and current storage generation. Read one old leaf/proof into existing leased buffers, verify against current SelectedPlainEntry/TreeSpec/root, then use the existing destination write -> payload/proof sync -> bitmap/header -> checkpoint sync protocol. Keep the page attempt armed through owned accept_local_scoped's final authority check and final original-ticket check.

Even valid all-zero sparse reads are explicitly rewritten before current marks. Missing/torn proof/bytes require reacquisition. Changed descriptor/context/entry/source generation/epoch/root invalidating old proofs cannot be handled by relabeling them. Initial scope is public plaintext candidate plus current explicit local whole-source adoption; protected/cipher/private/remote rights remain separate gates.

## Exact root/quota provenance proposal

QuotaSnapshot does not carry per-attempt basename/layout/allowance provenance. The .owner64 contains only attempt ID; CP400 is attacker-recomputable. Scanning file lengths or finding/deleting files therefore MUST NOT subtract/reissue charges or manufacture an authenticated snapshot.

Add a narrow child to the EXISTING quota owner:

    CustodyQuota::recovery_root_scope(&self) -> RecoveryRootScope<'root>

Private fields derive only from this actually admitted quota: borrowed configured root, original authenticated snapshot ID and opaque scope association if needed. No public constructor from caps/totals/root/Allow bool. It is root provenance for zero-growth inspection, NOT a budget token or per-run committed-membership assertion. Like StagingCharge, it need not retain a mutable quota borrow; new genuine admission must remain possible. Host holds the existing aggregate-quota/protected-namespace policy lease through every scope/source use and prevents concurrent independent quota owners from copied caps.

The fresh authenticated baseline MUST include old full uncertain allowances, even when files are short/missing. Lost/stale provenance stops new growth; never bootstrap usage=0 from a scan. Admit the NEW full checked destination allowance in addition to old uncertainty before any destination mutation. Successful copying, source disappearance, close/deletion or TTL never refund either charge. Only explicit canonical authenticated reconciliation may alter that baseline. No new vault, snapshot signature stack, raw signing key or checksum-as-authority.

## Bounded local locator/source API

No arbitrary incoming path or data filename. Private StagingLocator is issued by a bounded local lookup under RecoveryRootScope, or by an existing trusted host run-inventory owner. It contains one bounded native direct-child component, exact nonzero attempt32 and scope association; it is candidate location, not rights/quota.

Minimal lookup scans only the canonical configured root, shallowly, for the expected LOCAL attempt. Caller supplies fixed output slots plus entry/syscall/work bounds. No recursive scan or unbounded Vec. Accept only qualia-range-* direct children and exact .owner64; reject links/reparse/malformed/oversize/trailing markers. Limit exhaustion is Incomplete, duplicate matches Ambiguous. Never repair/delete unknown markers. Enumeration/path allocation is cold bounded work, not zero-heap hot I/O.

New proposed backend:

    MmapDriver::open_range_recovery_source(scope, locator, checked_layout)
        -> RetainedRangeSource | finite RecoveryError
    source.read_exact_at(RangeFile, offset:u64, caller_output)
    source.close() -> SourceClosedObservation

Require driver/scope canonical roots equal and resolved direct child remains inside that root. Open only constant payload/proof/checkpoint and .owner names. Read-only: no create/set_len/truncate/repair/rename/delete. Exact marker magic/version/reserved/attempt/length/no trailing. Component size may not exceed checked source geometry; short/missing material is unavailable, never synthesized zeros.

Reuse actual final-component opening owner: Unix O_NOFOLLOW/O_NONBLOCK/O_CLOEXEC plus post-open regular metadata and retained advisory try_lock; Windows OPEN_REPARSE_POINT, handle regular/nonreparse check, share_mode(0), retained actual try_lock where supported. Hold source/marker handles through use; no silent fallback after lock/sharing failure. Report Unix cooperative-only versus Windows observed ordinary sharing denial separately. No independent Unix writer exclusion or whole-directory identity guarantee inferred.

Host maintains root/ancestors/ACL/locality/current root identity and namespace stability through lookup/open/read/close. Path/marker checks do not establish these. Test parent replacement, final symlink/reparse, file substitution and competing live handles. A stronger handle-relative guarantee requires actual SDK/source review and separate qualification; do not claim it from canonicalize.

RetainedRangeSource is non-Copy and never adopts TempDir deletion authority. Failure/drop closes handles, preserving old artifacts/charges. Do not assume TempDir has a safe arbitrary-path adoption constructor. Old cleanup requires separate canonical marker-verified ownership, drained readers/pins/evidence holds and reconciliation policy. New temporary file barriers cannot authorize deleting old evidence.

## Resource/current-authority integration

Account actual bounded metadata/path buffers and retained handles using the SAME canonical governor/ledger, not another root budget or numeric token. Recommend a retained-source owner/table analogous to SparsePageWorkspace: exact owner/slot/incarnation, consuming release returning owner on refusal, Drop never fakes release, runtime.close blocked while sources remain. Cold open/lookup must be bounded; hot leaf I/O uses caller slices only.

Each source handle uses another existing parent reservation slot. With static/control two, w workspace slots and s retained source slots, desktop inflight <=30-w-s; constrained <=min(8,that ceiling), before other host use. Preserve essential control.

    NativeSparseStore::recover_leaf_from(
        actual StorageStage, actual LocalWholeSourceGrant + pins/host clock/policy,
        exact current workspace, actual RetainedRangeSource,
        leaf_index, caller typed siblings
    ) -> LeafBarrierObservation | finite error

Own the outer scoped grant call, validate source/root/table/incarnation and current signed selection, recheck before/after real bounded reads/barriers, verify via actual QBS05, then reuse destination protocol. No bitmap import. Clear unaccepted transient material/marks on error/unwind. Source records never supply a current grant. Cancellation retains actual source/I/O until drain; late old completion cannot set current marks or commit a consumer.

One16KiB leaf,1537 proof slot,400 header/4096 hint window and <=32 typed digests suffice; no whole-object read/mmap or full8MiB resident bitmap. Trusted callbacks may copy/act; HRTB restricts borrowed lifetimes, not a hostile host or copied-data revocation.

## Real subprocess-kill qualification

Actually terminate an owning process without destructors, reopen production retained files, re-admit current authority/quota and reverify/rewrite. Dropping an in-memory fixture is not restart evidence.

Parent trusted test coordinator owns private parent TempDir/protected root, unique nonce, bounded control records and existing QuotaSnapshotAuthority provenance. Only the child owns the canonical quota during its epoch. Child uses actual admission/staging/native code and acknowledges AFTER real admission: nonce, attempt/local locator, full allowance and actual charged totals. Parent verifies expected cold-plan allowance/counts and unique child/nonce through its private OS-owned control channel. This is test host provenance, not a production signature stack.

If kill timing makes admission uncertain, preserve the full permitted allowance; missing ack is not usage=0. Proven pre-admission refusal requires child blocked before permission to enter admission. Parent kills/waits under bounded deadline, verifies termination and retained artifacts, then creates the NEW canonical quota from its approved fresh baseline INCLUDING old uncertainty. Re-authenticate signed selection/pins/current grant; never deserialize old permits/workspace/tickets. Admit a distinct new destination/current generation and recover real source bytes. Bound stdout/stderr/control reads, scanning and waits; no unbounded read_to_end/Command.output. Always drain/kill helpers on failure.

Cases: before admission, after charge/before marker, marker before components, partial payload/proof, after each data sync, bitmap before header, header before checkpoint sync, after checkpoint sync before outer final check, after callback acceptance before process kill. Hooks between syscalls do not prove interrupted syscalls; separate real short-write/torn-prefix fixtures must say what they simulate.

Negatives: valid/repaired hints over changed bytes/holes, legitimate zero leaf, short final leaf, missing/oversize/trailing proof/CP, wrong attempt/root/scope/full identity/epoch/privacy, substituted links/reparse/parent, live competing source, denied/stale usage snapshot, insufficient old+new capacity, revoked/expired current pins/grant/policy/ticket, final-check failure and late completion. No current mark without actual reverify/rewrite; no quota refund from absence/cleanup.

Process kill/reopen while OS stays running is NOT directory power-loss qualification. Windows ordinary sharing and Unix cooperative locks are separate observations; physical cross-host/platform and device power-loss flags remain separate. No DurableVerifiedRange/serving/financial receipt/complete-consumer lease is exported.

## Proposed scope for root approval

- quota/recovery.rs new child and minimal root-owned route.
- storage_driver/ranges/recovery/{mod,locator,source,tests}.rs new children; narrow opener/marker/namespace factoring owned by root. Do not grow455-line session.rs with another lifecycle.
- Focused retained-source runtime child/stage bridge after exact same-ledger owner API agreement.
- storage/native/recovery.rs read-source verify + shared destination protocol.
- Root-managed real subprocess entry/harness and parent routes.

Decide locator issuer, RootScope association/lifetime, retained-source budget owner, strict missing-component refusal versus per-leaf incomplete reads, and bounded subprocess control/watchdog. Safe default: zero automatic source mutation/deletion and zero quota/progress reconstruction from artifacts.

## Inspected source fingerprints

HEAD: afbe431195167fde0fadc233ed79fd2fe85f73f0. These are actual dirty-worktree source bytes read, not a clean-checkout or test-execution claim.

| Core src-relative source | SHA256 |
|---|---|
| storage_driver/ranges/session.rs | 9f76a7fed9483bfb0b2e6b19a1eea2acd36c49459deea114a0776f8e957f8943 |
| sensing/local_custody/quota.rs | 997c22a7d0261177fd0f577d5cb22bff65db76fdbb5a254871fe5614e5b18c59 |
| sensing/local_custody/quota/staging.rs | f3ee7c8ab268f0ad60eb9322d273e0e8482a4bb3083fb54eb1ddba4aec238a8b |
| sensing/local_custody/recovery.rs | 4ac3e49b69cb9f6e78e32a8fe30c5673fc289c967b366a1b796c42c2afe5fd3e |
| sensing/local_custody/local_recovery.rs | 9dffae5b9002d4c3071af4eb58795128702468ae8fd0469436b05cecdbbd903e |
| sensing/local_custody/open_checked.rs | a27ae0d0ec8b2c5e7ed823e9b5f5c77e2be183020f5473e6ec87eeba80e33217 |
| sensing/local_custody/local_head.rs | e2e23947d51e7d32305d2c1c0a90907d4b1729afc7c49deece5bd75015e009b0 |
