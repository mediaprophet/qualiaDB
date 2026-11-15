# Common QBS09/QBS10 owner integration — exact parent patch request

2026-11-13. NEW owner children are intentionally unrouted during root's current core snapshot. No parent mutation/build by this agent. Review before applying. This supersedes the first draft's long authority-scope/resource wording: authority locks are SHORT per effect; resource scopes hold only actual ledger. Existing runtime lifetime prevents cooperative ledger release between network polls; this owner package alone does not close QBS09 asynchronous integration.

## Approved ownership shape and lock order

ProtectedChannel consumes the actual EstablishedSession plus B:Bearer exactly once. It retains the actual route, independently negotiated MTU, Intent, current paired receipt/transcript/role and ONE PacketProtection. Failure returns original session+bearer. Existing InstalledSessionKeys is zeroized after the one packet owner is made. No traffic key accessor/reinstall/public raw plaintext callback is added.

RetainedEndpointReservation consumes the EXACT original private ReservationHandle without releasing it. Bounded with_resources locks only its original Mutex<ReservationLedger> and lends actual ledger + ChannelIo; the view excludes the reservation and whole channel, so close/drop cannot relock during that borrow. ChannelIo::with_current obtains ONE fresh CurrentAuthorityGuard for one bounded nonblocking effect and drops it on every return, including WouldBlock. Both endpoint and later content checks use that same actual guard with pointer affinity. Lock order is ledger -> short authority. Endpoint consume uses authority without ledger; channel close runs outside resource scope with no authority guard. Adapter send/recv/shutdown must be prompt/non-reentrant and must not relock canonical owners.

try_close first invalidates future data, then requires actual owned bearer shutdown success and releases the SAME canonical charge. Shutdown/release failure retains the owner for retry; release restores only the exact private consumed handle on parent's checked failure-before-mutation. Drop attempts actual shutdown/erases keys/buffers but never refunds an uncertain retained charge. No new ledger/AuthorityOwner/caps reconstruction. available_handles observes actual canonical slots, including every session/other external charge. Original128KiB endpoint charge already includes24KiB transient allowance: new channel+buffers has a compile-time count within it; actual bearer backing remains separately charged by its actual owner.

## Exact parent changes for root (not applied)

1. net/qdnf/authority/mod.rs:

    mod current_guard;
    pub use current_guard::CurrentAuthorityGuard;

2. net/qdnf/session/handshake/endpoint/authority.rs:

    mod current;

3. endpoint/resource.rs:

    mod retained;
    pub(super) use retained::RetainedEndpointReservation;

4. endpoint/established.rs before existing types:

    mod protected_channel;
    pub use protected_channel::{ChannelBuildError, ProtectedChannel};

Add these private fields to EstablishedSession, alongside role:

    intent: super::Intent,
    route: super::Route,
    mtu: u16,

In install's final EstablishedSession initializer add:

    intent: c.intent,
    route: c.route,
    mtu: c.selected,

Keep observed Route.mtu unchanged; it is adapter state pinned by Route::check. Do NOT overwrite it with negotiated selected MTU. Both are retained independently.

5. endpoint/mod.rs change the existing established reexport to:

    pub use established::{ChannelBuildError, EstablishedSession, ProtectedChannel};

6. net/peer/runtime/ledger.rs route NEW read-only child:

    mod headroom;

7. bundle/stream/runtime/owner.rs replace ONLY supported_inflight with the following (const becomes fn):

    pub fn supported_inflight(&self) -> usize {
        let owned_limit = 30usize
            .saturating_sub(self.workspace_live)
            .saturating_sub(self.source_live);
        let actual_headroom = self.live.saturating_add(self.ledger.available_handles());
        owned_limit.min(actual_headroom).min(self.profile.inflight_limit())
    }

This observes reservations already held by the endpoint and all other owners; no32/30 guessed external-handle subtraction. Existing reserve remains the definitive atomic admission check. Review any const caller (none claimed here). Root should add actual external-reservation/runtime headroom regression in its harness. No parent ceiling increases.

8. net/qdnf/session/packet_protection.rs, zeroizing actual open scratch (preserve algorithm/bytes):

    use zeroize::{Zeroize, Zeroizing};

Replace ONLY open's existing:

    let mut body = [0u8; MAX_PROTECTED_BODY];

with:

    let mut body = Zeroizing::new([0u8; MAX_PROTECTED_BODY]);

Existing body slicing/copy use remains identical. This erases on success/error/unwind including replay rejection; channel also uses existing Zeroizing for its real plaintext/frame buffers. No replacement crypto primitive/provider.

9. endpoint/tests.rs add route under its existing private managed producer Fixture, avoiding a duplicated fixture/raw key provider:

    #[path = "established/protected_channel/endpoint_tests.rs"]
    mod channel_tests;

NEW protected_channel/tests.rs is routed by its own new mod.rs and holds pure framing tests. Actual producer/channel tests are endpoint/tests::channel_tests and can reuse the original private Fixture/Signer/new/now/owner/ledger without broad visibility changes.

## Cooperative resource-scope prerequisite for QBS09 service

Current TransferRuntime stores exclusive parent borrows for its entire lifetime. Dropping/recreating it to release the ledger between polls loses its private owner/generation/live tokens and must NOT be used to resume. Holding its MutexGuard across peer wait would monopolize unrelated admissions and block independent close/admission. Therefore with_resources here is bounded/nonblocking and cannot alone implement a cooperative long-lived transfer. No callback waits/loops until peer response; a WouldBlock returns promptly without retaining an authority guard.

Proposed later canonical change (not implemented in this package): extract actual existing persistent runtime state into a non-Copy RuntimeState owning SAME slots, owner identity, serial, static/control charge handles, retained workspace/source tables, control operation and counters. Keep parent ledger/lease/op/scheduler/governor borrows in a short RuntimeScope borrowing that SAME state. Move fields, do not copy mirrors, reconstruct tokens, new caps or make independent runtime roots. RuntimeState admission obtains original parent handles once; each scoped attach revalidates exact original owner affinity and original private handles/generations before effects. State Drop retains unresolved charges/blocks canonical close; consuming release drains actual queues/I/O and returns state on refusal. Existing public TransferRuntime API can be a compatibility owner borrowing parents plus state, while new internal scoped host owner drives the same functions over split state. All state transitions must be factored once, not independently reimplemented in two wrappers.

This is a separate reviewed source claim affecting runtime parents/dispatch/admission/workspace/source composition, with regression existing40+retained/recovery tests and raw-allocation checks. Root should approve exact layout before mutation. Alternative canonical single-host executor may own all these parents for its life and mediate every admission/cancel/close through a bounded queue; it cannot leave blocking Mutex admission paths bypassing that executor. No completed async-native claim until one actual ownership path is implemented/tested.

## New files (unrouted until root applies above)

* net/qdnf/authority/current_guard.rs and current_guard/tests.rs.
* net/peer/runtime/ledger/headroom.rs.
* endpoint/authority/current.rs.
* endpoint/resource/retained.rs and retained/tests.rs.
* endpoint/established/protected_channel/{mod,owner,scope,send,receive,tests,endpoint_tests,endpoint_fixtures}.rs (test children may split before500lines).

No heavy compile here. Root owns routing/builds. Carrier service magic/tags/layout remain a separate next freeze; use actual COSE MAX_ENVELOPE_BYTES for COSE payloads, never a generic96KiB allowance in this owner.

## Resumed source completion, 2026-11-13

Root resumed the task. All planned NEW test children now exist;16 NEW source files were scoped-rustfmt formatted, then the3 handoff files refreshed.21 source tests are authored, not compiled/executed. Handoff refusal preserves the original EndpointReservation via &mut transfer; the original session reservation remains intact until success. Parent routing/fields/zeroizing scratch remain unapplied/root-owned. No worker-owned build or helper process was launched. See qbs09-runtime-state-contract.md for the separate cooperative-runtime proposal when prepared.

## Approved parent batch applied, 2026-11-13

After core66283 completed successfully root authorized application of the exact common-owner recipe.9 parent files changed and16 new source files routed,25 scoped sources formatted successfully using actual crate edition2021. Initial2024 parent formatting refused existing test identifier gen; no identifier edit was made, and correct2021 formatting passed.21 authored tests await root compile/execution. Worker launched no build/helper processes. Runtime state split/incarnation/carrier codec are NOT included. Source is frozen for root integration compile.
