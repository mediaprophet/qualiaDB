# Wave 3-H — HTTP mixed-plan parity (senior)

File-disjoint. **Queued Wave 3A2** — start after E/Q/M stop. **Do not cargo. Do not commit.** Integrator adds `mod mixed;` under `bundle/stream/http/mod.rs` after you stop.

## Gap

C5 `HttpProvider` serves one `SelectedPlainEntry`. C4 `Schedule::next` is unused outside `plan/tests.rs`. Mixed-bundle HTTP must yield the **same** verified bytes and the **same** plan outcomes as native (delay M64 while live families continue, shared dep once, cancel, incomplete ≠ 200/206). Loopback only. `EvidenceClass::Component` (`net/qdnf/harness/qualification.rs`). Never Process. Never PhysicalNetwork.

Headers are the adapter. Bodies stay exact QBDL / representation bytes. JSON is not an application object. HTTP status is not a grant.

## Read first (read-only)

- `bundle/stream/http/{mod,range,status,client,provider,tests}.rs`
- `bundle/stream/plan/schedule.rs` `Schedule::compile` / `next` / `cancel` / `acknowledge_complete`
- `http/tests.rs` hostile `TcpListener` `127.0.0.1:0` pattern
- Review §6 items 3–5 and 8 (native and HTTP bindings)

Do **not** edit those files. Do **not** edit `wss*`, `cscp_h2.rs`, `q42/volume/range.rs`, `fetch_10d.rs`.

## Allowed writes

- NEW `crates/qualia-core-db/src/bundle/stream/http/mixed.rs`
- `docs/.../qbdl-streaming/evidence/swarm-wave3-h-summary.md`

HTTP bodies are **real QAC, EMF, and Q42** bytes. Family on the inventory must match format admit (`QacCompleteView` / `QacStreamView`, `EmfStreamView`, `Q42AuthenticatedRangeSource`). A blob is `Opaque`.

Q42 is queryable via HTTP range: a mixed-HMC Q42 entry is served with HTTP `Range` (206). The body is exact authenticated entry bytes for that inclusive range. Bind `Q42AuthenticatedRangeSource` (and `Q42RangeVolume` when the span covers what the query needs) over those bytes. A partial range is **not** a complete query answer (`validate_exact_range_response` / UnexpectedEof on a gap — never a silent full result). Do **not** wrap `q42/volume` `HttpRangeSource` (that is a raw-file HTTP client). C5 `HttpProvider` + `InclusiveRange` is the adapter.

Prefer `crate::bundle::stream::mixed` if F is already routed. If not, publish locally with `PlainPublisher`: two QAC `include_bytes!`, two EMF `include_bytes!`, one Q42 from `write_unified_volume` (same as `formats/q42_range_tests.rs`), plus M64 only when those bytes admit as M64. Do not edit F’s tree. Do not wait on cargo. EMF/QAC codec work is **done** — no deferral.

## Behavior

For each `Schedule::next` unit that has an `entry_id`:

1. `select_entry` that id.
2. `HttpProvider::bind(source, catalog, descriptor_body, entry_page)`.
3. `serve(HttpObject::Representation, Some(range), …)` with identity encoding, recheck `LocalEffectControl` (same as C5).
4. `acknowledge_complete`.

Control units: no HTTP body. M64/P64 units must not be served while a Ready live-family unit remains (`Schedule::next` already orders this — assert the serve order).

Shared dependency: second principal asking the same entry id does not `ReservationLedger::reserve` a second full representation if the first serve already charged that identity. If the current `HttpProvider` always reserves, skip the second `serve` when `Schedule` reports the entry already charged (`charged` is private — observe via `MuxReport` if M exported it, or by serving once and asserting a second `next` for that `EntryRef` does not appear). Matching C4 `next_delays_m64_while_ready_emf_qac_control_continue` is enough.

Do not wrap `HttpRangeSource`. Do not mint `BearerProfile` 8+. TLS/Web PKI stays `tls_is_pq_publisher_authority() == false`.

## Tests (write, do not run)

Hostile loopback as in `http/tests.rs`. Comment at top: `EvidenceClass::Component`.

1. **`schedule_order_matches_native_bytes_over_http`** — **two QAC + two EMF**. `Schedule::next` order; each 206 body equals `select_entry` slice for that id. Ledger charged per first serve. Each body admits as QAC or EMF.
2. **`http_delays_m64_while_ready_qac_and_emf_continue`** — QAC + EMF + M64; first served families are Qac/Emf; M64 not served in that step.
3. **`http_cancel_obsolete_qac_skips_body`** — cancel one QAC id; no 206 body for it; EMF sibling 206 still matches native.
4. **`http_incomplete_mandatory_is_not_200_or_206`** — missing mandatory dep; provider does not disclose that principal’s representation (`Admission` / `Unsatisfiable` / policy refuse — pick the existing `ProviderError` that C5 already uses for policy, do not invent a status code).
5. **`http_two_qac_and_two_emf_are_distinct_objects`** — four ids, four ETags/roots, four bodies. Many-per-family over HTTP.
6. **`http_q42_range_matches_authenticated_source`** — one real `.q42` entry in the same HMC. `HttpProvider::serve(Representation, Some(InclusiveRange), …)` 206 body equals `Q42AuthenticatedRangeSource::read_exact_at` / `read_range_into` for that span. A range that does not cover a needed query window is not a complete answer (no 200-as-full, no silent lexicon/index claim). Hostile loopback `get_range` of that Q42 object is Component evidence.

Reuse C5 range/If-Range/identity rules. Do not repeat the whole C5 hostile matrix.

## Stop

Summary: how the multi-entry HMC was built (local publish vs `mixed` crate), tests written **not run**, evidence class Component. Do not cargo. Do not commit. Do not edit `http/mod.rs`.
