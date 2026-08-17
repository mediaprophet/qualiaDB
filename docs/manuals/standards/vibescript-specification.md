# VibeScript & Poet Engine: Technical Specification
> *"Poet: Define your mindware, with VibeScript"*

**Copyright © 2026 Timothy Charles Holborn.** All rights reserved.  
**Principal / inventor:** Timothy Charles Holborn &lt;timothy.holborn@gmail.com&gt;

**Normative 0.1 language:** [`vibescript-core.md`](vibescript-core.md). This file is an architectural essay, not the parser contract.

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
HookDecl          ::= ('on_hover' | 'on_click' | 'on_change' | 'on_pulse' | 'on_aura') '(' ParameterList? ')' Block ;

(* --- RDF 1.2 Triple Terms, Nquins & Reification --- *)
TripleTerm        ::= '<<(' Subject Predicate Object ')>>' ;
NquinTerm         ::= '<<[' Subject Predicate Object GraphContext ProvenanceTag? ']>>' ;
ReifiedTriple     ::= '<<' (ReificationId '|')? Subject Predicate Object '>>' AnnotationBlock? ;
ReificationId     ::= IRI | BlankNode | Identifier ;
AnnotationBlock   ::= '[' PropertyValuePair (',' PropertyValuePair)* ']' ;
PropertyValuePair ::= (IRI | Identifier) ':' Expression ;

Subject           ::= IRI | BlankNode | TripleTerm | NquinTerm | Identifier ;
Predicate         ::= IRI | Identifier ;
Object            ::= IRI | BlankNode | Literal | TripleTerm | NquinTerm | Identifier ;
GraphContext      ::= IRI | BlankNode | Identifier ;
ProvenanceTag     ::= StringLiteral | Identifier ;

(* --- First-Class Namespaces: Graph, Pulse, Aura & Spatial --- *)
GraphMutation     ::= 'graph' '.' ('assert' | 'retract' | 'commit_delta') '(' (ReifiedTriple | Expression) ')' ';' ;
GraphQuery        ::= 'graph' '.' ('query' | 'sparql') '(' (PatternMatch | StringLiteral) ')' ;
PatternMatch      ::= 'match' '(' PatternElement (',' PatternElement)* ')' ;
PatternElement    ::= '?' Identifier | IRI | Literal | '_' ;

PulseDecl         ::= 'pulse' '.' ('broadcast' | 'emit' | 'subscribe') '(' ArgumentList? ')' ;
AuraDecl          ::= 'aura' '.' ('apply_schema' | 'validate' | 'resolve_context' | 'infer') '(' ArgumentList? ')' ;

SpatialExpr       ::= 'space' '.' Identifier '(' ArgumentList? ')'
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
StringLiteral     ::= '"' [^"]* '"' | '`' [^`]* '`' ;
NumericLiteral    ::= [0-9]+ ('.' [0-9]+)? ;
BooleanLiteral    ::= 'true' | 'false' ;
Identifier        ::= [a-zA-Z_][a-zA-Z0-9_]* ;
```

---

## 4. Idiomatic VibeScript Examples

### A. Reacting to a Network "Pulse" to Update an "Aura"
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
```vibe
// Evaluated within a <q-cell> in Poet
=COUNT(graph.query(?s, :hasCondition, :Diabetes) && aura.validate(?s, "schema:ActivePatient"))
```

### D. Generative Articulatory Speech Synthesis (Cross-Modal Pulse)
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

| Platform Target | Host Runtime | Poet Engine Mode | UI Rendering Pipeline | Wire Format & Sync |
| :--- | :--- | :--- | :--- | :--- |
| **Desktop / Daemon** | Native Rust AOT | Embedded Poet (Rust) | Webizen (`dioxus-desktop` / Wry) | Full Q42 DAG, Bao Stream, CBOR-LD |
| **Browser Client** | `wasm32-unknown-unknown` | Poet WASM interpreter | Vanilla DOM / Web Components | WebSockets / WebRTC, CBOR-LD |
| **Mobile (iOS/Android)** | Compiled `.a` / `.so` FFI | Pure Rust AST (Zero JIT, App Store compliant) | `dioxus-mobile` / WKWebView | Mobile Network, Merkle Sync |
| **IoT / Edge Devices** | Stripped Native Rust (`no_std`) | Micro-Poet (<2MB RAM) | Headless / Custom Sensor Bus | LoRaWAN / CoAP / MQTT, CBOR-LD Deltas |
