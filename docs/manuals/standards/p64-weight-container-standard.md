# P64 Weight Container Standard

**Document version:** 0.2  
**Container version:** 4  
**Date:** 2026-07-10  
**Status:** Internal Draft Standard (living — update when layout/decode gaps are found)  
**Canonical extension:** `.p64`  
**Normative implementation:** `crates/qualia-core-db/src/q42/p64_weight.rs`  
**Upgrade plan:** [`docs/plans/p64-decode-upgrade-plan.md`](../../plans/p64-decode-upgrade-plan.md)

> **Living draft rule.** If implementers find container or decode-profile improvements, update
> this standard **and** the upgrade plan in the same change set. Do not leave gaps only in chat.

## Abstract

P64 is QualiaDB's little-endian, cache-line-oriented container for local model
weights. It stores a model's hyperparameters, tensor descriptors, names,
10-dimensional manifold coordinates, tokenizer data, checksums, and opaque
tensor blobs in one memory-mappable artifact.

P64 is a sibling of, not a profile of, the semantic Q42 graph format. A P64
file begins with `p64\0`; a Q42 volume begins with `Q42\0`. P64 contains no
`NQuin` records and makes no truth or provenance claims by itself.

This document specifies the byte layout emitted and accepted by container
version 3 of the QualiaDB implementation.

## 1. Conformance language and scope

The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY**
are to be interpreted as normative requirements.

This standard covers:

- the P64 v3 file and section layout;
- tensor roles, shapes, data types, and source identity;
- embedded model hyperparameters and tokenizer data;
- 10D manifold records;
- page alignment and relative offsets;
- CRC-32C integrity checks; and
- validation required before tensor blobs are exposed to a runtime.

It does not standardize:

- the semantic Q42 volume format;
- GPU buffer layouts derived from P64;
- inference graph execution or sampling;
- model licensing or provenance policy; or
- an Internet media type.

## 2. Design boundary

| Artifact | Magic | Primary responsibility |
|---|---:|---|
| Q42 semantic volume | `Q42\0` | Quins, assertions, provenance, indexes, and graph history |
| P64 weight container | `p64\0` | Model metadata and page-aligned mathematical weight blobs |

A system MAY keep Q42 provenance and P64 weights co-resident, but it MUST
identify each artifact by its own magic. The historical `Q42W` weight format is
superseded and is not conformant with this standard.

The names `compile_gguf_to_q42*`, `from_q42`, and `Q42TensorIndex` remain in the
implementation as compatibility aliases. They read or write P64 v3 bytes and
do not change the on-disk magic or format identity.

## 3. Primitive encoding rules

1. Every integer and floating-point field is little-endian.
2. All file offsets are unsigned 32-bit byte offsets from the first byte of the
   P64 file.
3. A conforming v3 file therefore MUST be smaller than 2^32 bytes.
4. Fixed records are 64 bytes. Reserved bytes and alignment padding emitted by
   a conforming writer MUST be zero.
5. Tensor payload interpretation is selected by the tensor entry's `dtype`.
   Payload bytes are otherwise opaque to the container.
6. Arithmetic used to validate offsets, lengths, counts, and alignments MUST
   be checked for overflow.

## 4. File organization

The canonical v3 writer emits sections in this order:

```text
+-------------------------------+ 0
| P64WeightHeader        64 B   |
+-------------------------------+ hparams_offset
| P64HParams             64 B   |
+-------------------------------+ tensor_table_offset
| P64TensorEntry[tensor_count]  | 64 B each
+-------------------------------+ string_table_offset
| NUL-terminated string table   |
+-------------------------------+ align(64)
| ManifoldCoordinate10D[]       | 64 B each; n_layer + 1 entries
+-------------------------------+ tokenizer_offset
| optional Q42T tokenizer       |
+-------------------------------+ align(64)
| CRC-32C table                 | 4 * (tensor_count + 1) B
+-------------------------------+ align(page_size)
| tensor blob 0                 | start is page-aligned
| padding                       |
| tensor blob 1                 | start is page-aligned
| ...                           |
+-------------------------------+ align(64)
```

`align(N)` means rounding the next section offset upward to a multiple of `N`.
The default `page_size` is 16,384 bytes. A 4,096-byte page profile is also
used by tests and constrained deployments.

The section offsets in the header are authoritative. Readers MUST NOT rely
only on the canonical ordering diagram.

## 5. P64WeightHeader

The header occupies exactly 64 bytes.

| Byte range | Type | Field | v3 requirement |
|---:|---:|---|---|
| `0..3` | `u8[4]` | `magic` | MUST equal `70 36 34 00` (`p64\0`) |
| `4..5` | `u16` | `version` | MUST equal `4` (historical readers may accept `3` as a soft alias) |
| `6..7` | `u16` | `flags` | Format flags; bit 0 MUST be set |
| `8..11` | `u32` | `role_table_offset` | MUST be zero in the current v3 profile |
| `12..15` | `u32` | `tensor_table_offset` | Start of 64-byte tensor entries |
| `16..19` | `u32` | `tokenizer_offset` | Start of optional tokenizer section |
| `20..23` | `u32` | `hparams_offset` | Start of the 64-byte hyperparameter record |
| `24..27` | `u32` | `string_table_offset` | Start of the string pool |
| `28..31` | `u32` | `checksum_offset` | Start of the CRC-32C table |
| `32..35` | `u32` | `manifold_table_offset` | Start of 64-byte manifold records |
| `36..39` | `u32` | `tensor_count` | Number of tensor entries and blobs |
| `40..43` | `u32` | `page_size` | Power-of-two blob alignment, at least 256 |
| `44..63` | `u8[20]` | `reserved` | MUST be zero |

### 5.1 Header flags

| Bit | Constant | Meaning |
|---:|---|---|
| 0 | `P64_FLAG_LITTLE_ENDIAN` | Required. Numeric fields use little-endian encoding. |
| 1 | `FORMAT_FLAG_RAW_TRANSCODE` | Source was streamed from a high-fidelity tensor container without a complete GGUF model profile. |
| 2 | `FORMAT_FLAG_TERNARY` | One or more tensors use P64's BitNet-1.58b ternary encoding. |
| 3 | `P64_FLAG_Q4K_SOA` | At least one 2-D weight matrix uses `dtype = 112` (Q4_K_SOA). |
| 4 | `P64_FLAG_LAYER_MAJOR` | Known-role tensor blobs are stored in layer-major table order (see §7.3). |
| 5 | `P64_FLAG_LAYER_PACK` | Page-align only at layer boundaries; 256-byte align within a layer (decode/CUDA residency). |
| 6 | `P64_FLAG_LAYER_SCHEDULE` | `role_table_offset` points at `P64LayerScheduleEntry[n_layer]` (64 B each). |
| 7..15 | reserved | Writers MUST clear these bits until allocated in a later draft. |

The raw-transcode flag permits zero-valued model hyperparameters and an empty
tokenizer section. The ternary and Q4_K_SOA flags are container-level hints;
each tensor's `dtype` remains authoritative. Readers that care about residency
order SHOULD prefer files with `P64_FLAG_LAYER_MAJOR` set.

## 6. P64HParams

The hyperparameter record occupies exactly 64 bytes.

| Byte range | Type | Field | Meaning |
|---:|---:|---|---|
| `0..3` | `u32` | `n_layer` | Transformer layer count |
| `4..7` | `u32` | `n_embd` | Embedding width |
| `8..11` | `u32` | `n_head` | Attention head count |
| `12..15` | `u32` | `n_kv_head` | Key/value head count |
| `16..19` | `u32` | `vocab_size` | Token vocabulary size |
| `20..23` | `f32` | `rope_freq_base` | RoPE frequency base |
| `24..27` | `f32` | `rope_scale` | RoPE scale |
| `28..63` | `u8[36]` | `reserved` | MUST be zero |

For a raw Safetensors transcode, fields unavailable from the source metadata
MAY be zero. A runtime that requires those fields MUST reject or supplement
such a container before inference.

## 7. P64TensorEntry

Each tensor descriptor occupies exactly 64 bytes.

| Byte range | Type | Field | Meaning |
|---:|---:|---|---|
| `0..3` | `u32` | `name_offset` | Byte offset into the string table |
| `4..5` | `u16` | `role_id` | Semantic engine role |
| `6..7` | `u16` | `dtype` | GGML-compatible element type or P64 extension |
| `8..11` | `u32` | `manifold_idx` | Index into the manifold table |
| `12..15` | `u32` | `rank` | Tensor rank; MUST be in `1..=4` |
| `16..31` | `u32[4]` | `dimensions` | Shape; unused trailing dimensions are zero |
| `32..35` | `u32` | `blob_offset` | File-relative payload offset |
| `36..39` | `u32` | `blob_size` | Payload size in bytes |
| `40..47` | `u64` | `source_offset` | Original tensor offset in the source data region |
| `48..55` | `u64` | `source_name_hash` | Canonical hash of the original tensor name |
| `56..63` | `u8[8]` | `reserved` | MUST be zero |

`blob_offset` MUST be a multiple of `page_size`. Entries MUST describe blobs in
non-decreasing file order, and their byte ranges MUST NOT overlap.

`source_name_hash` preserves the hash convention of the producer profile. A
GGUF conversion uses full-width FNV-1a 64-bit with offset basis
`0xcbf29ce484222325` and prime `0x100000001b3`. The Safetensors profile uses
QualiaDB `q_hash`, which applies the same FNV-1a calculation and then masks the
result with `0x0fff_ffff_ffff_ffff`. Source-parity validators MUST compare
using the convention of the corresponding producer profile.

### 7.1 Tensor roles

| Value | Constant | Role |
|---:|---|---|
| 0 | `P64_ROLE_ATTN_K` | Attention key projection |
| 1 | `P64_ROLE_ATTN_V` | Attention value projection |
| 2 | `P64_ROLE_ATTN_Q` | Attention query projection |
| 3 | `P64_ROLE_ATTN_OUTPUT` | Attention output projection |
| 4 | `P64_ROLE_FFN_GATE` | Feed-forward gate projection |
| 5 | `P64_ROLE_FFN_UP` | Feed-forward up projection |
| 6 | `P64_ROLE_FFN_DOWN` | Feed-forward down projection |
| 7 | `P64_ROLE_ATTN_NORM` | Attention/input normalization |
| 8 | `P64_ROLE_FFN_NORM` | Post-attention/FFN normalization |
| 9 | `P64_ROLE_TOKEN_EMBD` | Global token embedding |
| 10 | `P64_ROLE_OUTPUT` | Global output or language-model head |
| 11 | `P64_ROLE_OUTPUT_NORM` | Global final normalization |
| `0xfffe` | `P64_ROLE_UNKNOWN` | Preserved tensor with no recognized engine role |

Unknown source tensors MUST be retained when performing a byte-preserving GGUF
conversion. A runtime MAY ignore an unknown role for execution, but validators
MUST still validate its descriptor, alignment, bounds, and checksum.

Global tensors use manifold index `n_layer`. The Rust constant
`P64_LAYER_GLOBAL = 0xffff` is an ingest-side role-mapping sentinel; it is not
written to the 32-bit `manifold_idx` field.

### 7.2 Tensor data types

For source-preserving tensors, `dtype` carries the source GGML type code. The
current engine explicitly supports these common codes:

| Value | Encoding |
|---:|---|
| 0 | F32 |
| 1 | F16 |
| 2 | Q4_0 |
| 6 | Q5_0 |
| 8 | Q8_0 |
| 12 | Q4_K |
| 14 | Q6_K |
| 30 | BF16 |
| **112** | **Q4_K_SOA** (Qualia decode-profile layout; not a stock GGML type) |
| 1158 | P64 BitNet-1.58b ternary |

Other source type codes MAY be preserved when the producer and consumer both
support them. Consumers MUST NOT infer a tensor encoding solely from the
container-level ternary or SoA flags.

### 7.2.1 Q4_K_SOA (`dtype = 112`)

Produced by convert layout `P64ConvertLayout::Q4kSoa` from source Q4_K matrices.
Each 256-element superblock is **160 bytes**:

```text
qs[128]           // nibble payload (same element order as stock Q4_K)
d_sub[8] × f16    // pre-expanded (d * sub_scale) for 8 groups
m_sub[8] × f16    // pre-expanded (dmin * sub_min) for 8 groups
```

Purpose: GEMV kernels read scales without re-decoding packed 6-bit headers.
When any tensor uses this dtype, writers MUST set `P64_FLAG_Q4K_SOA`.

### 7.3 Layer-major blob order

When `P64_FLAG_LAYER_MAJOR` is set, the tensor table and blob region for
**known roles** MUST be ordered as:

```text
for layer in 0..n_layer:
  attn_norm, attn_q, attn_k, attn_v, attn_output,
  ffn_norm, ffn_gate, ffn_up, ffn_down
then globals: token_embd, output, output_norm
then P64_ROLE_UNKNOWN tensors (stable by source_offset)
```

Writers MUST NOT re-sort known-role entries by the original GGUF source offset.
Decode residency and CUDA multi-weight fill SHOULD walk the table in order.

### 7.4 Layer-pack alignment and schedule table

When `P64_FLAG_LAYER_PACK` is set:

- The **first** tensor blob and the **first tensor of each new layer** (and globals
  after layers) MUST start at a multiple of `page_size`.
- Subsequent tensors **within the same layer** MUST start at a multiple of **256**.

When `P64_FLAG_LAYER_SCHEDULE` is set, `role_table_offset` is the start of
`n_layer` consecutive `P64LayerScheduleEntry` records (exactly 64 bytes each):

| Byte range | Type | Field |
|---:|---:|---|
| `0..3` | `u32` | `layer` |
| `4..7` | `u32` | `blob_begin` (inclusive file offset of first blob) |
| `8..11` | `u32` | `blob_end` (exclusive end of last blob) |
| `12..13` | `u16` | `tensor_count` |
| `14..15` | `u16` | `roles_mask` (bit `i` ⇒ role_id `i` present) |
| `16..63` | `u8[48]` | reserved, zero |

Runtimes MAY `mmap` or bulk-upload `[blob_begin, blob_end)` as one residency unit per layer.

The ternary payload is:

```text
scale             f32 little-endian
packed trits      ceil(element_count / 5) bytes
```

Five trits are packed in each byte as base-3 digits:

```text
digit = trit + 1                    # -1, 0, +1 -> 0, 1, 2
byte  = d0 + 3*d1 + 9*d2 + 27*d3 + 81*d4
```

The tensor element count is the product of the first `rank` dimensions.

## 8. String table

The string table begins at `string_table_offset` and ends at
`manifold_table_offset`. Each tensor entry's `name_offset` is relative to the
start of this table.

The canonical writer places a zero byte at relative offset 0, then stores each
UTF-8 tensor name followed by a NUL byte. A reader MUST confirm that every
referenced name starts inside the string table and has a terminating NUL before
the manifold table.

Known GGUF roles use canonical names such as:

```text
blk.3.attn_q.weight
blk.3.ffn_down.weight
token_embd.weight
output.weight
output_norm.weight
```

An unknown GGUF tensor MAY be represented as
`tensor.<source_name_hash-as-16-lowercase-hex-digits>` when its source spelling
is not retained by the conversion index.

## 9. 10D manifold table

The manifold table:

- begins at a 64-byte-aligned offset;
- contains exactly `n_layer + 1` records;
- uses records of exactly 64 bytes; and
- reserves record index `n_layer` for global tensors.

Each record stores ten little-endian `f32` values followed by 24 zero bytes:

| Byte range | Dimension |
|---:|---|
| `0..3` | `scale` |
| `4..7` | `attention_depth` |
| `8..11` | `epistemic_weight` |
| `12..15` | `topological_spin` |
| `16..19` | `temporal_decay` |
| `20..23` | `entropy_bias` |
| `24..27` | `spatial_phase` |
| `28..31` | `recurrence_frequency` |
| `32..35` | `density_threshold` |
| `36..39` | `manifold_curvature` |
| `40..63` | reserved, zero |

Every coordinate value MUST be finite. Each tensor's `manifold_idx` MUST be
less than `n_layer + 1`.

The canonical sequential-layer projection is:

```text
depth                  = layer / max(total_layers, 1)
scale                  = depth
attention_depth        = 1 - depth
epistemic_weight       = 1
topological_spin       = sin(depth * pi)
temporal_decay         = 0.1
entropy_bias           = 0.5
spatial_phase          = cos(depth * tau)
recurrence_frequency   = 1
density_threshold      = 0.8
manifold_curvature     = 0
```

## 10. Embedded tokenizer section

A full GGUF conversion embeds a version-1 `Q42T` tokenizer section at
`tokenizer_offset`. Its logical payload has this layout:

| Field | Encoding |
|---|---|
| magic | `Q42T` |
| section version | `u16`, currently 1 |
| flags | `u16`, currently 0 |
| BOS token id | `u32` |
| EOS token id | `u32` |
| add-BOS policy | `u8`, zero=false, nonzero=true |
| padding | 3 zero bytes |
| pre-tokenizer type | length-prefixed UTF-8 |
| vocabulary count | `u32` |
| vocabulary entries | repeated length-prefixed UTF-8 |
| merge-pair count | `u32` |
| merge pairs | repeated left and right length-prefixed UTF-8 strings |

A length-prefixed string is a little-endian `u32` byte length followed by that
many UTF-8 bytes. The implemented reader limits the vocabulary to 1,000,000
entries and merge pairs to 5,000,000.

The tokenizer section ends at `checksum_offset`; zero alignment padding MAY
follow its logical payload. A raw-transcode container MAY have
`tokenizer_offset == checksum_offset`, indicating no embedded tokenizer.

## 11. Integrity table

The checksum table begins at `checksum_offset` and contains
`tensor_count + 1` little-endian `u32` values:

```text
entry 0       CRC-32C(file bytes [0, checksum_offset))
entry 1       CRC-32C(tensor blob 0)
entry 2       CRC-32C(tensor blob 1)
...
entry N       CRC-32C(tensor blob N - 1)
```

P64 uses CRC-32C Castagnoli in reflected form with polynomial `0x82f63b78`,
initial value `0xffffffff`, and final bitwise complement.

The metadata checksum covers the header, hyperparameters, tensor table, string
table, manifold table, tokenizer, and all alignment padding before the checksum
table. It does not cover the checksum table itself.

CRC-32C detects accidental corruption and common tampering, but is not a
cryptographic signature. Authenticity and provenance MUST be supplied by a
separate trusted mechanism, such as the associated Q42 graph or signed release
metadata.

## 12. Tensor blob region

The blob floor is:

```text
align(checksum_offset + 4 * (tensor_count + 1), page_size)
```

Every `blob_offset` MUST be at or above that floor and aligned to `page_size`.
The end of every blob MUST be within the file. A reader MUST validate blobs in
descriptor order and reject overlap or backward ordering.

The canonical writer pads the complete file to a 64-byte boundary. Padding
between sections and blobs is not part of any tensor payload.

P64 preserves the source tensor's native dimension order. GPU or SIMD
implementations MAY construct a transposed, structure-of-arrays, paired-`u32`,
or otherwise specialized execution view, but MUST NOT describe that derived
view as the P64 disk layout.

## 13. Producer profiles

### 13.1 Byte-preserving GGUF profile

A byte-preserving GGUF producer:

- copies every source tensor, including unknown roles;
- preserves its type, shape, source offset, source-name hash, length, and bytes;
- assigns known engine roles where recognized;
- embeds GGUF hyperparameters and tokenizer data; and
- emits one manifold record per model layer plus one global record.

`P64TensorIndex::validate_against_gguf` additionally proves tensor-count,
metadata, and byte equality against the source GGUF. That source-parity check is
stronger than ordinary standalone P64 validation.

### 13.2 Quantized GGUF profile

A quantizing producer MAY replace selected tensor blobs and update their
`dtype`, `blob_size`, offsets, checksums, and relevant flags. QualiaDB's policy
profile permits ternary or Q4_0 conversion of FFN gate/up/down projections
while retaining attention, norms, embeddings, output tensors, and unknown
tensors at higher fidelity.

### 13.3 Raw Safetensors profile

A streaming Safetensors producer sets `FORMAT_FLAG_RAW_TRANSCODE`. It MAY omit
the tokenizer and use zero for unavailable hyperparameters. Recognized tensor
names receive engine roles; unrecognized names receive
`P64_ROLE_UNKNOWN`. Its `source_offset` is relative to the Safetensors data
region.

## 14. Required reader validation

Before exposing any tensor blob, a conforming reader MUST:

1. require at least 64 bytes for the header;
2. validate magic, version, and little-endian flag;
3. require a power-of-two `page_size` of at least 256;
4. bounds-check the 64-byte hyperparameter record;
5. calculate every section extent with checked arithmetic;
6. reject overlapping or out-of-order metadata sections;
7. require a 64-byte-aligned manifold table;
8. verify the metadata CRC-32C;
9. require every tensor rank to be in `1..=4`;
10. validate every manifold index;
11. validate every referenced NUL-terminated tensor name;
12. require every blob to be page-aligned, ordered, non-overlapping, and
    in-bounds; and
13. verify every tensor CRC-32C.

A reader SHOULD also validate tokenizer syntax, finite manifold values, shape
and payload-size consistency for recognized `dtype` values, and any
model-specific required-role set before beginning inference.

Validation is fail-closed: a failed check means no tensor from that container
is trusted for execution.

## 15. Versioning and extension rules

The current implementation accepts container version 3 only. An incompatible
layout change MUST increment the header version.

Within v3:

- reserved bytes MUST remain zero;
- reserved flag bits MUST remain zero in newly written files;
- `role_table_offset` MUST remain zero until a role-table extension is
  standardized; and
- new role or `dtype` values MUST NOT change the meaning of existing values.

Readers MAY preserve unsupported tensors without executing them, but MUST NOT
guess their encoding.

## 16. Security considerations

P64 files are untrusted binary input. Implementations must defend against
integer overflow, excessive counts, malformed names and tokenizer strings,
invalid floating-point values, overlapping ranges, checksum confusion, and
resource exhaustion.

Memory mapping does not make a file safe. All validation in Section 14 occurs
before offsets are used for zero-copy GPU upload or inference.

CRC-32C is not collision-resistant. Where a P64 is distributed outside a
trusted local cache, deployments SHOULD verify a cryptographic digest or
signature bound to the model identity, source artifact, conversion policy, and
P64 version.

## 17. Implementation references

- P64 reader, writer, transcoders, roles, CRCs, and validation:
  `crates/qualia-core-db/src/q42/p64_weight.rs`
- GGML type sizes and dequantization:
  `crates/qualia-core-db/src/inference/ggml_quants.rs`
- Ternary codec:
  `crates/qualia-core-db/src/inference/ternary.rs`
- Tensor-name to role mapping:
  `crates/qualia-core-db/src/inference/tensor_roles.rs`
- Embedded tokenizer:
  `crates/qualia-core-db/src/inference/gguf_sharder.rs`
- 10D manifold record:
  `crates/qualia-core-db/src/modalities/manifold.rs`
- Q42/P64 architectural boundary:
  `docs/manuals/standards/q42-format-internal-draft.md`
- End-to-end loading, inference, governance, and platform behavior:
  `docs/manuals/p64-q42-inference-pipeline.md`
