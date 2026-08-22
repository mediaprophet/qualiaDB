# Save Architecture — Q42 Checkpoints, Bifurcation, Provenance, and Pruning

> *"A workshop, not a webpage. A socket set, not a menu bar."*

**Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.**
**Principal / inventor:** Timothy Charles Holborn <timothy.holborn@gmail.com>

## 1. Problem

The current `save_all_manifolds()` function is a flat snapshot: it serializes
all `ManifoldSeed`s to base64 CBOR-LD and writes them to a single
`localStorage` key. This model has no:

- **Actor identity** — who saved?
- **Provenance graph** — what changed since the last save?
- **Bifurcation** — no branches or forks
- **Checkpoint metadata** — no name, no label, no purpose
- **Save mode selection** — one mode for everything
- **Pruning or archiving** — history grows unbounded
- **Streaming for collaboration** — no CRDT operations, no actor tags
- **Cryptographic integrity** — no Merkle root, no Bao hash
- **Constituency / consent tracking** — whose data is in this artifact?
- **Derivative chain** — no ancestry from original to current

The Q42 file format specification already defines a Merkle-CRDT DAG with
Nquins (`<<[ s p o g prov ]>>`) — every triple carries its provenance graph
inline. The provenance ontology (`ontologies/provenance.n3`) defines
contribution roles, derivative chains, constituencies, and credits. The
agency ontology (`ontologies/agency.n3`) distinguishes natural persons,
software entities, and collective entities. This document specifies how the
UI's save system should align with those existing specifications.

---

## 2. Save Modes

| Mode | Trigger | What it captures | Use case |
|------|---------|------------------|----------|
| **Auto** | Frequency-based (e.g. every 60s or on change) | Current canvas state + delta from last auto-save | Prevent data loss during work |
| **Checkpoint** | User-named save (Ctrl+Shift+S) | Full state + named label + actor + timestamp | "v0.3 draft", "before NLP extraction", "review copy" |
| **Stream** | Continuous (collaborative mode) | Individual operations as CRDT ops with actor tags | Real-time multi-agent collaboration |
| **Snapshot** | Manual export | Full state + complete provenance graph + derivative chain | Archival, distribution, publishing |
| **Pruned** | User-initiated | Active state only, tombstones pruned, new epoch hash | Production/distribution version |

### 2.1 Auto save

- Fires on a configurable interval (default: 60 seconds) and/or on
  significant state changes (container placed, deleted, moved, wired).
- Writes to a rolling buffer of auto-checkpoints (default: 5 retained).
- Does not prompt the user.
- Shows a subtle "saved" indicator in the status bar.

### 2.2 Checkpoint save

- User provides a label (e.g. "v0.3 draft", "before NLP extraction").
- Records the actor, timestamp, parent checkpoint, and operations since
  parent.
- Visible in the Checkpoint History view.
- Can be restored, branched from, or exported.

### 2.3 Stream save

- Used when collaborative mode is active.
- Each UI operation (place, delete, move, resize, wire, edit, annotate)
  is emitted as a CRDT operation with actor identity.
- Operations are tagged with `contribution_role` and `confidence`.
- The Merkle-CRDT DAG is updated incrementally.
- Conflicting operations from different actors are resolved by the CRDT
  merge rules (last-writer-wins for layout, semantic merge for content).

### 2.4 Snapshot save

- Captures the full state plus the complete provenance graph and
  derivative chain.
- Used for archival — the snapshot is a complete record of everything.
- May be exported as an `.hmc` archive (HMC container with Bao streaming).
- Snapshots are immutable — they cannot be edited, only branched from.

### 2.5 Pruned save

- Consolidates tombstones (deletion records) into state-based delta
  snapshots.
- Computes a new convergent Merkle root for the active state.
- Prunes historical operational tombstones while maintaining
  cryptographic provenance of the active graph.
- Used to produce a compact working or distribution version.
- The pruned history is archived before pruning (see §5).

---

## 3. Checkpoint data structure

```rust
/// A single checkpoint in the bifurcation tree.
pub struct Checkpoint {
    // --- Identity ---
    /// Unique identifier (UUID or content-hash-based).
    pub id: String,
    /// User-provided name (e.g. "v0.3 draft").
    pub label: String,

    // --- Content ---
    /// Full manifold seed state at this checkpoint.
    pub seeds: Vec<ManifoldSeed>,

    // --- Provenance ---
    /// The actor who created this checkpoint.
    pub actor: ActorRef,
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Previous checkpoint in this branch (forms a chain).
    pub parent_checkpoint: Option<String>,
    /// What changed since the parent (delta operations).
    pub operations_since_parent: Vec<Operation>,

    // --- Integrity ---
    /// BLAKE3 Merkle root over the CBOR-LD content (Q42 epoch hash).
    pub merkle_root: Option<String>,
    /// Content hash of the serialized seeds.
    pub content_hash: String,

    // --- Visibility / rights ---
    /// Who can see this checkpoint.
    pub visibility: CheckpointVisibility,
    /// Affected parties (data subjects, rights holders, stakeholders).
    pub constituency: Vec<ConstituencyRef>,
    /// Consent state per constituency.
    pub consent_state: ConsentState,

    // --- Bifurcation ---
    /// Which branch this checkpoint is on.
    pub branch_id: String,
    /// If this branch was forked from another, the parent branch.
    pub branch_parent: Option<String>,
    /// Save mode that produced this checkpoint.
    pub save_mode: SaveMode,
}

/// How a checkpoint was created.
pub enum SaveMode {
    Auto,
    Checkpoint,
    Stream,
    Snapshot,
    Pruned,
}

/// Who can access a checkpoint.
pub enum CheckpointVisibility {
    /// Only the actor who created it.
    Private,
    /// Named collaborators.
    Collaborators,
    /// Constituency members (data subjects, rights holders).
    Constituency,
    /// Anyone with access to the repository.
    Public,
    /// Visible but with watermark / traceability markers.
    Watermarked,
}

/// A single operation recorded in a checkpoint's delta.
pub struct Operation {
    /// What kind of operation.
    pub op_type: OpType,
    /// Who performed it.
    pub actor: ActorRef,
    /// When it happened (ISO 8601).
    pub timestamp: String,
    /// Target element (container ID, wire ID, etc.).
    pub target: String,
    /// State before the operation (None for creation).
    pub before: Option<String>,
    /// State after the operation (None for deletion).
    pub after: Option<String>,
    /// Provenance role (author, editor, extractor, annotator, etc.).
    pub contribution_role: ContributionRole,
    /// Confidence score (1.0 for human, 0.0–1.0 for agent).
    pub confidence: f32,
}

/// Types of operations that can be recorded.
pub enum OpType {
    PlaceContainer,
    DeleteContainer,
    MoveContainer,
    ResizeContainer,
    DrawWire,
    DeleteWire,
    EditContent,
    Annotate,
    Transform,
}

/// Reference to an actor (from the agency ontology).
pub struct ActorRef {
    /// DID or IRI identifying the entity.
    pub entity: String,
    /// Entity type (NaturalPerson, SoftwareEntity, etc.).
    pub entity_type: EntityType,
    /// The circumstance in which the actor acted (optional).
    pub circumstance: Option<String>,
}

pub enum EntityType {
    NaturalPerson,
    LegalPerson,
    SoftwareEntity,
    CollectiveEntity,
}

/// Reference to a constituency (from the provenance ontology).
pub struct ConstituencyRef {
    /// IRI of the constituency.
    pub iri: String,
    /// Type (dataSubject, rightsHolder, stakeholder, audience, community).
    pub constituency_type: String,
    /// Whether consent is required.
    pub consent_required: bool,
}

/// Aggregate consent state across all constituencies.
pub enum ConsentState {
    /// No constituencies require consent.
    NotRequired,
    /// Consent is pending for one or more constituencies.
    Pending,
    /// All required consents have been granted.
    Granted,
    /// One or more consents have been denied.
    Denied,
}
```

---

## 4. Bifurcation model

```
main ──── cp1 ──── cp2 ──── cp3 ──── cp4 (current)
                          │
                          └── branch: nlp-extraction
                              cp2.1 ──── cp2.2 (agent-annotated)
```

Each branch is a divergent line of work. A branch:

- **Forks** from a checkpoint on another branch, recording
  `branch_parent`.
- **Merges** back into its parent branch, producing a merge checkpoint
  that combines both contribution graphs.
- **Abandons** (tombstoned but retained in history for provenance).
- **Publishes** as a derivative, creating a `prov:Transformation` of
  type `flatten` or `render` with the output artifact being the
  published version.

### 4.1 Branch metadata

```rust
pub struct Branch {
    /// Unique branch identifier.
    pub id: String,
    /// Human-readable branch name (e.g. "nlp-extraction").
    pub label: String,
    /// Parent branch (None for the main branch).
    pub parent: Option<String>,
    /// Checkpoint where this branch forked from its parent.
    pub fork_point: String,
    /// Whether this branch is active, merged, or abandoned.
    pub state: BranchState,
    /// Actor who created this branch.
    pub created_by: ActorRef,
    /// Timestamp of branch creation.
    pub created_at: String,
}

pub enum BranchState {
    Active,
    Merged,
    Abandoned,
    Published,
}
```

### 4.2 Merge semantics

When branch B merges into branch A:

1. A new checkpoint is created on branch A with `parent_checkpoint`
   pointing to A's current head.
2. The merge checkpoint's `operations_since_parent` includes all
   operations from B that were not already in A.
3. Each operation retains its original `actor` and `contribution_role`.
4. Conflicting operations are resolved by CRDT merge rules:
   - Layout (position, size): last-writer-wins by timestamp.
   - Content (text, formulas): semantic merge (union of non-conflicting
     edits; conflicting edits flagged for human review).
   - Wires: union (both connections exist after merge).
   - Deletions: tombstone-based (a deletion in one branch does not
     resurrect a deleted container in the other).

---

## 5. Pruning and archiving

| Action | What it does | When to use |
|--------|-------------|-------------|
| **Prune tombstones** | Consolidate deletion records into state-based delta, compute new Merkle root | After frequent edits/retractions; produces a compact working version |
| **Archive history** | Export complete provenance graph + all checkpoints as an `.hmc` archive | Before pruning, to preserve the full audit trail |
| **Create distribution** | Export pruned state + credits + constituency consent state as a watermarked `.q42` | Publishing to external audience |
| **Strip metadata** | Remove provenance graph, constituency data, and cryptographic keys | For privacy-sensitive distribution (with fiduciary authorization) |

### 5.1 Prune tombstones

1. Identify all tombstoned (deleted) containers, wires, and annotations.
2. Remove their operational records from the active state.
3. Compute a new convergent Merkle root for the pruned active state.
4. The pruned state is a new checkpoint with `save_mode: Pruned`.
5. The pre-prune state is archived (see §5.2) before pruning.

### 5.2 Archive history

1. Serialize the complete provenance graph (all checkpoints, all
   operations, all branches) to CBOR-LD.
2. Package as an `.hmc` archive with Bao streaming for verified
   random-access.
3. The archive includes:
   - All checkpoints (full state + delta + provenance).
   - All branches (including abandoned).
   - All constituency and consent records.
   - All cryptographic keys and hashes.
4. The archive is immutable and can be stored locally or uploaded to
   the daemon.

### 5.3 Create distribution

1. Start from a pruned checkpoint.
2. Generate `prov:Credits` from the provenance graph (human-readable
   summary of all contributors, sources, and transformations).
3. Check `consent_state` — all required consents must be `Granted`.
4. Apply watermarking if `visibility: Watermarked`.
5. Export as a `.q42` file with:
   - Pruned active state.
   - Credits.
   - Derivative chain (from original to this version).
   - Constituency consent records.
   - Merkle root for integrity verification.

### 5.4 Strip metadata

1. Remove the provenance graph, constituency data, and cryptographic
   keys from the checkpoint.
2. This requires `fiduciary` authorization (a `prov:ContributionRole`
   of `fiduciary` from an authorised actor).
3. The stripped version retains only the active state and a minimal
   derivative chain (original → stripped).
4. Used for privacy-sensitive distribution where revealing contributor
   identity would endanger data subjects.

---

## 6. UI implications

### 6.1 File menu

```
File
 ├─ Save                    → last-used mode (default: Auto)
 ├─ Save As…                → opens Save Mode dialog
 │   ├─ Mode: [Auto | Checkpoint | Snapshot | Pruned]
 │   ├─ Label: [_______________]
 │   ├─ Visibility: [Private | Collaborators | Public | Watermarked]
 │   ├─ Branch: [main ▾] or [New branch…]
 │   └─ Constituency: [data subjects…] [rights holders…]
 ├─ Checkpoint History…     → shows checkpoint tree with branches
 ├─ Prune & Archive…        → prune tombstones + export archive
 ├─ Export Distribution…    → pruned + watermarked .q42
 └─ Import…                 → load checkpoint or branch
```

### 6.2 Status bar / telemetry

The status bar and telemetry sidebar should show:

- **Current branch** name
- **Last checkpoint** timestamp + actor
- **Unsaved operations** count (operations since last checkpoint)
- **Merkle root** (abbreviated, e.g. `blake3:7f3a…`)
- **Active constituencies** + consent state
- **Auto-save** indicator (next save in Ns)

### 6.3 Checkpoint History view

A modal or sidebar showing the bifurcation tree:

- Each checkpoint is a node with label, actor, timestamp.
- Branches are shown as divergent lines.
- Clicking a checkpoint shows its details (operations, provenance,
  constituency).
- Actions per checkpoint: Restore, Branch from here, Export, Compare
  with current.

### 6.4 Save Mode dialog

A modal dialog for "Save As…" that lets the user choose:

- **Mode**: Auto, Checkpoint, Snapshot, Pruned
- **Label**: text input (required for Checkpoint and Snapshot)
- **Visibility**: dropdown
- **Branch**: dropdown of existing branches + "New branch…" option
- **Constituency**: multi-select of known constituencies + "Add…"
- **Consent check**: if any constituency requires consent, show
  "Consent pending — publish will be blocked until granted"

---

## 7. Relationship to existing ontologies

### 7.1 Provenance ontology (`ontologies/provenance.n3`)

| Checkpoint field | Ontology mapping |
|------------------|------------------|
| `actor` | `prov:actor` → `agency:Actor` |
| `operations_since_parent` | `prov:hasEntry` → `prov:Contribution` |
| `contribution_role` | `prov:ContributionRole` (author, editor, extractor, etc.) |
| `constituency` | `prov:Constituency` (dataSubject, rightsHolder, etc.) |
| `consent_state` | `prov:consentRequired` + consent tracking |
| `parent_checkpoint` | `prov:derivedFrom` |
| `operations` with `OpType::Transform` | `prov:Transformation` |

### 7.2 Agency ontology (`ontologies/agency.n3`)

| ActorRef field | Ontology mapping |
|----------------|------------------|
| `entity` | `agency:NaturalPerson` / `agency:SoftwareEntity` etc. |
| `entity_type` | `agency:EntityType` |
| `circumstance` | `agency:Circumstance` |

### 7.3 Q42 file format (`TechDesign/FileFormat.md`)

| Checkpoint field | Format mapping |
|------------------|----------------|
| `merkle_root` | Q42 epoch hash (convergent Merkle root) |
| `content_hash` | BLAKE3 content hash |
| `operations_since_parent` | CRDT operations in the Merkle-CRDT DAG |
| Pruned save | Tombstone pruning + epoch hashing |
| Snapshot save | Full `.q42` with complete provenance graph |

### 7.4 Investigation ontology (`ontologies/investigation.n3`)

| Operation field | Ontology mapping |
|------------------|------------------|
| `op_type` | `inv:ContributionType` (add, update, link, revise, etc.) |
| `actor` | `inv:contributionAgent` |
| `timestamp` | `inv:contributionTimestamp` |

---

## 8. Implementation phases

### Phase 1 — Minimal provenance (current)

- `save_all_manifolds()` records actor identity (default:
  `did:qualia:timothy_charles_holborn`) and timestamp.
- Checkpoints are stored as a linear chain in localStorage.
- File menu "Save" opens a Save Mode dialog (most modes show "present,
  engine wiring pending").

### Phase 2 — Checkpoint chain

- Checkpoint data structure implemented in Rust.
- Checkpoint History view shows the linear chain.
- Named checkpoints with labels.
- Restore to any checkpoint.
- Branch from any checkpoint.

### Phase 3 — Bifurcation

- Branch data structure.
- Fork, merge, abandon operations.
- Bifurcation tree visualization.
- CRDT merge rules for conflicts.

### Phase 4 — Pruning and archiving

- Tombstone pruning.
- Archive export (`.hmc` with Bao streaming).
- Distribution export (`.q42` with credits and consent).
- Metadata stripping (with fiduciary authorization).

### Phase 5 — Streaming collaboration

- CRDT operation stream.
- Real-time multi-agent editing.
- Actor-tagged operations.
- Conflict resolution UI.

### Phase 6 — Cryptographic integrity

- BLAKE3 Merkle root computation.
- Bao verified streaming.
- Content hash verification on load.
- Watermarking for distribution.

---

## 9. Honesty labels

| Feature | Status |
|---------|--------|
| Flat snapshot save to localStorage | `live` |
| Actor identity on save | `live` (Phase 1) |
| Timestamp on save | `live` (Phase 1) |
| Save Mode dialog | `present` (Phase 1 — UI exists, most modes pending) |
| Named checkpoints | `missing` (Phase 2) |
| Checkpoint history view | `missing` (Phase 2) |
| Branching / bifurcation | `missing` (Phase 3) |
| CRDT merge | `missing` (Phase 3) |
| Tombstone pruning | `missing` (Phase 4) |
| Archive export | `missing` (Phase 4) |
| Distribution export with credits | `missing` (Phase 4) |
| Metadata stripping | `missing` (Phase 4) |
| Streaming collaboration | `missing` (Phase 5) |
| BLAKE3 Merkle root | `missing` (Phase 6) |
| Bao verified streaming | `missing` (Phase 6) |
| Watermarking | `missing` (Phase 6) |

---

## 10. Credential-Conditional Rendering

The same artifact renders differently for different consumers — not because
the text changes, but because the **context graph** (annotations, inferences,
links) is filtered by the consumer's credentials.

### 10.1 Presentation context extension

The `PresentationContext` in `ontologies/presentation.n3` currently
negotiates modality, form factor, accessibility, locale, and hardware tier.
It must be extended with a **credential context**:

```n3
pres:credentialContext a rdf:Property ;
    rdfs:label "credential context" ;
    rdfs:domain pres:PresentationContext ;
    rdfs:range  set:Capability ;
    rdfs:comment "The capabilities held by the current viewer. Determines
    which content, context markup, and inference links are visible." .
```

### 10.2 Rendering pipeline

1. **Load the artifact** (Q42 graph with all triples, markup, provenance)
2. **Resolve the viewer's credentials** (capabilities, access control
   policies, conditions)
3. **Filter the graph** — remove triples, markup nodes, and inference
   links that the viewer's credentials don't permit
4. **Negotiate the presentation context** — modality, form factor,
   accessibility, locale, hardware tier, **plus credential context**
5. **Render** — only show what the filtered graph and negotiated context
   permit

### 10.3 What gets filtered

| Content layer | Filter mechanism | Example |
|---------------|-----------------|---------|
| **Context markup nodes** | `doc:appendScope` (authorOnly, contributors, audience, public) | A non-author cannot see `authorOnly` markup |
| **Inference links** | Capability `context-markup:read` + confidence threshold | A viewer without the capability sees the link but not the confidence score, or doesn't see the link at all |
| **Provenance graph** | Capability `provenance:read` | Without it, viewer sees credits (human-readable summary) but not the full graph |
| **Constituency data** | Viewer's relationship to the constituency | A data subject sees their own data; a rights holder sees rights info; others see only public data |
| **Cryptographic keys** | Capability `crypto:key:read` | Without it, viewer sees only the watermarked content |
| **Watermarks** | Always visible (embedded in content) | All viewers see watermarks; only fiduciary can strip them |

### 10.4 Context markup language integration

The context markup system (`ontologies/document.n3` §4–6) is the mechanism
by which inference links change based on credentials:

1. **Author writes a document** — text is stored as plain content.
2. **NLP agent extracts entities and facts** — each extraction creates a
   `doc:ContextMarkup` node with:
   - `markupType` (entity, claimedFact, etc.)
   - `markupSpan` (byte range in the document)
   - `linksTo` (IRI of the knowledge graph node)
   - `hasProvenance` (who extracted it, when, with what confidence)
   - `appendScope` (who can see this markup)
3. **Consumer views the document** — the rendering pipeline:
   - Resolves the consumer's credentials
   - Filters markup nodes by `appendScope` and the consumer's capabilities
   - Renders visible markup as annotations (underlines, tooltips, side
     panels)
   - Hides markup that the consumer can't see
4. **Consumer with append capability adds their own markup** — their
   annotations are appended with `appendScope: audience` or
   `appendScope: public`, visible to others based on their credentials

---

## 11. Workflow Container Types

The save/publication/credential workflow requires specialized containers
that surface these functions on the manifold. These are **panel** and
**widget** containers per `ontologies/container.n3` §4–5.

### 11.1 Panel containers

| Container type | Purpose | Ontology basis |
|---------------|---------|----------------|
| `checkpoint-tray` | Shows checkpoint history as a vertical timeline with branch points. Click to restore. Branch from here. Shows actor, timestamp, save mode, label. | SAVE_ARCHITECTURE.md §3–4 |
| `credential-inspector` | Shows the current viewer's capabilities, access control policies, and conditions. Highlights granted, suspended, revoked, pending. Shows what content is visible vs hidden. | `ontologies/settings.n3` §5–6 |
| `context-markup-editor` | Edits the `ContextGraph` of the active document. Shows markup nodes (term, entity, claimedFact, etc.), links to sources, append scopes, temporal status. | `ontologies/document.n3` §4–6 |
| `provenance-panel` | Shows the `ProvenanceGraph` — contributors, roles, sources, transformations, derivative chain, credits. | `ontologies/provenance.n3` |
| `publication-workflow` | The save/publication workflow as an inline panel: choose mode, set visibility, select constituency, check consent, prune, archive, distribute, watermark, strip metadata. | SAVE_ARCHITECTURE.md §2, §5 |
| `constituency-manager` | Manages constituencies — data subjects, rights holders, stakeholders, audiences, communities. Tracks consent state per constituency. | `ontologies/provenance.n3` §8 |

### 11.2 Widget containers

| Container type | Purpose | Ontology basis |
|---------------|---------|----------------|
| `capability-badge` | Shows the capability scope of the active container or tool. Visual Sentinel indicator — green (active), yellow (suspended), red (revoked), grey (pending). | `ontologies/container.n3` §5 (container:CapabilityBadge) |
| `checkpoint-indicator` | Shows current branch + last checkpoint timestamp + unsaved operations count. Click to open checkpoint tray. | SAVE_ARCHITECTURE.md §6.2 |
| `consent-indicator` | Shows consent state — green (all granted), yellow (pending), red (denied). Click to open constituency manager. | `ontologies/provenance.n3` §8 |

### 11.3 Container kind field

`SeedContainer` now has a `kind` field (`content`, `panel`, `widget`)
that aligns with `container:ContainerKind` in `ontologies/container.n3`.
The kind is inferred from the container type at build time via
`ContainerKind::from_type()`.

---

## 12. References

- Q42 file format: `TechDesign/FileFormat.md` §1, §8
- Provenance ontology: `qualia-ui/ontologies/provenance.n3`
- Agency ontology: `qualia-ui/ontologies/agency.n3`
- Investigation ontology: `qualia-ui/ontologies/investigation.n3` (§Multi-agent contributions)
- HMC container format: `TechDesign/FileFormat.md` §1 (HCF/HMC)
- Bao verified streaming: `TechDesign/FileFormat.md` §8.A
- Merkle-CRDT state compaction: `TechDesign/FileFormat.md` §8.B

## 13. Saved search queries

The Search Workbench (`src/browser/search_workbench.rs`) allows users to save
search queries as persistent objects in `localStorage` under the key
`qualia-ui:saved-queries`.

### 13.1 Saved query object shape

```json
{
  "id": "q-<timestamp>",
  "name": "my-query",
  "mode": "faceted" | "builder" | "sparql",
  "query": "<query text or SPARQL>",
  "timestamp": "2026-08-18T12:34:56"
}
```

- `id` — unique, derived from `Date.now()`.
- `name` — user-supplied or auto-generated from mode + timestamp.
- `mode` — which workbench mode produced the query.
- `query` — for `faceted`: a comment header describing active facets plus a
  generic SELECT. For `builder`: the generated SPARQL text. For `sparql`: the
  raw editor contents.
- `timestamp` — ISO 8601 local time.

### 13.2 Persistence honesty

Saved query definitions are `live` (they really persist in localStorage and
survive reloads). Query **execution** is `present` (mock results only) —
actual SPARQL execution requires the QualiaDB daemon backend.

### 13.3 Query-as-container-source

A saved query can be placed on the canvas as a graph container via the
"Place" action in the Saved Queries tab, or the "Use as Container Source"
button in the Manual SPARQL tab. The placed container carries:

- `data-query` — the full query text
- `data-query-name` — the query name

This establishes the relationship between a saved query definition and a
materialised container. Refreshing/re-running the query against the backend
is engine-wiring pending.

### 13.4 Future integration with checkpoint model

When the save architecture matures to Phase 2+ (checkpoint chains), saved
queries should become first-class manifest entries rather than a separate
localStorage key, so that they participate in bifurcation, pruning, and
provenance tracking alongside other document state.
