# Swarm Wave 2.5-D — Describe class 8/9 bootstrap

Expert implementer. **Did not cargo. Did not commit.** Exclusive writes: `qbdl/service/describe.rs` and this summary. `provider.rs` is unchanged (header/index-only). Kind 1–12 frozen. Native interchange is exact-length binary / CBOR.

## Files written

| Path | Role |
|---|---|
| `crates/qualia-core-db/src/net/peer/replication/qbdl/service/describe.rs` | Class 8/9 Describe + `answer_describe_pages`; tests in this file |
| this summary | |

Not edited: `provider.rs`, `service/mod.rs`, `session.rs`, `protected.rs`, `two_process.rs`, `wire/**`, EMF/QAC owner files, Cargo.toml/lock.

## Exports

Added in `describe.rs` (not re-exported from `service/mod.rs`; integrator wires later):

- `DescribeClient::send_body_describe(session, offset, chunk, total, number)` — class 8 request, credit `PREFIX_BYTES + 32 + chunk`
- `DescribeClient::send_entry_page_describe(...)` — class 9 request, same credit
- `DescribeClient::recv_body_page` / `recv_entry_page` — parse DescriptorPage; require matching class / number / offset / chunk / total; copy chunk
- `answer_describe_pages(session, authority, identity, policy_generation, source, body, entry_page)`
- `DescribeTicket.number` (page index; class 8/9 `number < 32`)

Unchanged public surface: `DescribeClient::bind` / `send_header_describe` / `send_index_describe` / `recv_header_page` / `recv_index_page`; `answer_describe` (source-only).

## Invariants

- Class 1 → class 6 64-byte HMC header; class 7 → admitted index slice. Source-only `answer_describe` still refuses class 8/9 as `Unsupported` (does not serve from `source`).
- Class 8 = QASD descriptor body page; class 9 = QASD entry page. ObjectSelector: class `1|8|9` max 65536; class `8|9` `number < 32`. Fixture is one page (`number=0`, `offset=0`, `chunk=total=len`).
- `answer_describe_pages` shares the recheck / `recv_describe` / identity path. Class 8/9 require `total == bytes.len()`, `number=0`, `offset=0`, `chunk=total`; mismatch is `Error::Identity`.
- DescriptorPage credit: `credit_serial != 0` and `credit_bytes >= PREFIX_BYTES + 32 + chunk` (wire `credited` is `208 + body_len`). Describe request remains credited 240.
- Grant closures wrap service `Error` as `Ok((|| -> Result<(), Error> { ... })())`. No `Debug` on success types for `unwrap_err`; tests `match`.
- Session/Assembly still cap DescriptorPage at `MAX_LEAF_RECORD_BYTES` (18137). Fixture pages fit; a 65536-byte page would need a later session buffer lift.

## Tests written **not run**

| Test | What it asserts |
|---|---|
| `describe_body_and_entry_page_then_select_without_local_entry` | Client binds `CandidateIdentity` only. Header, index, body, entry page from the wire via `answer_describe_pages`. `select_entry` uses received body/header/index/entry_page; not `fixture.body()` / `fixture.entry_page()`. Selected `entry_id` / `root` match the publication. |
| `identity_mutation_of_class_eight_describe_refuses` | Mutated class-8 Describe → `Error::Identity`. |
| `class_eight_and_nine_without_pages_stay_unsupported` | `PlainProvider::answer_describe` (no pages) refuses class 8 and class 9 as `Unsupported`. |
| `class_eight_chunk_total_mismatch_refuses` | Class 8 `chunk`/`total` ≠ live body length → `Error::Identity`. |
| existing | header/index without selected entry; identity mutation of Describe; stale generation; kind 13 unsupported; zero descriptor/context bind |

Evidence class: in-process `ipc_pair` unit = **Component**. Not Process. Not PhysicalNetwork.

## Leftover

`PlainProvider::answer_describe` still calls source-only `answer_describe` (header/index). Integrator wires `answer_describe_pages` after this child stops. `service/mod.rs` does not re-export `answer_describe_pages`. Session DescriptorPage recv/send buffers remain `MAX_LEAF_RECORD_BYTES` until a later owner lifts them for max-size class 8/9 pages.
