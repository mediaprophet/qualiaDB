# QualiaDB file-format standards

**Revision:** 1 — 12 November 2026. **Status:** Internal implementation-aligned standards. These documents specify the inspected development tree, including the full BLAKE3 repair; they do not assert that installed binaries or released archives already contain it.

## Standards and scope

| Format | Standard | Current identity and integrity |
|---|---|---|
| EMF | [EMF container](emf-container-format.md) | `Q42EMF64`, v2; full 32-byte BLAKE3 per record payload plus header/directory CRC-32C |
| E64 | [E64 compatibility format](e64-compatibility-format.md) | `e64` + NUL; M64 v1/v2 wire alias; codec 3 full BLAKE3 |
| M64 | [M64 matter container](m64-matter-container.md) | `m64` + NUL; default v2; codec 3 full BLAKE3, raw codec 0 distinct; developmental v3 GRM3 |
| 10D | [10D container standard](10d-container-standard.md) | `10d` + NUL, v2; whole-file and section CRC-32C; typed payload profiles |
| P64 | [P64 weight container](p64-weight-container-standard.md) | `p64` + NUL, v4; metadata and primary-tensor CRC-32C; canonical implementation in Etalon |
| HMC | [HMC container profiles](hmc-container-format.md) | QBDL v1 transparent bundle (SHA-256 entries) or separate ZIP/JSON document profile (full BLAKE3 assets) |

Names and extensions do not override magic/version/profile. E64 is not EMF. HMC has two incompatible physical encodings. P64 manifold coordinates are not the same record as a Tensor10D node. A byte-valid container does not establish scientific correctness, provenance authority, execution permission, publication rights or a trustworthy network peer.

## Reading these standards

Offsets are decimal, zero-based byte positions. Ranges use half-open `[start,end)` notation. Integers are unsigned unless stated; `f32`/`f64` are IEEE binary32/binary64. Binary numeric fields are little-endian unless a separately identified codec says otherwise. Counts, multiplication, offset addition, alignment rounding and host-width conversion require overflow checks before slicing or allocation.

MUST/MUST NOT express the internal conformance requirements stated here. Each document distinguishes writer requirements, actual reader checks and additional consumer validation. Reader permissiveness is not permission for writers to emit ambiguous or insecure data. Implementation gaps are named explicitly. Preserve unknown bytes only through a documented preservation path; recognition of an outer section is not permission to execute its contents.

Zero-copy requires both a validated range and suitable actual memory alignment/lifetime. A file-relative alignment guarantee does not guarantee the starting address of an arbitrary borrowed slice or an embedded bundle entry. Consumers may use unaligned decoding or bounded aligned copies where their APIs permit it. No universal zero-allocation or whole-system memory guarantee follows from a fixed record size.

CRC-32C uses reflected Castagnoli polynomial `0x82f63b78`, initial state `0xffffffff`, final complement. The check value for ASCII `123456789` is `0xe3069283`. Coverage differs by format: consult its table rather than assuming all checksums zero their own field. Digests are raw bytes unless explicitly hex-encoded. CRCs are not cryptographic authentication; unkeyed full digests require trusted expected values for adversarial assurance.

## Full BLAKE3 repair and migration

Historical EMF 16-byte and M64/E64 12-byte BLAKE3 prefix checks were **bugs**, not standardized integrity features or Bao proofs. The repaired chunk encoding stores:

```text
N records of 64 bytes
32 bytes: full default BLAKE3 digest of exactly those record bytes
32 bytes: reserved zero padding
```

The trailer is 64 bytes and is not a record. Directory stored length includes it; record count and exposed record payload exclude it. EMF v2 requires row flag `0x0004`, clears legacy row `0x0002` and header `0x0008`, and zeros its old prefix. M64/E64 select codec identifier 3 and zero the old prefix. M64 raw codec 0 remains available without full-digest assurance. The GRM3 profile currently requires raw strata chunks; codec 3 is not silently introduced into it.

Default checked readers reject defective legacy prefix encodings. Explicit `parse_legacy_with_limits` APIs exist for migration; success there is not full-digest certification. Recompute new digests from actual records, rewrite offsets/lengths and CRCs, then validate with the default current reader. Do not zero-extend a prefix, relabel codec/version, or downgrade after a current verification failure. Enclosing HMC entry hashes and any signed external commitments must be regenerated when inner bytes change.

This repair does not change 10D or P64 into BLAKE3 formats, replace QBDL's SHA-256 index, or alter the already-full BLAKE3 ZIP HMC manifest. It provides no Bao range proof, signature, encryption or complete PQ trust chain.

## QDNF streaming boundary and advisor attachment

The [initial advisory brief](qbdl-streaming-expert-advisor-brief.md) and the [user-supplied expert proposal](references/qbdl-streaming-advisor-proposal-2026-11-12.md) inform future native QDNF streaming, HTTP compatibility and post-quantum protection. They are not adopted file-format extensions. The supplied proposal is preserved verbatim; statements within it are reference material, not instructions governing this documentation task.

Reconciliation with the current source:

| Advisor topic | Current standard boundary |
|---|---|
| Archived source lacked the repair | Current local tree has the full-digest implementation and tests; deployed/archive compatibility must be assessed separately |
| 10D section-table stride stated as 32 | Implemented descriptor size is **24** bytes |
| QBDL 16 KiB/extended alignment flags, reserved byte 40 | Proposed only; current writer flags are zero or `0x0001` for 4096, and bytes 40–63 remain zero |
| P64 default 16 KiB pages | Confirmed; 64/4096 outer alignment does not guarantee it. Some placements happen to satisfy it; check actual offset rather than claiming all are broken |
| 736-byte ETLIMD1 metadata | Confirmed; current Etalon also defines a 784-byte ETLMOE2 extension and a locator in header bytes 44–63 |
| Embedded Bank 1 LUT before M64 directory | Base M64 header defines no automatic LUT section offset; epoch/reference resolution belongs to the selected application profile |
| QASD, SHA-384 trees, segment AEAD, new service names, QBD2 | Proposed network profiles, not bytes emitted/accepted by the current container codecs |
| Etalon residency, operator packages and sensing | Consumer/admission dependencies; not implicit new sections of these six file formats |

The proposal's cryptographic constructions, identifiers, numerical thresholds and external citations require their own review and vectors before adoption. No algorithm allocation, new alignment flag or encrypted-stream framing is introduced here. Native QDNF remains the intended service architecture, with HTTP a compatibility binding. PQ network commitments must be explicitly versioned and must not truncate their values into existing compact fields.

## Evidence and maintenance

See [conformance and defect register](file-format-conformance.md) for tests, source ownership, remaining implementation limitations and migration cases. Existing focused 10D/GRM3 companions remain linked from the relevant standard. Update this set whenever a codec version, validation rule or producer changes; distinguish a newly authored test from an executed result.

Attached proposal SHA-256 (provenance of the saved reference, not a security approval): `39b472f2d62630155693cea6b57ca9ee504962c5508147b8938fef47c5686561`.
