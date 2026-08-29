# ADR 0012: Construct is the observer’s coherent scope

## Status
Accepted (2026-08-28) — directed by Timothy. Refined in the same session:
**Anatomy is a manifold, not a construct.** A construct is the full embodiment
of the observer’s (or agent’s) status — a coherent overall *scope* — not an
application.

The name is the same sense as the Matrix loading program: *the construct* is
the world instantiated for the observer, composed of lenses, not a packaged
app they launch from a store.

## Context
The Dioxus QApp catalogue (~333 rows, 200+ academic wrappers, three first-class
routes) mixed three jobs:

1. **A world-scope for someone looking** — what a person or agent *is in*.
2. **Lenses and work surfaces** — health, anatomy, research, studio.
3. **Academic stubs** — SHACL-form wrappers, often labelled Active.

POET already had tool / toolbox / chest, container, wire, manifold, partial
subcanvas, workspace (device session), HCF/HMC, and Library `software`.
Shortcuts already spoke `--construct=` but Construct was never typed. Calling
the former QApp an “app” flattened the observer: Anatomy is one lens, not a
second runtime.

## Decision

### D1 — Construct is observer-scope
A **construct** is the coherent overall scope that embodies the status of an
observer or agent. It is made of:

- an **observer** (principal or agent DID — whose embodiment this is)
- an **array of manifolds** (lenses — different ways of seeing within the scope)
- **containers** on those manifolds (structures of each lens)
- **linked manifolds** (nested surfaces and portals that keep the scope one)

Workspace is this machine’s live holding of an open construct (devices,
displays). It is not a visual peer of Construct.

| Kind | Role |
|------|------|
| **Construct** | Observer/agent embodiment; coherent scope. Open, pin, share, shortcut, kiosk. |
| **Manifold** | A lens / work surface *inside* a construct. Nested manifolds are still manifolds. |
| **Container** | Typed structure on a manifold. May portal to another manifold or construct. |
| **Workspace** | Live projection of an open construct onto this hardware. |

Tools/toolboxes/chest remain how you *act*. They are not a fifth occupant kind.

### D2 — Anatomy is a manifold
Anatomy is a **lens** on the Health construct (and on the default POET
construct). It is not a construct. The bundled Anatomy QApp migrates to
manifold `anatomy`, reached by a nested-manifold container on Health.

### D3 — QApp is not a runtime type
A former QApp is either a construct (a pre-built observer-scope), a manifold
or container on an existing construct, or a Library Software stub. `qapp.json`
is not a second package ABI. Interactive export is **HCF**; archival is **HMC**.

**Library Software** = how you *find* a scope. The **construct shelf** = the
desk of scopes this observer currently holds. Stubs stay stubs until they have
manifolds.

### D4 — Nesting and links
- Manifolds nest via a **nested-manifold container** (replaces the `subcanvas`
  stub). Opening it switches the pager and pushes a breadcrumb.
- Containers may target a manifold in the same construct, or a **portal** into
  another observer-scope (`target_construct` + optional `target_manifold`).
- LOD / breadcrumb depth stays bounded (max 6).

### D5 — Academic catalogue
Two hundred liberal-arts rows do **not** become pager tabs or constructs.
They remain Library Software stubs (`honesty: stub`) until a real seed exists.

### D6 — Migration map
| Former QApp-shaped thing | POET home |
|--------------------------|-----------|
| Anatomy (bundled package) | **Manifold `anatomy`** on construct `health` |
| Clinical / DICOM / comorbidity / vitals | Health construct + Domain Lab on Research |
| Chemistry / physics / ODE / bioinformatics / GBM | Domain Lab on Research |
| SPARQL, N3, RDF-Star, SHACL, ontology | Knowledge / Ontology manifolds |
| Dual Studio / Scene / Audio | Studio manifold |
| Chat / LLM / agents | Communications + Vibe |
| Context Studio | Lenses on Knowledge / personhood |
| QApp Studio | Authoring a construct (compose lenses → HCF) |
| Academic `*_qapp.rs` | Library Software stubs |

### D7 — What is not added
App, Scene, and Session are not extra top-level kinds. Anatomy is not a
construct. Workspace is not a visual peer of Construct. No second package
format beside HCF/HMC.

### D8 — Subject, project, and construct are not the same
A plant, a fictional world, and an SDG programme all *fan out* into nested
manifolds and typed containers. That fan-out is real. It is **not** each a
construct.

| Kind | Question it answers | Example |
|------|---------------------|---------|
| **Construct** | *Whose* modelled mindware environment is this? | A principal’s POET; an agent’s instrument-scope; Health as *their* health-scope |
| **Subject** (topic / matter / world-under-consideration) | *What* is being looked at, in all its aspects? | A plant; Star Trek as a diegesis; a catchment; a campsite |
| **Project** | *What are we delivering, together, over time?* | Camping-site network; an SDG programme |

- **A plant** is a subject. Cellular structure, nutrition, climate, geospatial
  range, medical use, cultural name — each aspect is a **manifold** (or nested
  manifold) of that subject, with different container types. Scale (cell →
  organism → biome → planet) is nesting, not a new top-level kind.
- **Star Trek / Star Wars / a game world** is a subject with a *diegetic*
  boundary (a defined world-under-consideration). Timeline, species, tech,
  politics, episodes are manifolds of that subject. Epistemic tag: fictional /
  intersubjective canon — not a new occupant kind. “Relatively flat” cinema is
  still a subject; POET can give it depth without pretending it is a person.
- **Camping sites / SDGs** are **projects** (already a POET family):
  multi-party, time-bound, deliverable. People (and agents) *author* the
  project’s manifolds **from their own constructs**. The project is shared
  work; the construct remains someone’s modelled environment.

A construct may *hold lenses on* many subjects and *host* several projects.
A subject may be held in many constructs (private notes vs commons). A
natural person is a **Principal**, never a subject flattened to `owl:Thing`
and never “a construct.”

Without this split, everything becomes a construct and the observer
disappears — the same flattening the QApp catalogue performed.

### D9 — Keep the word *construct*; gloss it in public
The engine id stays `construct` (`--construct=`, shelf, ADR). It means
**authored modelled environment** (you build your mindware), not a prison
simulation.

The Matrix loading program is the *useful* sense: a world instantiated for
someone looking, composed of lenses, that they can enter and leave. The
*fearful* sense (simulation as cage, persons as NPCs) is rejected: a
construct is representative state **modelled by the observer**, under their
agency. It is not the territory, not a substitute for a natural person, and
not a future in which people live inside someone else’s machine.

Public / non-engine glosses that may be used in UI copy without renaming the
type: **“your working environment”**, **“mindware environment”**, **“scope”**.

Terms considered and not adopted as the type name:

| Term | Why not the type id |
|------|---------------------|
| World | Collides with Qualia’s world-of-man / world-of-god; too big for a health-scope |
| Umwelt / lifeworld | Precise (Uexküll / Husserl) but opaque in the product |
| Setting / diegesis | Fits Trek; fails as a person’s mindware |
| Workspace | Already the device-bound session |
| Habitat / Chora / Studio / Sanctuary / Nexus | Already occupied in this repo |
| Model / simulation | The word the frightened audience hears |
| App / QApp | The flattening we are leaving |

Rename only if a better *type* word appears that still means observer-scope
and does not collide. Do not rename for etiquette.

### D10 — Construct is personal; infosphere / noosphere are broader
A construct is **this user’s environment**: QualiaDB / Webizen running POET on
**their** hardware — the analogue of “their computer, with windows on it,”
except the windows are interconnected **manifolds of interactivism**, unique
to that principal. It is the representative state of *their* mindware, modelled
by them, fiduciary to them.

It is **not** the infosphere and **not** the noosphere.

| Term | Scope | In this project |
|------|--------|-----------------|
| **Construct** | One observer’s machine-environment of linked manifolds | Personal; unique; on their device |
| **Infosphere** | The broader informational surround those constructs participate in | Already used (e.g. 10D Infosphere); too large to name a user’s POET |
| **Noosphere** | Intersubjective / planetary layer of thought and shared work | Commons, shared subjects, SDG programmes — *between* constructs |

A user’s construct **forms part of** a broader infosphere/noosphere (Library
commons, shared subjects, projects). It is not identical with them. Renaming
construct to infosphere would erase the personal boundary the whole stack is
built to protect (sanctuary vs commons, world-of-man vs world-of-god, Principal
vs instrument).

Each construct is unique because the manifolds, wires, held subjects, hosted
projects, and sensitivity lanes are *theirs*. Agents inside it remain
instruments of that principal.

### D11 — Author the *means*, not canned worlds
POET does **not** ship Star Trek, Star Wars, or other diegetic catalogues.
It ships the **means** to author lenses: a named manifold in the open
construct, optional nested-manifold portal on the current surface, and
VibeScript as the language (`Poet.manifold_create`, `Poet.container_place`,
`Poet.nested_link`, `Poet.subject_declare`, `Poet.participant_invite`). Plants, film-worlds,
camping programmes, SDGs — the user (or their agents) composes those
subjects as arrays of manifolds and container types **inside their
construct**. Vibe 0.1 stays 0.1; host receipts are layout-only (no DOM in
qualia-core-db). This is not a second app store of fictional worlds.

### D12 — A manifold may be social (many people); projects are the primary case
A **construct** is still one observer’s machine-environment. A **manifold**
inside it may be *personal* (health, anatomy, settings) or *social* (many
people participate in the same lens). Sociality is a property of the lens,
not a second runtime and not a group-construct.

| Lens | Sociality | Why |
|------|-----------|-----|
| Health, anatomy, sanctuary, settings | Personal | One principal’s body, care, and machine |
| **Projects** | **Social** | Shared, time-bound delivery — camping network, SDG programme |
| Social graph, communications | Social | Presence, members, discussion among people |

People join a social manifold as **participants** (DID + role: member /
steward / observer). Natural persons remain `rdfs:Class` / SHACL, never
`owl:Thing`. Pulse presence is who is *here now*; the participant roster is
who *belongs*. The construct that *hosts* the lens remains the inviting
principal’s.

Authoring: `Poet.manifold_create({ social: true })` and
`Poet.participant_invite({ did, role })`. Vibe 0.1, no version bump.

## Consequences
- **Positive:** observer status has a name; manifolds stay lenses; Anatomy
  cannot be mistaken for a second app; plant/Trek/SDG fan-out has a home
  (subject or project) without stealing the observer’s name.
- **Cost:** Construct registry, shelf, nested-manifold portal, and seeds land
  in `crates/poet` before further QApp wrappers are added. `SubjectSeed` is a
  thin authored-focus registry (not a construct, not a project).
- **Watch:** do not label stub scopes `live`. Honesty stays fail-closed.
  Do not call a person a construct.

## Implementation notes
- Type: `crates/poet/src/tool_chest/core/construct.rs` (`observer` DID + manifolds)
- Subject: `crates/poet/src/tool_chest/core/subject.rs`
- Catalogue: `crates/poet/src/tool_chest/constructs/`
- Shelf + portal: `crates/poet/src/browser/construct_shelf.rs`
- Authoring: `crates/poet/src/browser/manifold_authoring.rs` (shell apply)
- Social manifolds: `crates/poet/src/tool_chest/core/sociality.rs`, `crates/poet/src/browser/manifold_social.rs`
- Host receipts: `crates/qualia-core-db/src/poet_host/invoke/poet_shell.rs`
- Default construct `poet` is the full HyperCanvas observer-scope (compat pager)
