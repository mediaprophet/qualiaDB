# `.qualia` Vault Manifest Draft

**Status:** Internal draft  
**Date:** 2026-06-08  
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

- `.q42` for raw graph/data substrate
- `.q42.lex` and `.q42.bidx` for retrieval sidecars
- `.qchk` for capability envelopes
- `qapp.json` for qapp-local launch metadata

## 2. Why This Exists

Historically, the repo used `.qualia` in a looser way before the `q42` family
was formalized. At that earlier stage, `.qualia` could refer to a general
payload or blob checked into a Webizen or git-oriented workflow.

Now that `.q42`, `.c.q42`, `.q42.lex`, `.q42.bidx`, and `.qchk` have clearer
roles, `.qualia` should be stabilized as the human-centric vault manifest that
binds those artifacts together.

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
- `qualia:includesLexiconFile`
- `qualia:includesBlockIndexFile`
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
  qualia:includesLexiconFile <vault/main.q42.lex>, <vault/health.q42.lex> ;
  qualia:includesBlockIndexFile <vault/main.q42.bidx>, <vault/health.q42.bidx> ;
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
5. load associated sidecars when available
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

## 11. Open Questions

1. Should the manifest point to relative paths only, or allow URI-based
   references too?
2. Should multiple identifier contexts be allowed, or should one primary
   context always be required?
3. Should there also be a more formal human-context vocabulary that remains
   distinct from identifier, auth, and credential fields?
4. Should `.qualia` allow embedded display preferences, or should those remain
   environment-local only?
5. Should Wellfair-specific fields remain extension terms, or become part of
   the shared core vocabulary?
6. Should there be a signature wrapper for `.qualia`, or should integrity be
   handled through adjacent vault state and key-vault mechanisms?
7. Should a CBOR-LD projection be standardized later for compact sync or mobile
   transfer, while keeping Turtle as the primary human-facing form and N3 as an
   optional richer textual profile?

## 12. Immediate Next Steps

1. Decide whether Turtle is the canonical first shape, with N3 as an allowed
   richer textual profile and CBOR-LD as a later projection.
2. Define the minimal vocabulary namespace for `qualia:*` manifest terms.
3. Determine how `.qualia` relates to existing `DirectoryState` persistence.
4. Add a Flutter desktop open-flow backlog item for `.qualia`.
5. Add a Wellfair/mobile continuity note so the schema does not become desktop-
   only by accident.
