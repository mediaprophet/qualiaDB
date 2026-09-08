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

## 2026-09-07 — Semantic roles, evidence, Identifier Fabric and Webizen — design complete

- Incorporated the user's clarified ordinary 512 MiB multi-cell model, explicit exceptional workload
  category, dedicated router/provider purposes and built-in economic accounts. Semantics lead the
  design; byte layouts, quantity profiles and runtime sketches remain candidate realizations.
- Added typed compute, semantic network roles, electronic evidence retention, Identifier Fabric
  integration and a source-backed Webizen/core memory review. Extended work packages through P21
  and traced requirements R-29–R-34; reconciled prior Q42-only and whole-network 42 MiB assumptions.
- Reviewed the Identifier Fabric brief and linked written material from remote-ref commit
  6601dc811468d266f4ac952243ce65fb10372606 without changing branches or importing its WIP files.
  The integration records exact sections read. Diagram image fetches failed; only the crosswalk was
  reviewed. The consultation is not represented as visually or implementation-complete.
- Webizen is the Sentinel. Code establishes a 42 MiB slot buffer with hash-selected replacement
  and separate recent-slot ring. VM callers/RuleEngine own arenas; inspected worker cells do not
  establish one arena each or enforce their 512 MiB declarations. Vec construction/rule growth,
  cache scope, parity and bounded result completeness remain explicit integration requirements.
- Large persisted graph/network datasets use the same core range/cursor substrate; memory limits
  describe working state, not total ontology size. Enterprise scaling uses admitted cells, stable
  flow ownership and bounded partition/reduction/continuation, without claiming linear performance.
- QNF remains a candidate. Fixed a possible cross-artifact hash cycle and repeated-binding ambiguity;
  acknowledged full index-scan cost for partial reads. New formats must demonstrate benefit over
  existing core storage rather than follow merely from dataset or proof size.
- Evidence design includes balanced selection, protected dependencies, hold/GC ordering, lossless
  originals, custody, clock/authority uncertainty, crypto renewal and offline examination. No hash
  replaces deleted data or proves guilt; access grants and attribution remain separate claims.
- The earlier write was rejected by automatic approval review at an account usage limit; no alternate
  execution bypass was used. Work resumed after the user's continuation. Focused documentation
  contributions were read and integrated; a later reviewer pass hit its usage limit, so final
  consistency review remains the parent agent's work, not a claimed completed independent review.
- No runtime code, memory constant, ABI, dependency, commit, push, deployment or external message
  was changed by this documentation task. Unrelated concurrent workspace changes are preserved.
- Next: finish document/link/arithmetic validation and record its actual outcome below.

## 2026-09-07 — Requested ring-buffer and enterprise code audit — done

- Followed the user's clarification that Webizen is the Sentinel. Inspected VM borrowing, arena
  construction/writes, RuleEngine ownership, local and daemon WorkerCells, shard spawning and Q42
  range/manifest/reader/sorter paths. A focused independent caller/large-volume audit completed;
  parent inspection confirmed the principal findings in code.
- The 42 MiB buffer is per arena instance, not proven per cell. Hash-slot overwrite differs from
  FIFO retention; context-free table keys cannot authorize private network operations. Both worker
  abstractions declare 512 MiB without establishing enforced aggregate accounting or arena ownership.
- Existing paged/cross-segment reads and joins are real. Added explicit work quanta for sparse scans,
  manifest metadata budgets, rejection of whole-graph fallback based on a small root-file size,
  and the current external sorter's 48,000,000-byte slot buffer to the integration review.
- Defined enterprise scaling through admitted ordinary cells and bounded core partitions; no network
  format change is required by ontology size. All runtime repairs remain tracked implementation
  requirements; this authorized review/design task changed documentation only.
- Initial validation found one stale economics-source link after a concurrent library refactor;
  inspected its new model owner and corrected the reference without changing concurrent code.

## 2026-09-07 — Integrated documentation validation — passed

- Checked 32 Markdown documents: 28 fabric-directory documents, the Q42 format and networking
  modality, QNF candidate and existing FrameLayout ADR. All 327 local links and 20 linked heading
  anchors resolve. Fence/table structure and the JSON authoring example passed the checks used.
- Verified the seven pinned consultation source files exist in the reviewed Git commit. No external
  image availability or complete SHACL/ontology conformance is implied by these link checks.
- Checked arena/cell byte counts, 192 MiB and 2 GiB host examples, QNF descriptor/header sums,
  evidence storage, compute examples, existing Quin/PQ arithmetic and scoped `git diff --check`.
- Source inspection and the independent large-volume/caller audit support the recorded findings.
  No Rust tests, allocation/RSS measurement, large-graph benchmark, crypto proof or legal-compliance
  evaluation was run for these documentation-only edits.
- Requested design and code review are complete. Runtime repairs and format/schema freeze remain
  explicitly specified future implementation work; no user approval or deployment action is pending.

## 2026-09-07 — Comprehensive swarm implementation programme — complete

- Added [qdnf-imp](./qdnf-imp/README.md): eight workstreams, 30 dependency-ordered packages and
  418 individually numbered to-do items, with a JSON register, single-purpose library rules,
  ownership/claim protocol, review and evidence gates, milestones and reusable handoff templates.
- Core/Webizen, native replacement, PQ integration, semantics, provider economics, multi-cell
  scaling and electronic evidence remain coordinated through explicit domain interfaces and
  source-audit repair tasks. QNF adoption remains a measured format decision.
- Final structural validation passed 72 dependency edges, all child IDs, 164 local links and
  P0–P21 domain-owner coverage. Roadmap/register dependencies and whitespace checks passed.
- The account usage limit interrupted the final validator write after 19 files were saved.
  Following the user's follow-up, confirmed those files and completed the same write and checks;
  all 20 plan files are now saved. Detailed outcomes are in the programme progress log.
- Focused agent contributions completed; their later independent integration reviews hit the usage
  limit and did not complete. Parent consistency review and executable structural checks completed.
- This delivers the implementation plan, not its runtime execution. All implementation work remains
  pending; no new runtime, deployment, commit or push is claimed.

## 2026-09-07 — Purpose-designed replacement for Kademlia — designed

- Added [Qualia Scoped Rendezvous](./qualia-scoped-rendezvous.md) and
  [evaluation/worked traces](./qsr-evaluation.md). The target lookup uses scoped semantic/exact
  lanes, authenticated radix coverage, core posting indexes and admitted query execution.
- Defined bootstrap, publication/withdrawal, snapshot completeness levels, bounded traversal,
  private tokens, epoch conflicts, hot-key replication and fenced multi-cell handover.
- Independent review identified and informed corrections for index omission claims, circular
  bootstrap/commitments, checkpoint visibility and acknowledged-write loss during migration.
- Integrated QSR into target routing/resolution and added NET-04.15–30. The programme now has
  434 checks; historical libp2p/Kademlia evidence remains distinct from the replacement design.
- Comparison requirements preserve Kademlia's asynchronous queries, caching and equivalent semantic
  indexes, and charge QSR's bootstrap/maintenance/proof costs. Better performance remains a
  workload-specific hypothesis; no implementation, benchmark or universal superiority is claimed.

## 2026-09-07 — Humanitarian compensation and protection requirements — updated

- Added [finite project compensation](./finite-project-compensation.md) and
  [socially defined protection](./socially-defined-protection.md) as explicit application profiles.
  Existing generic threshold release and relationship controls covered only part of the user goals.
- Compensation now specifies free personal/humanitarian eligibility, incorporated-principal
  contribution duties under applicable instruments, a positive allocated recovery component above
  event costs/fees, finite shared caps, final-payment races, contributor allocation and terminal release.
- Protection covers private child/PEP discovery, separate contact consent, nontransitive roles,
  abusive-guardian help/recovery, private evidence, and unavailable-freshness handling for offline delivery.
- Added 46 pending checks; the plan now contains 480 across the same 30 packages. Independent review
  informed corrections for authorized waivers, zero targets and safeguards controlled by an alleged abuser.
- The account usage limit interrupted the final correction write. After continuation, confirmed saved
  work and completed the same write. Runtime implementation remains pending; no production protection,
  settlement or legal sufficiency is claimed by this documentation update.
