# POET, Webizen Desktop, and Health Capability Programme

**Date:** 2026-09-04  
**Status:** Active implementation direction  
**Decision:** Build health as portable Qualia capabilities and governed Q42 assets first. Present the same contracts through POET manifolds/containers and, later, focused mini-apps. Webizen Desktop is the general native host and administrative control plane, not a POET-only wrapper.

## 1. Product boundary

| Layer | Owns | Must not own |
|---|---|---|
| QualiaDB | Canonical data model, Q42 assets, ingestion, validation, query, inference, provenance, licensing obligations, consent, bounded execution | POET-specific layout or a single mini-app's navigation |
| App contract | Portable manifest, required capabilities/assets, commands, state schema, permissions, presentation hints | Native implementation details or an assumption that POET is the only host |
| POET | Flagship spatial workspace; manifolds, containers, semantic wiring, multi-domain composition, accessible task views | Duplicated health algorithms or private copies of canonical datasets |
| Focused mini-app | A narrow, guided workflow over the same app/capability contract | A second data model, ingestion pipeline, or clinical truth store |
| Webizen Desktop | General app host; POET launch; package installation; daemon/runtime supervision; key/identity vault; permissions; resources; diagnostics; updates; app lifecycle | Domain-specific business logic that belongs in Qualia or an app package |

The same underlying app must be projectable as:

1. a POET manifold for a complete domain workspace;
2. one or more POET containers for composable tasks;
3. a focused mini-app for a guided single-purpose experience; and
4. a Webizen Desktop hosted application with native permissions and lifecycle.

Presentation is therefore a projection. Capability identity, state, permissions, provenance, and asset identity remain stable across projections.

## 2. Health sequencing decision

Do not begin with a separate health mini-app. First implement one complete vertical slice through Qualia and POET:

1. governed dataset asset descriptor and bounded import job;
2. identifier normalization and evidence-preserving Quin projection;
3. query/capability contract independent of UI;
4. person-controlled health state and consent contract;
5. excellent POET manifold/container workflow;
6. reusable app manifest; then
7. a focused mini-app using exactly the same capabilities and assets.

This sequence avoids implementation drift while still using UI work to validate the contracts early. The POET view should be developed alongside the capability—not postponed until every dataset is ingested.

## 3. Q42 health asset envelope

Every imported source needs a first-class descriptor graph containing at least:

- stable Qualia asset ID and upstream release/version;
- source URL, retrieval time, byte length, and SHA-256 digest;
- exact upstream licence/terms URL, attribution text, use class, and redistribution class;
- parser and mapping versions;
- raw format and canonical media type;
- source record count, accepted count, quarantined count, and rejection reasons;
- canonical identifier namespaces and cross-reference policy;
- evidence grade, citation/DOI links, and source-specific curation status;
- sensitivity and commons-routing lane;
- derived-from links and the union of upstream licence obligations;
- SHACL profile and validation report;
- chunk plan proving every ingestion pass stays under the 42 MB Sentinel limit.

Raw downloads remain immutable source artifacts. Normalized entities, relationships, and evidence claims are separate derived assets. A claim about a food, compound, gene, disease, phenotype, or exposure must retain its source record and evidence; it must never silently become medical advice.

## 4. Source triage

| Source | Role | Access / formats | Initial disposition |
|---|---|---|---|
| ChEBI | Canonical chemical identity and ontology spine | OWL, OBO, JSON, TSV, SDF, PostgreSQL; CC BY 4.0 | **Wave 1.** Best first production importer and identifier authority. |
| Monarch Initiative | Disease, phenotype, gene, treatment association graph | KGX TSV, JSONL, RDF, Neo4j, DuckDB; upstream licences vary by source | **Wave 1/2.** Import source-partitioned edges; propagate each source's obligations. |
| ABCkb | Plant–chemical–human condition knowledge graph | Neo4j data artifact and GPLv3 code | **Wave 2 discovery.** Verify the data artifact's own licence; do not infer it from the software licence. |
| FoodAtlas | Food–chemical–health literature graph and pipeline reference | Public Apache-2.0 code; API credentials by request | **Wave 2 connector.** Reuse concepts/pipeline where compatible; keep extracted claims evidence-labelled. |
| Cytoscape | External network analysis/visualization application | Desktop import formats and CyREST automation | **Integration target, not dataset.** Add an optional Webizen Desktop connector/export hand-off. |
| FooDB | Food composition, compounds, proteins, nutrients, spectra | CSV, XML, JSON, SQL and spectral formats; CC BY-NC 4.0 | **Restricted asset lane.** Useful for non-commercial/research builds; do not place in a generally redistributable commercial bundle. |
| HMDB | Human metabolites, structures, spectra, pathways | XML/MetaboCard, SDF, FASTA, spectra; commercial redistribution needs permission | **Permission-gated.** Support user-supplied/local import; do not redistribute without explicit permission. |
| CTD | Chemical–gene–disease–phenotype/exposure relationships | Downloadable tabular datasets; non-commercial free, commercial licence required | **Permission-gated/restricted.** Preserve terms and keep commercial distribution separate. |
| Phenol-Explorer | Polyphenol food content, metabolism, pharmacokinetics | Public web database and exports; licensing needs direct confirmation | **Legal review before bundling.** Prototype a local adapter only after terms are recorded. |
| FOODBALL portal | Methods, guidelines, and directory of food-metabolome resources | Web resources rather than one canonical bulk dataset | **Source catalogue.** Use to guide ontology/evidence design, not as a monolithic asset. |
| PhInd | Polyphenols in agri-food by-products with methods and DOI provenance | Web search and per-result Excel export; public-use statement in project material | **Wave 2 after licence confirmation.** Strong evidence/method fields; preserve DOI and extraction method. |

## 5. First health capability family

Start with food–compound exploration, not diagnosis or recommendation:

- `HealthAsset.import_release`
- `HealthAsset.validate_release`
- `HealthAsset.describe_release`
- `HealthKnowledge.resolve_chemical`
- `HealthKnowledge.food_compounds`
- `HealthKnowledge.compound_foods`
- `HealthKnowledge.evidence_for_claim`
- `HealthKnowledge.related_phenotypes`
- `HealthKnowledge.export_subgraph`

Each query returns identifiers, units, evidence, provenance, licence obligations, and uncertainty. Health-effect associations are clearly labelled as research evidence and are not converted into treatment advice.

## 6. POET experience direction

The health manifold should be a calm, legible person workspace rather than a wall of database forms. Its primary jobs are:

- understand what is in the person's record now;
- add a measurement or event with explicit units and provenance;
- inspect trends without fabricated interpretation;
- find records chronologically;
- manage who can see exactly what and until when;
- explore food/compound evidence with source and confidence visible;
- create a portable, governed view or mini-app without copying the data.

The first implemented slice replaces the generic Health overview with typed measurement entry, real-record summaries, a vitals plot, and a cross-family timeline. Next slices add editing/correction receipts, consent grant/revocation, and the food–compound explorer.

## 7. Webizen Desktop direction

Webizen Desktop becomes the native control plane and multi-app host with these workspaces:

1. **Apps:** install, verify, launch, stop, update, and inspect POET and other application packages.
2. **Node:** daemon health, 42 MB Sentinel telemetry, storage, CPU/GPU, jobs, and logs.
3. **Identity & permissions:** DID keys, app grants, filesystem/network/device permissions, consent sessions, and revocation.
4. **Assets:** Q42 asset releases, licences, digests, update channels, validation reports, and storage budgets.
5. **Connections:** local/remote peers, connectors, Cytoscape hand-off, APIs, and transport status.
6. **Recovery:** backups, migrations, failed jobs, quarantined imports, and auditable repair actions.

Launching POET is one app-host action. POET-specific authoring remains in POET.

## 8. Delivery programme

### Phase A — interface foundation

- Establish a readable type scale, density modes, focus/keyboard behavior, modal focus traps, loading skeletons, and consistent inline validation.
- Replace global fabricated operational decorations with real state or clearly labelled preview content.
- Build task-level UAT for each restored workflow.

### Phase B — health vertical slice

- Complete timeline and vitals editing/correction lifecycle.
- Add scoped disclosure grant, expiry, and one-click revocation receipts.
- Implement ChEBI release import as the first governed Q42 dataset asset.
- Add food/chemical entity search and evidence drawer in the POET health manifold.

### Phase C — portable app contract

- Define manifest, capability, state, asset, permission, and presentation schemas.
- Demonstrate one health app rendered as a POET manifold, POET container, and focused mini-app from the same manifest.
- Add conformance tests that compare state and authorization outcomes across projections.

### Phase D — desktop control plane

- Refactor Webizen Desktop navigation around Apps, Node, Identity, Assets, Connections, and Recovery.
- Implement general app lifecycle and permission gates.
- Register POET as the first hosted app, then the health mini-app as the second proof.

## 9. Completion evidence

A capability is not complete because a route, form, or record family exists. Completion requires:

- a real user job completed end to end;
- correct domain interaction design;
- persisted and queryable state with provenance and licence obligations;
- permission/consent behavior that fails closed;
- empty, loading, error, partial, offline, and success states;
- keyboard and screen-reader operation;
- desktop and browser-hosted UAT;
- no fabricated health, clinical, daemon, peer, or job state presented as live;
- the same authorization and data results from every supported app projection.
