# QBS-06 descriptor/publisher schema proposal

2026-11-12. **Root approved this schema/domain/128-row geometry as a candidate plaintext profile v1; implementation is authored and awaits executed qualification.** Write scope: new bundle/stream/{descriptor,publish}/ only; root owns routing/CDDL/registries/status. Initial implementation supports plain public-delivery **candidates**. Candidate bytes confer no disclosure permission, controller authority or consumer admission. Private/mixed projections, ciphertext representations and recipient wraps explicitly return Unsupported.

## Reuse and limits

Use QBS02 bounded full/index admission; QBS05 TreeSpec/TreeBuilder/OutboardNode, Plaintext representation and16384-byte logical leaves; crypto::network::digest::sha384; and crypto::network::cose::Binding. No new primitive/key serialization.

All CBOR maps/arrays/strings are definite, lengths/unsigned integers shortest, map integer keys exactly ascending as listed. Unknown/duplicate/reordered/nonminimal/tagged/floating/indefinite/truncated/trailing input fails. No recursive generic decoding, normalization or context URL fetch. P is exact tstr "qbdl-stream-pq-1"; V is uint1. Digests are exact bstr48. Kind is exact nonempty UTF8 tstr <=64 bytes, preserving its bytes.

Max4096 entries;128 rows/page; max32 pages of each type. Nonfinal pages contain128 rows, final contains the positive remainder; zero entries means zero pages. Body, context root and each page <=65536 encoded bytes. Cumulative actual context+body+all context/entry pages+detached proof <=8388608 bytes. Existing paired COSE envelope limit4608 also applies (architecture envelope ceiling96KiB is not increased). Source extent <=1TiB and remains distinct from resident metadata limits.

Entry IDs are caller-supplied unique opaque u32 values, one per original index ordinal; no filename hash IDs. Generation is u64: initial0 has null predecessor, later generation requires a supplied predecessor digest; QBS18 verifies history separately. Nonce is supplied nonzero bstr32. Three pinned reference digests below are nonzero opaque values, not fetched or semantically admitted here.

## Exact proposed schemas

PageRef is the fixed array:
    [page_number, first_index_ordinal, row_count, encoded_page_length, digest_bstr48]

Refs exactly cover source index order0..entry_count with no gap/overlap/duplicate page. First ordinal equals page_number*128. All numeric fields are unsigned with profile-bound widths/ranges.

SourceCommit, eight-key map:
    {0: source_length_u64, 1: qbdl_version_uint1, 2: qbdl_flags_uint0_or_1,
     3: exact_header_sha384_bstr48, 4: index_offset_u64, 5: index_length_u64,
     6: exact_index_sha384_bstr48, 7: complete_source_sha384_bstr48}

Header is exactly64 source bytes. Raw header/index/complete-source digests hash actual corresponding bytes directly, not legacy short digests or tree roots. Complete-source digest commits gaps/padding too. Geometry/flags must match actual bounded QBDL admission.

ContextPage, five-key map (NO context ID/tree root/signature):
    {0:P, 1:V, 2:page_number, 3:first_index_ordinal,
     4:[[entry_id_u32, logical_length_u64, representation_uint0, key_epoch_uint0], ...]}

ContextRoot, thirteen-key map:
    {0:P, 1:V, 2:nonce_bstr32, 3:generation_u64,
     4:predecessor_descriptor_digest_bstr48_or_null, 5:SourceCommit,
     6:leaf_bytes_uint16384, 7:entry_count_uint, 8:[ContextPageRef,...],
     9:protection_uint0, 10:semantic_contract_ref_bstr48,
     11:policy_ref_bstr48, 12:consumer_profile_ref_bstr48}

Protection0 is plain candidate only. This public root must not be used for confidential inventory. QBS07 must authorize the entire raw bundle including index keys, metadata, gaps and sidecars; hiding descriptor rows is not a public projection.

EntryPage, six-key map:
    {0:P, 1:V, 2:context_id_bstr48, 3:page_number, 4:first_index_ordinal,
     5:[[entry_id_u32, source_index_ordinal_u32, kind_tstr, inner_version_u64,
         absolute_source_offset_u64, logical_length_u64,
         representation_uint0, key_epoch_uint0, entry_tree_root_bstr48], ...]}

Ordinal/offset/length/kind cross-check actual admitted index; IDs/lengths/representation/epoch cross-check context rows. inner_version is explicit caller-declared candidate metadata: QBS12 must check it before inner consumer admission; no filename guesses. Plain leaf i maps to offset+i*16384 with an exact shorter final length. Empty entry uses QBS05 context-bound empty_root and no data leaf. Roots hash actual entry bytes via QBS05, never legacy digest bytes.

DescriptorBody, eight-key map:
    {0:P, 1:V, 2:context_id_bstr48, 3:exact_ContextRoot_CBOR_bstr,
     4:generation_u64, 5:predecessor_descriptor_digest_bstr48_or_null,
     6:entry_count_uint, 7:[EntryPageRef,...]}

Embedded bounded ContextRoot allows independent context-ID recomputation. Duplicate generation/predecessor/count must match it. Context-page contents remain detached. Renewable paired proof is detached from the body/context identity. Locators/grants/wraps are absent, not silently skipped extensions.

## Exact proposed digest frames

Existing crypto sha384 hashes ASCII domain, one NUL byte, then exact deterministic CBOR, with no length prefix:
- context_id: "QBDL-QASD-CONTEXT-V1" ||00||context_root
- context-page digest: "QBDL-QASD-CONTEXT-PAGE-V1" ||00||context_page
- entry-page digest: "QBDL-QASD-ENTRY-PAGE-V1" ||00||entry_page
- descriptor_id: "QBDL-QASD-BODY-V1" ||00||descriptor_body

Use caller-owned bounded framing scratch (domain+NUL+max64KiB metadata) before crypto::network::digest::sha384. No hash implementation, unbounded concatenation or full-source copy. These are working application domains, not public registration/conformance claims.

## Acyclic construction and source ownership

1. Caller provides immutable complete source slice (potentially excluded-writer native snapshot), explicit resident index limits, one opaque entry ID/declared inner version per index ordinal, nonce, generation/predecessor and three pinned refs. No source clone. BundleReader::parse_with_limits validates complete QBDL CRC and complete-entry SHA256, with explicit caller-selected file_bytes admission and unchanged resident index caps. This may retain bounded index metadata; no unbounded metadata/whole-source Vec is created.
2. Compute actual source/header/index SHA384. Encode context pages and their commitments. Encode ContextRoot, then context_id. No tree/cipher roots, descriptor ID or proof appear in this dependency chain.
3. Build each QBS05 tree from actual borrowed entry chunks with that context, supplied ID, generation, Plaintext/epoch0 and exact length. Optional outboard sink is fallible; sink failure/poison rejects the candidate. Durable outboard commit is a distinct caller gate.
4. Encode entry pages and hash their bytes, then DescriptorBody and descriptor_id. Do not retain complete outboards/full source in memory. Source must remain immutable for the full borrow/snapshot lease.
5. Existing cose::Binding uses descriptor_id plus paired key IDs/controller epoch/purpose/audience. CandidateIdentity::binding yields only a binding candidate. A separate admitted operation uses CryptographicLibrary/KeyManager managed signing to write the proof; the publisher accepts no signing keys. Re-signing does not change immutable identity; ML-DSA randomness is reported separately. QBS04 trust/current authority and QBS07 disclosure are independent gates.

Initial producer accepts a borrowed complete-source slice, not a remote fetch/allocation. It is bounded extra memory, not measured whole-source RSS or arbitrary32-bit source addressability. Streaming readers retain QBS02's remote-u64 path. Large sequential source abstraction can follow a reviewed crypto streaming-hash owner rather than a duplicate SHA implementation.

## Proposed implementation API

descriptor/: fixed DescriptorError; SourceCommit/ContextParameters/PageRef/ContextEntry/borrowed EntryRecord; bounded encoded_len/encode_*_into caller slices; strict decode_* into caller row outputs and borrowed root/body views; domain_digest(kind,encoded,framing_scratch). Any failed encode invalidates its output prefix. MetadataBudget counts actual bytes with fixed page masks and refuses duplicate/cumulative page admission.

publish/: caller-driven PlainCandidate preparation over immutable source and admitted assignments. Caller owns entry/root/page-ref scratch, context/body/page outputs and hash-framing scratch; fallible sinks receive candidate pages/outboard nodes. Separate finish_body and sign_detached helpers require a completed candidate state. Prefer caller-driven pages to retaining output borrows across callbacks. No externally usable public-authority handle is constructed.

Root approval was received for the exact keys/row order/domains,128-row geometry, mandatory complete-source digest, declared inner-version candidate status, borrowed whole-source input and candidate-only plain scope. Independent encoder/hash vectors, malformed/cumulative-page tests, actual COSE binding reuse checks and resource measurements follow. This increment cannot complete QBS06: private projections/confidential inventory, disclosure authority, recipient wraps, durable sinks and end-to-end provider/consumer integration remain open.

## Implementation handoff (source inspection only)

Actual public owner is PlainPublisher. It retains immutable source/index/assignments and exclusive caller page-ref scratch; cheap global-ID/output-capacity checks precede cold complete-source validation. Its bounded full reader metadata is dropped before the retained publisher. Caller-provided entry/ID/page/root/body/framing storage remains caller-owned. All encode/decode and page/tree callback paths contain no owned collections in these new production directories; only the existing bounded cold BundleReader path allocates during prepare. No measured allocation/test pass is claimed.

InventoryVerifier checks exact raw header/index commitments, context-ID recomputation, per-page digests, duplicate/omitted/cross-context pages, mapping against the original admitted index, context/entry length+ID agreement, global ID uniqueness through caller scratch, and context-bound empty roots. finish returns CandidateIdentity only after complete coverage. This does not verify the complete remote source digest from absent payloads, trusted controller authority or disclosure permission.

Metadata accounting conservatively reserves existing COSE MAX_ENVELOPE_BYTES (4608) before adding actual context/body/pages. Thus any single accepted envelope fits the cumulative bound; metadata_reserved_bytes is a reservation, not measured RSS. Actual signing is intentionally absent: CandidateIdentity::binding returns only the existing detached COSE Binding candidate. It accepts no SigningKeys, serializes no secret, and calls no primitive/provider signing API. The corrected CryptographicLibrary/key-manager/KeyVault facade remains the separate integration owner.

All failed context-page, entry-page or outboard sink callbacks poison the publisher before callback entry; unwind retains poison. Finalization and context encoding are idempotent without duplicate sink callbacks or metadata charges. Inner QBDL bytes and existing writer/format/parent registries were not edited. PublicationClass::PrivateProjection and RecipientWrapped refuse Unsupported.

Authored13 tests: four canonical codec/boundary fixtures, three deterministic/paged identity fixtures and six refusal/poisoning/global inventory/cumulative budget fixtures. Root must execute bundle::stream::descriptor:: and bundle::stream::publish:: plus its direct production-source allocator/independent wire harness. rustfmt --edition2024 on both new mod.rs roots completed exit0; no build/test was run by this worker.

| Source path relative to core src | Lines | SHA256 |
|---|---:|---|
| bundle/stream/descriptor/cbor.rs | 208 | 4539a729a071ed440e7e4c1cdcf885a734d5d610888ef118b51fc830ace09757 |
| bundle/stream/descriptor/digest.rs | 36 | 68814735ad373cfb76280cafed33f4c630dfa223300a8dc4096f2eb49d957021 |
| bundle/stream/descriptor/inventory.rs | 217 | 5067c154a6eb72eca3b5c68de2abf526bdcf303281e1d3b19b9b0178e4ddffa5 |
| bundle/stream/descriptor/mod.rs | 27 | 41459de155cd821c6e2d1338d879e48ee1c321ffa14ed17631c94f6eb196a03b |
| bundle/stream/descriptor/pages.rs | 183 | 85695aed15c4581a87e63b69c4284449c21255c253662def81e2b96f9c1fed38 |
| bundle/stream/descriptor/refs.rs | 90 | 2769340f477b5c816e057cd91f231b8192dc213069095cd5f905826c10b86658 |
| bundle/stream/descriptor/roots.rs | 286 | 3e6fb44c4e415191a7525be6bd156a3b91a37cb11e850c0dc1b3eb0f31681b8f |
| bundle/stream/descriptor/tests.rs | 199 | 0457e91a4e5bb8b8501aa415a061722d00beea56e693e22be8b4a5a9eaada5ee |
| bundle/stream/descriptor/types.rs | 206 | c355513a9e73040d8f66f253fba263741979da3961afa7de1d41e909a7a4bcea |
| bundle/stream/publish/context.rs | 77 | b740a63288f78fd17ece6170b4fb0e8e2c585056157bfe2b9227866e28f40ac9 |
| bundle/stream/publish/entries.rs | 99 | 7e2cd23438d52046ee40208afe288ddea96b1657b5ecffdd6f26781ef3f01e83 |
| bundle/stream/publish/finish.rs | 70 | 6e479f447766044a1209c346c746fd1d1796b3055e11fdf3e3909bb6147ab1b8 |
| bundle/stream/publish/mod.rs | 9 | 10b311a91b041a491e76f2a99165a1c9cd3921e6acf7a119fafc7137e5c6a9c3 |
| bundle/stream/publish/owner.rs | 163 | f5b2bc8ead67962de6ebdf3c0603abd6f2582b2c794ed0bbac7aaead02e25724 |
| bundle/stream/publish/tests.rs | 126 | 74c31267c699b42d62ec0b282e1c0b531a11d1e23335d48de7a228445a7e9e03 |
| bundle/stream/publish/tests/rejection.rs | 372 | 4783ad6aae68980de42c627580aa6e3a94d4e21ac7be1360944078acd989265b |
| bundle/stream/publish/tests/roundtrip.rs | 106 | 3a0db5007493828e22629573c787b374fa86015b1cbcec43fddccd4bdb5b8981 |
