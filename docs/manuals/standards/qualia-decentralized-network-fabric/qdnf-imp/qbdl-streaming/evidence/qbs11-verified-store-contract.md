# QBS11 phase C — verified sparse candidate integration contract

Status: proposed, not implemented or tested by this agent. Phase B sources remain stable for root testing. Repository HEAD inspected: `afbe431195167fde0fadc233ed79fd2fe85f73f0`. This proposal supplies bounded integrity and ordered file-barrier observations. It does not complete remote policy, crash reopening, directory power-loss durability, serving authority or consumer handoff.

## Actual owners inspected

All paths below are under `crates/qualia-core-db/src/`; line references are source-inspection evidence, not test execution.

- `storage_driver/ranges/session.rs:32,161,178,194,215,264`: actual `MmapDriver::create_range_staging`, caller-buffered u64 reads/writes, real `File::sync_all`, marker-guarded Drop and retained cleanup retry. Three data files plus a fixed64-byte ownership marker; Windows retained share denial, Unix cooperative locking only. Backend reports directory power-loss qualification false.
- `sensing/local_custody/quota/staging.rs:27`: `CustodyQuota::admit_staging` charges the existing root/usage/attempt slots and returns a private non-Copy root-borrowed charge. No refund on failure/drop. Legacy sensing publication uses the same shared u64 charge implementation.
- `bundle/stream/runtime/storage_stage.rs:22,69,79`: real `StorageStage` binds payload/proof/byte scratch to the original in-flight ticket, checks deadline/policy/cancellation, and retains reservation until caller completes actual I/O. No second host budget.
- `bundle/stream/storage/{identity,leaf,bitmap,checkpoint}.rs`: full256-byte identity, actual `verify_leaf_view` (`leaf.rs:26`), fresh bitmap marks requiring a matching private-constructor view, separate untrusted resume hints, exact400-byte checkpoint (`checkpoint.rs:55,73`). SHA384 checkpoint integrity is attacker-recomputable.
- `bundle/stream/proof/{types,codec,verify}.rs`:16KiB logical leaves, exact shorter final leaf, ciphertext tag geometry,32-sibling ceiling, existing caller-output proof codec and existing verification. Do not implement another Merkle algorithm.
- `bundle/stream/descriptor/{inventory,pages,roots}.rs`: exact body/context/page commitments and complete coverage; `InventoryVerifier::finish` at210 returns a candidate identity after complete validation. Entry rows carry their committed root/mapping.
- `bundle/stream/policy/{authenticated,grant}.rs`: `BoundClosure::recheck` revalidates actual paired issuer/publisher receipts. `LocalWholeSourceGrant::accept_local` (`grant.rs:128`) uses the concrete canonical authority owner; this is explicit local whole-source adoption, not remote Share/consent/Sentinel admission.
- Existing canonical storage, peer cache/intent/durable queue and Etalon model lease/snapshot owners were actually inspected in `evidence/qbs11-storage-contract.md`. This proposal composes those findings; it does not replace them, invent a second cache/quota registry, or infer missing algorithms from file names.

## Minimal phase-C profile and ownership

Exact proposed adapter scope: `bundle/stream/storage/native/{mod,error,geometry,binding,owner,write,checkpoint,resume,read,tests}.rs`, with focused test children, each implementation file below500 lines. Root owns routing and the small runtime/policy bridge changes below. No changes to general StorageDriver read_range, parent ledgers, economics, Cargo, crypto primitives or the existing phase-A wire bytes.

One `NativeSparseStore` owns one real `RangeSession`, one fully checked identity, fixed state/counters and a descriptor-selected entry binding. Root owns a separate canonical retained `SparsePageWorkspace`: a non-Copy actual owned integrity page, metadata and private runtime-owner/slot/incarnation token, charged through the SAME runtime ReservationLedger. It survives transient transfer completion without transferring or duplicating a transfer charge. An operation borrows that validated workspace plus its record/readback scratch from the actual StorageStage. Cold metadata/descriptor borrows retain their existing caller-owned budget.

Initial accepted-input proposal is plaintext public *candidate* storage only, under explicit local whole-source adoption. Reject private plaintext and unresolved ciphertext descriptor/projection/wrap profiles with Unsupported. Phase-A ciphertext inclusion remains mathematical support, not evidence that QBS06/07 authorizes that profile. No method converts CandidateIdentity, bitmap bits, an unkeyed checkpoint, or a bool into disclosure permission.

A private `SelectedStoreBinding` constructor must check the exact BoundClosure identity against body bytes, then selected entry-page commitment/ordinal/context/ID/root/length/mapping against that body and the complete admitted inventory. Derive the TreeSpec from those exact fields; never accept an arbitrary caller root merely because its supplied descriptor ID matches. Privacy partition and policy/storage generation must come from the existing host/authority binding; accepting numeric fields is not minting those authorities. Bind full identity equality at every page/checkpoint/view boundary.

Proposed facade signatures (new proposals, not existing exports):

```text
StoreLayout::for_binding(binding: &SelectedStoreBinding) -> Result<StoreLayout, StoreError>
layout.range_limits() -> exact checked RangeLimits
layout.aggregate_allowance() -> checked u64 for existing CustodyQuota admission
NativeSparseStore::new(session: RangeSession, binding: SelectedStoreBinding,
                       workspace: &mut SparsePageWorkspace)
    -> Result<NativeSparseStore, StoreError>
store.write_leaf(stage: &mut StorageStage, grant: &mut LocalWholeSourceGrant,
                 pins: &PinAuthority, trusted_clock: impl Fn()->u64,
                 workspace: &mut SparsePageWorkspace, leaf_index: u64, host_clock/current_policy callbacks)
    -> Result<LeafBarrierObservation, StoreError>
store.load_resume_hint(stage, page_number, expected_old_identity)
    -> Result<ResumePlan, StoreError>
store.reverify_and_rewrite_leaf(stage, grant, pins, trusted_clock, resume_plan, leaf_index, ...)
    -> Result<LeafBarrierObservation, StoreError>
store.read_verified_leaf_into(stage, grant, pins, trusted_clock, leaf_index, ...)
    -> Result<VerifiedLeafView borrowing actual caller readback bytes, StoreError>
store.into_staging() -> RangeSession
store.close() -> CleanupObservation | CleanupPending
```

Names/types above must be finalized with root before code. The read signature above is conceptual: with the accepted HRTB runtime bridge, the actual API must instead invoke a bounded callback with VerifiedLeafView INSIDE StorageStage::with_proof_buffers; no view/borrowed scratch may escape. `LeafBarrierObservation` reports only exact identity/index/sequence and actual completed file barriers. It has no durable/serving/custody/financial meaning. `ResumePlan` is untrusted scheduling data, never a verified map. Store methods never call runtime complete_io or release a transfer ticket.

## Frozen errors, state and owned test scope

`StoreError` is fixed: Unsupported, Empty, Closed, Poisoned, Bounds, Overflow, ScratchCapacity, IdentityMismatch, PageMismatch, Descriptor, CheckpointConflict plus wrapped finite existing StorageError/ProofError, RangeError, QdnfError and PolicyError. No String/Vec error payload. Exact wrapper derives follow the existing error types; no error formatting in hot paths. NativeSparseStore transitions Active -> Poisoned on possible effects followed by failure/cancel/final-check refusal; Poisoned cannot write or report accepted progress. Explicit read/reverify recovery is required. A preflight refusal has no filesystem effect and does not poison an otherwise valid store. Close delegates the original RangeSession lifecycle; it does not release runtime page workspace or complete transfer I/O implicitly. Constructor checks exact layout ceilings against its real session before admitting the store. Empty entries return the no-files Empty plan, never a fabricated zero allowance.

Tests are new actual-module children only: `native/tests/{fixtures,geometry,binding,ordering,resume,state,allocation}.rs`; keep helpers bounded and reuse actual published descriptor/proof/policy/runtime owners. Test-only failpoints may stop the real protocol at boundaries, but cannot replace native sync with fake success or count as power-loss evidence. Parent exports, quota/backend/runtime/policy/workspace implementations and Cargo remain root-owned. No production backend edits from the adapter worker.

Root-owned workspace prerequisite: an OwnedIntegrityPage under storage/bitmap/owned.rs (root route/reexport) contains exact full identity/page and one4096-byte array. Private construction initializes zero once. Its only insertion route consumes a matching VerifiedLeafView; its scoped with_candidate_page reborrows the actual page without clearing, enabling existing encode_checkpoint. No raw mutable bitmap/from_hint/boolean verified setter. SparsePageWorkspace owns that exact typed page and a private canonical runtime owner/slot/incarnation token. Stage verifies the token before providing it to the bounded HRTB callback and prevents release until its borrow ends. Release consumes the actual workspace and canonical charge; Drop never pretends release. Runtime.close refuses outstanding workspaces. Stale/wrong-runtime/wrong-page/identity workspace is a pre-effect rejection. Charge size_of::<SparsePageWorkspace>() including metadata, and profile-window accounting, not merely4096 bytes.

Each retained workspace uses an additional existing parent ledger handle. With static+control two handles and w workspace handles, desktop inflight is at most30-w; constrained is at most min(8,30-w), before any other existing host use. One workspace therefore permits at most29 desktop transfers. Parent ceilings and essential control reserve remain unchanged. Root must expose/report effective headroom rather than silently claiming30 or64 simultaneous I/O. Workspace charges are not duplicated in per-transfer storage scratch, while checkpoint/proof/readback/typed scratch are still charged per transient stage.

### Approved mechanical poison/recovery prerequisite

OwnedIntegrityPage has a one-way invalidate that clears marks only. SparsePageWorkspace creates a non-Copy WorkspacePageAttempt borrowing that actual owner; Drop clears its page unless commit consumes the guard. The adapter arms the guard and sets the store Poisoned BEFORE the first possible effect. The PUBLIC write wrapper owns the call to grant.accept_local_scoped (actual grant, pins, trusted clock), rather than receiving only an inner LocalEffectControl. The attempt stays armed outside that callback. Only after the scoped accept INCLUDING its final authority check and the final canonical runtime check succeed may the adapter commit the guard and restore Active. Any write/sync/checkpoint error, canceled final check, callback error or unwind therefore mechanically invalidates every current page mark. No quota/workspace charge is released by invalidation.

Every new NativeSparseStore facade takes and clears its actual workspace, then records the opaque runtime-owner/slot/incarnation key and exact full identity/page plus its actual backend attempt. Every operation requires the same opaque key and full binding. This prevents both (a) uncertain marks being reused after fault -> into_staging -> new facade and (b) valid old marks being attached to a new empty backend with the same source identity. Rebuilt hints remain untrusted; fresh actual proof/read/rewrite is required. No public raw token constructor, from_hint initializer or raw mutable bitmap exists. Required native tests explicitly reuse the old workspace after a fault and after constructing a new facade: they must observe reset/rejection before any accepted checkpoint.

## Exact proposed native file geometry

No phase-A codec change. Checked u64 arithmetic precedes quota admission/creation.

Let n be TreeSpec.leaf_count, p = ceil(n/32768). Empty entries have n=p=0 and no bitmap/checkpoint page; they need descriptor-root admission only and do not manufacture a zero-byte staging attempt.

- Payload ceiling: plaintext logical length L. Leaf i offset i*16384, length exactly TreeSpec.encoded_leaf_length(i). Future ciphertext profile would need L+16*n and stride16400; it is Unsupported initially.
- Proof ceiling: n*1537, with fixed slot i*1537. Slot starts with existing QBS05 encode_proof bytes (1+48*count, count<=32); remaining bytes through1537 are zero. Decode checks zero padding, then invokes existing decode_proof and verify_leaf. A zero-filled proof slot is not a presence bit.
- Checkpoint ceiling: p*8992. Each page has two4496-byte banks. Bank offset (page*2+bank)*4496. Bytes0..400 are the existing exact checkpoint;400..4496 are its4096-byte bitmap payload. The checkpoint's full identity/page/digest binds the payload. Both copies remain untrusted hints after loss of trusted live state.
- Full explicit allowance must cover the checked sum of all three ceilings before any filesystem mutation. Existing quota owns that charge. No file set_len/preallocation and no per-leaf second disk charge. The64-byte .owner plus filesystem/control overhead remain host-covered as phase B/existing quota documents.

This deliberately simple proof-retention profile is expensive and must be admitted honestly: at1TiB plaintext, n=67108864, proof ceiling=103146323968 bytes (96.0625GiB), checkpoint ceiling=18415616 bytes, total data ceilings=1202676367360 bytes. A1TiB TreeSpec ceiling is not a claim that a device quota can hold its proof store. A compact outboard is a later owner-reviewed optimization, not permission to omit recoverable evidence.

Retained resident state is one canonically charged4096-byte page plus workspace metadata. Transient scratch is one4496-byte checkpoint bank, one1537-byte proof record, caller32-digest decode scratch, and (for read/reverify) one<=16400-byte encoded leaf. The actual transient aggregate, including typed sibling scratch, must fit the already admitted storage component. Network payload/proof views are reused without copying or charging them twice. No8MiB resident bitmap or whole-source read. Hot proof/page/codec/range operations and rejects must have zero raw allocations; native cold session creation/cleanup and host filesystem allocations are separate.

## Required narrow root bridges

The present Stage API cannot support repeated recheck while a verified view remains borrowed: `buffers(&mut self)` ties the returned views to the entire mutable stage, preventing `stage.recheck` while those views live. Do not work around this by copying a ledger or retaining unchecked slices.

Root has now prepared `runtime/storage_stage/controlled.rs`: actual `StorageStage::with_buffers(now,policy, for<'b> FnOnce(&mut StorageStageControl<'b,'owners>, StorageBuffers<'b>)->T)`. Reviewed source supplies disjoint real payload/proof/byte-scratch plus same-runtime recheck/cancel, with no escaped borrows/new charge. Root also proposes `with_proof_buffers(..., &mut[StrongDigest], effect)`, checking <=32 nodes and byte scratch.len + size_of_val(nodes) <= original slot.storage_bytes; this avoids unsafe alignment casts or uncharged typed proof arrays. Native layout partitions only the admitted buffers.

Root has also prepared `policy/grant/scoped.rs`: actual `LocalWholeSourceGrant::accept_local_scoped<T>(&mut self,pins:&PinAuthority,clock:impl Fn()->u64,effect:impl for<'g> FnOnce(&mut LocalEffectControl<'g,'data>)->Result<T,PolicyError>)->Result<T,PolicyError>`. Source review found the intended same-owner semantics: spent=true before checks/effect; control retains exact AuthorityOwner/handle/binding/ceiling/BoundClosure plus fixed short pins/clock borrows; recheck samples the trusted clock twice and checks both paired receipts and the same grant; terminal=true before checks, so ignored errors cannot revive it; cancel revokes the same handle; initial/final rechecks surround the effect; raw permits stay local. Existing grant close/Drop reclamation remains unchanged. No async callback or new boolean authority.

Both gates must be rechecked using fresh host time/current policy before every side effect and after every blocking file barrier, especially immediately before checkpoint-header publication and before returning accepted progress. OS file sync cannot be forcibly interrupted; cancellation during a syscall is noticed on its return. The runtime retains all actual I/O ownership through drain, and no late/canceled result can publish a current checkpoint or consumer effect.

## Leaf ordering and failure state

1. Validate active store/generation, exact selected identity/page/index/final encoded length, scratch/file ceilings and complete proof framing. Recheck both concrete gates. Verify actual immutable payload with verify_leaf_view.
2. Recheck immediately before effects. Write exact payload and fixed proof slot into the canonical RangeSession. A write error may have changed a prefix: transition Poisoned/Uncertain, return fixed error, no current checkpoint progress.
3. Call actual payload sync_all and proof sync_all. Recheck gates after each returns. No bitmap/checkpoint success may precede both successful barriers.
4. Borrow the exact retained workspace current page after owner/incarnation/full identity/page checks. It starts zero on workspace admission and retains ONLY marks inserted using matching actual VerifiedLeafView. Reborrow it as CandidateBitmapPage without clearing previously verified current-generation marks. Resume bits never initialize it as verified.
5. Encode the400-byte checkpoint using that page and the checked current live sequence. Select the alternate bank. Recheck gates; write its4096-byte bitmap payload first, then freshly recheck before writing its400-byte header. Finally call actual checkpoint-file sync_all.
6. Recheck both gates after the final barrier. Only then return LeafBarrierObservation and advance the store's live checkpoint frontier. Cancellation/expiry/revocation at any intermediate/final check is terminal for this effect. Synced orphan bytes do not become accepted progress. A failure poisons the live page/store; reopen/rebuild through reverify, never restore a trusted bit from hints.
7. Repeated identical delivery to the same store does not create another quota attempt or economic operation. Either repeat the bounded verified write/barrier protocol or re-read/reverify actual stored bytes before any optimization; a bitmap alone is insufficient. Conflicting identity/representation/epoch/page/root refuses. No checkpoint sequence rollover; checked overflow refuses before mutation.

The exact array-copy/bitmap transaction strategy should preserve one page budget; failure poisoning avoids a hidden second full-page rollback copy. A bitmap that persists between completed transfers requires its own explicit canonical lifetime accounting; it cannot remain charged only to a finished StorageStage. Root has selected cumulative progress with a real retained SparsePageWorkspace charged as its own distinct resource in the existing ledger. No full-page O(n-squared) reverify loop is hidden in each leaf write. Missing hint bits mean Unknown, never Absent. Persisted checkpoints are hints even when all barriers returned. No financial idempotence or durable receipt type is introduced.

## Resume and verified reads

Read at most two fixed banks for the requested page into leased caller buffers. Validate exact400-byte records, full expected old identity/page, payload digest, reserved/tail bits and file bounds. Highest valid sequence may prioritize work, but is not authoritative freshness; ties/conflicts/forged extreme sequences cannot mint progress. New current-generation sequence comes from live owner state, not blindly from an attacker-recomputable checkpoint.

For each claimed leaf, read actual payload and retained proof into bounded caller buffers, decode existing proof, and call actual verify_leaf_view against freshly admitted descriptor/root. A hint, file extent, cache key, sparse hole or returned all-zero bytes never becomes a current integrity mark by itself. Missing/torn proof or payload is incomplete and requires reacquisition. Even a mathematically valid all-zero leaf must be explicitly re-written with retained proof and pass the write/barrier/checkpoint protocol in the current storage generation after loss of writer exclusion. A decoded bitset cannot construct a VerifiedLeafView or CandidateBitmapPage.

Verified reads always re-read and verify the actual bytes under current gates; they invoke a bounded callback with the existing borrowing integrity view inside the Stage HRTB callback. No borrowed data escapes Stage. This is mandatory on Unix where cooperative locks do not exclude independent ordinary writers. A view does not survive source-buffer reuse. No sparse file reaches a complete-entry tensor/container reader or consumer merely because every hint bit is set.

Concrete phase-C qualification can cover live-session page reload/reverify and a new store facade around the SAME retained RangeSession. Actual process-loss reopening is not supplied by the inspected phase-B constructor: it creates new TempDirs and cleans on normal close/drop. A production crash-recovery path needs a separately reviewed canonical backend recovery owner that verifies exact attempt marker/root ownership, opens retained files with the same bounded/exclusion rules, and ties uncertain old charges to existing authenticated quota recovery. No arbitrary incoming path, copied quota snapshot, or new forgeable charge may supply that authority.

Safe cross-generation recovery recommendation: retain old artifacts under a bounded read-only canonical recovery owner; charge a NEW attempt through the existing CustodyQuota for the new current-generation session; reverify old actual bytes/proofs and rewrite into the new session. Include both old uncertain and new charges until the canonical authenticated snapshot reconciles them. That avoids pretending lost in-process quota tokens can be re-created. This requires root backend/recovery contract approval before implementation; until then, no process-restart completion claim.

## Fault/loss qualification and remaining gates

Actual-module tests should cover:

- Exact payload/proof/checkpoint u64 ceilings; final leaf; page32767/32768; zero entry; invalid count/padding/trailing proof; wrong descriptor/context/entry/source generation/storage generation/privacy/policy/representation/epoch; no I/O on preflight refusal.
- Real native success plus event-order assertions: payload/proof writes -> both real syncs -> bitmap/header write -> checkpoint real sync -> progress. Fault injection before/after every boundary must leave no accepted current checkpoint. Injection is a protocol test only, never a fake backend claiming durable success.
- Torn/corrupt/missing/mismatched banks and repaired unkeyed digests; forged bit for an unwritten hole; legitimate all-zero leaf; missing proof; changed stored bytes. All resume cases reverify/rewrite actual data before current marks.
- Cancel/timeout/revoke between verification, each write, each sync and final progress, including expiry during a blocked operation. Stage release does not release reservations; actual drain/complete_io through the original owner is required. Late completion cannot affect a reused ticket generation.
- Duplicate delivery with no extra quota slot/debit, overflow without mutation, poisoned-store refusal, page switching under one4KiB bitmap, zero raw allocator-call positive control on hot success/reject/read/reverify paths.
- Marker-guarded close/drop/unwind and cleanup-pending retention from phase B; conservative quota usage across every failure.
- Separate actual child-process termination/reopen/loss experiments only after canonical recovery-owner implementation. Windows retained sharing and Unix cooperative-only behavior must remain separate reports. Directory entry/fsync/power-loss qualification is currently unsupported, so even successful file barriers/process-restart tests cannot mint DurableVerifiedRange.

Root decisions needed before code: approve the fixed1537/4496/8992 geometry and expense plus the frozen child/error/test scope; confirm the prepared concrete gates, typed-buffer aggregate check and exact retained-workspace API. Root has explicitly selected live-session-only phase-C scope; canonical crash-recovery ownership remains a separately scoped dependency. Remote policy/projection/private plaintext, complete consumer leases, directory power-loss qualification and authenticated persistent accounting remain separate programme dependencies.

## Approved prerequisite interface (new-source implementation, not tested here)

Root authorized the fixed owned-page child and retained runtime owner first. New sources: storage/bitmap/owned.rs; runtime/sparse_workspace/{mod,types,admission,tests}.rs; runtime/storage_stage/sparse.rs. Parent routes/fields remain root-owned. OwnedIntegrityPage uses the actual existing CandidateBitmapPage fields and mutators, with zero-once initialization, stable reborrow, exact full identity/page and one-way invalidation. No duplicated bit/proof algorithm.

Runtime APIs: acquire_sparse_page(identity,page), retained_sparse_pages(), sparse_inflight_ceiling(), release_sparse_page(workspace)->Result<(),WorkspaceReleaseError{error,workspace}>. Eight bounded slots, same canonical governor/ledger, full workspace size charged, nonwrapping retired generations, wrong-owner/identity/slot/page refusal, close blocked by outstanding workspaces. Release restores only the exact stored owner-issued ChargeRef on the inspected parent release_checked_bytes pre-mutation refusal; every failure returns the actual non-Copy workspace. Drop does not fake a release.

Approved bridge: StorageStage::with_sparse_page_buffers<T>(now,policy,&mut workspace,&mut[StrongDigest],for<'b,'p>FnOnce(&mut StorageStageControl<'b,'owners>,ProofStorageBuffers<'b>,&mut CandidateBitmapPage<'p>)->T). Validate actual workspace owner/table/incarnation first; use actual with_proof_buffers for ticket/lifecycle and ordinary+typed aggregate bound; borrow the already charged real page without another charge. No control/input/page borrow escapes. Parent tables/routes and fixed profile-headroom changes require root integration before this source can compile/run.
