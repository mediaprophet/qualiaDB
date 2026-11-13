# P64 weight container standard

**Revision:** 2, 12 November 2026. **Status:** Internal implementation-aligned standard for **P64 version 4**. This revision supersedes the older v3 description at this path. Canonical ownership is Etalon's `etalon-model`; QualiaDB provides compatibility re-exports and consumers. See the [format overview](file-formats-overview.md).

## 1. Scope and admission boundary

P64 is a page-aware model-weight container with tensor descriptors, model hyperparameters, tokenizer data, manifold coordinates, optional layer schedules and primary tensor blobs. It preserves source identity alongside supported transformed representations. A valid file is not proof that a model architecture, tokenizer, alternate precision, accelerator or operator package is supported by a runtime.

Canonical magic is `70 36 34 00` (`p64` plus NUL). The current `P64View` reader requires version 4; it does not automatically accept v3. All numeric wire fields are little-endian. Core metadata structures are 64 bytes, but a P64 manifold record is ten f32 values plus padding, not the 40-byte Tensor10D node record from 10D.

## 2. Header: 64 bytes

| Offset | Bytes | Type | Meaning |
|---:|---:|---|---|
| 0 | 4 | bytes | Magic |
| 4 | 2 | u16 | Version 4 |
| 6 | 2 | u16 | Flags |
| 8 | 4 | u32 | Role/schedule table offset |
| 12 | 4 | u32 | Tensor-table offset |
| 16 | 4 | u32 | Tokenizer-region offset |
| 20 | 4 | u32 | Hyperparameter offset |
| 24 | 4 | u32 | String-table offset |
| 28 | 4 | u32 | Checksum-table offset |
| 32 | 4 | u32 | Manifold-table offset |
| 36 | 4 | u32 | Tensor count |
| 40 | 4 | u32 | Page size in bytes |
| 44 | 20 | bytes | Zero in base profile, or ETLMDTA extension locator (§8) |

Flags: bit 0 little-endian; bit 1 raw transcode; bit 2 ternary; bit 3 at least one Q4_K_SOA conversion; bit 4 layer-major ordering; bit 5 layer-pack alignment; bit 6 layer schedule. A flag advertises a profile; it does not replace per-tensor dtype or runtime capability checks. Unknown flag bits are not allocated by this standard even where low-level readers preserve them.

The default page size is **16,384 bytes**, not 4096. Builder configuration `page_log2=0` selects the default; supported configured exponents are 8–30. The serialized header contains the byte size, not the exponent. A valid page size is a power of two at least 256. All alignment computations require overflow checks.

Header metadata offsets and tensor blob offsets/lengths are u32. Normal serialized output must fit the builder's checked u32 extent constraints. The u64 source offset in a tensor entry does not enlarge the output addressing space. Wrapping P64 inside a 64-bit-offset HMC also does not remove this limitation.

## 3. Hyperparameters: 64 bytes

| Offset | Bytes | Type | Meaning |
|---:|---:|---|---|
| 0 | 4 | u32 | Layer count |
| 4 | 4 | u32 | Embedding width |
| 8 | 4 | u32 | Attention heads |
| 12 | 4 | u32 | KV heads |
| 16 | 4 | u32 | Vocabulary size |
| 20 | 4 | f32 | RoPE base |
| 24 | 4 | f32 | RoPE scale |
| 28 | 4 | u32 | Head dimension |
| 32 | 4 | u32 | Sliding-window head dimension |
| 36 | 4 | u32 | Sliding-window size |
| 40 | 4 | u32 | Shared KV setting |
| 44 | 4 | f32 | Final-logit soft cap |
| 48 | 4 | u32 | Architecture identifier |
| 52 | 4 | u32 | Architecture flags |
| 56 | 8 | bytes | Reserved |

Interpret architecture-specific settings through the model preparation layer. Low-level layout parsing does not establish compatible head ratios, finite values, valid tensor dimensions or a supported execution graph.

## 4. Tensor table: 64 bytes per entry

| Offset | Bytes | Type | Meaning |
|---:|---:|---|---|
| 0 | 4 | u32 | Relative offset into string table |
| 4 | 2 | u16 | Role identifier |
| 6 | 2 | u16 | Primary dtype |
| 8 | 4 | u32 | Manifold index |
| 12 | 4 | u32 | Rank |
| 16 | 16 | u32[4] | Dimensions |
| 32 | 4 | u32 | Primary blob file offset |
| 36 | 4 | u32 | Primary blob byte length |
| 40 | 8 | u64 | Original source offset |
| 48 | 8 | u64 | Source name hash |
| 56 | 2 | u16 | Alternate dtype |
| 58 | 2 | u16 | Available precision mask |
| 60 | 4 | u32 | Alternate blob offset |

The last eight bytes are **not uniformly reserved** in v4. However, this entry contains no independent alternate byte length or alternate CRC slot. The current generic reader validates primary payloads, not a complete alternate-payload contract. Consumers MUST separately validate any selected alternate representation with a profile that supplies its size, alignment, integrity and admission rules. `Full` integrity does not silently certify alternate bytes.

Names are NUL-terminated within the string table and addressed relative to its beginning. Validate bounds and required text encoding before application use. Name hashes/source offsets identify source material; they are not collision-resistant content authentication.

### Roles

| ID | Role |
|---:|---|
| 0, 1, 2, 3 | Attention K, V, Q, output |
| 4, 5, 6 | FFN gate, up, down |
| 7, 8 | Attention norm, FFN norm |
| 9, 10, 11 | Token embedding, global output, output norm |
| 12, 13 | Attention sublayer norm, FFN sublayer norm |
| 0x20 | MoE router |
| 0x21, 0x22, 0x23 | MoE gate/up/down experts |
| 0x24, 0x25, 0x26 | Shared-expert gate/up/down |
| 0xfffe | Preserved source tensor without a known execution role |

The ingest layer sentinel `0xffff` means global; serialized global tensor manifold index is `n_layer`. Do not confuse a layer sentinel with a role. Preserving unknown source tensors is deliberate; it does not mean the inference engine executes them.

### Dtypes and packing

Common identifiers are F32=0, F16=1, Q4_0=2, Q5_0=6, Q8_0=8, Q4_K=12, Q6_K=14, BF16=30, Q4_K_SOA=112 and ternary=1158. GGML-family payloads require their selected block codec; byte length cannot generally be calculated as elements times a scalar size.

Q4_K_SOA represents 256 elements in 160 bytes: 128 packed quantization bytes, eight f16 sub-block scales (16 bytes) and eight f16 sub-block minima (16 bytes). It is not ordinary Q4_K with only a new tag. Ternary stores an f32 scale followed by `ceil(element_count/5)` base-3 packed bytes; five trits encode values -1/0/+1 as digits 0/1/2. Valid full packed-byte values are 0–242. Shape, block and tail validation belongs to the codec.

The precision-mask bits 0–6 represent F32, F16, BF16, Q8_0, Q4_K, Q4_K_SOA and ternary respectively. A mask is availability metadata, not proof that every corresponding representation is present and admitted.

## 5. Layout, alignment and schedules

The admitted metadata ordering is hyperparameter end ≤ tensor table; tensor-table end ≤ string table ≤ manifold table; manifold-table end ≤ tokenizer ≤ checksum table. The manifold table contains `n_layer + 1` records. The checksum table holds `tensor_count + 1` u32 entries and must fit in the file. The primary blob floor is `align_up(checksum_table_end, page_size)`.

The low-level reader verifies ranges and ordering used by its views, but is not a canonical writer validator for every unused byte, flag or possible metadata gap. For example, passing a raw header through a preservation API is different from authoring a canonical header. Canonical writers use nonoverlapping metadata regions, initialize padding and retain no unannounced extension data.

Primary blobs are ordered and nonoverlapping. Without layer-pack, each starts on a page boundary. With layer-pack, an entry immediately following one with the same manifold index may start on a 256-byte boundary; a changed manifold index requires page alignment. It is **adjacency**, not whether that layer appeared anywhere earlier, that selects the smaller alignment. Output is padded to a final 64-byte boundary.

Layer-major conversion groups known tensors by layer/role under the selected conversion profile. MoE and preserved unknown tensors make a single old dense-model role list insufficient as a universal ordering rule. Consumers should walk validated table/schedule order and select required roles rather than infer missing blobs.

With flag bit 6, `role_table_offset` locates `n_layer` schedule rows, aligned to 64 and placed after hyperparameters before the tensor table. Each row is:

| Offset | Bytes | Type | Meaning |
|---:|---:|---|---|
| 0 | 4 | u32 | Layer index |
| 4 | 4 | u32 | Inclusive first-blob offset |
| 8 | 4 | u32 | Exclusive last-blob end |
| 12 | 2 | u16 | Tensor count |
| 14 | 2 | u16 | Role mask for IDs below 16 |
| 16 | 48 | bytes | Reserved |

Empty layers use zero begin/end. MoE roles above 15 are not representable in this mask. Schedule bounds are checked on schedule access; a successful initial view parse does not imply the schedule was accessed or semantically checked. The builder's saturating u16 count is not a reliable exact count beyond its range; such files need a consumer limit or future profile before schedule-based execution.

## 6. Manifold records

Each 64-byte record contains ten little-endian f32 coordinates in order: scale, attention depth, epistemic weight, spin, temporal decay, entropy, spatial phase, recurrence, density and curvature, followed by 24 zero bytes in canonical authored records. Layer records occupy indices 0 through `n_layer-1`; the global record is at `n_layer`.

Raw preservation paths retain existing padding and nonfinite bit patterns. Finite/domain validation is a separate helper/admission step; do not describe a successful raw parse as proof that all manifold coordinates are finite. Consumers using those values for scheduling or geometry must make that check explicitly.

## 7. Tokenizer region

The tokenizer occupies the region from its header offset to the checksum offset, potentially including model extension records and alignment padding. A reader must select the tokenizer codec and consume its actual bounds rather than assume all bytes are vocabulary text.

The existing Q42T tokenizer profile begins with `Q42T`, u16 version (current writer 2; legacy 1), u16 flags zero, BOS u32 at 8, EOS u32 at 12, add-BOS u8 at 16 and three zero padding bytes. At offset 20 it stores u32 byte-length-prefixed UTF-8 pre-tokenizer text, a u32 vocabulary count followed by length-prefixed token strings, then a u32 merge count followed by pairs of length-prefixed strings.

Version 2 appends 36 bytes: stop-token count u8, three zero bytes and eight u32 stop-token slots. Valid authored counts are at most eight. Version 1 has no stored stop set and reconstructs it from vocabulary specials. The QualiaDB compatibility decoder caps vocabulary at 1,000,000 and merges at 5,000,000, but uses lossy UTF-8, ignores flags/padding, does not strictly reject unknown versions, clamps stop count and can fall back when a v2 trailer is missing. These permissive behaviors are not canonical writer rules or strong tokenizer admission. Strict consumers must validate the selected version, UTF-8, count, complete trailer and actual tokenizer end before interpreting following extension bytes. Tokenizer compatibility is distinct from P64 structural admission; model families can require different tokenization behavior. The [Q42T implementation](../../../crates/qualia-core-db/src/inference/gguf_sharder/tokenizer.rs) defines this compatibility codec; Etalon's separate GGUF-derived tokenizer planning does not automatically make every Q42T file executable.

## 8. Etalon inference metadata extensions

These are implemented extensions, not proposed QDNF fields. Header bytes 44–63 may contain a 20-byte locator:

| Header offset | Bytes | Meaning |
|---:|---:|---|
| 44 | 8 | `ETLMDTA` plus NUL |
| 52 | 4 | u32 record offset relative to tokenizer-region start |
| 56 | 4 | u32 record length |
| 60 | 2 | u16 locator revision: 1 or 2 |
| 62 | 2 | Zero |

Without a locator, canonical base reserved bytes are zero. With one, model preparation validates magic, revision, record length, region bounds and the selected extension's CRC/semantics. The ordinary P64 metadata CRC covers these bytes, but does not substitute for extension decoding. Builder discovery locates an extension at the end of its supplied tokenizer data; alignment padding may follow in the final file.

### ETLIMD1: 736-byte record

| Record offset | Bytes | Meaning |
|---:|---:|---|
| 0 | 8 | `ETLIMD1` plus NUL |
| 8 | 2 | u16 version 1 |
| 10 | 2 | Zero |
| 12 | 4 | u32 length 736 |
| 16 | 16 | u32 layer count, embedding width, heads, KV heads |
| 32 | 8 | f32 RoPE base, scale |
| 40 | 16 | u32 head dimension, SWA head dimension, sliding window, shared KV |
| 56 | 4 | f32 final-logit soft cap |
| 60 | 32 | u32 SSM convolution/state/groups/rank/inner, full-attention interval, architecture, architecture flags |
| 92 | 4 | u32 PLE-present marker |
| 96 | 20 | u32 PLE ngram, heads/ngram, EOS, row width, layer count |
| 116 | 32 | u32[8] PLE layers |
| 148 | 4 | u32 multiplier count |
| 152 | 64 | i64[8] multipliers |
| 216 | 4 | u32 head count |
| 220 | 256 | i64[32] head offsets |
| 476 | 256 | i64[32] head vocabulary sizes |
| 732 | 4 | CRC-32C over [0,732) |

PLE-absent records require default/zero PLE data. Embedded counts must not exceed their fixed arrays; model preparation checks relationships. Some i64 arrays start at offsets not divisible by eight: decode little-endian bytes rather than assuming native aligned references. The extension does not replace the P64 vocabulary-size field with an undocumented extra value.

### ETLMOE2: 784-byte record

| Record offset | Bytes | Meaning |
|---:|---:|---|
| 0 | 8 | `ETLMOE2` plus NUL |
| 8 | 4 | u32 version 2 |
| 12 | 4 | u32 length 784 |
| 16 | 736 | Complete nested ETLIMD1, including its CRC |
| 752 | 4 | u32 presence flags: bit 0 expert count, bit 1 experts per token |
| 756 | 4 | u32 expert count |
| 760 | 4 | u32 experts per token |
| 764 | 16 | Zero |
| 780 | 4 | CRC-32C over [0,780) |

Only the two presence bits are allocated; at least one is set. An absent value must be zero. Selected routing settings must pass inference admission, including positive counts and applicable top-k/expert bounds. Both nested and outer CRCs matter. The attached expert proposal describes ETLIMD1; this standard also records the newer ETLMOE2 implementation found in the current tree.

## 9. Integrity and security

Checksum entry 0 is CRC-32C over bytes [0,checksum_table_offset). Subsequent entries are CRC-32C over each primary tensor blob, in tensor-table order. The checksum table itself and inter-blob padding are not thereby cryptographically authenticated.

`Structure` mode checks structural admission without CRC validation. Metadata mode additionally checks metadata CRC. `Full` additionally checks every primary tensor CRC. Modes must not be silently upgraded in reports or silently downgraded after failure. Alternate payloads, tokenizer semantics, schedules, extension semantics and execution readiness remain their separate obligations.

P64 has **no built-in full BLAKE3 trailer** in this version. The EMF/M64 truncation repairs do not change P64 CRCs or authorize placing a digest in its extension locator. For adversarial delivery use a trusted, authenticated outer content commitment. Post-quantum encryption/signatures, segment proofs, capability admission and operator-package authorization belong to separately versioned QDNF/runtime profiles.

Embedding a page-aligned P64 inside a QBDL entry aligned only to 64 or 4096 does not automatically preserve its 16 KiB absolute alignment. Check `entry_base + inner_offset` and actual mapped address. Bounded aligned copies or an explicitly supported mapping strategy may be needed. Proposed outer alignment flags are not part of current QBDL v1.

## 10. Migration, determinism and evidence

Do not upgrade v3 by changing only its version number. Parse through a version-aware migration tool, rebuild v4 entries and supported extensions, preserve source offsets/name identity, declare any transformed dtypes and regenerate all CRCs. Revalidate with the current reader and model preparation layer. A deterministic container requires stable metadata, ordering, source bytes and conversion parameters; raw-preservation and quantizing compilation are different contracts.

The current P64-focused Etalon suite passed **21 tests** during this documentation work, covering layout/read-write, integrity modes, padding, alignment, overflow refusal, schedule access, streaming emission and preservation. See [conformance](file-format-conformance.md) for commands and scope; this is not a claim that every inference backend was exercised.

Canonical sources: [layout](../../../../etalon/crates/etalon-model/src/p64/layout.rs), [reader](../../../../etalon/crates/etalon-model/src/p64/reader.rs), [builder](../../../../etalon/crates/etalon-model/src/p64/build/mod.rs), [views](../../../../etalon/crates/etalon-model/src/p64/views.rs), [conversion codecs](../../../../etalon/crates/etalon-model/src/p64/convert/codecs.rs), [inference extension](../../../../etalon/crates/etalon-model/src/inference/extension.rs) and [MoE extension](../../../../etalon/crates/etalon-model/src/inference/moe_metadata.rs). Cross-repository links assume the documented sibling checkout layout. QualiaDB's [compatibility module](../../../crates/qualia-core-db/src/q42/p64_weight/layout.rs) re-exports the canonical layout rather than defining a competing one.
