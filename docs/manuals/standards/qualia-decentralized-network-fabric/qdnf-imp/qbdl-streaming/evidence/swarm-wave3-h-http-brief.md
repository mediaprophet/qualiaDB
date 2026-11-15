# Wave 3-H — HTTP mixed-plan parity (senior)

File-disjoint. **Do not cargo. Do not commit.** Integrator adds `mod mixed;` under `bundle/stream/http/mod.rs` after you stop.

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

You may `include_bytes!` the same canonical QAC/EMF candidate files F uses. You may also publish two+ entries with `PlainPublisher` like `publish/tests.rs`. If `crate::bundle::stream::mixed` is not routed yet, **do not wait and do not edit F’s tree** — construct a small multi-entry HMC locally in this test module (two QAC `include_bytes!`, two EMF `include_bytes!`, one M64 synthetic or a third distinct payload tagged M64 **only if those bytes admit as M64**; otherwise tag the delayed entry `Opaque` and still delay it via `EntryFamily::M64` **only when the bytes are real M64**).

Honest tagging: family on the inventory must match format admit. A blob is `Opaque`.

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

1. **`schedule_order_matches_native_bytes_over_http`** — two live-family entries (two QAC or QAC+EMF). `Schedule::next` order; each 206 body equals `select_entry` slice for that id. Ledger charged per first serve.
2. **`http_delays_m64_while_ready_qac_continues`** — QAC + M64 in inventory; first served family is Qac; M64 not served in that step.
3. **`http_cancel_obsolete_range_skips_body`** — cancel one id; no 206 body for it; sibling 206 still matches native.
4. **`http_incomplete_mandatory_is_not_200_or_206`** — missing mandatory dep; provider does not disclose that principal’s representation (`Admission` / `Unsatisfiable` / policy refuse — pick the existing `ProviderError` that C5 already uses for policy, do not invent a status code).
5. **`http_two_qac_entries_are_distinct_objects`** — two QAC ids, two different ETags / roots, two bodies. Proves many-per-family over HTTP.

Reuse C5 range/If-Range/identity rules. Do not repeat the whole C5 hostile matrix.

## Stop

Summary: how the multi-entry HMC was built (local publish vs `mixed` crate), tests written **not run**, evidence class Component. Do not cargo. Do not commit. Do not edit `http/mod.rs`.
