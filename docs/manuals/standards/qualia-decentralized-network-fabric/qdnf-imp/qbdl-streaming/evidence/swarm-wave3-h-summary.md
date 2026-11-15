# Swarm Wave 3-H — HTTP mixed-plan parity (senior)

File-disjoint. **Did not cargo. Did not rustc. Did not commit.** Exclusive writes: `bundle/stream/http/mixed.rs`, this summary. Integrator has **not** routed `mod mixed;` in `http/mod.rs` yet (this Q42 addendum is required before accept).

## Files written

| Path | Role |
|---|---|
| `crates/qualia-core-db/src/bundle/stream/http/mixed.rs` | Production `HttpMux` / `BoundHttpEntry` / `HttpMuxReport` / `serve_scheduled` plus `#[cfg(all(test, not(target_arch = "wasm32")))]` tests (five QAC/EMF + Q42 range) |
| this summary | |

Not edited: `http/mod.rs`, `http/{range,status,client,provider,tests}.rs`, `wss*`, `cscp_h2.rs`, `q42/volume/range.rs`, `fetch_10d.rs`, `qbdl/service/**`, `formats/**`, `plan/**`, F’s `bundle/stream/mixed/**`, codec-owner trees, Cargo.toml/lock, registries.rs.

Not wrapped: `HttpRangeSource` (`q42/volume`). No `BearerProfile` 8+. `tls_is_pq_publisher_authority()` stays false.

## Multi-entry HMC

F is now routed (`pub mod mixed` in `bundle/stream/mod.rs`). This slice **kept local `PlainPublisher` + `BundleWriter`** (same pattern as mux `Scene::publish` / `http/tests.rs` Fixture) so HTTP tests do not depend on F’s 10d/M64 corpus. Q42 bytes are the same `write_unified_volume` construction as `formats/q42_range_tests.rs` / F `q42_volume_bytes`.

1. `BundleWriter::add_file` for each candidate (key, kind, exact bytes).
2. `PlainPublisher::prepare` / `emit_context_page` / `encode_context_root` / `emit_entry_page` / `finish`.
3. Signed closure + `adopt_local_whole_source` + one-shot `accept_local_scoped`.
4. `admit_header` / `admit_index` / `select_entry` per ordinal.

Family tags after `select_entry` **and** admit:

- QAC: `QacStreamView::admit` (complete-entry) or `QacStorageView::open` on the whole entry (`logical_length == bytes.len()`).
- EMF: `EmfStreamView::admit` (envelope v4, `Family::EmfV4Candidate`). Not `EmfDirectoryView`.
- Q42: whole-entry `AuthenticatedRange` + `Q42AuthenticatedRangeSource::new`. A blob is not Q42.
- Failed admit → omit from `present[]`, never transfer as that family.
- A blob is `Opaque`. Extra M64 bytes are delay-only, never tagged Qac/Emf/Q42.

| File | Size | Role |
|---|---|---|
| `qac_candidate-opaque.qac` | 768 B | live QAC HTTP object |
| `qac_candidate-pcm-signed-zero.qac` | 960 B | live QAC HTTP object |
| `emf_candidate-opaque.emf` | 1088 B | live EMF HTTP object |
| `emf_candidate-aligned4096.emf` | 8192 B | live EMF HTTP object |
| `write_unified_volume` `.q42` | >256 B, ≤ `LEAF_BYTES` | live Q42 HTTP range object |

Two QAC + two EMF are **distinct HTTP objects**. HMC rule: many entries per family. Q42 is queryable via HTTP Range against the authenticated entry, not via `HttpRangeSource`.

## Driver

`mixed.rs` (not re-exported from `http/mod.rs` until the integrator adds `mod mixed;`):

- `HttpMux::compile` / `next_unit` / `acknowledge` / `cancel` / `push_control`
- `BoundHttpEntry { entry_id, provider }`
- `serve_scheduled(...)` — drive `next` until empty
- Direct `HttpProvider::serve(HttpObject::Representation, Some(InclusiveRange), …)` for Q42 subranges

Headers are the adapter. Bodies stay exact QBDL / representation bytes. JSON is not an application object. HTTP status is not a grant.

Grant closures wrap provider errors as `Ok((|| -> Result<(), ProviderError> { ... })())`. Tests `match` on `Result`. No `Debug` on success types.

## Q42 HTTP Range

Uses `crate::bundle::stream::formats::Q42AuthenticatedRangeSource` over a whole-entry `AuthenticatedRange` (entry root, not a filename). Does **not** wrap `q42/volume` `HttpRangeSource`.

- 206 subrange body == `Q42AuthenticatedRangeSource::read_exact_at` / `read_range_into` for that span.
- Full-object 200 is **in addition** to 206, via explicit `FallbackPolicy`.
- `validate_exact_range_response` checks the 206 window.
- Whole-entry source: `Q42RangeVolume::open` succeeds.
- Partial header/lex window (first 128 B < 256 B header): volume open fails; `read_exact_at` past the window is `UnexpectedEof`; buffer unchanged.
- Sparse left 8 + right 8: `read_exact_at(8)` is `UnexpectedEof`; buffer unchanged. Never a silent complete query answer.

## Tests written **not run**

Hostile `TcpListener` `127.0.0.1:0` as in C5. In-process `HttpProvider::serve` is still Component. `std::net` gated `not(target_arch = "wasm32")`. Comment at top: `EvidenceClass::Component`. Never Process. Never PhysicalNetwork.

| Test | What it asserts |
|---|---|
| `schedule_order_matches_native_bytes_over_http` | Two QAC + two EMF. `Schedule::next` order; each 206 (or 200) body equals `select_entry` slice; ledger charged per first serve; each body admits as QAC or EMF; one hostile loopback GET identity/If-Range matches native |
| `http_delays_m64_while_ready_qac_and_emf_continue` | QAC + EMF + M64; first served families are Qac/Emf; M64 only after live families |
| `http_cancel_obsolete_qac_skips_body` | Cancel one QAC id; no 206/200 body for it; EMF sibling bodies still match native; occupancy until `acknowledge_complete` |
| `http_incomplete_mandatory_is_not_200_or_206` | Missing mandatory Q42 calib; `ProviderError::Policy` or `Unsatisfiable`; out buffer unchanged; no ledger charge |
| `http_two_qac_and_two_emf_are_distinct_objects` | Four ids, four ETags/roots, four bodies. Many-per-family over HTTP |
| `http_q42_range_matches_authenticated_source` | Real `.q42` in a mixed HMC with two QAC + two EMF; 206 window equals `Q42AuthenticatedRangeSource`; full 200 in addition; `validate_exact_range_response`; sparse/partial gaps are `UnexpectedEof`; hostile loopback GET of the Q42 object |

Reuse C5 range / If-Range / identity rules. Did not repeat the whole C5 hostile matrix. Did not copy Open/Credit/RequestLeaves encoding. Did not invent Kind 13+.

## Integrator

```rust
// bundle/stream/http/mod.rs — after this child stops
mod mixed;
```

Do not `pub use` from `http/mod.rs` unless a later slice needs it. Tests live inside `mixed.rs`; there is no `mixed_tests.rs`.
