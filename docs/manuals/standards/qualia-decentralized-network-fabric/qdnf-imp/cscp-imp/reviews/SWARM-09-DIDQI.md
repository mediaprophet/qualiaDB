# Independent review — CSCP-09.02 + DID-QI spec/impl

**Reviewer:** swarm review lane (did not author `cscp_h2.rs`, `h2_capsule.rs`, `did-qi-method.md`, or `did_qi/`). Not approved from the implementer handoffs.  
**Date:** 2026-09-10  
**Branch:** `0.0.38`  
**HEAD read:** `a8164506` plus integrator-wired working tree (`pub mod cscp_h2`, `pub mod h2_capsule`, `pub mod did_qi`)  
**Scope:** child CSCP-09.02 (loopback TLS HTTP/2 Extended CONNECT + capsules + CSCP/QSession) and children DID-QI-SPEC / DID-QI-IMPL. Parent CSCP-09 Internet/MASQUE/HTTP/3 and parent DID-QI registration are out of scope for accept.

This reviewer does not edit `.rs`, honesty flags, `workstream.md`, or `task-registry.json`.

## Verdicts

| Delivery | Verdict |
|---|---|
| **CSCP-09.02** loopback TLS HTTP/2 Extended CONNECT + RFC 9297 DATAGRAM capsules + CSCP/QSession | **accept** (child only) |
| **DID-QI-SPEC** `did:qi` method specification | **accept** |
| **DID-QI-IMPL** `did_qi/` CRUD runtime | **accept-with-fixes** |

Parent **CSCP-09** (Internet HTTP/2 CONNECT / MASQUE) remains incomplete. Parent **DID-QI** (registered method, in-crate spec-vector harness, live git/UTXO) remains incomplete. Do not tick CSCP-08/12. Do not set Internet honesty flags.

A clean **accept** of DID-QI-IMPL would require F1–F4. **Reject** of CSCP-09.02 would require POST-as-CONNECT, raw DATA without DATAGRAM capsules, an honesty flag true, a public relay URL, or `Vec` on `fabric/capsule.rs` encode/decode. Those were not found.

---

## Independent checks (this session)

```text
CARGO_TARGET_DIR=/tmp/qdnf-continue-target
cargo test -p qualia-core-db --lib --offline p2p::connectivity::cscp_h2 -- --test-threads=1
# 4 passed, 0 failed; 0.25s  (rustc 1.98.1, Linux x86_64)

cargo test -p qualia-core-db --lib --offline did_qi -- --test-threads=1
# 22 passed, 0 failed; 0.32s
```

Logs: `/opt/cursor/artifacts/review-cscp-09-h2-cargo-test.log`, `/opt/cursor/artifacts/review-did-qi-cargo-test.log`.

Spec §20 vectors (Python `hashlib` + `cryptography` Ed25519, no network): genesis digest, DID, unsigned digests, `proofValue`, git blob ids, live and tombstone txids, and RFC 8032 signatures all match the published table. Log: `/opt/cursor/artifacts/review-did-qi-spec-vectors.log`.

Author-claimed “4 H2 + 22 DID-QI” reproduces in-crate after integrator `pub mod`. That measures the in-tree tests. It does not measure missing DID-QI Read reconstruction or Vector 2/3 byte identity at runtime.

---

## CSCP-09.02

Brief: two loopback TLS HTTP/2 peers, RFC 8441 Extended CONNECT `:protocol=capsule`, RFC 9297 DATAGRAM capsules carrying CSCP ConnectRequest/Accept, then QSession. Not MASQUE Internet. Honesty flags stay false.

### 1. Extended CONNECT, not POST — **hold**

`h2_capsule.rs` `loopback_tls_h2_connect`:

- Server: `h2::server::Builder::enable_connect_protocol()` → SETTINGS_ENABLE_CONNECT_PROTOCOL=1.
- Client: waits until `is_extended_connect_protocol_enabled()`, then `Method::CONNECT` + `Version::HTTP_2` + `extension(Protocol::from_static(protocol))` + URI `https://localhost/` (`:scheme` / `:authority` / `:path` as RFC 8441 requires for extended CONNECT).
- Server rejects unless `req.extensions().get::<Protocol>()` is `Some("capsule")`. Wrong-protocol test uses `"websocket"` and is rejected.

`h2` 0.4.15 `server.rs`: `:protocol` on a non-CONNECT request is malformed; classic CONNECT forbids `:scheme`/`:path`; extended CONNECT requires them. This is RFC 8441 Extended CONNECT, not a POST upgrade.

**Residual:** the local server match does not re-check `req.method() == CONNECT`. The `h2` crate already refuses `:protocol` on non-CONNECT. Not a fake POST.

### 2. Capsules actually used — **hold**

CSCP bytes are not written as raw HTTP/2 DATA.

- `CapsuleEndpoint::send_payload` → `encode_stream_datagram` → `encode_datagram_capsule` (fabric, DATAGRAM type `0x00`) when payload ≤ 1024.
- Recv: `take_capsule` / `drain_capsules`. Type `0x00` yields the value; `0x01..=0x3f` skipped; `>= 0x40` → `unknown critical capsule`.
- Unframed CSCP magic (`0x43…`) would classify as unknown-critical (`>= 0x40`). The happy-path test cannot pass unless DATAGRAM framing is on the stream.
- Unknown-critical test injects fabric `encode_capsule(0x40, b"y")` via `send_raw_capsule` and fails closed.

`fabric/capsule.rs` remains framing-only: caller buffers, `MAX_CAPSULE_VALUE` 1024, no `Vec`/`String`/`Box`.

### 3. Honesty flags — **hold** (false)

`public_relay_dialed` and `masque_bound_udp_internet_executed` are `pub const fn` returning `false` in `net/peer/connectivity/evidence.rs`. This child cannot set them. Tests assert both remain false. No MASQUE/HTTP/3/QUIC stack.

### 4. Invented URL — **hold** (not a public relay)

Bind is `127.0.0.1:0`. TLS SNI is `localhost`. Request URI is `https://localhost/` as the Extended CONNECT authority, not an operator MASQUE URL. No `wss://`, no `/dns4/`, no invented public relay.

**Residual:** QSession still uses `did:q42:cscp-a` / `did:q42:cscp-b` as local binding hashes (copied from `cscp_wss.rs`). That is QRC-shaped test identity, not a DID method claim and not a public URL.

### 5. File length — **hold**

| File | Lines |
|---|---|
| `h2_capsule.rs` | 430 |
| `cscp_h2.rs` | 253 |
| `fabric/capsule.rs` | 346 |

All under 500.

### 6. Vec on hot CSCP encode path — **hold** on CSCP/capsule encode; I/O pump allocates

`encode_connect_request` writes a stack `[u8; 512]`. `fabric/capsule.rs` encode/decode: no `Vec`. `encode_stream_datagram` uses `[u8; 2064]` then `Bytes::copy_from_slice`.

`h2_capsule.rs` channel pump: `Outgoing::Datagram(Vec<u8>)`, `payload.to_vec()`, `recv_payload -> Vec<u8>`, `acc: Vec<u8>`, `drain_capsules -> Vec<Vec<u8>>`. That is cold I/O, same class as WSS. Not the CSCP TLV encoder.

### 7. Sequence vs brief — **hold** (with residual)

`cscp_then_qsession_over_local_tls_h2`: ConnectRequest DATAGRAM → `connect_from_wire` → kernel Relayed → ConnectAccept DATAGRAM → `handshake_over_fragments` on the same CONNECT stream → `SessionBinding` Active. `ProtectionPolicy::RELAY_ONLY`, `public_dht` checked false, `PathClass::Relayed`. Nested recovery labelled degraded / not UDP / not MASQUE in rustdoc.

**Residual:** Accept evidence is `TransportWitness::from_local(PathClass::Relayed, …)` with no relay hop (same fixture class as CSCP-05 WSS). `H2CapsuleBearer::profile()` is `BearerProfile::UdpTransitionV1` on an HTTP/2 stream. QSession fragments (MIN_QDNF_MTU 1280) use a local DATAGRAM encoder with `STREAM_VALUE_MAX` 2048 because fabric `MAX_CAPSULE_VALUE` is 1024. Still not UDP/MASQUE.

Parent CSCP-09 Internet path is not this child.

### CSCP-09.02 defects

None that fail the child brief. Residuals above are not must-fix for loopback accept.

---

## DID-QI-SPEC

Brief: complete W3C DID method spec (CRUD, security, privacy, test vectors).

### Hold

- DID Core operations: Create §11, Read §12, Update §13, Deactivate §14, with algorithms and fail-closed tokens (§4). Not stubs.
- Security §18 (12 items) and privacy §19 (7 items) are present. GitHub-as-operator forbidden (§16.4). Read is not a public DHT. `did:q42` is QRC, not this method. `did:web` is Frontdoor alias only. ☉ appears in **prose** only; DID strings in §6/§20 are ASCII `did:qi:z…`.
- §20 vectors are internally consistent: this session recomputed SHA-256, Bitcoin base58btc, RFC 8032 signatures, git `blob <len>\0` SHA-256 ids, and Bitcoin-family txids from the published QCDE-1 bytes. They match. They are reproducible offline from the spec text.
- Naming matches DID-QI-01 and `human-centric-nomenclature.md`: method `qi`, not `hcai`/`hci`/`qualia`/`q42`. `did:hcinet` reserved, not specified. Method not registered (stated).

### Residual (not blocking spec accept)

- Namespace IRI `https://webizen.network/ns/did-qi/v1` is an unpublished JSON-LD name; spec forbids fetching it for Read. Honest, not a live vocab.
- Constitution list for `chain_id` is specified, not populated (vectors demonstrate `bip122:` parameterization).
- Enforcement of 8192/9216 octet caps is specified, not measured (no runtime in this child).

---

## DID-QI-IMPL

Brief: working create/read/update/deactivate against the spec.

Create/update/deactivate are real Ed25519 operations over QCDE-1 unsigned bytes and a 32-slot in-process store. They are not `todo!()`. Tests: 22 passed including Vector 1 DID, CRUD happy path, UTXO OP_RETURN, reject `did:q42`/`did:hcai`/`did:hci`/`did:qualia`/hostname, relay-only Direct via `LocatorClass::Direct`. No github.com clone, no DHT, no Chronik. `METHOD_NAME` is `qi`. ☉ is rustdoc prose only.

That is not a complete implementation of spec §12 Read or of the §20 harness the spec child assigned to this runtime.

### Spec vs runtime mismatches

| Spec | Runtime |
|---|---|
| Read returns the current document (§12, DID Core) | `decode_canonical` scrapes `generation` / `deactivated` / `createdUnix` / `publicKeyMultibase` / optional `locatorKind:direct`. It does **not** restore `previousDigest`, `alsoKnownAs`, contact key, relay hints, `publicDht`, or `HostnameAlias`. `update` writes `aka`; `read` after update only asserts `generation == 1`. |
| `stale_generation` when a verified gen *n* is presented after current *m* > *n* (§12.7, Vector 2 pass) | `QiError::StaleGeneration` is **never constructed**. `GitObjectStore::is_stale` exists; `read` always returns `find_latest`. |
| §16 git object = `blob` \|\| QCDE-1(**signed** document); Vector 1 `qiGitObjectId` | Store payload is `flags \|\| generation u64 BE \|\| unsigned QCDE-1 \|\| 64-byte sig`. `blob_object_id` hashes **that** packing. Spec Vector 1 git id is not what `GitObjectStore` records. `encode_signed` exists and is unused by the slot. |
| Vector 2: `HostnameAlias` + `alsoKnownAs` QCDE-1 | Encoder emits only `CscpMailbox`. No `HostnameAlias` type. Vector 2 unsigned bytes cannot be produced. |
| Vector 3 gen 2 tombstone QCDE-1 | `deactivate` after create is gen 1 (algorithm current+1 is correct; Vector 3 needs Vector 2 first). Encoder still cannot match Vector 2 `previousDigest` without HostnameAlias. |
| §10.3 forbidden locator **field names** and IP string values | `check_relay_only` only tests `LocatorClass::Direct` and `public_dht`. No scan for `ipv6`/`port`/`host`. Encoder has no such fields; a foreign QCDE-1 with `locatorKind: mailbox` plus `ipv6` would not be rejected by the struct path. `decode_canonical` does catch `"locatorKind":"direct"`. |
| Unsigned cap 8192; services 8; hints 8; hintId 64 | `MAX_CANONICAL` 2048; `MAX_SERVICES` 4; `MAX_HINTS` 2; `MAX_HINT_ID` 32. Spec vectors 1–3 fit 2048. A conformant 3000-octet document cannot be encoded. Handoff states the 2048 subset. |
| UTXO `txid` metadata = SHA-256d **display** (byte-reversed) (§17.2) | `Outpoint.txid` is SHA-256d **wire** (handoff says so). Vector 1 test checks digest prefix + `vout`, not the spec display txid. |
| §14 / §17.3 tombstone = new signed document, empty `service`, `qi.deactivated` true | `apply_utxo_attestation` on `5149ff` re-packs the **same** unsigned bytes + same signature with `FLAG_UTXO_TOMBSTONE` and generation+1. `read` then forces `deactivated` from the flag. JSON still has the old mailbox. That is not a §14 tombstone document. `deactivate()` itself does empty services and re-signs. |
| Error tokens `invalid_did` / `not_found` / … | `QiError` enum (`RejectedMethod`, `NotFound`, …). Mapping is local, not the spec token strings. |

### Stub / incomplete CRUD

Not stub Create/Update/Deactivate. **Read is incomplete:** signature over stored unsigned bytes is verified; the returned `QiDocument` is not the stored document. That fails DID Core Read and spec §12 output.

No public DHT. GitHub is not the method. QRC is rejected. Vector 1 DID and Vector 1 live tx parse are real. Vector 2 QCDE-1 and Vector 1 git object id are not reproduced by this runtime.

### DID-QI-IMPL must-fix

### F1 — Read must reconstruct the stored document (must fix)

**Where:** `did_qi/document.rs` `decode_canonical`; `method.rs` `read`.

**Scenario:** `create` a mailbox with contact key + hint; `update` with `alsoKnownAs: did:web:example.invalid`. `read` returns `generation` and `controller_pk` only. Contact key is `[0;32]`; `aka_count` is 0; `has_previous` is false.

**Fix:** parse QCDE-1 members this encoder emits (or keep a typed record alongside unsigned bytes). `read` must return services, aka, `previousDigest`, deactivated, and generation consistent with the signed unsigned document. Add a round-trip test that asserts those fields.

### F2 — Spec §20 Vector 1 unsigned / proof / git id, and Vector 2 QCDE-1 (must fix)

**Where:** `encode_unsigned` / `encode_signed` tests; `emit_service`; git slot payload.

**Scenario:** Vector 1 pass criteria require QCDE-1 unsigned match, Ed25519 verify of published `proofValue`, and `qiGitObjectId`. Only the DID (genesis digest) is asserted. Vector 2 requires a `HostnameAlias` service object this encoder cannot emit.

**Fix:** test Vector 1 `unsignedDigest` and `proofValue` against RFC 8032 vector 1. Hash `encode_signed` as spec §16 and match `18962a63…` (or stop claiming §16 git ids for packed slots). Emit `HostnameAlias` (or fail the type as unsupported and **do not** claim Vector 2). `StaleGeneration` must be returned when an old signed generation is presented as current.

### F3 — Relay-only Direct is more than `locatorKind` (must fix)

**Where:** `service.rs` `check_relay_only`; `decode_canonical`.

**Scenario:** spec Vector 4 JSON includes `locatorKind: direct`, `ipv6`, and `port`. The struct API rejects `LocatorClass::Direct`. A document that only has forbidden field names (spec §10.3 item 2) is not checked. If Read ever accepts foreign QCDE-1, those locators can sit in the store.

**Fix:** on encode and on any JSON ingest, apply §10.3 items 1–4 (kind, field names, IP/`host:port` strings, `publicDht`). Test Vector 4 bytes, not only `CscpMailbox::direct`.

### F4 — UTXO tombstone must not impersonate §14 Deactivate (must fix)

**Where:** `utxo.rs` `apply_utxo_attestation`.

**Scenario:** live OP_RETURN matches; tombstone tx `6a235149ff` + 32 zeros bumps store generation, sets FLAG, `read` reports deactivated while unsigned JSON still contains `CscpMailbox`. Further `update` fails. Spec §14 requires empty `service` and a new signature. Spec §17.3 corroborates Deactivate; it does not rewrite the document without a proof.

**Fix:** tombstone corroboration either (a) requires an already-deactivated current document, or (b) writes a new signed §14 tombstone. Do not keep a live mailbox in the current unsigned bytes.

---

## Residual (not blocking the verdicts above)

- CSCP-09.02 local Relayed evidence without a relay; `UdpTransitionV1` profile on H2; stream DATAGRAM cap 2048 vs fabric 1024; I/O `Vec`.
- DID-QI-IMPL: 2048 vs 8192 cap (stated in handoff); no constitution list (`unsupported_chain`); `proof.created` frozen to `2026-09-10T00:00:00Z`; `DID:QI:` case folding is spec MAY, not implemented; `document.rs` 497 lines.
- Method `qi` is not registered. CSCP-08 URL and CSCP-12 datatracker remain human-owned.
- `independent_protocol_review_executed()` stays false. This markdown is not that flag.

## What is complete enough (do not over-claim)

CSCP-09.02: loopback rustls HTTP/2 Extended CONNECT with `:protocol=capsule`, CSCP ConnectRequest/Accept as DATAGRAM capsules, QSession Active, honesty flags false. Not MASQUE, not QUIC, not Internet.

DID-QI-SPEC: a DID method spec with CRUD, encodings, security/privacy, and reproducible offline vectors. Not a registry entry.

DID-QI-IMPL: in-process create/update/deactivate and a verifying Read of a **restricted** document profile; Vector 1 DID; method-string rejects; relay-only Direct on the struct path; caller-supplied Bitcoin-family OP_RETURN. Not spec §20 complete, not GitHub, not a DHT, not Gate B.

## Recommended integrator actions

1. Do not tick CSCP-09 parent, DID-QI parent, CSCP-08, or CSCP-12 from this review.
2. Apply DID-QI-IMPL F1–F4; re-run `cargo test -p qualia-core-db --lib --offline did_qi -- --test-threads=1`.
3. CSCP-09.02 child may move to checked **after** this accept; keep parent CSCP-09 open.
4. Do not set Internet honesty flags. Do not register `qi`.
