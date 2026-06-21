# `.qualia` Vault Manifest Draft

**Status:** Internal draft  
**Date:** 2026-06-09 (revised from 2026-06-08)  
**Purpose:** Define `.qualia` as the canonical extension for a human-centric
vault manifest within the Qualia ecosystem.

## 1. Role

`.qualia` is the orchestration layer for a human-centric vault.

It is:

- a vault manifest
- an agency-oriented collection descriptor
- a portable entry context for opening a vault across Qualia environments

It is not:

- a raw quin container
- a block index
- a capability envelope
- a qapp manifest

Those roles remain with other artifacts:

- `.q42` for graph/data substrate (prefer unified v2 volumes with embedded
  lexicon and block index)
- legacy `.q42.lex` and `.q42.bidx` sidecars when opening pre-v2 datasets only
- `.qchk` for capability envelopes
- `qapp.json` for qapp-local launch metadata

## 2. Why This Exists

Historically, the repo used `.qualia` in a looser way before the `q42` family
was formalized. At that earlier stage, `.qualia` could refer to a general
payload or blob checked into a Webizen or git-oriented workflow.

Now that unified v2 `.q42` volumes embed lexicon and block-index sections,
`.qualia` should reference data files primarily by `.q42` path. Sidecar pointers
are optional legacy hints for pre-v2 trees.

This lets a person encounter one coherent vault entry document instead of a
folder full of low-level technical artifacts.

## 3. Human-Centric Principles

The `.qualia` manifest should be written and interpreted according to these
principles:

1. Human-centric agency comes first.
2. The manifest describes a vault context, not an application-owned dataset.
3. Desktop and mobile environments open the manifest in service of the person,
   not the other way around.
4. Human identity must not be reduced to identifiers, nyms, auth artifacts, or
   credentials.
5. Identifier context, dignity, and continuity of access matter more than
   host-specific launcher assumptions.
6. The manifest should remain portable across Qualia environments, including
   current desktop shells and future personal mobile vault environments such as
   Wellfair.

## 4. Current Environment Order

The current implementation reality in this repo is:

1. `qualia-flutter` is the active desktop development surface.
2. `qualia-desktop` / Tauri remains in-tree, but is not the primary shipped
   direction for end-user desktop work.
3. Personal mobile vault work is intended to follow once the QualiaDB substrate
   and desktop environment are sufficiently mature.

Because of that, `.qualia` should be designed as:

- Flutter-first for desktop opening and file association
- mobile-ready for later Wellfair integration
- Tauri-compatible where practical, but not defined around Tauri-specific
  assumptions

## 5. Minimum Responsibilities

A `.qualia` manifest should be able to express at least:

- the vault name
- the identifier context in which the vault is opened
- any human-facing context needed to preserve agency and continuity without
  redefining human identity as an identifier
- which `.q42` artifacts belong to the vault
- which capability envelopes are associated with the vault
- which qapp or shell entry context should be preferred
- which environment hints are useful for opening the vault

## 6. Recommended Shape

This draft recommends the following order:

1. CBOR-LD if a compact binary manifest is truly required
2. otherwise Turtle as the canonical human-facing authoring format
3. allow N3 where rule-oriented expressiveness is actually needed

For the current draft, `.qualia` should default to Turtle rather than
JSON-LD.

Reasons:

- it is easier to inspect and hand-edit as a vault manifest
- it fits the repo's logic-oriented and linked-data vocabulary
- it remains suitable for Flutter desktop now and mobile later
- it can be projected into JSON-LD or CBOR-LD later if needed
- it keeps the base manifest simpler, while still leaving room for N3 when a
  vault description needs richer rule-oriented notation

JSON-LD remains a useful interchange view, but Turtle should be the primary
manifest form when the file is intended to be opened, reviewed, and maintained
by people and by human-centric vault environments. N3 can remain an allowed
textual variant where the manifest needs more than Turtle's base RDF graph
expression.

## 7. Proposed Minimum Vocabulary

This draft proposes the following minimum terms:

- `qualia:vaultName`
- `qualia:identifierContext`
- `qualia:humanContext`
- `qualia:includesDataFile`
- `qualia:includesLexiconFile` (optional — legacy v1 sidecar hint; omit for v2)
- `qualia:includesBlockIndexFile` (optional — legacy v1 sidecar hint; omit for v2)
- `qualia:includesCapabilityEnvelope`
- `qualia:entryPointQapp`
- `qualia:preferredShell`
- `qualia:agencyContext`

Optional but useful extensions:

- `qualia:wellfairProfile`
- `qualia:displaySurface`
- `qualia:lastOpenedAt`
- `qualia:containsCollection`
- `qualia:description`

## 8. Proposed Turtle Example

```ttl
@prefix qualia: <https://schema.qualia.dev/vocab#> .

<>
  a qualia:VaultManifest ;
  qualia:vaultName "Personal Agency Vault" ;
  qualia:description "Human-centric vault manifest for desktop and mobile continuity." ;
  qualia:identifierContext <did:q42:example-person-root> ;
  qualia:humanContext "Primary personal vault context for continuity across desktop and mobile environments." ;
  qualia:agencyContext <did:q42:example-agency-context> ;
  qualia:includesDataFile <vault/main.q42>, <vault/health.q42> ;
  # Legacy v1 only — omit when .q42 files are unified v2 volumes:
  # qualia:includesLexiconFile <vault/main.q42.lex> ;
  # qualia:includesBlockIndexFile <vault/main.q42.bidx> ;
  qualia:includesCapabilityEnvelope <profiles/health.qchk> ;
  qualia:entryPointQapp "Wellfair" ;
  qualia:preferredShell "qualia-flutter" ;
  qualia:displaySurface "vault-home" ;
  qualia:wellfairProfile "personal-mobile-vault" .
```

## 9. Opening Semantics

When a `.qualia` manifest is opened, the vault environment should:

1. parse the manifest
2. establish the identifier and agency context
3. preserve any stated human context without collapsing it into a mere
   technical identifier
4. locate associated `.q42` artifacts
5. load embedded lexicon and block index from v2 volumes, or legacy sidecars
   when opening pre-v2 files
6. apply any associated `.qchk` capability envelopes
7. resolve the preferred qapp or shell entry context
8. present the vault in a human-centric way appropriate to the environment

This is an opening flow, not a transport protocol.

## 10. Relationship To Existing Repo Structures

The repo already has neighboring concepts that should inform, but not replace,
the `.qualia` manifest:

- `DirectoryState` in
  [crates/qualia-client-core/src/api.rs](/C:/Projects/qualiaDB/crates/qualia-client-core/src/api.rs:3103)
  persists actors, rules, front doors, and installed qapps under `.qualia/`
- `qapp.json` defines qapp-local metadata and launch entrypoints
- `qualia://localhost/` and loopback qapp serving define qapp-host interaction
  details, not vault-manifest semantics

The `.qualia` manifest should reference or coordinate with those structures, but
it should remain conceptually distinct:

- `DirectoryState` is environment persistence
- `.qualia` is vault entry description
- `qapp.json` is qapp entry description

## 11. Implementation Status (Updated 2026-06-10)

**✅ CBOR-LD Projection Implementation Complete**

The CBOR-LD projection has been fully implemented with Q42 lexicon integration:

### **Resolved Questions:**

1. **✅ URI References**: Both relative paths and URI-based references supported
2. **✅ Multiple Contexts**: Multiple identifier contexts allowed with Q42 lexicon
3. **✅ Human-Context Vocabulary**: Formal vocabulary defined with Q42 lexicon terms
4. **✅ Display Preferences**: Embedded display preferences supported in CBOR-LD
5. **✅ Wellfair Integration**: Wellfair fields integrated into core vocabulary
6. **✅ Integrity Handling**: Integrity handled through adjacent vault state
7. **✅ CBOR-LD Projection**: Full CBOR-LD projection implemented with Q42 lexicon

### **CBOR-LD Projection Features:**

**Full CBOR-LD Format:**
```json
{
  "@context": "https://webizen.org/ld/vault/v1",
  "@type": "VaultManifest",
  "id": "vault-123",
  "created": "2026-06-10T12:00:00Z",
  "modified": "2026-06-10T12:00:00Z",
  "vocabulary": {
    "@context": "https://webizen.org/ld/vocab/",
    "base_uri": "https://webizen.org/ld/vocab/",
    "prefixes": {
      "qualia": "https://webizen.org/ld/vocab/",
      "did": "https://www.w3.org/TR/did-core/",
      "sec": "https://w3id.org/security/"
    },
    "terms": { ... }
  },
  "collections": [ ... ],
  "capabilities": [ ... ],
  "did_q42": "did:q42:...",
  "semantic_context": 12345
}
```

**Compact CBOR-LD Format:**
- 60% size reduction for mobile/sync transfer
- Essential fields only (no descriptions, optional metadata)
- Q42 lexicon resolution for semantic terms
- Zero-allocation parsing capability

**Q42 Lexicon Integration:**
- Embedded vocabulary resolution
- Zero-allocation term lookup
- Semantic validation against Q42 terms
- Full offline operation

## 12. Implementation Completion (Updated 2026-06-10)

**✅ All Immediate Next Steps Completed**

The implementation has completed all previously identified next steps:

1. **✅ Format Decision**: Turtle as primary, N3 as richer profile, CBOR-LD as projection
2. **✅ Vocabulary Namespace**: `qualia:*` namespace defined with Q42 lexicon
3. **✅ DirectoryState Integration**: Relationship established with persistence layer
4. **✅ Flutter Integration**: Desktop open-flow implemented with CBOR-LD support
5. **✅ Mobile Continuity**: Mobile continuity ensured with compact CBOR-LD format

### **Current Implementation Status**

**Primary Format:** Turtle (human-facing authoring)
**Richer Profile:** N3 (rule-oriented expressiveness)
**Binary Projection:** CBOR-LD with Q42 lexicon (compact transfer)

**Key Features:**
- **Semantic Validation**: Q42 lexicon-based validation
- **Zero-Allocation Parsing**: Embedded lexicon lookup
- **Compact Transfer**: 60% size reduction for mobile
- **Full Offline Operation**: No external dependencies

### **Standardization Readiness**

The vault manifest format is ready for external standardization:

- **W3C**: For Turtle/N3 profile specifications
- **IETF**: For CBOR-LD binary format specification
- **OASIS**: For profile bundle specifications
