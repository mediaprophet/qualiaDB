# VibeScript & Poet Engine: Technical Specification
> *"Poet: Define your mindware, with VibeScript"*

**Copyright © 2026 Timothy Charles Holborn.** All rights reserved.  
**Principal / inventor:** Timothy Charles Holborn &lt;timothy.holborn@gmail.com&gt;

> ## ⚑ STALE / ASPIRATIONAL — read `vibescript-core.md` instead
>
> **This document is an architectural essay, not the parser contract.** It predates
> the implemented `vibe-0.1` language and describes a v1.0 destination. Several
> constructs below are **rejected by the real parser** (`poet-vibe`) and **must not
> be taught to agents or used in examples**:
>
> | In this essay | Status in `vibe-0.1` | Use instead |
> |---|---|---|
> | `<<[ s p o g prov ]>>` NquinTerm literal | **Illegal** (E001) | `quin.statement(subject:, predicate:, object:, context:)` |
> | `<< id \| s p o >>` pipe reifier | **Illegal** (E001) | RDF 1.2 `<< s p o ~ reifier >>` |
> | `pulse.broadcast` / `pulse.emit` / `pulse.subscribe` | Not in 0.1 | `pulse.publish(topic, payload)` |
> | `aura.apply_schema` / `aura.infer` / `aura.resolve_context` | Not in 0.1 | `aura.validate(node, shape)` |
> | `graph.assert` / `graph.retract` / `graph.commit_delta` / `graph.sparql` / `graph.get_node` | Not 0.1 bindings | `graph.stage` / `graph.commit` / `graph.query`; SPARQL via `capability.invoke("GraphDatabase.sparql", …)` |
> | `space.*` / `geom.*` as first-class grammar | Not in 0.1 grammar | `capability.invoke("Geometry.Hull2", …)` |
> | `on_hover` / `on_click` / `on_change` / `on_pulse` / `on_aura` hooks | Not in 0.1 | `on pulse:message(…)`, `on tick(…)`, etc. (core §3 `EventPath`) |
> | Backtick string literals | Not in 0.1 | `"…"` only |
> | Micro-Poet `<2MB` / `no_std` edge | Not claimed for 0.1 (core §10) | `native-desktop` + `wasm32` only |
> | `.d10` as canonical 10D extension | Non-normative alias | `.10d` (core §10) |
>
> **Normative 0.1 language:** [`vibescript-core.md`](vibescript-core.md). The live
> parser is `crates/poet-vibe/`; the live host is
> `crates/qualia-core-db/src/poet_host/`. Where this essay and the core spec
> disagree, **the core spec wins**. The essay is retained as the architectural
> rationale for the v1.0 destination (CBOR-LD AST, SHACL-AF, N3Logic interop,
> multi-platform matrix) that 0.1 is the closed core of — not a denial of.

## 1. Vision & Execution Philosophy
**VibeScript** (or **Vibe**, `.vibe`) is an embedded, interpreted domain-specific language (DSL) evaluated by the **Poet Engine** in Rust/WASM. It is designed for **dynamic hypermedia documents, reactive semantic mindware, spatiotemporal reasoning, and decentralized graph automation** within the QualiaDB / Webizen ecosystem.

- **The Mindware Philosophy:** Replaces rigid mechanical scripting with expressive, qualitative semantic programming. Authors don't just write instructions; they capture the *vibe* and structure of human perception.
- **The Poet Engine:** A lightweight, pure safe-Rust AST/bytecode interpreter (compiling to `wasm32-unknown-unknown` and native targets) that executes Vibe scripts in memory with **zero JIT compilation** and **zero AOT compile wait times**.
- **First-Class Domain Primitives:**
  - **`pulse` (Communications & Sync):** First-class namespace for WebSockets, WebRTC channels, IoT MQTT/CoAP telemetry streams, and Merkle DAG synchronizations.
  - **`aura` (Ontology & Semantics):** First-class namespace for OWL-RL reasoning, SHACL shape validation, SNOMED/custom ontology application, and context resolution.
  - **`graph` / `space` / `geom` (Data & Geometry):** Direct manipulation of Q42 Nquins (`<<[ s p o g prov ]>>`), RDF 1.2 triple terms (`<<( s p o )>>`), and GeoSPARQL geometry.
- **No Node.js:** 100% native Rust, WASM, and browser-standard execution without npm or Node dependencies.

---

## 2. 3-Tier Execution Architecture

```
┌──────────────────────────────────────────────────────────────────────────┐
│                   Tier 1: Poet Workbench & Hypermedia UI                 │
│  - Authoring: Text, 3D meshes (D10), tabular data (p64), slide canvases  │
│  - Web Components: <q-doc>, <q-entity>, <q-relation>, <q-cell>, <q-event>│
│  - Reactive Presentation: Vanilla DOM, CSS Custom Properties, WebGL/GPU  │
└────────────────────────────────────┬─────────────────────────────────────┘
                                     │ DOM CustomEvents & Reactive Formula Triggers
┌────────────────────────────────────▼─────────────────────────────────────┐
│                 Tier 2: Poet Engine (VibeScript Runtime)                 │
│  - Zero-Compile DSL: Live formulas, validation rules, graph traversal    │
│  - Aura Namespace: Ontological shaping, SHACL validation, context lookup │
│  - Pulse Namespace: Real-time streams, telemetry, collaborative sync     │
│  - Cooperative Multitasking: Async `await`/`yield` & AST Gas Metering    │
│  - Decentralized Auth: DID / Verifiable Credentials & Solid Pod Scopes   │
└────────────────────────────────────┬─────────────────────────────────────┘
                                     │ Zero-Copy API & Async Merkle DAG Sync
┌────────────────────────────────────▼─────────────────────────────────────┐
│          Tier 3: Heavy Compute & qualia_core_db (Compiled Rust AOT)       │
│  - Core Store: Q42 Merkle-CRDT DAG with State-Based Snapshot Compaction  │
│  - Streaming: Bao Verified Streaming (BLAKE3 1-KiB chunk trees)          │
│  - NLP & Geometry: FST tokenizers, Aho-Corasick tries, spatial indexes   │
│  - SPARQL Engine: SPARQL 1.1 + GeoSPARQL spatial & topological functions │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Formal EBNF Grammar Specification (v1.0)

> **⚠ Not the implemented grammar.** This is the v1.0 essay grammar. The
> implemented `vibe-0.1` grammar is in
> [`vibescript-core.md` §3](vibescript-core.md) and
> `crates/poet-vibe/grammar/vibe-0.1.ebnf`. Lines below marked `✗ REJECTED`
> produce `E001` in the real parser and must not be taught:

```ebnf
(* ===========================================================================
   VibeScript Formal Grammar (Poet Engine / W3C RDF 1.2 / Pulse / Aura)
   =========================================================================== *)

Program           ::= StatementList ;
StatementList     ::= Statement* ;

Statement         ::= VariableDecl
                    | Assignment
                    | ReactiveFormula
                    | GraphMutation
                    | PulseDecl
                    | AuraDecl
                    | ControlFlow
                    | FunctionDecl
                    | HookDecl
                    | Expression ';' ;

(* --- Declarations & Types --- *)
VariableDecl      ::= 'let' Identifier (':' TypeAnnotation)? ('=' Expression)? ';' ;
Assignment        ::= Identifier '=' Expression ';' ;
TypeAnnotation    ::= 'Iri' | 'TripleTerm' | 'Nquin' | 'ReifiedTriple' | 'Aura' | 'Pulse' | 'Graph' | 'Spatial' | 'Number' | 'String' | 'Bool' ;

(* --- Reactive Document & Cell Formulas --- *)
ReactiveFormula   ::= '=' Expression ;
HookDecl          ::= ('on_hover' | 'on_click' | 'on_change' | 'on_pulse' | 'on_aura') '(' ParameterList? ')' Block ;   (* ✗ REJECTED in 0.1 — use on <EventPath>(…) e.g. on pulse:message(…), on tick(…) *)

(* --- RDF 1.2 Triple Terms, Nquins & Reification --- *)
TripleTerm        ::= '<<(' Subject Predicate Object ')>>' ;   (* OK in 0.1 *)
NquinTerm         ::= '<<[' Subject Predicate Object GraphContext ProvenanceTag? ']>>' ;   (* ✗ REJECTED in 0.1 — use quin.statement() *)
ReifiedTriple     ::= '<<' (ReificationId '|')? Subject Predicate Object '>>' AnnotationBlock? ;   (* ✗ REJECTED in 0.1 — use << s p o ~ reifier >> *)
ReificationId     ::= IRI | BlankNode | Identifier ;
AnnotationBlock   ::= '[' PropertyValuePair (',' PropertyValuePair)* ']' ;
PropertyValuePair ::= (IRI | Identifier) ':' Expression ;

Subject           ::= IRI | BlankNode | TripleTerm | NquinTerm | Identifier ;
Predicate         ::= IRI | Identifier ;
Object            ::= IRI | BlankNode | Literal | TripleTerm | NquinTerm | Identifier ;
GraphContext      ::= IRI | BlankNode | Identifier ;
ProvenanceTag     ::= StringLiteral | Identifier ;

(* --- First-Class Namespaces: Graph, Pulse, Aura & Spatial --- *)
GraphMutation     ::= 'graph' '.' ('assert' | 'retract' | 'commit_delta') '(' (ReifiedTriple | Expression) ')' ';' ;   (* ✗ REJECTED in 0.1 — use graph.stage / graph.commit *)
GraphQuery        ::= 'graph' '.' ('query' | 'sparql') '(' (PatternMatch | StringLiteral) ')' ;   (* ✗ 'sparql' not a 0.1 binding — use capability.invoke("GraphDatabase.sparql", …); 0.1 query is graph.query(s,p,o, take: N) *)
PatternMatch      ::= 'match' '(' PatternElement (',' PatternElement)* ')' ;
PatternElement    ::= '?' Identifier | IRI | Literal | '_' ;

PulseDecl         ::= 'pulse' '.' ('broadcast' | 'emit' | 'subscribe') '(' ArgumentList? ')' ;   (* ✗ REJECTED in 0.1 — use pulse.publish(topic, payload) *)
AuraDecl          ::= 'aura' '.' ('apply_schema' | 'validate' | 'resolve_context' | 'infer') '(' ArgumentList? ')' ;   (* ✗ only aura.validate is in 0.1 *)

SpatialExpr       ::= 'space' '.' Identifier '(' ArgumentList? ')'   (* ✗ Not in 0.1 grammar — use capability.invoke("Geometry.*", …) *)
                    | 'geom' '.' Identifier '(' ArgumentList? ')' ;

(* --- Control Flow & Cooperative Multitasking --- *)
Block             ::= '{' StatementList '}' ;
ControlFlow       ::= IfStatement | ForLoop | ReturnStatement | YieldStatement ;
IfStatement       ::= 'if' Expression Block ('else' (IfStatement | Block))? ;
ForLoop           ::= 'for' Identifier 'in' Expression Block ;
ReturnStatement   ::= 'return' Expression? ';' ;
YieldStatement    ::= 'yield' Expression? ';' ;

FunctionDecl      ::= 'async'? 'fn' Identifier '(' ParameterList? ')' Block ;
ParameterList     ::= Identifier (',' Identifier)* ;
ArgumentList      ::= Expression (',' Expression)* ;

(* --- Expressions & Async Await --- *)
Expression        ::= AwaitExpr ;
AwaitExpr         ::= 'await'? LogicalOrExpr ;
LogicalOrExpr     ::= LogicalAndExpr ('||' LogicalAndExpr)* ;
LogicalAndExpr    ::= EqualityExpr ('&&' EqualityExpr)* ;
EqualityExpr      ::= RelationalExpr (('==' | '!=') RelationalExpr)* ;
RelationalExpr    ::= AdditiveExpr (('<' | '<=' | '>' | '>=') AdditiveExpr)* ;
AdditiveExpr      ::= MultiplicativeExpr (('+' | '-') MultiplicativeExpr)* ;
MultiplicativeExpr::= UnaryExpr (('*' | '/' | '%') UnaryExpr)* ;
UnaryExpr         ::= ('!' | '-' | '+')? PrimaryExpr ;

PrimaryExpr       ::= Literal
                    | IRI
                    | TripleTerm
                    | NquinTerm
                    | ReifiedTriple
                    | GraphQuery
                    | PulseDecl
                    | AuraDecl
                    | SpatialExpr
                    | Identifier ('(' ArgumentList? ')')?
                    | '(' Expression ')' ;

(* --- Terminals & Lexical Elements --- *)
IRI               ::= '<' [^>]+ '>' | PName ;
PName             ::= (PrefixName)? ':' LocalName ;
PrefixName        ::= [a-zA-Z_][a-zA-Z0-9_-]* ;
LocalName         ::= [a-zA-Z_][a-zA-Z0-9_.-]* ;
BlankNode         ::= '_:' [a-zA-Z0-9_-]+ ;

Literal           ::= StringLiteral | NumericLiteral | BooleanLiteral ;
StringLiteral     ::= '"' [^"]* '"' | '`' [^`]* '`' ;   (* ✗ backtick strings not in 0.1 — use "…" only *)
NumericLiteral    ::= [0-9]+ ('.' [0-9]+)? ;
BooleanLiteral    ::= 'true' | 'false' ;
Identifier        ::= [a-zA-Z_][a-zA-Z0-9_]* ;
```

---

## 4. Idiomatic VibeScript Examples

> **⚠ These examples use v1.0 essay syntax that the `vibe-0.1` parser rejects.**
> They are retained as architectural illustration only. For examples that
> actually parse and evaluate today, see
> [`vibescript-core.md` §12](vibescript-core.md) and
> `crates/poet-vibe/fixtures/`. Each block below is marked with what 0.1 rejects.

### A. Reacting to a Network "Pulse" to Update an "Aura"
> ✗ `on_pulse`, `pulse.broadcast`, `aura.apply_schema`, `graph.get_node` are not 0.1.
> 0.1 form: `on pulse:message(…)`, `pulse.publish(topic, payload)`, `aura.validate(node, shape)`.
```vibe
// When the document receives an IoT or remote collaborator Pulse:
on_pulse(stream) {
    let reading = stream.read_telemetry();
    if reading.value > 85.0 {
        // Broadcast alert pulse to collaborative peers
        pulse.broadcast("topic:alerts", "Critical threshold exceeded");
        
        // Update the entity's ontological Aura
        let clinic = graph.get_node("qualia:Clinic_A");
        aura.apply_schema(clinic, "snomed:EmergencyStatus");
    }
}
```

### B. W3C RDF 1.2 Claims & Reification
> ✗ `<< :claim_42 | … >>` pipe reifier and `<<[…]|>>`-style provenance are illegal in 0.1.
> 0.1 form: `<<( s p o )>>` and `<< s p o ~ reifier >>`; provenance is a graph/receipt, not a literal.
```vibe
// 1. Abstract proposition term (unasserted claim)
let candidate_claim = <<( :TreatmentX :cures :DiseaseY )>>;

// 2. Asserted proposition with cryptographic provenance
let asserted_fact = << :claim_42 | :TreatmentX :cures :DiseaseY >> [
    confidence: 0.98,
    author: "did:key:z6MkhaXgBZsJnNSMMGzCguy5e27wGZ6KP5dQbwtFTJP7Gi8C",
    crdt_clock: 1042
];

await graph.commit_delta(asserted_fact);
```

### C. Reactive Mindware Cell Formula
> ✗ `COUNT(…)` and `&&` between a query and `aura.validate` are not 0.1 cell syntax;
> cells are Pure (`= expr`) and `aura.validate` is fine but `COUNT` is not a 0.1 binding.
```vibe
// Evaluated within a <q-cell> in Poet
=COUNT(graph.query(?s, :hasCondition, :Diabetes) && aura.validate(?s, "schema:ActivePatient"))
```

### D. Generative Articulatory Speech Synthesis (Cross-Modal Pulse)
> ✗ `on_event`, `graph.resolve_phonology`, `hmc.get_mesh`, `pulse.speak` are not 0.1.
> These are post-0.1 `capability.invoke` families, not first-class grammar.
```vibe
// Project a multilingual phoneme sequence through a .d10 vocal tract profile
on_event(user_query) {
    let phonemes = graph.resolve_phonology("concept:Greeting", "lang:example_unencoded");
    let vocal_tract = hmc.get_mesh("asset://tracts/avatar_voice.d10");
    
    // Generates real-time acoustic wave & exact 3D avatar lip-sync
    pulse.speak(phonemes, vocal_tract, [emotion: "warm", pitch_contour: 1.05]);
}
```

---

## 5. Execution Models & Semantic Backends

VibeScript supports three interoperable execution models, enabling zero runtime overhead on resource-constrained devices:

### A. Homoiconic CBOR-LD AST (Code as a Graph)
In VibeScript, code is an RDF Graph. When authored in Poet, scripts are parsed into an Abstract Syntax Tree (AST) composed of Nquin nodes and compressed into **CBOR-LD 1.0**:
- **Zero Text Parsing at Runtime:** When loaded in WASM, Mobile, or IoT nodes, the engine executes logic by traversing binary graph nodes without string lexing or parsing.
- **Cryptographic Provenance:** The AST graph is an immutable node in the Q42 Merkle DAG, allowing script executions to be verified against author DIDs.

### B. SHACL-AF (SHACL Advanced Features) & ShEx Integration
The `aura` namespace maps directly to QualiaDB’s native `qualia_core_db::shacl_compiler` and ShEx engine:

| VibeScript `aura` Construct | SHACL-AF / ShEx Mapping | QualiaDB Native Backend |
| :--- | :--- | :--- |
| `aura.validate(node, shape)` | `sh:NodeShape` / ShEx Shape conformance | `qualia_core_db::shacl_compiler` |
| `aura.infer(graph)` | `sh:SPARQLRule` / `sh:TripleRule` (SHACL-AF) | In-memory forward-chaining deduction |
| `aura.apply_schema(node, iri)` | Target node binding (`sh:targetNode`) | Graph index mutation |
| Custom `aura` functions | `sh:SPARQLFunction` / `sh:declare` | Custom SPARQL / GeoSPARQL registration |

### C. N3Logic Interoperability
VibeScript hooks compile directly to W3C N3Logic implication rules:
```n3
# N3Logic representation of an on_pulse rule
{ ?stream a pulse:TelemetryStream ; pulse:value ?val . ?val math:greaterThan 85.0 }
  =>
{ :Clinic_A aura:hasStatus snomed:EmergencyStatus .
  :Alert pulse:broadcast "Critical threshold exceeded" } .
```

---

## 6. UI Integration: Poet Hypermedia Components

| Component | Purpose | Semantic / Mindware Role |
| :--- | :--- | :--- |
| `<q-doc>` | Top-level hypermedia container | Poet Mindware canvas (HCF / HMC container asset) |
| `<q-entity>` | Inline annotated text span | Binds to an IRI / RDF Node with byte offsets & Aura |
| `<q-relation>` | Semantic link between entities | Binds to an RDF 1.2 statement (`<< id \| s p o >>`) |
| `<q-event>` | Spatio-temporal event block | Binds to GeoSPARQL / TimeML / Pulse triggers |
| `<q-cell>` | Embedded computational data block | Evaluates reactive VibeScript expressions |
| `<q-graph-view>` | Interactive visual knowledge graph | Renders live Q42 subgraphs and Auras |

---

## 7. Multi-Platform Deployment Matrix

> **⚠ v1.0 destination matrix.** `vibe-0.1` (core §10) claims only
> `native-desktop` and `wasm32`. The `no_std` / Micro-Poet `<2MB` edge row is
> **not** claimed for 0.1 and must not be advertised until measured.

| Platform Target | Host Runtime | Poet Engine Mode | UI Rendering Pipeline | Wire Format & Sync |
| :--- | :--- | :--- | :--- | :--- |
| **Desktop / Daemon** | Native Rust AOT | Embedded Poet (Rust) | Webizen (`dioxus-desktop` / Wry) | Full Q42 DAG, Bao Stream, CBOR-LD |
| **Browser Client** | `wasm32-unknown-unknown` | Poet WASM interpreter | Vanilla DOM / Web Components | WebSockets / WebRTC, CBOR-LD |
| **Mobile (iOS/Android)** | Compiled `.a` / `.so` FFI | Pure Rust AST (Zero JIT, App Store compliant) | `dioxus-mobile` / WKWebView | Mobile Network, Merkle Sync |
| **IoT / Edge Devices** | Stripped Native Rust (`no_std`) | Micro-Poet (<2MB RAM) | Headless / Custom Sensor Bus | LoRaWAN / CoAP / MQTT, CBOR-LD Deltas |
