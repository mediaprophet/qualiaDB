# QBS09 common owner pause checkpoint

2026-11-13. USER REQUESTED PAUSE for laptop battery. Implementation stopped after a complete file write. No builds were started by this agent. Do not resume until root receives user authorization to resume.

## Saved decisions and exact review document

Root reviewed qbs09-native-service-contract.md and approved the FIRST common QBS09/QBS10 endpoint/current-authority/reservation owner scope, NEW unrouted children only during its core snapshot. The exact proposed parent edits are saved in qbs09-common-owner-integration.md. No parent routes, existing endpoint/session/ledger/runtime/PacketProtection files, Cargo, registry or programme status files were changed by this agent in this scope.

ONE consumed EstablishedSession + actual owned B:Bearer -> persistent ProtectedChannel, retaining actual receipt/transcript/role/Intent/observed Route and negotiated MTU separately. ONE PacketProtection only; no copied/repeated nonce owner. Original canonical reservation handle is moved without release/re-admit. Explicit actual shutdown precedes charge release; failure returns the same mutable owner for retry. Drop erases/closes but retains unresolved canonical charge.

CurrentAuthorityGuard holds exact actual MutexGuard plus mutex affinity and exposes no public owner/permit. It is SHORT per bounded frame/effect, dropped on WouldBlock. Resource scope holds only the original ledger, not authority; lock order ledger -> short authority. Never wait/await/reenter owners while authority guard held. Bounded resource callback must not wait/loop until peer completion.

CRITICAL OPEN PREREQUISITE: existing TransferRuntime retains exclusive parent borrows for its lifetime, so it cannot release/relock actual ledger between polls while preserving tickets/state. Bounded with_resources alone does NOT complete asynchronous service integration. Root agreed a separate reviewed canonical persistent runtime-state/short parent-borrow separation is needed (move SAME state/handles, no mirror/new ledger), or a true single canonical host executor mediating all admission/cancel/close. Detailed plan is in the parent integration document; not implemented.

Actual ledger available_handles counts real occupied slots including endpoint/external owners; proposed parent supported_inflight becomes non-const and min(existing ceiling, live+actual free slots). No32/30 guessed subtraction for external charges. Root also must replace PacketProtection::open's actual4096-byte body array with existing Zeroizing; exact2-line parent patch is in review document and is NOT applied.

Future service codec still needs exact per-variant freeze and registry/schema collision review. Root suggested QBS1/version1/profile1 but no allocations/code were made. Use actual COSE MAX_ENVELOPE_BYTES per proof, not generic96KiB allowance. OS local_stream bearer is approved only as a later distinct claim. Remote Share/Sentinel bridge remains required; local admin demo does not waive it.

## Saved NEW source files and current state

The9 files below were written at complete file boundaries. They are intentionally UNROUTED and have NOT been rustfmt'd, compiled or executed. Parent references/fields in owner children deliberately require root's reviewed routing/field patch. Test modules referenced by current_guard.rs, retained.rs and protected_channel/mod.rs are NOT YET WRITTEN; this source is not claimed ready to route/build. Planned actual endpoint tests reuse the existing private managed producer Fixture through one new child route in endpoint/tests.rs, avoiding raw-key/duplicate fixture constructors.

Implementation currently includes guard affinity/check/revoke; read-only actual handle headroom; endpoint same-guard helpers; retained original reservation with checked failure restore; consuming channel ownership and shutdown retry; resource/short-authority scopes;1024-byte common chunk geometry/canonical QFrame/actual route checks; exact retained ciphertext retry; existing library AEAD/replay; actual Zeroizing plaintext/frame scratch; receive output guard retained through the outer final session check. Common raw frame APIs are crate-private and explicitly confer no application disclosure authority. Actual service/private codecs still must validate domain and per-record total bound before assembly.

Remaining before root routing/build:

1. Review/refine source and lifetime/visibility assumptions; no compile has verified HRTB resource/current scopes or consuming constructor.
2. Write guard and retained-reservation tests, pure geometry tests, actual received-Finished managed producer/channel tests and required helper children. Test monotonic PN, exact WouldBlock retry, wrong actual owner/route/frame/AAD/replay, between-frame revocation, inner+outer expiry output erasure, catch-error/unwind terminal state, shutdown failure/retry, Drop-retained charge, same ledger/no release gap/real headroom, raw allocation positive control.
3. Current allocation_counter.rs actually exports assert_zero_alloc(strict raw), assert_zero_net_alloc and AllocGuard; no assert_no_alloc_calls symbol in the inspected source. Use actual AllocGuard::begin(label,true)+check/positive control; no new global allocator.
4. Run rustfmt ONLY on owned new files; send stable exact route list/parent diff. Root applies reviewed parents/zeroizing scratch and owns all builds. Do not route before missing test files exist.
5. Review canonical runtime-state separation as separate scope before claiming cooperative QBS09 service ownership. Then freeze service variants and implement provider/client; real OS bearer and remote policy integration follow approved separate claims.

## SHA256 at pause (paths relative to crates/qualia-core-db/src)

net/qdnf/authority/current_guard.rs eae474287c9a036e923c79bad7bfe4680ae22c5b3c1f5d18f3891231329a964e
net/peer/runtime/ledger/headroom.rs e3d92d04a1e189df24a39f135fa5c295cb03a8471c83ac48c637dd334c924b11
net/qdnf/session/handshake/endpoint/authority/current.rs 30c26ed1f43514943adc316f6dce3c0b7ac13c155a871255731d916310c28ca7
net/qdnf/session/handshake/endpoint/resource/retained.rs b6664bdf19883b5b74d3ee309cd0fb4589099173e121f4edf5b93cbde2a534d9
net/qdnf/session/handshake/endpoint/established/protected_channel/mod.rs cc0ed95ed613d232f7483dc464ce229e8f8ed57dd5ed31d72fbab650db43d1e2
net/qdnf/session/handshake/endpoint/established/protected_channel/owner.rs c998a8d15ed01408ed284e5ffb85c421b75d0924d3192333037de1b6b1c15ba0
net/qdnf/session/handshake/endpoint/established/protected_channel/scope.rs 4dfd70fdb6e208c659ac58f35245ee4314a59b28b0ef99db7412099b43fb229d
net/qdnf/session/handshake/endpoint/established/protected_channel/send.rs 5332329f4688d790a4194d4de9fd01d7517e83524c0ef59988347f545d804002
net/qdnf/session/handshake/endpoint/established/protected_channel/receive.rs fcca5cec33be475454960549839da6c8e932583a3e23b64012c6278734b22838
