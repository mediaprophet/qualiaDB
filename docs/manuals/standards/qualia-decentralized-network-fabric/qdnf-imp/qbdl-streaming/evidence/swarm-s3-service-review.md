open_bugs: 0
pass: true

# Swarm S3.1 — in-process QBDL service independent review

Read-only source review of Wave S3.1 in-process provider/client. Production Rust was not edited. Cargo was not run. Author summary (`swarm-s3-service-summary.md`) was not treated as proof. Contract: `qbs09-native-service-contract.md` in-process / first-profile sections.

Reviewed:

- `crates/qualia-core-db/src/net/peer/replication/qbdl/service/{mod,session,provider,client,tests}.rs`
- `crates/qualia-core-db/src/net/peer/replication/qbdl/mod.rs`
- `qbdl/wire`, `qbdl/carrier`, `qbdl/provider/outboard` (must be unchanged)
- `crates/qualia-core-db/src/net/qdnf/bearer/ipc.rs`
- `evidence/qbs09-native-service-contract.md` in-process sections
- `evidence/swarm-s3-service-summary.md` (claims only, not proof)

## Review

S3.1 is the unrouted in-process first profile: Open → Grant (protocol ack) → Credit → RequestLeaves → LeafData for one selected plaintext entry and one leaf, over the existing `ipc_pair` Arc/Mutex queue. Wire is the already-frozen QBS1 208-byte prefix and `q42:QBDLStream/1`. Carrier assembly and `OutboardCatalog` are reused. `LocalWholeSourceGrant::accept_local_scoped` is the authority; wire `Kind::Grant` is not. Mixed/private/protected wraps are `Unsupported`. Client and provider both call existing `verify_leaf`. Malformed prefix, Cancel, and Close fail-close and shut the pair down (queues cleared). No JSON, no new magic, no second Merkle, no OS two-process, no `PacketProtection`, no `write_leaf` migration.

`qbdl/mod.rs` does not declare `mod service`. `git status` on `qbdl/` plus `bearer/ipc.rs` shows only untracked `qbdl/service/`; wire, carrier, provider, `qbdl/mod.rs`, and `ipc.rs` have no diff. Integrator still owes the one `pub mod service;` line. That is the intended S3.1 split, not a missing implementation.

## Attacks (source, not summary)

1. **Reuses existing wire `PREFIX_BYTES=208` / `SERVICE_IRI`; no new magic; no JSON.** Holds.
   - Service does not define a codec. `service/mod.rs:13` imports `super::wire::{self, Limits, PREFIX_BYTES, SERVICE_IRI}` and `service/mod.rs:23` only aliases them as `SERVICE_PREFIX_BYTES` / `STREAM_SERVICE_IRI`. Compile-time `assert!(PREFIX_BYTES == 208)` and `assert!(SERVICE_IRI.len() == 16)` (`service/mod.rs:25-26`; `session.rs:376`).
   - Records go through existing `wire::encode` / `Header::parse_prefix` / `Message::parse` (`session.rs:124`, `session.rs:217-220`, carrier `accept_inner`). Magic remains `QBS1` (`wire/prefix.rs:73,97`). IRI remains `b"q42:QBDLStream/1"` (`wire/mod.rs:14`).
   - No `serde`, `json`, or new domain/magic in `service/`. Open/Grant bodies are the existing 64-byte `Limits` plus ordinal/page (`session.rs:337-358`). Useful-leaf test asserts the 208 / IRI aliases (`tests.rs:113-114`).

2. **Bearer is in-process `ipc_pair` only; not OS two-process; not HTTP/DNS/IP/libp2p.** Holds.
   - `Session` owns `IpcEndpoint` by value, not a generic `Bearer` and not `local_stream` (`session.rs:1,11,37,50-51`). Tests construct `(client_ep, provider_ep) = ipc_pair(..., 1280)` and assert `BearerProfile::LocalIpcV1` plus `native_independent()` (`tests.rs:134-157`).
   - `ipc.rs:1,64-70,97-117` is an `Arc<Mutex<PairState>>` same-process queue with locators `0x01`/`0x02`. Comment and implementation: no IP/DNS/libp2p. `shutdown` sets `closed` and zeros both queue lengths (`ipc.rs:190-196`).
   - Grep of `service/` finds no `http`, `dns`, `libp2p`, `TcpStream`, `UnixStream`, `named_pipe`, or `std::process`. Two-process join is absent and not claimed.

3. **Wire Grant is protocol ack, not `LocalWholeSourceGrant`.** Holds.
   - `encode_grant_body` writes phase `2`, status `0`, negotiated `Limits`, ordinal, page (`session.rs:348-358`). `Kind::Grant` is the wire tag (`provider.rs:110-121`). Variants already treat Grant as an 80-byte ack (`wire/variants.rs:96-111`).
   - Client `recv_grant_send_credit_request` checks `Kind::Grant`, identity, transfer/entry/leaf/count, `n == 80`, `phase == 2 && status == 0` (`client.rs:1,79-105`). It does not call `adopt_local_whole_source` or install anything on `AuthorityOwner`.
   - Actual authority is the caller’s `LocalWholeSourceGrant`. `transfer_one_leaf` runs the exchange only inside `grant.accept_local_scoped` (`service/mod.rs:152-153,166-176`). Provider `authorize` rechecks that control’s closure identity against the catalog (`provider.rs:252-263`). Wire Grant cannot mint the local grant.

4. **Mixed/private/protected wraps are `Unsupported`.** Holds.
   - `PublicationClass` is only `PlainCandidate | PrivateProjection | RecipientWrapped`. `Representation` is only `Plaintext | Ciphertext`. `refuse_non_plain` allows solely `(PlainCandidate, Plaintext)` and `_ => Unsupported` (`provider.rs:14-22`).
   - `PlainProvider::bind` calls that plus `key_epoch() != 0 => Unsupported` (`provider.rs:53-56`). `PlainClient::bind` refuses non-plaintext or nonzero epoch (`client.rs:40-42`).
   - Test `mixed_private_protected_wraps_are_unsupported` covers PrivateProjection, RecipientWrapped, Ciphertext, the allowed pair, and `bind(PrivateProjection, ...)` (`tests.rs:384-421`). There is no separate Mixed class; the catch-all is the mixed/private/protected gate.

5. **Useful leaf transfer while unrelated payload unrequested; client `verify_leaf`.** Holds.
   - Fixture payload is 2 full leaves + 19 bytes with distinct markers (`tests.rs:21-29`). Spec `leaf_count() == 3`. `transfer_one_leaf` requests only `tail = leaf_count - 1` (`tests.rs:158-171`).
   - Received 19 bytes equal the tail slice; they contain neither leaf-0 `0xA1`/`DEADBEEF` nor leaf-1 `0DD0FEED` (`tests.rs:172-183`). Client `RequestLeaves` uses `count = 1` and that leaf (`client.rs:128-138`). Provider serves exactly that range (`provider.rs:160-238`).
   - Provider looks up via existing `OutboardCatalog::proof_into`, `decode_proof`, then `verify_leaf` before send (`provider.rs:194-205`). Client `recv_leaf` decodes into the 32-sibling scratch and calls the same `verify_leaf` before copying to `payload_out` (`client.rs:184-199`). No new hash/tree in `service/` (grep: no `sha256`/`blake3`/`sha384`/`TreeBuilder`/`merkle`).

6. **Malformed prefix / cancel / close fail-closed; no leaf disclosure after cancel.** Holds.
   - `fail_close` sets `closed` and `cancelled`, `assembly.abort()` (zeroize), and `endpoint.shutdown()` (`session.rs:87-93`). `shutdown` drops both queues (`ipc.rs:190-196`), so a later LeafData cannot remain readable.
   - Malformed: attacker corrupts prefix reserved byte 10 after a valid encode (`tests.rs:228`; `wire/prefix.rs:75` `zero(&b[10..16])`). First `answer_open` is `Error::Malformed`; second is `Error::Closed` (`tests.rs:246-249`). `recv_complete` fail-closes on parse/carrier errors (`session.rs:201-251`).
   - Cancel: after Grant, client `send_cancel` then `answer_leaf` (`tests.rs:301-307`). Recv path treats `Kind::Cancel` as `Recv::Cancel` before any body consume / `split_leaf` (`session.rs:226-245`). `answer_leaf` never reaches source slice or `send_message` of `LeafData`. Error is `Cancelled`; provider session `closed()`; subsequent client send is `Closed`.
   - Close: same shape with `Error::Closed` (`tests.rs:364-375`). Control Cancel/Close bypass the expected-kind check so they cannot be misparsed as leaf payload (`session.rs:226-228`).

7. **No second Merkle. Outboard lookup reused.** Holds.
   - `PlainProvider` stores `OutboardCatalog` (`provider.rs:24-27,64-68`). Proof bytes come from `catalog.proof_into` (`provider.rs:194-197`; `provider/outboard.rs:135-163` uses existing `sibling_ranges` / `encode_proof`).
   - Tests build nodes with existing `TreeBuilder` once, sort, and bind `OutboardCatalog::new` (`tests.rs:63-108`). Service does not emit or hash outboard nodes itself.

8. **Compile hazards: `CarrierError`/`MessageSequence` visibility; `SessionOwner` stack; `LocalEffectControl` lifetimes.** No mismatch in these sources.
   - `carrier` is `pub(crate) mod carrier` (`qbdl/mod.rs:4`). `CarrierError` and the `MessageSequence` re-export are `pub(crate)` (`carrier/mod.rs:9-18`). `session.rs:4-6` imports them as private fields (`session.rs:39-40`). Public API (`Session`, `Error`, `PlainProvider`, `PlainClient`) does not name those types, so routing `pub mod service` should not hit E0446.
   - `SessionOwner` is two `MAX_LEAF_RECORD_BYTES` (18137) lanes plus 1024 control (`session.rs:20-24`) ≈ 37 KiB, not `MAX_RECORD_BYTES` (65776). `Assembly::new` accepts `PREFIX_BYTES..=MAX_RECORD_BYTES` for data and exactly 1024 control (`carrier/receive.rs:38-39`). Tests stack two owners (~75 KiB) plus Fixture heap vecs; that is below the existing ipc pair’s “never materialize ~512KiB on the caller stack” bar (`ipc.rs:30-31`). Extra 16 KiB / 18 KiB scratch in `recv_leaf` / `answer_leaf` is leaf-sized, not a compile failure.
   - `accept_local_scoped` is `for<'g> FnOnce(&mut LocalEffectControl<'g, 'data>) -> Result<T, PolicyError>` (`policy/grant/scoped.rs:55-59`). Service methods take `&mut LocalEffectControl<'_, '_>` (`provider.rs:78,133`; `service/mod.rs:187`). `transfer_one_leaf` wraps `run_one_leaf` in `Ok(...)` so `T = Result<ServedLeaf, Error>` and policy errors map through `map_policy` (`service/mod.rs:166-179`). Clock is `impl Fn() -> u64` forwarded by value. Lifetimes of provider/client/session are independent of `'g`.

## Residuals (allowed; not S3.1 bugs)

- Integrator has not added `pub mod service;` in `qbdl/mod.rs`. Files are unrouted until that line exists. Cargo was not run here; tests are a `service` submodule and will only execute after routing.
- Two-process is not present. No Windows named pipe / Unix-domain QBDL join, no spawn. `Session` is hardcoded to `IpcEndpoint`. Summary lists S3.2/S3.3 as owed; it does not claim OS IPC.
- QBS10 consumed common channel / one `PacketProtection` is not present. Frames are plaintext `ServiceRecordHeader` (88) + ≤1024 chunk (`session.rs:18,132-150`). Summary says so.
- `write_leaf` still holds `StorageStage` across OS wait. Service never calls storage. Summary lists native storage job handoff as owed.
- `policy/grant/current.rs` same-guard bridge is not used. First profile is `LocalWholeSourceGrant` only.
- Describe / DescriptorPage / Select / Resume / Outcome / remote Share/Sentinel are unimplemented, as the first-profile contract allows.
- Recheck is at `answer_open` / `answer_leaf` entry plus `accept_local_scoped` entry/exit, not immediately before every `send_message`. For this sequential in-process queue that is not a disclosure bug; OS/async join must tighten it.
- Open and Credit use distinct `transfer_id` values (`client.rs:55,113`). Cancel is keyed to the Open ticket (`client.rs:202-214`). First profile never credits then cancels the credit id. Later multi-message transfers must keep one transfer id for Credit/Request/LeafData/Cancel.

Author summary does **not** claim two-process, PacketProtection, or `write_leaf` migration. Those residuals stay residuals.

## Issues
