# HMC container format standards

**Revision:** 1, 12 November 2026. **Status:** Internal implementation-aligned standards for two existing HMC profiles. See the [format overview](file-formats-overview.md). A `.hmc` extension alone is not a sufficient format discriminator.

## 1. Profiles and intended use

| Profile | Physical encoding | Typical producer | Integrity |
|---|---|---|---|
| Transparent asset bundle | `QBDL`, version 1 | QualiaDB Core bundle writer; Synth production | Whole-file CRC-32C and full SHA-256 per indexed entry |
| Document archive | ZIP with JSON `manifest.json` | Qualia semantic library container | Full BLAKE3 for source/assets in manifest, plus ZIP checks |

These encodings are not byte-compatible. Dispatch by actual magic and then validate the selected profile. The QBDL tag survives the historical `.qualia` to `.hmc` naming change. Neither profile is automatically HLS, DASH, a new QDNF transport, or an encrypted archive. Do not invent an `HMC1` writer solely because a sniffer mentions that name.

## 2. QBDL v1 envelope

QBDL concatenates intact entry bytes and appends a CBOR index. Entry contents are not compressed, rechunked or reframed by this container. An entry slice is byte-identical to the standalone inner file, and inner offsets remain relative to that slice.

### 2.1 Header: 64 bytes

| Offset | Bytes | Type | Meaning |
|---:|---:|---|---|
| 0 | 4 | bytes | ASCII `QBDL` |
| 4 | 2 | u16 | Version 1 |
| 6 | 2 | u16 | Flags |
| 8 | 4 | u32 | Entry count |
| 12 | 8 | u64 | Absolute CBOR index offset |
| 20 | 8 | u64 | Index byte length |
| 28 | 8 | u64 | Exact total file length |
| 36 | 4 | u32 | Whole-file CRC-32C |
| 40 | 24 | bytes | Reserved zero |

Numeric fields are little-endian; the u64 fields at offsets 12/20/28 are not naturally 8-byte aligned. Decode them as bytes. CRC-32C covers the entire object with bytes [36,40) treated as zero.

Flags zero selects 64-byte entry/index alignment. Flag `0x0001` selects 4096-byte alignment. Both are emitted by current writer APIs; other bits are unallocated. In particular, the expert proposal's larger alignment flags and uses for byte 40 are not current-format features. Writers zero gaps/padding and reserved header bytes. The footer ends at the file end without an extra alignment tail.

### 2.2 CBOR index

The footer is an array of entry maps. Each entry carries:

| Key | Value | Contract |
|---|---|---|
| `key` | Text | Nonempty logical identifier, unique in bundle |
| `kind` | Text | Nonempty producer/consumer-agreed format label |
| `offset` | Unsigned integer | Absolute aligned entry start |
| `length` | Unsigned integer | Exact intact entry length |
| `sha256` | Byte-valued sequence | Exactly 32 raw SHA-256 digest bytes |
| `meta` | Optional byte-valued sequence | Opaque metadata, conventionally nested CBOR |

The implemented Serde fields are `Vec<u8>`; the ordinary Ciborium writer serializes these as arrays of byte-valued integers, not a promised canonical CBOR byte-string representation. A low-level preflight recognizing byte strings does not guarantee the later Serde decoder accepts every such alternative. Interoperable authors should match the current writer's arrays until a separate canonical CBOR profile is adopted. Map keys are strings; this is not the separate CBOR-LD manifest schema.

`meta` is optional and omitted when absent. It is not recursively interpreted or authenticated as a credential by the generic bundle layer. Consumers must select its domain schema explicitly. The entry SHA-256 covers only `[offset,offset+length)`, not its key/kind/metadata or the rest of the index. Trusted bundle-level authentication must therefore bind the index as well as payloads.

The writer preserves insertion order and rejects empty or duplicate keys. Determinism requires deterministic insertion order, metadata and entry bytes; there is no automatic sorting promise for arbitrary caller input. A `kind` label does not replace checking the inner file's magic, version and profile.

### 2.3 Reader profiles and limits

The general reader checks the header, version, declared length, CRC, index/payload bounds and selected page alignment, then exposes entries. It does not automatically perform all of the stricter admission checks or verify every entry SHA-256.

The bounded `parse_with_limits` path preflights the index before materialization: definite-length, untagged CBOR structures, known fields, five or six fields per entry, no duplicate field keys, bounded sizes and valid byte-valued digest/metadata representations. It then checks nonempty key/kind, 32-byte hashes, unique logical keys, payload nonoverlap, ranges, 64-byte alignment and all entry SHA-256 values.

**Current mismatch:** bounded parsing accepts flags zero only and rejects the writer's valid 4096-aligned flag profile. Do not work around this by silently using an unbounded parser for untrusted content. Use the bounded 64-byte profile until a reviewed reader change reconciles flag admission, or provide a separately audited bounded path.

Default bounded limits are 128 MiB file bytes, 1 MiB index bytes, 4096 entries, 4096 bytes per key, 64 bytes per kind, 64 KiB metadata per entry and an 8 MiB decoded-size reservation. The preflight's four-times-size accounting is a reservation model, not an exact process-RSS guarantee. Applications may select stricter limits; large model bundles require explicit budget choices, not assuming the defaults permit arbitrary P64 files.

Logical keys are not safe filesystem paths. Extraction must separately reject traversal, absolute paths, unsafe separators, collisions and platform-specific names within a verified destination. Zero-length entries, if admitted by a selected API, still require valid ranges and the SHA-256 of empty bytes; they do not waive key or overlap rules.

## 3. Range access and progressive delivery

A client can first obtain the 64-byte header, then the indexed footer, then selected intact entries. An inner byte range maps to `entry.offset + inner.offset`, after checked addition and bounds validation against both entry and object. This makes QBDL **range-addressable**. It does not supply a standardized adaptive-stream manifest, timeline, dependency scheduler, segment proof, live-update protocol or QDNF service negotiation by itself.

Whole-file CRC cannot be verified from header/index alone. An entry SHA-256 can be verified only after all bytes of that entry have been processed unless an independent authenticated range-proof scheme is supplied. Partial inner data is not authenticated just because the index contains a full entry hash. No Bao proof follows from the EMF/M64 full-digest trailer either.

The current streaming loader is a separate, weaker path: header probing does not verify whole-file CRC; index decoding is not equivalent to bounded full admission; malformed digest length can be converted to an absent optional digest, and verification without a digest can degrade to a length check. These are admission defects/limitations, **not standardized optional-security features**. A security-sensitive consumer must reject a missing/invalid expected digest, authenticate the index and selected content, and enforce its own bounded admission rather than infer these guarantees from the loader's name.

HTTP range delivery can carry these bytes, but the file format does not itself define HTTP validators, response status handling, range coalescing or cross-origin policy. A transport binding must pin one immutable object/version across requests and reject mixed-version ranges. Native QDNF is the intended extension architecture; HTTP is a compatibility binding. No proposed packet type, QASD structure, segment AEAD, proof-tree hash suite or QBD2 encoding is allocated by this document.

## 4. Embedded alignment and integrity layers

64-byte outer alignment preserves a 64-byte file-relative alignment requirement. It does not universally preserve P64's default 16 KiB pages. Even 4096-byte alignment is insufficient as a general 16 KiB guarantee. Check the actual entry base, inner offset and backing memory/mapping address; use a supported mapping strategy or bounded aligned copy where needed.

A repaired EMF or M64 entry carries its own current full BLAKE3 chunk digests. QBDL continues to hash the entire entry with **SHA-256**. Do not replace the SHA-256 field with BLAKE3 while leaving its name/version unchanged. Rewriting any inner bytes requires rebuilding that entry hash and whole-file CRC, and any authenticated outer commitment. An untrusted sender can recompute both unkeyed hashes and CRCs; trusted signatures/capabilities remain separate.

## 5. Synth production profile

Synth packages production artifacts into QBDL with a `production.json` manifest, schema version 1, producer/version/source revision information and retained-input identities including key, kind and SHA-256. Conventional entry locations include inputs and outputs, with content-addressed texture handling where the producer selects it. These names belong to the Synth production profile rather than the general QBDL wire grammar.

The manifest and any entity proof, Commons declaration or credential are data until validated by their owning policy system. Packaging does not itself authorize reuse, execution or publication. Stable production inputs and ordering are necessary for deterministic output. Refer to [Synth packaging source](../../../../synth/src/production/package.rs) for the producer-specific schema; do not infer its requirements for unrelated QBDL applications.

## 6. ZIP document HMC profile

This separate archive uses standard ZIP structures and a JSON manifest named `manifest.json`. The producer writes the manifest first using Stored compression. Already-compressed asset types such as PDF, PNG, JPEG, WebP, ZIP and EPUB use Stored; other content may use Deflate. The underlying ZIP codec handles member framing and checksums. There is no QBDL header or footer in this profile.

### 6.1 Manifest schema

| Field | Meaning |
|---|---|
| `format` | Literal `hmc` |
| `format_version` | Version 1 |
| `doc_id` | Full source BLAKE3 digest encoded as hexadecimal |
| `source` | Source filename, MIME type, byte size, full BLAKE3; title defaults empty, page count defaults zero |
| `created` | Creation timestamp (RFC 3339 representation) |
| `pipeline` | Tool, extractor, embedder and embedding dimension metadata |
| `status` | Extracted/chunked/embedded/analyzed booleans and notes |
| `tags` | Tags list |
| `assets` | Asset declarations described below |

Each asset declares `path`, `kind`, `mime`, `blake3` and `bytes`. Kinds are source, html, text, cml, chunks, embeddings, meta and other. Digests are the **full default BLAKE3 output, 32 bytes represented by 64 hexadecimal characters**, over the uncompressed member content. Declared byte sizes likewise describe uncompressed contents. Typical paths are `source/<sanitized basename>`, `derived/…`, `embeddings/…`, `meta/…` and `assets/…`.

The source identity is content-based; timestamps and processing metadata can still change archive bytes. The default current-time creation field means ordinary ZIP HMC authoring is not automatically byte-deterministic across runs. A reproducibility profile must pin metadata, member order and ZIP options explicitly.

### 6.2 Reader behavior and stronger conformance

The current `from_bytes` path reads ZIP/JSON and checks the literal format name. It does not by itself enforce every version, identity, declared-size and source/asset coherence rule listed here. The verification helper recomputes the full BLAKE3 for each listed asset's uncompressed bytes. That does not authenticate the manifest, establish a trusted publisher, or guarantee that unlisted ZIP members are absent.

Conforming security-sensitive admission additionally checks version 1, full digest encodings, unique member/declaration names, byte-size equality, source/doc-id consistency and a policy for unlisted members. Apply compressed/uncompressed byte budgets, member-count limits and expansion limits before materializing archive contents. The existing convenience reader's allocations do not supply those complete hostile-archive budgets.

Source basename sanitization does not make arbitrary user-supplied asset paths safe for extraction. Validate all extracted paths against the destination. MIME and kind declarations are untrusted labels until the content parser admits them. A status such as `analyzed=true` records pipeline state, not truth or legal permission.

### 6.3 Hash repair scope

This ZIP profile already stores complete BLAKE3 digests. It must not be described as using the defective 12/16-byte prefixes from M64/EMF. There is no need to append their binary 64-byte trailers to ZIP assets. If an asset is itself a repaired M64/EMF container, first rewrite that asset correctly, then regenerate its ZIP manifest digest and size. Retain source identity semantics rather than replacing `doc_id` with a hash of the entire ZIP without an explicit schema change.

## 7. Examples, conversion and evolution

An empty default QBDL can occupy 65 bytes: header [0,64), then the one-byte CBOR empty array at index offset 64. A nonempty bundle's entries and footer receive their selected alignment; its entry hashes cover no padding. For an inner range at relative offset 128 in an entry starting at 4096, the outer start is 4224; admission must also check requested length and the object's pinned identity.

QBDL-to-ZIP conversion is a semantic repackaging operation, not an extension rename. It requires choosing a document identity/source, manifest schema and asset kinds; the reverse requires explicit logical keys/kinds and regenerates SHA-256. Preserve original files if no lossless mapping for metadata/provenance exists.

Future QDNF streaming must define immutable object identity, bounded index admission, authenticated range/dependency proofs, cancellation/resume, scheduling, content/permission separation and PQ trust/key lifecycle in its own versioned service profiles. The [initial brief](qbdl-streaming-expert-advisor-brief.md) and [supplied proposal](references/qbdl-streaming-advisor-proposal-2026-11-12.md) are design inputs. They do not amend the bytes above or bypass existing Commons governance.

## 8. Source and verification

QBDL: [format](../../../crates/qualia-core-db/src/bundle/format.rs), [writer](../../../crates/qualia-core-db/src/bundle/writer.rs), [reader](../../../crates/qualia-core-db/src/bundle/reader.rs), [bounded reader](../../../crates/qualia-core-db/src/bundle/reader_limits.rs), [streaming loader](../../../crates/qualia-core-db/src/bundle/streaming_loader.rs). ZIP profile: [container](../../../crates/qualia-semantic-library/src/container/mod.rs), [manifest](../../../crates/qualia-semantic-library/src/container/manifest.rs), [tests](../../../crates/qualia-semantic-library/src/container/tests.rs). See [conformance and defect register](file-format-conformance.md) for executed evidence and unresolved admission gaps.
