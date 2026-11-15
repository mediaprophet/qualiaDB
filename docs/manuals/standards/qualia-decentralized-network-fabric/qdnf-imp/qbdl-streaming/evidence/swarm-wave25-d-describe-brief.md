# Wave 2.5-D — Describe class 8/9 bootstrap (expert)

File-disjoint. **Do not cargo. Do not commit.** Exclusive write: `crates/qualia-core-db/src/net/peer/replication/qbdl/service/describe.rs` plus evidence `swarm-wave25-d-summary.md`.

## Gap

`DescribeClient` binds `CandidateIdentity` without a pre-built `SelectedPlainEntry`. `answer_describe` currently serves class 1 → class 6 (64-byte HMC header) and class 7 (index slice). Other classes refuse `Unsupported`.

`select_entry` needs descriptor **body** and **entry page** in addition to header+index. Until class 8/9 are served from the live publication, the client cannot Select/Open from the wire.

ObjectSelector (`wire/selector.rs`):

- class `1 | 8 | 9` max 65536; class `8 | 9` `number < 32` (page index)
- class 8 = QASD descriptor **body** page
- class 9 = QASD **entry page**
- class 6 header total must be 64; class 7 index up to 8388608
- class 1 request may have `total=0` (already used for header describe)
- Do not mint Kind 13+. Kind 1–12 stay frozen.

`Fixture` (`policy/tests.rs`) already has `body()`, `entry_page()`, `source()`. `MAX_ENCODED` is 65536, so one page (`number=0`, `offset=0`, `chunk=total`) is enough for the fixture.

## Keep

Existing `send_header_describe` / `send_index_describe` / `answer_describe(..., source)` behaviour. Class 1 and 7 stay as they are. When `answer_describe` sees class 8/9 with **no** body/entry_page slices, keep refusing `Unsupported` (do not silently serve from `source`).

## Add in `describe.rs`

1. `DescribeClient::send_body_describe(session, offset, chunk, total, number)` — class 8 request, credit `PREFIX_BYTES + 32 + chunk`.
2. `DescribeClient::send_entry_page_describe(...)` — class 9 request, same credit rule.
3. `recv_body_page` / `recv_entry_page` — parse DescriptorPage, require matching class/offset/chunk/total/number, copy chunk.
4. `answer_describe_pages(session, authority, identity, policy_generation, source, body, entry_page)` — same recheck/recv_describe/identity path as `answer_describe`, then:
   - class 1 → existing `header_page(source)`
   - class 7 → existing `index_page(source, selector)`
   - class 8 → slice `body` by selector (number, offset, chunk, total must equal `body.len()` as u64; number 0 for the fixture one-page case)
   - class 9 → slice `entry_page` the same way
   - other classes → `Unsupported`
5. Credit: DescriptorPage needs `credit_serial != 0` and `credit_bytes >= PREFIX_BYTES + 32 + chunk` (wire `credited` for DescriptorPage is `208 + body_len`). Describe request is credited 240.

Do **not** edit `provider.rs`. Existing `PlainProvider::answer_describe` keeps calling `answer_describe` (header/index only). Tests in this file call `answer_describe_pages`.

Grant closures that return service `Error` must wrap: `Ok((|| -> Result<(), Error> { ... })())` because `accept_local_scoped` expects `Result<T, PolicyError>`.

Do not derive Debug on success types just for `unwrap_err`. Use `match`.

## Tests (write, do not run)

1. **`describe_body_and_entry_page_then_select_without_local_entry`**
   - Provider is bound however it needs (it may `select(&fixture)` to build the catalog — that is the **provider**).
   - **Client** uses only `DescribeClient::bind(fixture.identity(), POLICY)` then header, index, body, entry_page from the wire.
   - Client `admit_header` / `admit_index` / `select_entry(&closure, received_body, received_header, &index, received_entry_page, 0, rows, scratch)`.
   - Assert selected entry_id/root match the publication. Client must not call `select(&fixture)` / `fixture.body()` / `fixture.entry_page()` for the bytes it feeds to `select_entry`.
2. Identity mutation of a class-8 Describe refuses (`Error::Identity`).
3. Class 8/9 with `answer_describe` (no pages) stays `Unsupported`.
4. Chunk/total mismatch on class 8 refuses.
5. Existing header/index/stale-generation/kind-13/zero-descriptor tests stay passing in source.

## Stop

Write `docs/.../qbdl-streaming/evidence/swarm-wave25-d-summary.md`: exports, invariants, tests written **not run**, leftover (provider still header/index-only until integrator wires `answer_describe_pages`). Do not cargo. Do not commit. Do not touch EMF/QAC owner files.
