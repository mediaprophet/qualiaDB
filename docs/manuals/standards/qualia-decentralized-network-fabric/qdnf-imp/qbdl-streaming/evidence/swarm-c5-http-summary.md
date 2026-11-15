# Swarm C5 — HTTP/LIG range adapter (QBS-14)

Senior implementer. **Did not cargo. Did not commit.** Integrator adds `mod http;` to `bundle/stream/mod.rs`. Loopback unit tests are `EvidenceClass::Component`, never Process. `ci_buckets` not touched.

HTTP/1.1 byte-range delivery of the **same** authenticated QBDL objects as native service. Headers are the adapter surface. Bodies stay exact descriptor / page / `encode_proof` / representation bytes. JSON is not an application object. HTTP status is not a grant. TLS/Web PKI is not PQ publisher authority (`tls_is_pq_publisher_authority() == false`).

## Files written

| Path | Role |
|---|---|
| `crates/qualia-core-db/src/bundle/stream/http/mod.rs` | Crate-private adapter surface. No proof/policy/ledger copies. |
| `.../http/range.rs` | Single nonempty inclusive range; `end = start + length - 1`; exact `Content-Range`; identity encoding; refuse multipart and `bytes */` except 416 metadata. |
| `.../http/status.rs` | Classify 200/206/416 **before body**; abort over-budget 200; 3xx / changed validators / encoding = refusal. |
| `.../http/client.rs` | HTTP/1.1 GET: `Range`, `Accept-Encoding: identity`, strong `If-Range` when known; bounded read; LIG `authorize_fetch` + `reveal_observed` on redirects; no URL grants. |
| `.../http/provider.rs` | Serve descriptor/pages/`OutboardCatalog::proof_into` (canonical `encode_proof`) / representation bytes from `OutboardCatalog` + `SelectedPlainEntry` + `LocalEffectControl`; charge existing `ReservationLedger`. |
| `.../http/tests.rs` | Named §5 tests. Hostile `TcpListener` on `127.0.0.1:0`. No live/paid network. |
| this summary | |

Not edited: `bundle/stream/mod.rs`, `p2p/connectivity/wss*.rs`, `cscp_h2.rs`, `q42/volume/range.rs`, `fetch_10d.rs`, `registries.rs`, Cargo.toml/lock, Wave 1 files.

Not wrapped: `HttpRangeSource`, `Fetch10dService`. No second Merkle codec. No second ledger. No `BearerProfile` 8+.

## Range / If-Range / encoding

**Request (client always):**

- One nonempty inclusive range. Stored as `(start, length)` so `end = start + length - 1` cannot wrap. Open-end, suffix, `bytes=*`, and comma lists (`multipart`) refuse.
- `Range: bytes={start}-{end}`
- `Accept-Encoding: identity` on every GET
- Strong `If-Range` only when the caller supplies a quoted ETag (`"…"`). Weak `W/"…"` is not If-Range material and is refused before send.
- Origin-form path only. `@`, `://`, CR/LF in path or Host → credentials-in-URL refusal. No `Authorization` / Cookie / query grant.

**Response (classify headers, then maybe copy body):**

- HTTP/1.1 only. HTTP/2 preface / 1.0 → `Unsupported`.
- Duplicate critical headers refuse.
- `Content-Type: multipart/…` refuse (R1).
- `Content-Encoding` must be absent or exact `identity`. gzip / lists / any transform → `Encoding`, even if 206 `Content-Range` and length match.
- `Transfer-Encoding` other than identity refuse (this parser is not a chunked stack).
- **206:** exact `Content-Range: bytes {start}-{end}/{pinned_total}` plus `Content-Length == range.length`. `bytes */total` is never a 206 body.
- **200:** only with explicit `FallbackPolicy` and `Content-Length` in `(0, min(policy, 1 MiB, out.len())]`. Ignored-range huge 200 → `OverBudget` **without** body consume.
- **416:** `bytes */total` is metadata. Unsatisfiable remains refuse/reselect; no clamp to 206.
- **3xx:** not data. Location userinfo → `CredentialsInUrl`. Every redirect runs LIG `authorize_fetch(FetchKind::Lig)` + `reveal_observed`. Origin change after a valid handle → `OriginChange` (does not follow). Group/paid do not substitute for `private_consent`.
- Strong ETag / `Last-Modified` must match `Validator` when known. Changed or weak response ETag → `ChangedValidator` (no splice, no silent full-object).

**Provider:**

- Rechecks `LocalEffectControl` before disclosure. 200/206 is not a grant.
- Satisfiable range → 206 + exact `Content-Range` + identity + quoted SHA-384 ETag of the representation root (descriptor_id for the descriptor object).
- Unsatisfiable → 416 `bytes */total`, no body, no ledger charge.
- Full-object 200 only when `range == None` and explicit ≤1 MiB fallback.
- Failed `If-Range` → `ChangedValidator`, not a 200 upgrade.
- Proof sidecar: `OutboardCatalog::proof_into` (canonical `encode_proof`) then `verify_leaf` on source leaf bytes before send.
- `ReservationLedger::reserve` for served body bytes; caller `release`s the handle.

## Tests written **not run**

Hostile local server. Names from inventory §5:

| Test | What it asserts |
|---|---|
| `status_200_full_entry_matches_native_bytes` | Provider 200 fallback body == native `SelectedPlainEntry` bytes; ledger charged; client 200 identity GET matches; fallback-forbidden 200 is `OverBudget`. ≤1 MiB. |
| `status_206_content_range_exact` | Exact `bytes start-end/total`; provider 206 + ledger; client body == native slice; `If-Range` sent; wrong total is `Mismatch`. |
| `status_416_unsatisfiable` | 416 `bytes */total` refuses; body not copied; multipart / `bytes=*` parse refuse; no clamp. |
| `changed_validator_or_etag_refuses` | Different ETag on a length-matching 206 refuses; weak `W/"…"` refuses; `If-Range` was sent. |
| `content_encoding_transform_refused` | `Content-Encoding: gzip` on 206 refuses even if range/length match. |
| `redirect_does_not_bypass_authority` | 302 to another origin without consent → `Lig(Denied)` (group/paid ignored); userinfo Location → `CredentialsInUrl` even with consent; consented origin change → `OriginChange`, no follow, no `Authorization`. |
| `huge_200_aborted_by_budget` | Ignored-range 200 with `Content-Length = 1 MiB + 1` returns `OverBudget`; counted read ≪ 1 MiB; output untouched. |

Evidence class: loopback unit = **Component**. Not Process (named-pipes). Not PhysicalNetwork (two-host TLS).

## Residual (HTTP/2 / HTTP/3)

HTTP/1.1 is the mandatory compatibility binding in this module. HTTP/2 and HTTP/3 **reuse the same Range / identity / If-Range / LIG semantics** when a selected client supports them; they are **not** native dependencies and were **not** implemented here. Do not wrap `cscp_h2.rs` (session capsules, `BearerProfile::Http2CapsuleTransitionV1`) as byte-range GET. Do not mint a QFrame HTTP-range bearer.

Other residuals: no auto-follow after a successful LIG reveal (caller must open a new authorized hop); chunked transfer-coding is refused rather than decoded; browser JS `Accept-Encoding` / CORS / OPFS are out of this slice; two-host TLS evidence would be `PhysicalNetwork` and is not claimed.

## Integrator

```rust
// crates/qualia-core-db/src/bundle/stream/mod.rs
mod http;
```

Do not cargo from this claim. No crate added (std `TcpListener` + tiny header parser).
