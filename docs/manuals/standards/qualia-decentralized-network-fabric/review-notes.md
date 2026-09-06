# QDNF review notes

## 2026-09-05 — Commons and resource economics: review — done

- Reviewed architecture, routing metrics, sessions, governance, registries, deployment guidance,
  and implementation planning alongside the existing Permissive Commons manual.
- Found no economic agreement/settlement lifecycle in QDNF and no common scope contract behind its
  energy and monetary route metrics. Existing commons payment claims require source verification.
- Direction: energy and time remain independent resource dimensions; agreed valuation, obligations,
  voluntary contributions, and optional settlement have distinct records and evidence.
- Evidence: BIPM SI, W3C ODRL, and Interledger STREAM primary references checked. Runtime and
  performance were not measured. Implementation evidence review is in progress.
- Human input needed: none this step. Rates, threshold targets, and community funding allocations
  remain participant decisions.
- Next: integrate the new economics profile into the fabric and verify links and consistency.
- Workspace note: edits use the supplied `C:\github\qualiaDB` tree. The historical coordination feed
  was absent; creation of the missing `docs/plans` directory was denied by filesystem permissions.
  This workstream's progress record is therefore kept here beside the specification.

## 2026-09-06 — Economic and ontological contract design — done

- Added `commons-and-resource-economics.md` and `ontological-contracts.md`, with scoped joule/second
  accounts, nonmonetary funding, accepted quotes/caps, settlement reconciliation, and threshold
  release rules. CBOR-LD binds exact contract bytes to pinned ontology/context/table/shapes/rules.
- Integrated the profiles into architecture, wire, session, policy, registry, operations, and
  implementation guidance. Source review separates existing gates/codec/arithmetic primitives from
  unsupported finality and semantic-validation claims; requirements R-19–R-23 track the additions.
- Evidence: source inspection and primary standards references; no runtime or performance measured.
  The initial editing pass was interrupted by an automatic approval-review usage limit, then resumed
  on the user's instruction. Final storage research was completed locally after the parallel review
  was also interrupted by that limit.
- Human input needed: none this step. Published vocabulary bundles, rates, and release thresholds
  are implementation/community decisions rather than silently assumed production defaults.
- Next: complete core/Q42 reuse and the documentation validation pass.

## 2026-09-06 — Core/Q42 integration — done

- Added `core-storage-and-cache.md` and integrated core reuse into architecture, contracts, QResolve,
  operations, and work packages. R-24 tracks exact signed bytes, scoped caches, bounded generations,
  and recoverable accounting over existing core storage.
- Read the Cloudflare cache article and compared its contiguous record/storage techniques with
  actual Q42 volume, lexicon, range/index/cursor, publication, WAL, and graph-index code.
- Evidence: code inspection only. Cache speed, memory savings, and attributable energy were not
  measured; Cloudflare results are not presented as QualiaDB results.
- Human input needed: none this step.
- Next: validate local links/anchors, Markdown structure, the JSON-LD example, and final diff scope.

## 2026-09-06 — Documentation verification — done

- Checked all 18 Markdown documents: 105 local links, six linked heading anchors, and one JSON
  authoring example; no missing targets/anchors, unbalanced fences, trailing whitespace, conflict
  markers, or invalid JSON were found. Checked the worked-example arithmetic.
- `git diff --check` passed for the fabric directory. Reviewed the architecture/wire/registry
  integration and requirement/work-package links. Four new documents and changes to 12 existing
  fabric documents constitute this revision; unrelated concurrent work was preserved.
- This is documentation verification, not CBOR-LD/RDF semantic conformance, a Rust test run, a
  deployed payment integration, or a performance/energy benchmark. Future implementation tests
  and unverified adapter/storage guarantees remain explicitly identified in the specification.
- Human input needed: none for this documentation task.
- Next: implementation can proceed against the recorded work packages and acceptance criteria;
  no implementation or publication was performed as part of this review.

## 2026-09-06 — Qualia peer runtime design — done

- Extending QDNF into an embeddable alternative to libp2p: the proposed Qualia Peer Runtime,
  application service contracts, deterministic bounded execution, and semantic replication.
- Reviewed the present libp2p wrapper and the upstream protocol and resource-manager specifications.
  libp2p already supports modular transports, secure streams, resource scopes, and defensive gossip;
  the proposed difference is integrated QualiaDB semantics, Q42 persistence, and resource governance.
- Independent source inspection checked existing subscription, replication, and scheduler primitives.
  Added the runtime architecture, API ownership contract, semantic service profiles and P10–P13.
- A subsequent semantic review identified reliable-carrier endpoint/migration constraints,
  authenticated conflict handling, durable applied resume cursors and projection provenance.
  The design now specifies each explicitly; the reviewer made no file edits.
- Human input needed: none for this design. Names and capacities are draft choices; implementation
  and measured performance remain separate work.
- Next: integrate the user's additional PQ and Q42-modality direction, then validate the document set.

## 2026-09-06 — Independent replacement, PQ security and Q42 modality — done

- Clarified the target as a new independent implementation replacing libp2p, with any migration
  carrier in a separate optional package. Production acceptance requires operation without libp2p.
- Added the PQ design and P14 using inspected ML-KEM/ML-DSA/SLH-DSA libraries and current NIST/IETF
  references. Hybrid establishment, dual controller/record proofs, typed SHA-384 commitments,
  downgrade resistance, bounded bootstrap and key/evidence lifecycle are explicit proposed work.
- Added the Q42 networking modality and P15; updated the canonical format draft and ADR 0008's
  inaccurate approximate byte split. Current physical layout is 40 non-parity + eight parity bytes;
  a 60-bit index is one field interpretation. A true 42+6 encoding remains a separate format decision.
- Recorded the four-field FrameLayout versus five-field NQuin persistence checksum mismatch and
  existing metadata role overlaps. No runtime code, ABI, stored records or cryptographic primitive
  implementation was changed; source comments still need the tracked code-owner reconciliation.
- Human input needed: none for the design. Exact protocol/schema vectors and independent crypto
  review remain implementation release gates, not claims of completed conformance.
- Next: verify links/anchors, document structure, arithmetic, diff scope and final design consistency.

## 2026-09-06 — Replacement design verification — done

- Checked 25 Markdown documents: the 22-document fabric directory, new Q42 networking modality,
  canonical Q42 format draft and corrected FrameLayout ADR. All 187 local links and 14 linked
  heading anchors resolve. Fences/tables and the existing JSON example passed structural checks.
- Verified the 42 MiB partition sum, 384-bit Quin layouts, 40-to-42-byte comparison and selected
  PQ key-share/signature byte totals. Scoped `git diff --check` passed. Fixed a requirements-table
  separator found by the documentation check.
- The independent reviewer confirmed all four scoped semantic/lifecycle findings were resolved.
  Source inspection and current primary crypto/libp2p references support the design; no performance,
  crypto security proof, RDF/CBOR-LD semantic conformance or compiled API test result is implied.
- Added five design documents for the new replacement work (runtime, API, semantic services, PQ,
  and Q42 networking modality), integrated P10–P15, and preserved unrelated concurrent changes.
  Earlier commons/contracts/core-storage work remains integrated. No runtime code, dependency lock,
  stored data, ABI, commit, push or deployment was changed by this design task.
- Human input needed: none for completion of the requested design/specification update.
- Next: implementation follows the explicit work packages and release gates; parity reconciliation,
  exact schema/wire vectors and independent cryptographic review remain tracked implementation work.

## 2026-09-06 — Purpose-specific format, networking cell and compute — in progress

- The user authorized a purpose-specific networking format where it fits better than Q42, review of
  memory boundaries and a separate networking cell, and explicit compute accounting alongside time
  and energy. This revises the earlier all-Q42 storage/default single-runtime budgeting assumption.
- Direction: an immutable QNF network artifact format under the existing core storage lifecycle,
  Q42 semantic projections, and a separately budgeted network cell with bounded Sentinel calls.
  Compute is typed work with declared semantics and evidence, not an assumed universal scalar.
- Independent source inspection is checking the P64/10D and runtime-cell precedents; the main work
  defines format layout, ownership/publication, cell supervision and resource dimensions.
- Evidence: source/standards review only; no speed, memory saving or energy saving measured.
- Human input needed: none for this design revision; the user's direction authorizes the design choices.
- Next: write the format/cell/compute specifications, reconcile prior requirements and validate them.
