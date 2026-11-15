# QBS09 runtime MOVE/restore — exact parent integration batch

2026-11-13. Prepared while root core20380 freezes routed sources. THIS RECIPE IS NOT APPLIED.25 new children only; scoped rustfmt2021 exit0,13 tests authored but not type-checked/run. Root review required before lifting routes. Apply the WHOLE batch atomically before build: the old constructor/close bodies must be removed when shared children route, leaving ONE active implementation. Selected design: qbs09-runtime-state-contract.md.

## 1. Canonical parent incarnation/inspection routes and private fields

net/peer/runtime/mod.rs add:

    pub(crate) mod incarnation;

For EACH existing parent file ledger.rs, leases.rs, cancel.rs, scheduler.rs add:

    mod incarnation;
    mod inspect;

Add private field in ReservationLedger, LeaseTable, OperationTable and FairScheduler:

    incarnation: super::incarnation::CanonicalIncarnation,

Their existing const constructors initialize this field:

    incarnation: super::incarnation::CanonicalIncarnation::UNASSIGNED,

Locations: ReservationLedger::new Self initializer; LeaseTable::new Self initializer; OperationTable::new Self initializer; FairScheduler::with_quanta Self initializer (new calls with_quanta). No numeric caps/slot generations/queues/limits otherwise change. Parent values remain non-Clone/non-Copy. Child bind methods assign once at first actual registration; reattach checks existing ID and never binds/reassigns a replacement. No public setter/reset/import. This assignment is local value identity, not network/crypto/budget authority.

NEW children route to those fields: runtime/incarnation.rs; each parent’s incarnation.rs and inspect.rs. Inspectors read exact private actual handle/slot/capacity/current cancel/backing lease/budget/full queued record without releasing/minting. Existing public API unchanged. Actual native endpoint reservation shared ledger admission can proceed between scopes.

## 2. Existing streaming runtime module and struct

bundle/stream/runtime/mod.rs add:

    mod state;
    pub(crate) use state::{RetainedRuntime, RuntimeAccess, RuntimeCloseError, RuntimeParents};

Do NOT replace the existing public TransferRuntime export or any ticket/credit/page/source/stage types. Existing transition/stage files remain unchanged.

bundle/stream/runtime/owner.rs add:

    mod counted;
    mod close;
    use super::state::RuntimeClosePhase;

Add private pub(super) metadata fields to TransferRuntime alongside its control operation:

    pub(super) static_charge_bytes: u64,
    pub(super) control_budget: ResourceBudget,
    pub(super) close_phase: RuntimeClosePhase,

These exact fields are included by new RuntimeData::take/attach destructuring. All other current fields stay exactly as-is. Keep NEXT_OWNER as the ONE existing runtime owner ID counter. Do not create another owner ID allocator in state code; per-poll owner/generation never changes.

## 3. ONE shared counted constructor

Replace existing TransferRuntime::new BODY ONLY with:

    let static_bytes = u64::try_from(core::mem::size_of::<Self>())
        .map_err(|_| QdnfError::Range)?;
    Self::new_counted(ledger, leases, operations, scheduler,
        governor, profile, control, static_bytes)

Preserve its existing exact public signature. Existing complete admission algorithm lives ONCE in NEW owner/counted.rs. It preserves validated nonzero control, checked window, SAME ResourceGovernor/SAME ledger all-or-nothing reserve/lease/operation rollback, same schedule ID/owner counter. It initializes new metadata. Private new_counted rejects bytes below sizeof<TransferRuntime>; retained admission supplies full actual shell+scope+move-transit+guard+registration/access size. Static charge remains the ONE original handle, no new cap/token/second handle/release gap. Existing WorkClass/essential floor/32-slot ceilings stay unchanged.

After replacement remove now-unused owner.rs Ordering import if any (child counted.rs imports its own Ordering). Keep parent Slot/schedule_id imports used by its private counted child; compiler may identify any other cleanup. No unrelated formatting/semantic edits.

## 4. ONE shared phase-tracked close

REMOVE old public TransferRuntime::close body/method from owner.rs when owner/close.rs routes. The same public close method is implemented ONCE in NEW owner/close.rs. Recorded phases keep actual already-completed control operation/charge progress on refusal. Cancelled completion means actual control completion occurred and progresses once. Ledger checked release restores only exact original handle on pre-mutation failure; counters decrement after successful release. No replay of consumed control operation and no fake refund on failure.

Update existing require_open condition ONLY:

    if self.closed || self.close_phase != RuntimeClosePhase::Open {
        Err(QdnfError::Closed)
    } else {
        Ok(())
    }

This prevents fresh effects after a partially progressed close. Existing live/queued/workspace/source drain refusals remain. Normal legacy close behavior and public signatures stay the same; consuming RetainedRuntime close returns original state on refusal and clears registration only after actual final close.

## 5. State implementation/source ownership

New state/data.rs holds the exact current nonborrowed state fields, moved not cloned, including native source/workspace slots/generations/charge handles and close metadata. state/restore.rs Drop moves them back unconditionally/infallibly. state/parents.rs captures actual immutable mutex bindings plus4 per-value IDs and one active owner; no new ledger. state/loans.rs acquires fixed try_lock order. state/preflight.rs validates all actual backing/queues/accounting BEFORE state take. state/scope.rs catches callback unwind inside guard lifetime, restores, drops actual guards, then resumes original panic. state/access.rs only forwards existing engine APIs; no DerefMut/raw runtime/parent escape or second transition implementation. state/close.rs consumes owner or returns original on refusal.

All current transfer.rs/dispatch.rs/plan.rs/storage_stage.rs and controlled/sparse/recovery children/admission routines remain unchanged. StorageStage still borrows the actual temporary legacy engine, and HRTB prevents escape. No format, descriptor, proof, crypto, wire or provider edits. No authority-incarnation parent changes: existing trusted host MUST NOT replace AuthorityOwner contents while endpoint grants are live; separate follow-up is documented.

New short scope explicitly forbids peer wait/await or actual synchronous file read/write/sync. Source cold opening/native storage-job handoff and common channel no-double-ledger-lock bridge are later REQUIRED integration, not waived. Do not call full service cooperative/native-complete from this state batch alone. RuntimeAccess exposes storage_stage for bounded memory/proof checks only; no blocking source-open forwarder is exposed. Existing synchronous legacy API remains intact.

## 6. Focused validation after review

Root owns builds. First type-check/new tests using canonical project recipe, filters:

* bundle::stream::runtime::state::tests
* net::peer::runtime::incarnation::tests
* Existing bundle::stream::runtime tests (including workspace/source/control/storage-stage)
* Parent net::peer::runtime tests and previous common endpoint channel21 tests

New state tests: same-address replacement EACH actual table including deliberately matching occupied slots/generations/budgets; restore exact original values and use same original ticket; foreign registration; callback panic followed by original ticket cancellation/drain/generation reuse and no poisoned parent locks; unrelated actual EndpointReservation admission between polls/real headroom; contended later mutex/no earlier lock leak; lost state retains charge/blocks close; partial-close checked refusal and no repeated control completion; actual bitmap pointer/integrity marks/charge preserved; real OperationTable cancel observed; actual raw allocator positive control;256-poll actual time/logical move/static-byte observation without arbitrary latency claim.

No builds/processes started by this worker. Current tests/fingerprints are source evidence only. Fix any compiler/lifetime errors in scoped children after root reports; do not modify frozen routed sources during build.

## Actual direct-source harness boundary

Inspected tools/qbdl-stream-conformance/src/lib.rs and Cargo.toml: actual stream_runtime and parent_runtime are imported; full endpoint tree/managed endpoint owners are NOT present. No custom harness cfg convention was found in inspected files. Pure state tests use actual ResourceGovernor::charge_unknown admission/release on the canonical ledger between polls; they do not import/stub EndpointReservation. Genuine endpoint/channel/resource integration is in new endpoint_runtime_tests.rs under the existing full-core managed endpoint Fixture. No replacement owner or Cargo/cfg bypass was added. Root owns harness routing.

## Exact NEW file ownership / fingerprints

net/peer/runtime/incarnation.rs 94a53b5d8e789601014a6bbcd41cbba1900cac6c0dfdbf0b6b7af08c2f88aba5
net/peer/runtime/ledger/incarnation.rs d685a1460dd09c23675a8f5453f87a596f46c9d2004e97c60bf7e66d943728e0
net/peer/runtime/leases/incarnation.rs 533c81d2705b6e70d7fb6556f6f382f396c84aa878826e17b4e42639b42bd0d4
net/peer/runtime/cancel/incarnation.rs d43287d269953679ba18a8f20910a901593c6f94a975b21d474ddff14cede02b
net/peer/runtime/scheduler/incarnation.rs f05ec1fa5164447d33f1441446a7ec7ca90c15ec66d67a82e87447b13f6ac7f1
bundle/stream/runtime/state/data.rs 06d91891e90fb22184c393ea192e7f2f53d5c89a53189a8fc93c40df8bca0f91
bundle/stream/runtime/state/restore.rs 55d9dc3986ba25b21ed4d21c5fd0660793014f0e45bee83584137df6274a176b
net/peer/runtime/ledger/inspect.rs e33e6fc4946814d42ce821428c526134e606bdf570933bb7a91b884db20888f7
net/peer/runtime/leases/inspect.rs cf730a428a568f44838f2c79a473d08d9714f4a4c728d624343332798ca880b3
net/peer/runtime/cancel/inspect.rs 26abc3cf9b291f024b98395505842b899208dc9ccdd7b4ca3ec284935a506bfe
net/peer/runtime/scheduler/inspect.rs 2bd0e8f86d467446f0568ce7ab5b82c7673059e8eecf3ccfc4daf1f7288d3d92
bundle/stream/runtime/state/parents.rs 5bf5952189d7ef018fb5a527ac1b047e5d0e176a43cae21ef498ca1fb4ba165b
bundle/stream/runtime/state/loans.rs 14bf18797c65d5848c94c19f559bc899a7788f7aed06b88c64f6fb80b66993c0
bundle/stream/runtime/state/mod.rs 340b2b928ec5bec101194a672c71652145d831ae411d6ac3e991204daf1de5c1
bundle/stream/runtime/owner/counted.rs 6947e4290731f542334318422c3597b05f9499dff424a3f89359ee807729054e
bundle/stream/runtime/owner/close.rs df6d01bfe7cda2d35aeb278dbccee7c9e9fb3d7c9687b7407d684508ed786077
bundle/stream/runtime/state/access.rs ebaf620aec6dec8044f31255d9b96764889ecaf2b0ee4b686d0768f09046f517
bundle/stream/runtime/state/preflight.rs 319722f4ebc73e90e8d006a47bb8195f82979f90d9927327b1fdba31d68f6ed9
bundle/stream/runtime/state/scope.rs 1a997d462a9c6a6cb3f0f3a4969b11dee6e81608728810d3d06a4b6c202a4bf4
bundle/stream/runtime/state/close.rs 435a63a04186609d3f0c5b4e1de351faebb50639cb519bc8a7739fbd52668d02
bundle/stream/runtime/state/tests.rs 6d61671495a7e54a559c34999beeb8b26cd18ab2bd9911a40518e993ab7d7654
bundle/stream/runtime/state/tests/fixtures.rs f570b61474107e7374ad0c04a845786c5c5dfeb787cfd8e0de02d68ee445ad87
bundle/stream/runtime/state/tests/lifecycle.rs ee43bc6676bd10ab4f29c13d4c2ab7eed5c5f38af5a9906cac08d75310a31125
bundle/stream/runtime/state/tests/identity.rs 229504283e4ef33f4cff6b876b3f9f607c90ba9309a82c05d8304db646ab4f6f
bundle/stream/runtime/state/tests/memory.rs 38a0a634b4cd771e4ddbd52c05629da8801e80cef40fadf0d0550b852ab8e770
