# C5 brief — HTTP/LIG range adapter (QBS-14)

Senior implementer. File-disjoint. **Do not cargo.** Integrator routes `bundle/stream/mod.rs` `mod http;` after you stop.

## Objective

HTTP/1.1 byte-range delivery of the **same** authenticated QBDL objects as native service. Headers are the adapter surface. Bodies stay exact QBDL/representation bytes. JSON is not an application object.

## Read first (required)

- `docs/.../qbdl-streaming/evidence/swarm-s5-http-inventory.md` (complete; follow it)
- `docs/.../qbdl-streaming/architecture.md` §8
- Borrow: `verify_leaf`, `OutboardCatalog`, `SelectedPlainEntry`, `ReservationLedger`, `authorize_fetch` / `reveal_observed` in `net/qdnf/session/fetch.rs`

**Do not wrap** `q42/volume/range.rs` `HttpRangeSource` or `net/fetch_10d.rs`.

## Allowed writes

- NEW `crates/qualia-core-db/src/bundle/stream/http/mod.rs`
- NEW `range.rs` — single inclusive range, `end=start+length-1`, exact `Content-Range`, identity encoding, refuse multipart and `bytes */` except 416 metadata
- NEW `status.rs` — classify 200/206/416 before body; abort over-budget 200; 3xx / changed validators / encoding = refusal
- NEW `client.rs` — HTTP/1.1 GET: `Range`, `Accept-Encoding: identity`, strong `If-Range` when known; LIG reauthorize redirects; no URL grants
- NEW `provider.rs` — serve descriptor/pages/`encode_proof`/immutable representation bytes; charge existing `ReservationLedger`
- NEW `tests.rs` — named tests in inventory §5; hostile **local** server; no live network
- `docs/.../qbdl-streaming/evidence/swarm-c5-http-summary.md`

## Forbidden

- `p2p/connectivity/wss.rs`, `wss_bearer.rs`, `wss_tls.rs`, `cscp_h2.rs`
- `q42/volume/range.rs`, `fetch_10d.rs`
- `registries.rs` / minting `BearerProfile` 8+
- Second Merkle codec or second ledger
- multipart/byteranges
- Treat 200 of a transformed (gzip) body as verified
- Credentials in URLs; TLS as PQ publisher authority
- Claim `EvidenceClass::Process` for HTTP (loopback unit = `Component`; two-host TLS = `PhysicalNetwork`)

## Tests (names from inventory §5)

- `status_200_full_entry_matches_native_bytes` (≤1 MiB explicit fallback)
- `status_206_content_range_exact`
- `status_416_unsatisfiable`
- `changed_validator_or_etag_refuses`
- `content_encoding_transform_refused`
- `redirect_does_not_bypass_authority`
- `huge_200_aborted_by_budget`

Hostile local listener. No paid network. Prefer std/`TcpListener` plus a tiny request parser; do not add crates (integrator owns Cargo.toml). If you believe a dep is required, stop and write it in the summary instead of editing Cargo.toml.

## Stop

Summary: files, how Range/If-Range/encoding are checked, tests written **not run**, residual (HTTP/2/3 reported not required). Do not cargo. Do not commit.
