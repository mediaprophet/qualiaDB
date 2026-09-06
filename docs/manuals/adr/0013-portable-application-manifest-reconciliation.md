# ADR 0013 — Portable application manifest reconciliation

- **Status:** Accepted (documentation only; no product manifest ABI in this ADR).
- **Date:** 2026-09-06
- **Packet:** `APP-01` (Wave 4 — portable app contract)
- **Relates:** ADR 0012 (Construct is observer-scope; QApp is not a runtime type);
  playbook `docs/POET_LOWER_COST_AGENT_EXECUTION_PLAYBOOK_2026-09-04.md` §13;
  forward to `APP-02` (portable application manifest v1).
- **Supersedes (naming only):** competing use of “manifest” / “qApp” as if they named
  one package ABI. Does **not** replace ADR 0012 construct/manifold semantics.

---

## Acceptance checkboxes (`APP-01`)

- [x] Relationship among POET `Manifest` / `ManifoldSeed` / `ConstructPackage`, HCF/HMC
      envelopes, Webizen qApp host/export, and the portable application contract is
      documented in this ADR.
- [x] No duplicate competing portable-app manifest is implemented in this lane
      (ADR + naming map only; product ABI deferred to `APP-02`).
- [x] Legacy `qApp` / `QApp` naming is mapped or deprecated explicitly (type map below).
- [x] POET is **not** special-cased as the only host model — POET is one **projection**
      of a portable application among several targets.
- [x] Non-goals exclude Webizen Desktop (WD) lifecycle work from this ADR.
- [x] Forward link to `APP-02` is present.

---

## Context

Several names in the repo say “manifest”, “package”, or “app”, but they solve
different jobs:

| Surface today | What it actually is |
|---------------|---------------------|
| POET toolbox `Manifest` (`crates/poet/.../tool_chest/core/manifest.rs`) | Declarative **toolbox plugin** discovery (id, chains, tools, capabilities). |
| POET `ManifestRecord` (`crates/poet/.../browser/manifest.rs`) | **Canvas checkpoint** of `ManifoldSeed`s (revision / persistence), not a distributable app. |
| `ManifoldSeed` | Initial layout for one **lens / work surface** inside a construct. |
| `ConstructPackage` | Observer-scope payload: `ConstructSeed` + its `ManifoldSeed`s. |
| HCF / HMC envelopes | Wire envelopes for interactive (`.hcf`) vs archival (`.hmc`) construct/manifold export. |
| Legacy QApp / `qapp.json` / Desktop `qapp_*` | Former catalogue / host tab / message-bus naming; ADR 0012 already denies a QApp runtime type. |
| Planned portable application contract (`APP-02`) | Versioned, host-neutral package identity + projections + permission intents. |

Without reconciliation, agents risk implementing a second “manifest” next to
`ConstructPackage` / HCF before the portable contract exists, or treating POET as
the only host model. Wave 4 requires the opposite: one portable contract, many
projections; POET and Webizen Desktop are peers as hosts, not owners of the ABI.

---

## Decision

### D1 — One portable application contract; projections are adapters

The **portable application manifest** (to be defined in `APP-02`) is the sole
versioned, host-neutral contract for “an installable / launchable application
package” across the Qualia stack.

Hosts and shells **project** that contract; they do not each own a competing
package ABI:

| Projection target | Role of the portable app |
|-------------------|--------------------------|
| **Poet manifold** | App exposes (or lands as) one or more manifold entries inside an observer construct. |
| **Poet container** | App exposes focused container placements / tools on a manifold without becoming a second runtime. |
| **Focused mini-app** | App exposes a minimal standalone entry (shell later in `APP-05`) driven by the same identity and permissions. |
| **Webizen Desktop launch descriptor** | App appears under Desktop **Apps** as a launch/inspect record; Desktop does not redefine the package. |

POET is **one projection surface**, not the definition of “application.” Webizen
Desktop is another. Construct / manifold / container vocabulary from ADR 0012
remains authoritative for *observer-scope composition*; the portable manifest
does not replace Construct as “the world you are in.”

### D2 — Type map (old / local name → portable role)

| Existing name | Status | Portable role |
|---------------|--------|---------------|
| Toolbox `Manifest` | **Keep** (distinct ABI) | Toolbox plugin discovery only. **Not** an application package. May later be *referenced* by capability requirements; must not be renamed into the portable contract. |
| `ManifestRecord` | **Keep** (session checkpoint) | Live canvas revision store. **Not** a package or install unit. |
| `ManifoldSeed` | **Keep** | Projection / composition unit: lens layout. Portable apps may declare manifold entry projections that materialize as seeds. |
| `ConstructSeed` / Construct | **Keep** (ADR 0012) | Observer-scope composition. A portable app may *author* or *open into* a construct; a construct is not synonymous with an app. |
| `ConstructPackage` | **Keep** (POET composition export payload) | Payload inside HCF/HMC for construct composition. **Not** the portable application manifest. May be an *artifact* an app ships or exports. |
| `HcfConstructEnvelope` / `HmcConstructEnvelope` (and manifold HCF/HMC) | **Keep** | Distribution envelopes (interactive vs archival). Carriers for construct/manifold bytes; not the portable identity/permissions contract. |
| `qApp` / `QApp` / `qapp.json` | **Deprecated as runtime/package type** | Legacy UI catalogue and Desktop host/tab naming. Map former rows per ADR 0012 (construct, manifold, container, or Library Software stub). Do not implement a new `qApp` package format. Prefer “application” / portable manifest in new docs and APIs. |
| Desktop `qapp_id` / `qapp_host` / `qapp_url` | **Legacy host labels** | Treat as transition identifiers until WD packets remap navigation to Apps / Node / … (`WD-01+`). Not a second manifest schema. |
| Portable application manifest (`APP-02`) | **To implement** | Canonical package: identity/version/author, entry projections, required capabilities/assets, state schema, permission intents, presentation hints, compatibility, integrity, update channel. |

### D3 — Naming rules until `APP-02` lands

1. Do **not** add a parallel Rust product module that competes with the planned
   `APP-02` portable manifest (e.g. a second top-level “app manifest” under poet
   or a revived `qapp.json` ABI).
2. Prefer precise names in new prose: *toolbox manifest*, *checkpoint record*,
   *construct package*, *HCF/HMC envelope*, *portable application manifest*.
3. When writing “manifest” alone, say which one — or stop and use the type map.

### D4 — Relationship to HCF/HMC and qApp export

- **HCF** = interactive hypermedia content / construct composition envelope.
- **HMC** = archival / transparent container-of-files style envelope (provenance
  chain on construct HMC).
- Export of a construct composition continues to use `ConstructPackage` inside
  those envelopes.
- A **portable application** may *include* HCF/HMC artifacts as assets or entry
  payloads; the portable manifest itself is the outer identity + projection +
  permission contract (`APP-02`), not a rename of HCF.

Legacy QApp host/export paths remain reachable during migration; they are not
the target ABI.

---

## Non-goals (explicit)

This ADR does **not**:

- Implement the portable application manifest schema or serializers (`APP-02`).
- Implement projection adapters (`APP-03`), Health proof packaging (`APP-04`),
  focused mini-app shell (`APP-05`), or conformance suite (`APP-06`).
- Define or implement **Webizen Desktop lifecycle** (install / launch / stop /
  update / uninstall), registry storage, or managed paths (`WD-02`, `WD-03`, …).
- Change Construct / Manifold / Container semantics from ADR 0012.
- Replace toolbox `Manifest` or canvas `ManifestRecord` with the portable
  contract.
- Invent Host/Vibe IDs or widen frozen vibe-host surfaces.

---

## Consequences

### Positive

- Agents and humans share one vocabulary before `APP-02` code exists.
- POET and Desktop can both host apps without claiming ownership of the package
  format.
- Legacy qApp naming has an explicit deprecation / migration map instead of
  silent coexistence as a second ABI.

### Cost / follow-through

- `APP-02` must introduce the versioned portable manifest (deterministic
  serialize; unknown version/permission fail closed) under the core package
  home chosen in that packet (e.g. `q42/app_manifest/` or `portable_app/`).
- `APP-03` must prove one manifest → Poet manifold, Poet container, focused
  mini-app, Desktop launch descriptor without duplicated state or permissions.
- Desktop IA (`WD-01+`) should retire user-facing “QApps” navigation in favour
  of **Apps**, with POET listed as an app projection — outside this ADR’s
  write scope.

### Watch

- Do not implement a competing manifest “to unblock” Desktop before `APP-02`.
- Do not call a Construct an “app” or an app a Construct.
- Presentation hints must never grant authority (enforced when `APP-02` lands).

---

## Forward link — `APP-02`

**Next packet:** `APP-02` — Portable application manifest v1  
**Playbook:** `docs/POET_LOWER_COST_AGENT_EXECUTION_PLAYBOOK_2026-09-04.md` §13  

**Expected outcome (not done here):** define the versioned manifest fields
(identity/version/author, entry projections, required capabilities/assets,
state schema, permission intents, presentation hints, compatibility, integrity,
update channel) with deterministic serialization and fail-closed unknown
version/permission behaviour.

**Verify when `APP-02` lands:** round-trip, malformed, unknown-version,
permission-escalation, and deterministic-hash tests — and that this ADR’s type
map still holds (no second competing ABI).

---

## Optional naming-map checklist (docs-only)

Use when reviewing PRs before `APP-02`:

- [ ] New code does not introduce `qapp.json` or a poet-local “application
      Manifest” as the portable package.
- [ ] Toolbox `Manifest` changes stay scoped to tool discovery.
- [ ] Construct export continues to mean `ConstructPackage` ± HCF/HMC, not
      “the app ABI.”
- [ ] Prose that says “app” points at the portable contract (or explicitly
      marks legacy qApp).
- [ ] POET is described as a projection/host, not the sole application model.
