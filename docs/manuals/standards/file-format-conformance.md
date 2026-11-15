# File-format conformance, migration and defect register

**Revision:** 1, 12 November 2026. This is the evidence and maintenance companion to the [six format standards](file-formats-overview.md). It records the inspected working tree, not a claim about a published release or every installed binary.

## 1. Executed verification

| Check | Result and scope |
|---|---|
| Focused production-source container harness | **61 passed**, rerun during completion of these standards. Imports actual shared CRC, full-hash, EMF and M64/E64 production modules; does not substitute a separate codec implementation. |
| Etalon P64-focused library tests | **21 passed** during completion: layouts, alignment, integrity modes, schedules, overflow/refusal, streaming writes and raw preservation. |
| Etalon default-feature library suite | **33 passed**, including those same 21 P64 tests plus GGUF/tokenizer tests. These counts are overlapping, not 54 distinct tests. |
| Hypermedia electromagnetics producer tests | **7 passed** during the preceding integrity repair. |
| Hypermedia geodesy producer tests | **5 passed** after replacing an obsolete prefix-based producer self-check during that repair. |
| Documentation review | Wire-table arithmetic, local Markdown links, source ownership, current constants and attachment identity checked during completion. |

The full QualiaDB workspace, 10D payload suite, ZIP HMC suite, QDNF transport and every feature-gated Etalon inference extension were **not** rerun as part of this documentation completion. Their descriptions are based on source inspection and existing companions; absence from the test list is not evidence that their implementations are missing. No live MCP executable was replaced or restarted.

Reproduction commands (PowerShell, from the named repository):

```powershell
# QualiaDB
cargo test --manifest-path tools/container-format-conformance/Cargo.toml --target-dir target-container-standards --offline

# Etalon
cargo test -p etalon-model --lib --target-dir C:\github\etalon\target-container-standards --offline

# Synth workspace, for the previously executed producer checks
cargo test -p asset-production-vibe --lib electromagnetics_ops --target-dir C:\github\synth\target-prod --offline
cargo test -p asset-production-vibe --lib geodesy_ops --target-dir C:\github\synth\target-prod --offline
```

The focused harness is at [tools/container-format-conformance](../../../tools/container-format-conformance/lib.rs). It avoids unrelated render/inference dependencies while exercising the production codec modules. The P64 package must be tested from the Etalon workspace: invoking its dev-dependency tests as a nonmember dependency from Synth is not supported.

## 2. Full BLAKE3 repair acceptance

The shared [chunk integrity implementation](../../../crates/qualia-core-db/src/q42/chunk_integrity/mod.rs) writes and verifies a 64-byte trailer: complete 32-byte BLAKE3 digest of records followed by 32 zero bytes. The [cross-format regressions](../../../crates/qualia-core-db/src/q42/chunk_integrity/container_tests.rs) check:

- M64 and E64 use the same full-digest contract, including coexistence with per-record CRCs.
- Mutating **each of the 64 trailer bytes** fails, including formerly discarded digest tails and reserved padding.
- Record mutation, truncation and a codec downgrade with incompatible length fail admission.
- Readers expose records only, never the trailer as an extra record/cell.
- EMF v2 emission is deterministic for the fixture; defective v1 needs explicit opt-in and can be resealed.
- Missing or mixed EMF integrity flags fail, rather than silently selecting a weaker comparison.

The two simple one-record examples in the EMF/M64/E64 docs are 256-byte objects: 64-byte header, 64-byte directory, 64-byte record and 64-byte trailer. Full brick payloads are 32,768 + 64 bytes for EMF and 262,144 + 64 for the M64 16³ profile. Directory stored lengths include the trailer; record counts do not.

These tests establish use of all digest bytes. They do not establish scientific truth, authenticated provenance, a Bao range proof, a signature, or a complete post-quantum security profile. An attacker able to replace both content and its untrusted digest can recompute the hash.

## 3. Compatibility and migration matrix

| Input | Current default behavior | Required action for current full-hash use |
|---|---|---|
| EMF v2 with full flag/trailer | Accepted after all applicable checks | No migration |
| EMF v1 prefix encoding | Rejected by default; explicit legacy API exists | Validate available evidence, rewrite v2, regenerate lengths/CRCs/digests |
| M64/E64 v1/v2 codec 3 | Accepted after all applicable checks | No migration |
| M64/E64 raw codec 0, zero prefix | Accepted, no full chunk digest | Reseal with codec 3 if full-hash policy is required |
| M64/E64 codec 2 or nonzero old prefix | Rejected by default; explicit legacy API exists | Rewrite with codec 3; do not pad a prefix into a digest |
| M64 GRM3 v3 raw strata | Admitted under its developmental profile | Preserve its raw-only constraint; do not silently invent a new GRM3 codec |
| 10D v1/v2 | CRC-based container under its version rules | No invented BLAKE3 field; use authenticated outer commitment when needed |
| P64 v3 | Current canonical reader requires v4 | Version-aware rebuild, not a version-byte patch |
| QBDL v1 | SHA-256 entry hashes and whole CRC | Preserve algorithm; rehash changed inner files |
| ZIP HMC v1 | Full BLAKE3 manifest assets already present | Regenerate digest/size only when asset bytes change |

Migration must be explicit, bounded and auditable. Preserve originals until the new object passes the current reader and any application checks. Rehashing damaged or untrusted bytes cannot recover historical authenticity. Recompute enclosing bundle digests and signed commitments; do not fall back to legacy parsing after a current-format integrity failure.

## 4. Remaining limitations and defects

These items are recorded to prevent reader permissiveness or stale comments from becoming approved format features. They are not all changes implemented by the hash repair.

| Area | Observed limitation | Documentation/conformance treatment |
|---|---|---|
| Historical EMF/M64 prefixes | Truncated 16/12-byte comparisons incorrectly presented as Bao | **Repaired for current full-hash profiles**; migration-only compatibility, no Bao claim |
| M64 raw and GRM3 | Some admitted profiles have no full chunk digest | State assurance explicitly; no universal claim that every M64 file is full-hashed |
| M64 header compatibility | Legacy CRC excludes flags | Current writers include flags; retained compatibility weakness identified |
| EMF spatial decoding | Hilbert order inferred; not serialized | State inference/order limits and duplicate-coordinate rejection; no universal order claim |
| Scientific records | Structural readers do not prove units, finite values or physical validity | Require consumer domain checks |
| 10D fields | FIEL name differs from implemented LE magic spelling | Document actual integer/bytes; any change needs an explicit migration/version decision |
| 10D preservation | Strict parser rejects unknown types; separate opaque path exists | Preserve without execution only through the preservation contract |
| P64 alternate views | No independent alternate length/CRC in entry; Full checks primary blobs | Require explicit alternate-profile validation |
| P64 tokenizer | Q42T compatibility parser is permissive on versions, text and trailer | Canonical authored v2 is explicit; strong consumers validate independently |
| P64 metadata/schedules | Some semantic checks occur on access/preparation | Do not equate initial view parse with execution readiness |
| QBDL bounded reader | Rejects writer-supported 4096 flag | Use bounded 64-byte profile or separately reviewed bounded handling; no silent bypass |
| QBDL streaming loader | Missing/malformed optional digest may reduce verification to length | Defect, not a standardized insecure mode; require valid authenticated commitment |
| QBDL embedding | 64/4096 alignment does not guarantee P64 16 KiB alignment | Check actual placement/mapping; proposal flags not adopted |
| ZIP HMC | Convenience reading/verification lacks complete hostile-archive budgets and schema-coherence checks | Additional admission and extraction policy required |

Compact DID/name hashes such as FNV are semantic lookup identifiers, not truncated versions of the repaired cryptographic payload digests. The standards do not replace those identifiers with BLAKE3 or claim they authenticate content.

## 5. Independent conformance work

An independent implementation should add cross-language golden files for each admitted version/profile, exact digest/CRC vectors, malformed offset/count/alignment cases, every record mode, unsupported-version refusal, legacy resealing and nested-container rehashing. Exercise all fields beyond the former digest prefixes and all zero trailer padding. Verify that unknown/opaque preservation retains bytes without granting execution. Distinguish whole-file, entry, chunk, record and semantic checks in reports.

Further QDNF work needs its own authenticated manifest/proof/key-lifecycle vectors, transport interoperability and adversarial bounded-decoder tests. The supplied expert brief is a proposal; its instructions, algorithm choices and field allocations are not incorporated as normative changes to the six existing formats.

## 6. Source ownership

| Owner | Format responsibility |
|---|---|
| QualiaDB Core `q42/emf_container` | EMF header, directories, cells, authoring and admission |
| QualiaDB Core `q42/m64_matter` | M64/E64 records, frames, directories and GRM3 wire checks |
| QualiaDB Core `q42/chunk_integrity` | Shared full BLAKE3 trailer |
| QualiaDB Core `container_10d` | 10D envelope, payload registry and codecs |
| Etalon `etalon-model` | Canonical P64 layout, build/read and inference metadata extensions |
| QualiaDB Core `bundle` | QBDL HMC; bounded and streaming readers are separate paths |
| Qualia semantic library `container` | ZIP/JSON document HMC |
| Hypermedia / Synth | Producer profiles, sealed artifacts and bundle packaging |

Source-linked specifications at [the overview](file-formats-overview.md) are the finished entry points. The [travel checkpoint](file-formats-standards-work-in-progress.md) is historical research and must not be used to override the repaired versions or current tables.
