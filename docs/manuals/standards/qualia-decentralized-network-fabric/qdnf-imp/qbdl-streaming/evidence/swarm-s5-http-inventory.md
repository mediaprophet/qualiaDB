# Swarm S5 QBS-14 HTTP/LIG implementer inventory

Read-only brief. QBS-14 is **pending** (`work-packages.md:259–275`; `task-registry.json:540–565`). Wave S5 is **WSS then HTTP serial**. Proposed write scope is **NEW** `crates/qualia-core-db/src/bundle/stream/http/` (directory **not present**; `bundle/stream/mod.rs:1–16` has no `http`). Integrator adds `mod http;`. This inventory does not implement, run cargo, or claim files.

Native interchange is **CBOR / CBOR-LD / exact-length binary**. JSON is exceptional ingest only (Solid/LDP, GIS, MCP). HTTP **headers** are the adapter surface; application objects stay **QBDL**.

Depends on QBS-09/10/12. Reviewer: HTTP + privacy. HTTP/1.1 required; HTTP/2/3 separately reported (`work-packages.md:275`).

## 1. Reusable owners (borrow, do not copy)

| Path | Type / export | HTTP must |
|---|---|---|
| `bundle/stream/proof/` | `verify_leaf` (`verify.rs:5–32`), `encode_proof`/`decode_proof` (`codec.rs:4–33`), `TreeSpec`, `Representation::{Plaintext,Ciphertext}`, `LEAF_BYTES=16384`, `MAX_SIBLINGS=32`, `MAX_LOGICAL_BYTES=1<<40` (`types.rs:3–37`), `verify_leaf_span` / `VerifiedLeafSpan` (`span.rs:16–40`) | **Borrow.** Never a second Merkle codec. |
| `net/peer/replication/qbdl/provider/outboard.rs` | `OutboardCatalog`, `OutboardLimits`, `proof_into` (`outboard.rs:38–137`) | **Reuse** for proof sidecars. Catalog does not authenticate; caller still `verify_leaf`. |
| `bundle/stream/descriptor/` | `SourceCommit` (`types.rs:37–63`; `version!=1 \|\| flags>1` refuse), `PROFILE="qbdl-stream-pq-1"`, `encode_*_page_into` / `decode_*_page` (`pages.rs`) | Same descriptor/pages. Flags 0/1 only. |
| `bundle/stream/index/` | `AdmittedIndex`, `admit_header`; `flags & !1 != 0` → `IndexError::Flags` (`header.rs:21–22`); align 4096 iff `flags==1` else 64 (`header.rs:75–77`); `ExtentLimit.max_logical_bytes=1<<40` (`limits.rs:34–41`) | Same structural admission. Structure ≠ authority. |
| `bundle/stream/policy/` | `authenticate_closure`, `BoundClosure`, `AuthenticatedClosure` (`authenticated.rs:17–40`), `LocalWholeSourceGrant`, `LocalEffectControl` | Identical policy/consent. HTTP status is not a grant. |
| `bundle/stream/selection/entry.rs` | `SelectedPlainEntry` (`entry.rs:38–42`) | Bind served/fetched bytes to this identity. |
| `bundle/stream/formats/range.rs` | `AuthenticatedRange` over `VerifiedLeafSpan` (`range.rs:24–47`) | Consume **after** proof. |
| `bundle/stream/formats/q42_ranges.rs` | `Q42AuthenticatedRangeSource` (`q42_ranges.rs:15–19`) | Q42 over authenticated HTTP bytes. **Not** `HttpRangeSource`. |
| `net/peer/replication/qbdl/service/` | `PlainProvider`, `PlainClient`, `Kind` 1–12 (`wire/prefix.rs:5–17`), `SERVICE_IRI=b"q42:QBDLStream/1"` (`wire/mod.rs:14`) | Semantic parity with Describe/Open/LeafData. HTTP is **not** this 208-byte prefix wire. |
| `net/peer/runtime/ledger.rs` | `ReservationLedger` (`ledger.rs:115–126`) | Charge this ledger. Jobs already say they are not a second ledger (`runtime/jobs/mod.rs:1–2`). |
| `net/qdnf/session/fetch.rs` | `FetchKind::Lig=3` (`fetch.rs:19–23`), `authorize_fetch` (`fetch.rs:95–126`), `reveal_observed` (`fetch.rs:129–139`) | LIG locator reveal after consent. Group/paid ≠ `private_consent`. |
| `net/qdnf/registries.rs` | `BearerProfile` 1–7; `Http2CapsuleTransitionV1=6`; `from_u16(8)=UnknownProfile` (`registries.rs:155–181`) | **Do not mint** an HTTP-range profile. Range GET is not a QFrame bearer. |
| `q42/volume/range.rs` | `Q42ByteRange`, `validate_exact_range_response` (`range.rs:112–125`), private `parse_content_range` (`range.rs:425–447`) | **Contract idea only.** Rewrite in `http/range.rs`. |
| `net/fetch_10d.rs` | `Fetch10dService::fetch_10d_range_by_hash` (`fetch_10d.rs:42–120`) | Keep existing callers. **Not** the QBS-14 contract (`baseline-network.md:70`). |
| `query/ingest_job/source.rs` | `OpenedSource.etag`, `Content-Encoding` → gzip/zstd decode (`source.rs:211–241,353–365`) | Exceptional RDF ingest (incl. `.json` → `JsonLd`, `source.rs:257–258`). **Not** QBDL. |

Architecture (borrow semantics, not registry IDs): single nonempty inclusive range, `end=start+length-1`, `Accept-Encoding: identity`, strong `If-Range`; exact 206 `Content-Range` + pinned total + identity encoding + actual length; abort huge 200; 416 / changed validators = refuse; **reject multipart in R1**; LIG/CSCP reauthorize redirects; cache by descriptor/root/protection/privacy partition (`architecture.md:109–115`). Legacy whole-object fallback ≤ **1 MiB** with explicit policy (`architecture.md:96`). Example paths `/objects/{id}/descriptor`, `.../proof/{leaf}`, `.../representations/{rep}` are **deployment conventions, not allocations** (`architecture.md:111`).

## 2. Defects if `HttpRangeSource` is used as a QBDL adapter

`q42/volume/range.rs` `HttpRangeSource` (`range.rs:168–335`) is a trusted-gateway Q42 tool. Do **not** wrap it.

1. **Allocating.** `reqwest::blocking::Client::builder()`, `url.clone()`, `format!("bytes={}-{}", …)` (`range.rs:190–194,280–286`), `io::Error` `String`s, SHA-256 hasher in `verify_source_sha256` (`range.rs:132–165`). `Q42ByteRange.length: usize` (`range.rs:30–32`) collapses remote `u64`. Not a QBS zero-heap hot path.
2. **JSON.** `HttpRangeSource` does not parse JSON. Adjacent `Q42VolumeProvenance::write_beside_volume` writes `<q42>.provenance.json` (`q42/volume/publication.rs:122–128`). Native QBDL objects are CBOR (`descriptor/` minicbor, `index/` CBOR). Do not import JSON sidecars as representation identity. JSON-LD remains ingest-only (`ingest_job/source.rs:257–258`).
3. **Missing proof.** `verify_source_sha256` compares `[u8; 32]` SHA-256 (`range.rs:132–165`). QBS proofs are SHA-384 `StrongDigest` inclusion (`verify_leaf`, 48-byte siblings, `codec.rs:8`). No `TreeSpec`, `OutboardCatalog`, `SelectedPlainEntry`, or policy.
4. **Transformed content.** No `Accept-Encoding: identity`, no `Content-Encoding` refuse, no `If-Range` / `ETag` / `Last-Modified`. A 206 of gzip bytes with a matching `Content-Range` would pass. `Client::builder()` does not set a redirect policy; reqwest default follow is **UNVERIFIED** against the pinned crate — QBS-14 must reauthorize via LIG/CSCP (`architecture.md:115`).
5. **Status mismatch.** Never accepts 200 (`range.rs:168–169,289–293`) — QBS-14 needs **bounded** 200 fallback (≤1 MiB). `parse_content_range` requires `bytes start-end/total` and `end >= total` fails (`range.rs:444–445`); RFC 416 `bytes */total` is unparsed. Multipart, CORS validators, privacy partition: absent.

`Fetch10dService` is worse: `length as usize` (`fetch_10d.rs:63`), saturating Range end (`:70`), accepts 200 **and** 206 (`:78`), `resp.bytes()` then slice (`:81–90`). Keep separate.

## 3. Proposed NEW files (`bundle/stream/http/`)

Integrator owns `bundle/stream/mod.rs`. HTTP writer adds only these:

| File | One sentence |
|---|---|
| `mod.rs` | Crate-private HTTP/LIG adapter: exports range/status/client/provider; no proof, policy, or ledger copies. |
| `range.rs` | Non-allocating single-range / `Content-Range` exact match (`end=start+length-1`, pinned total); identity encoding; refuse multipart and `bytes */` except as 416 metadata. |
| `status.rs` | Classify 200 / 206 / 416 before body; abort over-budget 200; treat 3xx, changed validators, and encoding as refusal — not data. |
| `client.rs` | HTTP/1.1 required GET: `Range`, `Accept-Encoding: identity`, strong `If-Range` when known; bounded read; LIG `authorize_fetch`+`reveal_observed`; no URL grants. |
| `provider.rs` | Serve descriptor/pages/`encode_proof`/immutable representation bytes from `OutboardCatalog` + `SelectedPlainEntry` + current policy; charge existing `ReservationLedger`. |
| `tests.rs` | Named acceptance tests in §5; native/HTTP security-outcome parity; no JSON fixtures. |

Optional later (not this claim): LIG owner files under `net/qdnf/` by **exact claim**. Do not add `BearerProfile` variants.

## 4. Forbidden

- Duplicate Merkle (`verify_leaf` / `encode_proof` / `OutboardCatalog::proof_into`).
- Second `ReservationLedger` (see `runtime/jobs/mod.rs:1–2`).
- Treat **200 of a transformed body** as a verified range (`architecture.md:113`; G11 gzip, `qualification.md:19`).
- `multipart/byteranges` unless a later package explicitly supports it (R1 reject).
- Weaken **416**: unsatisfiable remains refuse/reselect; do not clamp into 206.
- JSON application objects; headers may be text, bodies are exact QBDL/representation bytes.
- Mint `BearerProfile` 8+ or wrap CSCP H2 as range GET.
- Edit `p2p/connectivity/wss.rs`, `wss_bearer.rs`, `wss_tls.rs`.
- Cross-generation assembly on changed `ETag` / `Last-Modified` / representation.
- Credentials in URLs (`architecture.md:117`).
- Claim TLS/Web PKI as PQ publisher authority (`interfaces.md:11`).
- Infer G01 native independence from HTTP success (`qualification.md:9`).

Header search under `crates/qualia-core-db/src`: **no** `Accept-Ranges`, `If-Range`, `Last-Modified`, or `multipart/byteranges` hits in streaming/HTTP adapters. `ETag`/`Content-Encoding` exist only on RDF ingest (`ingest_job/source.rs:353–365`). QBS-14 must implement validators, not copy ingest decode.

## 5. Acceptance tests to implement (names)

Implement under `bundle/stream/http/tests.rs` (hostile local server; no live network):

- `status_200_full_entry_matches_native_bytes` — identity 200 body equals native entry bytes; only ≤1 MiB explicit fallback.
- `status_206_content_range_exact` — `Content-Range` start-end/total, encoding, and length match request + pinned extent.
- `status_416_unsatisfiable` — 416 (incl. `bytes */total`) refuses; no clamp.
- `changed_validator_or_etag_refuses` — changed `ETag` / `If-Range` / representation → reselect, not splice.
- `content_encoding_transform_refused` — gzip/`Content-Encoding` ≠ identity refused even if 206 lengths match.
- `redirect_does_not_bypass_authority` — 3xx / origin change reauthorized via LIG/CSCP; no credential forward.
- `huge_200_aborted_by_budget` — ignored-range 200 aborted before large body consume.

G11 also wants truncation/extra bytes (`qualification.md:19`). G02: native/HTTP identical verified-byte identity and policy (`qualification.md:11`). HTTP/2/3 = reported, not required.

## 6. Evidence class

`EvidenceClass` is `Component=1`, `Process=2`, `PhysicalNetwork=3`, `Application=4` (`harness/qualification.rs:11–16`; `ci_classes.rs:13–33`). **No** `ApplicationHttp` variant exists — that label is documentation only (**UNVERIFIED** as a Rust enum).

HTTP/LIG evidence is **`PhysicalNetwork`** (two-host TCP/TLS) or **`Application`** (LIG/HTTP application fetch). **Never** `Process` (named-pipes / `local_stream`; `harness/two_process.rs:1,30–34`). Loopback unit tests are `Component` unless a real two-host/LIG run is recorded. Do not flip `ci_buckets()[*].qualified` from this module.

## 7. WSS / CSCP H2 overlap

S5 serial: **WSS repair first**, then HTTP. HTTP implementer **must not** edit:

- `crates/qualia-core-db/src/p2p/connectivity/wss.rs`
- `.../wss_bearer.rs` (`WssBearer` → `BearerProfile::TlsWssTransitionV1`, `wss_bearer.rs:42–44`)
- `.../wss_tls.rs`

**CSCP H2 is a different bearer.** `cscp_h2.rs:1–4,46–55,95–97`: loopback TLS HTTP/2 **Extended CONNECT** + RFC 9297 capsules; `H2CapsuleBearer::profile() = Http2CapsuleTransitionV1`; “not a public relay and not MASQUE Internet.” Tests: `cscp_control_then_qsession_on_local_tls_h2`, `wrong_connect_protocol_rejected`, capsule critical-type reject (`cscp_h2.rs:230–266`); `h2_capsule_tests.rs` owns listener/runtime drop only.

QBS-14 is **byte-range object delivery**. CSCP H2 is **session capsules**. **Do not wrap or rewrite** `cscp_h2.rs` / `h2_capsule.rs` for ranges. Redirects named in QBS-14 (`work-packages.md:270`) are LIG/CSCP **reauthorization of HTTP origin/mirror changes** (`architecture.md:115`; `legacy-internet-gateway.md:56–64`), not capsule CONNECT.

`net/qdnf/bearer/` has IPC, local_stream, raw Ethernet, Nym — **no HTTP/TCP range bearer**. Do not add one under that tree for QBS-14.

## Start condition

Start HTTP only after WSS repair lands. QBS-14 remains pending until named tests and G02/G11 HTTP slices have evidence. No cargo, no `mod.rs` edit, no WSS/H2 rewrite from this claim.
