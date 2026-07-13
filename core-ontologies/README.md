# core-ontologies — Values Credentials

**Purpose.** General-purpose, engine-wide **values credentials**: the UN human-rights instruments
re-expressed as machine-readable, *affirmable* undertakings that bind **every agent** (human or
otherwise). They are the shared baseline of "basic expectations" used to distinguish **useful activity
from abuse** — consumed by the deontic logic layer, the MCP collaboration bridge, and the
agreement-forming process (`docs/manuals/standards/.../agreements`). Not MCP-specific.

> Source: `UN Instruments Table - UN_data.csv` (22 instruments). These are the **amended** versions:
> `States → parties/party` (the legal entity — natural or legal person — making the undertaking), and
> framed as an **affirmation** (the honourable agent affirms behaviour that is *ordinary* for them), not
> an oath sworn out of the ordinary.

## Layering (per instrument)
1. **`*.n3` — foundational** (this layer): N3/Turtle triples + N3Logic rules + SHACL shapes. Source of truth.
2. **`*.cml.html`** — CML-annotated human-readable rendering (generated from the N3).
3. **`*.q42`** — engine-native compiled volume (via the ingest pipeline), for zero-copy runtime use.

## The two transforms (curation, not mechanical conversion)
1. **`States → parties`** — every duty/right attaches to a *legal entity* (`values:Agent`), not a nation-state.
2. **Universalisation of responsibility** — where an article expresses a *"responsibility of the State"*,
   it is modelled as a duty borne by **every agent** (`values:borneBy values:Agent`), from which the
   **State's** responsibility follows as a *specialisation* (`values:State rdfs:subClassOf values:Agent`;
   an N3 rule entails the state-duty from the universal-agent-duty). This makes the values bind AI agents
   and institutions alike, and *supports* (rather than replaces) the state's responsibility.

## Vocabulary (`values:` = `https://ns.webcivics.net/values/`)
| Term | Kind | Meaning |
|---|---|---|
| `values:Agent` | rdfs:Class | Any entity bearing rights/duties — human **or** otherwise. Root of the duty model. **Not** `owl:Thing`. |
| `values:NaturalPerson` | ⊑ Agent | aligned to `hcai:NaturalPerson` (a person is not a thing — see `crates/qualia-core-db/shapes/qualia-agency.shacl.ttl`). |
| `values:LegalPerson`, `values:State`, `values:ArtificialAgent` | ⊑ Agent | specialisations; the State is one kind of agent. |
| `values:ValuesCredential` | rdfs:Class | A whole affirmable instrument (e.g. the UDHR). |
| `values:Undertaking` | rdfs:Class | One affirmable provision (article / paragraph). |
| `values:Right` | ⊑ Undertaking | A right `values:heldBy` an Agent, with a `values:correlativeDuty`. |
| `values:Obligation` / `values:Prohibition` / `values:Permission` | ⊑ Undertaking | Deontic operators (Obligate / Forbid / Permit). Align to ODRL where useful. |
| `values:borneBy` | property | The Agent class a duty binds (default `values:Agent` = all). |
| `values:heldBy` | property | The Agent class a right is held by. |
| `values:specialisedFor` | property | Marks a duty that is *intensified* for a specific agent kind (e.g. State). |
| `values:affirms` | property | Agent → ValuesCredential/Undertaking (the affirmation). |
| `values:source` / `values:originalText` / `values:amendedText` | property | Provenance: citation URI; verbatim text; parties-amended text. |
| `values:violates` | property (inferred) | An agent intent/act that breaches a `values:Prohibition`/`values:Obligation` — the **abuse signal**. |

## How "useful vs abuse" is judged
An agent `AgentIntent` (see `AGENT_INTENT_LOGGING_SPEC.md`) is evaluated against the credentials: an N3
rule / SHACL shape flags `values:violates` when an intended act falls under a `values:Prohibition` borne
by `values:Agent` (i.e. binding the actor). This is the deontic baseline the MCP and agreement layers
check. **Note:** `n3logic.rs` today *routes to* the deontic modality but does not *evaluate* rules — the
actual evaluation must run through `n3_parser`/`n3_compiler` or `deontic_logic.rs`; that evaluator is the
gap to close when wiring (tracked as a `PendingImplementation` backlog item, not faked).

## Status
- ✅ Vocabulary + **UDHR pilot** (`un-instruments/udhr.n3`) — for review/approval of the model.
- ⏳ Other 21 instruments, CML-HTML + `.q42` layers, and the deontic/MCP evaluator wiring — after approval.
