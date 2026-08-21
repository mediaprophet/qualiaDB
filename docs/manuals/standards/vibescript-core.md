# VibeScript Core Language — Version 0.1

**Status:** Normative for implementation (conformance verified: 64 conformance tests across 9 domain verticals)  
**Language version:** `vibe-0.1`  
**Engine:** Poet  
**Copyright © 2026 Timothy Charles Holborn.** All rights reserved.  
**Principal / inventor:** Timothy Charles Holborn &lt;timothy.holborn@gmail.com&gt;  
Assignment: [`COPYRIGHT.md`](../COPYRIGHT.md) · Licence: [`LICENSE`](../LICENSE)

This document **is** the implementable language.  
[`scriptingLang.md`](scriptingLang.md) is the architectural essay. It is **not** normative.

Consult inputs incorporated here (not restated as competing specs):

| Source | What this document takes |
|---|---|
| `consult/20260815_codex.md` | Small closed grammar; generated capability bindings; no raw NQuin overlays; exact RDF 1.2 Turtle spellings; QApp stays declarative; effects, budgets, receipts; staged delivery |
| `consult/20260815_qapp-script-gemini.md` | Three-tier split (UI / interpreted DSL / compiled engine); document+graph DSL not a JS replacement; zero JIT (iOS); mobile FFI and edge profiles as *profiles*, not extra grammars |
| `consult/branding.md` | Poet = engine; Vibe / VibeScript = language; Pulse = transport/events; Aura = ontology/schema |
| `consult/Rust Extensible Runtime Architecture.pdf` | Scripts call **capabilities**, never plugins. Sandboxed extensions = WASM components. Hardware/GPU/HSM/codecs = native child processes under Qualia, registered into the same capability table |

RFC 2119 words (`MUST`, `MUST NOT`, `SHOULD`, `MAY`) apply only in numbered normative sections.

---

## 0. What 0.1 is, and what it is not

**Vibe 0.1 is** a typed, capability-bounded, non-JIT interpreted language for:

- reactive HCF cell formulas (Pure only);
- signed document/agent modules that query and transact on a graph snapshot;
- host event handlers (`on pulse…`, `on ui…`) that the Poet host dispatches.

**Vibe 0.1 is not** a general-purpose language, a JavaScript clone, a second Webizen VM opcode set, a replacement for `SlgOpcode`, or a surface that authors raw 48-byte Quin overlays. It MUST NOT grow a DOM or `eval`. Replacing JavaScript as Qualia’s *application* language is the product destination; 0.1 is the closed core that destination builds on, not a denial of it. The matching wire is CBOR-LD (HCF), not JSON — Canonical AST in the table below.

A package that uses only this document’s grammar, types, effects, and the **0.1 binding profile** (§11) is a conforming 0.1 program. Later bindings (`geom`, `audio`, `model`, `extension`) MUST NOT change the grammar.

---

## 1. Artifacts (do not conflate)

| Artifact | Form | Role |
|---|---|---|
| Source | UTF-8 `.vibe` | Human-authored, hashed |
| Cell body | UTF-8 starting with `=` | Pure formula; only legal in an HCF/QApp cell, not in a `.vibe` module |
| Canonical AST | versioned nodes (later RDF/CBOR-LD) | Interchange / agents |
| Bytecode | `vibe-bc-0.1` (implemented) | Stack-based VM; binary encode/decode; optional fast path |
| Package manifest | later | Capabilities, profile, hashes, signature |

0.1 implementers MUST parse source → typed AST → evaluate. Bytecode (`vibe-bc-0.1`) is implemented as an optional execution path: `poet_vibe::bytecode::compile` compiles a checked AST into a `Chunk`, `poet_vibe::bytecode::Vm` executes it, and `poet_vibe::bytecode::encode_chunk`/`decode_chunk` provide binary serialization. The bytecode VM supports arithmetic, comparison, logical operators, control flow (if/else, while, for, match), user-defined functions with proper local scoping, list/record construction and access, and host capability dispatch.

---

## 2. Source text and lexer

1. Source MUST be UTF-8. A leading U+FEFF BOM, if present, MUST be stripped and MUST NOT enter the source hash.
2. Newlines `LF` and `CRLF` are equivalent. Canonical hash form is LF-only.
3. Identifiers follow UAX #31 with XID_Continue classification. ASCII `[A-Za-z_][A-Za-z0-9_]*` is the base; Unicode identifiers are accepted subject to the T40 security policy (BiDi control rejection per CVE-2021-42574, homoglyph/confusable detection per TR39, mixed-script restriction, max 255 chars). Identifiers are normalized to NFC form.
4. Keywords (reserved):  
   `module` `import` `as` `prefix` `requires` `capability` `fn` `async` `on` `let` `mut` `const` `if` `else` `for` `in` `while` `match` `return` `yield` `transaction` `await` `true` `false` `null` `effect`
5. Comments: `//` to end of line; `/* … */` non-nested. Comments are not tokens.
6. Strings: `"…"` with escapes `\\` `\"` `\n` `\r` `\t` `\u{XXXX}`. No interpolation in 0.1. No backtick strings in 0.1.
7. Integers: decimal digits, optional `_` separators, optional suffix `i32` `u32` `i64` `u64`. Unsuffixed integer is `i64`. Hex `0x…` is `u64` if unsuffixed.
8. Floats: require a `.` or exponent, optional suffix `f32` `f64`. Unsuffixed float is `f64`. `NaN` / infinities are not literals.
9. **IRI vs relational `<`:** an IRI token is `<` immediately followed by a non-whitespace character and consumed until the next unescaped `>`. Relational `<` `>` `<=` `>=` MUST be separated from both operands by whitespace. `a<b` is a parse error.
10. Prefixed names: `prefix:local` where `local` is `[A-Za-z_][A-Za-z0-9_.-]*`.
11. Blank nodes: `_:label`.
12. Query variables: `?` Identifier. Legal in any expression (type `Var`).
13. Triple term opener is the three-character sequence `<<(` ; closer `)>>`.  
    Reifying opener is `<<` not followed by `(` ; closer `>>`.

---

## 3. Grammar (normative)

Every fenced `vibe` example in **this** document MUST be accepted by this grammar. Examples in older essays are non-normative.

```ebnf
Program        ::= ModuleDecl? ImportDecl* PrefixDecl* RequiresDecl? Item* ;
ModuleDecl     ::= 'module' (IRI | Identifier) ';' ;
ImportDecl     ::= 'import' StringLiteral ('as' Identifier)? ';' ;
PrefixDecl     ::= 'prefix' Identifier ':' IRI ';' ;
RequiresDecl   ::= 'requires' '[' CapList? ']' ';' ;
CapList        ::= CapSpec (',' CapSpec)* ;
CapSpec        ::= 'capability' '(' StringLiteral (',' NamedArg)* ')' ;

Item           ::= FunctionDecl | HookDecl | ConstDecl | Statement ;
ConstDecl      ::= 'const' Identifier (':' Type)? '=' Expression ';' ;
FunctionDecl   ::= EffectClass? 'async'? 'fn' Identifier '(' Params? ')'
                   BudgetClause? ('->' Type)? Block ;
HookDecl       ::= 'on' EventPath '(' Params? ')' BudgetClause? ('->' Type)? Block ;
EventPath      ::= Identifier (':' Identifier)* ;
EffectClass    ::= 'pure' | 'hot' | 'cold' | 'async' | 'effect' ;
BudgetClause   ::= 'budget' '(' NamedArg (',' NamedArg)* ')' ;

Params         ::= Param (',' Param)* ;
Param          ::= Identifier ':' Type ;
NamedArg       ::= Identifier ':' Expression ;

Statement      ::= LetStmt | AssignStmt | IfStmt | ForStmt | WhileStmt
                 | MatchStmt | ReturnStmt | YieldStmt | TransactionStmt
                 | EffectStmt | Expression ';' ;
LetStmt        ::= 'let' 'mut'? Identifier (':' Type)? ('=' Expression)? ';' ;
AssignStmt     ::= PostfixExpr '=' Expression ';' ;
IfStmt         ::= 'if' Expression Block ('else' (IfStmt | Block))? ;
ForStmt        ::= 'for' Identifier 'in' Expression Block ;
WhileStmt      ::= 'while' Expression Block ;
MatchStmt      ::= 'match' Expression '{' MatchArm* '}' ;
MatchArm       ::= Pattern '=>' (Block | (Expression ',')) ;
ReturnStmt     ::= 'return' Expression? ';' ;
YieldStmt      ::= 'yield' Expression? ';' ;
TransactionStmt::= 'transaction' ('(' NamedArg (',' NamedArg)* ')')? Block ;
EffectStmt     ::= 'effect' Expression ';' ;
Block          ::= '{' Statement* '}' ;

(* Cell host only — not part of Program *)
CellBody       ::= '=' Expression ;

Expression     ::= LogicalOr ;
LogicalOr      ::= LogicalAnd ('||' LogicalAnd)* ;
LogicalAnd     ::= Equality ('&&' Equality)* ;
Equality       ::= Relational (('==' | '!=') Relational)* ;
Relational     ::= Additive (('<' | '<=' | '>' | '>=') Additive)* ;
Additive       ::= Multiplicative (('+' | '-') Multiplicative)* ;
Multiplicative ::= Unary (('*' | '/' | '%') Unary)* ;
Unary          ::= ('!' | '-' | '+' | 'await')? PostfixExpr ;
PostfixExpr    ::= Primary ('.' Identifier | '(' Args? ')' | '[' Expression ']' | '?')* ;
Args           ::= Arg (',' Arg)* ;
Arg            ::= (Identifier ':')? Expression ;

Primary        ::= Literal | Identifier | QueryVar | IRI | PrefixedName
                 | BlankNode | TripleTerm | ReifiedTerm
                 | ListLiteral | RecordLiteral | '(' Expression ')' ;

Literal        ::= StringLiteral | IntegerLiteral | FloatLiteral
                 | 'true' | 'false' | 'null' ;
ListLiteral    ::= '[' (Expression (',' Expression)*)? ']' ;
RecordLiteral  ::= '{' (NamedArg (',' NamedArg)*)? '}' ;

TripleTerm     ::= '<<(' Subject Predicate Object ')>>' ;
ReifiedTerm    ::= '<<' Subject Predicate Object '~' Reifier '>>' ;
Subject        ::= IRI | PrefixedName | BlankNode | TripleTerm | Identifier | QueryVar ;
Predicate      ::= IRI | PrefixedName | Identifier ;
Object         ::= Subject | Literal ;
Reifier        ::= IRI | PrefixedName | BlankNode | Identifier ;

Type           ::= TypeName ('<' Type (',' Type)* '>')? ;
TypeName       ::= 'i32' | 'u32' | 'i64' | 'u64' | 'f32' | 'f64'
                 | 'bool' | 'string' | 'bytes'
                 | 'Iri' | 'BlankNode' | 'Did' | 'Hash' | 'Var'
                 | 'Literal' | 'TripleTerm' | 'Reifier' | 'Quin' | 'QuinRef'
                 | 'AssetRef' | 'TensorRef' | 'GeometryRef'
                 | 'Option' | 'Result' | 'List' | 'Record'
                 | 'Receipt' | 'Stream' | 'Future' | Identifier ;

Pattern        ::= '_' | Identifier | Literal | 'Ok' '(' Pattern ')'
                 | 'Err' '(' Pattern ')' | 'Some' '(' Pattern ')' | 'None' ;
```

### 3.1 Loop bound rule

A `for` or `while` that the type checker cannot prove bounded MUST be rejected. A loop is bounded if:

- the iterated value has type `List<T>` with a static or `budget` cardinality cap; or
- the iterator is produced by a binding that declares a max length (e.g. `graph.query(…, take: 64)`); or
- a `budget(steps: N)` on the enclosing function/hook remains the hard cap.

Unbounded recursion is forbidden. Maximum call depth default is 64.

---

## 4. Types

| Type | Notes |
|---|---|
| `i32` `u32` `i64` `u64` `f32` `f64` `bool` | Overflow on signed/unsigned integer ops is an evaluation error, not wrap, unless the binding documents wrap. Division by zero is an error. `f64` NaN comparisons are false. |
| `string` `bytes` | UTF-8 / raw. Length counts against workspace budget. |
| `Iri` `BlankNode` `Did` `Hash` | Distinct. A `u64` hash is **not** an `Iri`. |
| `Literal` | RDF literal (value + optional datatype/lang). |
| `TripleTerm` | RDF 1.2 triple term. Unasserted until committed. |
| `Reifier` | IRI/blank of a reifying triple. |
| `Var` | Query variable. |
| `Quin` | Sealed 48-byte statement produced **only** by `quin.statement` / host constructors. |
| `QuinRef` `AssetRef` `TensorRef` `GeometryRef` | Opaque handles. Dense media MUST NOT be copied into ordinary values. |
| `Option<T>` `Result<T,E>` | `?` unwraps `Err`/`None` as an early return of the enclosing `Result`. |
| `List<T>` | Bounded. Default max 4096 elements unless budget says less. |
| `Record` | Closed at type-check when a shape is known; otherwise a bounded map. |
| `Receipt<T>` | Result of an external effect. |
| `Stream<T>` `Future<T>` | Only in `async` functions or `AsyncRequired` hooks. |

JavaScript hosts MUST carry `u64` / `Hash` / Quin field values as strings or BigInt, never IEEE-754 `Number`.

---

## 5. Effects

Every function and hook has exactly one effect class:

| Class | Keyword | May do | Must not do |
|---|---|---|---|
| Pure | `pure` (default for cells and `const`) | Compute on values/snapshots | Graph write, I/O, time, entropy, devices, models |
| Hot | `hot` | Zero-heap work on caller buffers | Allocate, I/O, query the live graph |
| Cold | `cold` | Bounded workspace allocation | Network, credentials, unbounded query |
| Async | `async` | Jobs, streams, `await` | Block the UI tick |
| External | `effect` | Graph commit, pulse publish, UI mutate, extension call | Anything undeclared in `requires` |

Effect is inferred if omitted: a body that only uses Pure bindings is Pure; any `graph.stage` / `pulse.*` / `effect` statement raises it to External. A Pure cell that would infer External MUST be rejected.

`on tick` handlers, if present, MUST be `hot` and MUST NOT query the graph or allocate.

---

## 6. Evaluation and reactivity

1. A hook sees one immutable **graph revision** (snapshot) for the duration of the call.
2. `transaction { … }` stages assertions/retractions, then atomically: datatype check → shape (`aura`) → deontic/sensitivity/capability → seal Quins → commit or reject with no partial writes.
3. Successful commit returns `Receipt<GraphCommit>` and emits a deterministic graph-change pulse.
4. Reactive cells recompute when a declared dependency (explicit `graph.query` / cell refs) changes. Cycles MUST be detected and fail the cell.
5. `time.unix()` is External (or forbidden in Pure). Replay uses the receipt’s recorded clock.

---

## 7. RDF 1.2 and Quin

### 7.1 RDF 1.2 (exact Turtle 1.2 spellings)

- Triple term: `<<( s p o )>>` — unasserted proposition.
- Reifying triple: `<< s p o ~ reifier >>` — RDF 1.2 explicit reifier.

Vibe MUST NOT call any other spelling “RDF 1.2”. Older `<< id | s p o >>` is illegal.

Asserted vs unasserted: a term is data until `graph.stage` / `graph.commit` inside a transaction.

### 7.2 Quin (implemented ABI, not a source literal)

The live `NQuin` is six `u64` fields: `subject`, `predicate`, `object`, `context`, `metadata`, `parity`.

Vibe 0.1 MUST NOT offer a source literal that pretends the fifth field is provenance or lets scripts write `parity`.

```vibe
let q: Quin = quin.statement(
    subject: subj,
    predicate: pred,
    object: obj,
    context: ctx
)?;
```

The host seals parity and metadata. Scripts MAY read `q.subject` etc. as `u64` **only** through typed accessors that do not expose overlay mutation.

Provenance is a graph/receipt: source content hash, UTF-8 byte span, principal DID, transform, confidence — not a fifth Quin field.

---

## 8. Modules, capabilities, authority

```vibe
module <https://qualiadb.org/modules/example>;
import "vibe:0.1/graph" as graph;
prefix q: <https://qualiadb.org/schema/>;
requires [
    capability("graph.read", context: q:default),
    capability("graph.write", context: q:default),
    capability("aura.validate")
];
```

1. A module with no `requires` has **no** ambient authority: no filesystem, network, clock, graph write, model, or extension.
2. The host supplies only granted imports. Missing required capability ⇒ load-time error.
3. QApps stay **declarative**. A QApp MAY reference a signed Vibe package by content hash. It MUST NOT embed effectful source as anonymous document strings.
4. Inline HCF `<q-cell>` bodies are `CellBody` and MUST type-check as Pure.
5. Extensions (WASM component or native child process) appear only as new capability IDs in the registry. They never add keywords.

Capability IDs are strings matching existing Qualia families where possible (`graph.read`, `aura.validate`, `pulse.publish`, `logic.deontic.evaluate`). Maturity (`stable` / `partial` / `experimental` / `fail-closed`) comes from `CAPABILITY_DESCRIPTORS` and per-op manifests — not from this grammar.

---

## 9. Diagnostics

Every reject MUST produce:

- stable code (`E001` parse, `E100` type, `E200` effect, `E300` capability, `E400` budget, `E500` policy);
- primary UTF-8 byte span;
- optional notes and a safe fix that MUST NOT add authority.

---

## 10. Runtime profiles (0.1)

| Profile | Host | Poet mode | 0.1 obligation |
|---|---|---|---|
| `native-desktop` | Qualia / Webizen Desktop | AST interpreter in-process | Required |
| `wasm32` | Browser / studio wasm | same interpreter, `wasm32-unknown-unknown` | Required |
| `native-mobile` | iOS/Android FFI | same, **no JIT** | Source-compatible; crate feature later |
| `edge` | stripped host | subset: Pure + declared `pulse` | Not claimed until measured |

`qualia-core-db` is `std`. 0.1 MUST NOT claim `no_std` or “&lt;2MB Micro-Poet.”

wgpu version, AVX-512, raw sockets, and Bao/HMC v2 are **not** language semantics.

Canonical 10D extension is **`.10d`**. Spec prose that says `.d10` is non-normative alias only.

---

## 11. Binding profile 0.1 (minimum to implement)

These are **library functions**, not grammar. Hosts MUST implement the ones marked required on `native-desktop` and `wasm32` before claiming 0.1.

| Binding | Effect | Required | Notes |
|---|---|---|---|
| `math.abs` `math.min` `math.max` `math.clamp` | Pure | yes | i64/f64 overloads |
| `math.sqrt` `math.sin` `math.cos` `math.tan` | Pure | yes | f64 only |
| `math.exp` `math.log` `math.log10` | Pure | yes | natural log and base-10 |
| `math.pow` `math.atan` `math.atan2` | Pure | yes | power and inverse trig |
| `math.floor` `math.ceil` `math.round` | Pure | yes | i64/f64 overloads |
| `time.unix()` | External | no | forbidden in Pure cells |
| `rdf.triple(s,p,o)` | Pure | yes | builds `TripleTerm` |
| `rdf.reify(term, reifier)` | Pure | yes | |
| `quin.statement(…)` | Pure (seal is host) | yes | fail if host cannot seal |
| `graph.snapshot()` | Pure | yes | current revision handle |
| `graph.query(s,p,o, take: n)` | Pure | yes | `s`/`p`/`o` may be `Var`; MUST take `take` |
| `graph.stage(term)` | External | yes | only in `transaction` |
| `graph.commit()` | External | yes | only in `transaction`; returns `Receipt` |
| `aura.validate(node, shape)` | Pure | yes | SHACL subset actually implemented |
| `pulse.publish(topic, payload)` | External | yes | destination allowlisted |
| `capability.resolve(id)` | Pure | yes | inspect registry; no invoke |
| `capability.invoke(id, args)` | External | no | 0.1 MAY omit; if present, still gated |

> **Note on Time Bindings (0.1 vs Post-0.1):** `time.unix() -> i64` (seconds) is the 0.1 binding. Structured `Instant`, `time.unix_nanos`, and `time.monotonic_nanos` are post-0.1 (per the decisions register X6 and vibe-design to-do T19).

Logic, geometry, inference, vision, audio, and extension codecs are **out of 0.1** except as later `capability.invoke` IDs.

### 11.1 Cosmic coordinate bindings (OCS)

The Omniversal Coordinate System (OCS) bindings expose the `poet_vibe::cosmic` library via `capability.invoke` IDs in the `Cosmic.*` namespace. These are post-0.1 extensions available on `native-desktop` targets.

| Binding | Effect | Input | Output |
|---|---|---|---|
| `Cosmic.geodetic_to_ecef` | Pure | `{ lat_deg, lon_deg, alt_m }` | `{ x, y, z }` |
| `Cosmic.ecef_to_geodetic` | Pure | `{ x, y, z }` | `{ lat_deg, lon_deg, alt_m }` |
| `Cosmic.ecef_to_enu` | Pure | `{ x, y, z, ref_lat_deg, ref_lon_deg, ref_alt_m }` | `{ east, north, up }` |
| `Cosmic.enu_to_ecef` | Pure | `{ east, north, up, ref_lat_deg, ref_lon_deg, ref_alt_m }` | `{ x, y, z }` |
| `Cosmic.geodetic_distance` | Pure | `{ lat_deg, lon_deg, lat2_deg, lon2_deg }` | `f64` (meters) |
| `Cosmic.body_profile` | Pure | `{ name: string }` | `{ name, class, equatorial_radius_m, mass_kg, rotation_period_s }` |
| `Cosmic.surface_gravity` | Pure | `{ name: string }` | `f64` (m/s²) |
| `Cosmic.flrw_distance` | Pure | `{ z: f64 }` | `f64` (meters, comoving) |
| `Cosmic.flrw_redshift` | Pure | `{ a_emit: f64 }` | `f64` (redshift z) |
| `Cosmic.flrw_hubble_velocity` | Pure | `{ distance_m: f64 }` | `f64` (m/s) |
| `Cosmic.stardate_to_gregorian` | Pure | `{ stardate: f64 }` | `f64` (Gregorian year) |
| `Cosmic.warp_velocity` | Pure | `{ warp: f64, scale: "tos"\|"tng" }` | `f64` (m/s) |
| `Cosmic.cochrane_units` | Pure | `{ warp: f64, scale: "tos"\|"tng" }` | `f64` (cochranes) |
| `Cosmic.atmosphere_pressure` | Pure | `{ body: string, altitude_m: f64 }` | `f64` (Pa) |
| `Cosmic.atmosphere_temperature` | Pure | `{ body: string, altitude_m: f64 }` | `f64` (K) |
| `Cosmic.magnetosphere_field` | Pure | `{ body: string, distance_m: f64, body_radius_m: f64 }` | `f64` (T) |
| `Cosmic.scale_factor` | Pure | `{ from_level: string, to_level: string }` | `f64` |
| `Cosmic.compton_wavelength` | Pure | `{ particle: "electron"\|"proton" }` | `f64` (meters) |
| `Cosmic.de_broglie_wavelength` | Pure | `{ particle: string, velocity_m_s: f64 }` | `f64` (meters) |
| `Cosmic.usri_parse` | Pure | `{ uri: string }` | `Record` (parsed USRI) |

Hierarchy levels for `Cosmic.scale_factor`: `L-2` through `L12` (Planck scale through cosmological horizon).

### 11.2 Computational economics bindings

Post-0.1 extensions exposing the `specialized_libs::computational_economics` backend via `capability.invoke` IDs in the `Econ.*` namespace. Available on `native-desktop` and `wasm32` (with `wasm-scientific` feature).

| Binding | Effect | Input | Output |
|---|---|---|---|
| `Econ.capm_expected_return` | Pure | `{ risk_free, beta, market_return }` | `f64` |
| `Econ.capm_beta` | Pure | `{ asset_returns: [f64], market_returns: [f64] }` | `f64` |
| `Econ.gordon_growth_price` | Pure | `{ dividend, growth_rate, discount_rate }` | `f64` |
| `Econ.multi_period_ddm` | Pure | `{ dividends: [f64], terminal_price, discount_rate }` | `f64` |
| `Econ.ccapm_equity_premium` | Pure | `{ gamma, consumption_growth }` | `f64` |
| `Econ.lucas_asset_price` | Pure | `{ dividend_paths: [f64], consumption_paths: [f64], n_paths, n_periods, beta, gamma }` | `f64` |
| `Econ.black_scholes` | Pure | `{ spot, strike, rate, vol, time, kind: "call"\|"put", dividend_yield }` | `{ price, delta, gamma, vega, theta, rho }` |
| `Econ.binomial_option` | Pure | `{ spot, strike, rate, vol, time, steps, kind, style: "european"\|"american", dividend_yield }` | `f64` |
| `Econ.put_call_parity` | Pure | `{ spot, strike, rate, time, dividend_yield }` | `{ call, put }` |
| `Econ.prospect_value` | Pure | `{ x, alpha, beta, lambda }` | `f64` |
| `Econ.probability_weight` | Pure | `{ p, gamma }` | `f64` |
| `Econ.hyperbolic_discount` | Pure | `{ t, beta, delta }` | `f64` |
| `Econ.endowment_effect_wta` | Pure | `{ wtp, lambda }` | `f64` |
| `Econ.present_biased_utility` | Pure | `{ utilities: [f64], beta, delta }` | `f64` |
| `Econ.reference_dependent_utility` | Pure | `{ x, reference, alpha, beta, lambda }` | `f64` |
| `Econ.mixed_nash_2x2` | Pure | `{ row_payoffs: [f64], col_payoffs: [f64] }` | `{ p1, p2, q1, q2 }` |
| `Econ.cournot_duopoly` | Pure | `{ a, b, c1, c2 }` | `{ q1, q2, price }` |
| `Econ.bertrand_duopoly` | Pure | `{ c1, c2 }` | `{ p1, p2 }` |
| `Econ.bertrand_with_demand` | Pure | `{ demand_intercept, demand_slope, cost_1, cost_2 }` | `{ price, quantity }` |
| `Econ.stackelberg_duopoly` | Pure | `{ a, b, c1, c2 }` | `{ q1, q2, price }` |
| `Econ.pure_nash_equilibria` | Pure | `{ payoff_row: [f64], payoff_col: [f64], n_row, n_col }` | `{ equilibria: [{row, col}], count }` |
| `Econ.repeated_game_payoff` | Pure | `{ stage_payoffs: [f64], discount, n_rounds }` | `f64` |
| `Econ.solow_steady_state` | Pure | `{ s, alpha, delta, n, g }` | `{ k_star, y_star }` |
| `Econ.olg_steady_state` | Pure | `{ alpha, beta, n_pop_growth }` | `{ k_star, c_star }` |
| `Econ.ramsey_steady_state` | Pure | `{ alpha, beta, delta }` | `f64` |
| `Econ.ramsey_euler_residual` | Pure | `{ capital, capital_next, beta, alpha, delta, sigma }` | `f64` |
| `Econ.new_keynesian_solve` | Pure | `{ r_prev, beta, kappa, sigma, phi_pi, phi_y, rho_r, r_nat }` | `{ output_gap, inflation, interest_rate }` |
| `Econ.gini_coefficient` | Pure | `{ incomes: [f64] }` | `f64` |
| `Econ.lorenz_curve` | Pure | `{ incomes: [f64] }` | `{ lorenz_points: [f64], count }` |
| `Econ.atkinson_inequality` | Pure | `{ incomes: [f64], epsilon }` | `f64` |
| `Econ.headcount_poverty` | Pure | `{ incomes: [f64], poverty_line }` | `{ headcount, poverty_rate }` |
| `Econ.poverty_gap` | Pure | `{ incomes: [f64], poverty_line }` | `f64` |
| `Econ.utilitarian_welfare` | Pure | `{ utilities: [f64] }` | `f64` |
| `Econ.rawlsian_welfare` | Pure | `{ utilities: [f64] }` | `f64` |
| `Econ.nash_welfare` | Pure | `{ utilities: [f64] }` | `f64` |
| `Econ.npv` | Pure | `{ benefits: [f64], costs: [f64], discount_rate, n_periods }` | `f64` |
| `Econ.distributional_npv` | Pure | `{ benefits: [f64], costs: [f64], weights: [f64], discount_rate, n_periods }` | `{ weighted_npv, unweighted_npv, assumptions }` |
| `Econ.mean_return` | Pure | `{ returns: [f64] }` | `f64` |
| `Econ.sample_variance` | Pure | `{ returns: [f64] }` | `f64` |
| `Econ.portfolio_returns` | Pure | `{ asset_returns: [f64], weights: [f64], n_periods, n_assets }` | `{ portfolio_returns: [f64], count }` |
| `Econ.covariance_matrix` | Pure | `{ returns: [f64], n_periods, n_assets }` | `{ covariance: [f64], n_assets }` |
| `Econ.portfolio_variance` | Pure | `{ weights: [f64], covariance: [f64], n_assets }` | `f64` |
| `Econ.max_drawdown` | Pure | `{ returns: [f64] }` | `f64` |
| `Econ.historical_var` | Pure | `{ returns: [f64], confidence }` | `f64` |
| `Econ.historical_cvar` | Pure | `{ returns: [f64], confidence }` | `f64` |
| `Econ.gaussian_var` | Pure | `{ mean, std_dev, confidence }` | `f64` |
| `Econ.simple_returns` | Pure | `{ prices: [f64] }` | `{ returns: [f64], count }` |
| `Econ.log_returns` | Pure | `{ prices: [f64] }` | `{ returns: [f64], count }` |
| `Econ.cumulative_wealth` | Pure | `{ returns: [f64] }` | `{ wealth: [f64], count }` |
| `Econ.drawdown` | Pure | `{ wealth: [f64] }` | `{ drawdown: [f64], count }` |
| `Econ.rolling_mean` | Pure | `{ values: [f64], window }` | `{ rolling_mean: [f64], count }` |
| `Econ.rolling_variance` | Pure | `{ values: [f64], window }` | `{ rolling_variance: [f64], count }` |
| `Econ.autocorrelation` | Pure | `{ values: [f64], lag }` | `f64` |
| `Econ.cross_correlation` | Pure | `{ a: [f64], b: [f64], lag }` | `f64` |
| `Econ.gbm_simulate` | Pure | `{ s0, mu, sigma, dt, n_steps, seed }` | `{ path: [f64] }` |
| `Econ.stress_scenario` | Pure | `{ returns: [f64], shock }` | `{ stressed_returns: [f64], count }` |
| `Econ.block_bootstrap` | Pure | `{ returns: [f64], block_size, n_resamples, seed }` | `{ bootstrap_means: [f64] }` |
| `Econ.interpolate_zero_rate` | Pure | `{ maturities: [f64], rates: [f64], target_maturity }` | `{ zero_rate }` |
| `Econ.discount_factor` | Pure | `{ maturities: [f64], rates: [f64], target_maturity, compounding_per_year }` | `{ discount_factor }` |
| `Econ.forward_rate` | Pure | `{ maturities: [f64], rates: [f64], t1, t2, compounding_per_year }` | `{ forward_rate }` |
| `Econ.par_yield` | Pure | `{ maturities: [f64], rates: [f64], target_maturity, compounding_per_year }` | `{ par_yield }` |
| `Econ.gravity_flow` | Pure | `{ mass_1, mass_2, distance, alpha, beta, gamma }` | `{ flow }` |
| `Econ.total_transport_cost` | Pure | `{ flows: [f64], distances: [f64], n }` | `{ total_cost }` |
| `Econ.morans_i` | Pure | `{ values: [f64], weights: [f64], n }` | `{ morans_i }` |
| `Econ.nearest_facility` | Pure | `{ demands: [f64], facilities: [f64], n_demands, n_facilities }` | `{ assignments: [u64] }` |
| `Econ.transfer_payment` | Pure | `{ base, income, threshold, phaseout_rate }` | `{ transfer }` |
| `Econ.fiscal_multiplier` | Pure | `{ initial_spending, mpc, leakage_rate }` | `{ gdp_impact }` |
| `Econ.laffer_curve` | Pure | `{ tax_rate, tax_base, elasticity }` | `{ revenue }` |
| `Econ.progressive_tax` | Pure | `{ thresholds: [f64], rates: [f64], income }` | `{ tax_per_bracket: [f64], total_tax, effective_rate }` |
| `Econ.check_ir` | Pure | `{ valuations: [f64], payments: [f64] }` | `{ individually_rational }` |
| `Econ.check_budget_balance` | Pure | `{ payments: [f64] }` | `{ is_balanced, net_transfer }` |
| `Econ.vcg_payment` | Pure | `{ valuations: [f64] }` | `{ payments: [f64], total_revenue }` |
| `Econ.strategy_proofness` | Pure | `{ valuations: [f64], allocation: [bool], payments: [f64] }` | `{ strategy_proof }` |
| `Econ.validate_transition_matrix` | Pure | `{ matrix: [f64], n }` | `{ valid }` |
| `Econ.transition_probability` | Pure | `{ matrix: [f64], n, from, to }` | `{ probability }` |
| `Econ.expected_holding_time` | Pure | `{ matrix: [f64], n, state }` | `{ holding_time }` |
| `Econ.stationary_distribution` | Pure | `{ matrix: [f64], n, max_iter, tolerance }` | `{ distribution: [f64] }` |
| `Econ.simulate_chain` | Pure | `{ matrix: [f64], n, start, n_steps, seed }` | `{ path: [u64] }` |
| `Econ.mean_first_passage` | Pure | `{ matrix: [f64], n, target, max_iter, tolerance }` | `{ first_passage_times: [f64] }` |
| `Econ.labor_supply` | Pure | `{ wage, time_endowment, non_labor_income, alpha }` | `{ labor, consumption }` |
| `Econ.household_production_ces` | Pure | `{ time, goods, alpha, rho }` | `{ output }` |
| `Econ.efficiency_units` | Pure | `{ raw_labor, human_capital }` | `f64` |
| `Econ.social_cost_of_carbon` | Pure | `{ emissions_tons, damage_per_ton }` | `f64` |
| `Econ.pollution_damage` | Pure | `{ emissions, damage_coeff }` | `f64` |
| `Econ.marginal_damage` | Pure | `{ emissions, damage_coeff }` | `f64` |
| `Econ.optimal_pollution` | Pure | `{ baseline_emissions, abatement_coeff, damage_coeff }` | `f64` |
| `Econ.optimal_abatement` | Pure | `{ baseline_emissions, abatement_coeff, damage_coeff }` | `f64` |
| `Econ.abatement_net_benefit` | Pure | `{ baseline_emissions, actual_emissions, abatement_coeff, damage_coeff }` | `f64` |
| `Econ.bellman_update` | Pure | `{ rewards: [f64], transitions: [f64], discount, values: [f64], n_states, n_actions, state }` | `f64` |
| `Econ.value_iteration` | Pure | `{ rewards: [f64], transitions: [f64], discount, n_states, n_actions, max_iter, tolerance }` | `{ values: [f64], policy: [u64] }` |
| `Econ.malfeasance_delta` | Pure | `{ capital_allocated, delivered, inverted }` | `{ capital_allocated, delivered_utility, delta, governance_yield_inverted }` |
| `Econ.narrative_divergence` | Pure | — | `{ status }` (requires NquinVector structs) |
| `Econ.ols` | Pure | `{ x: [f64], y: [f64], n_obs, n_reg }` | `{ beta: [f64], residuals: [f64], r_squared }` |
| `Econ.wls` | Pure | `{ x: [f64], y: [f64], weights: [f64], n_obs, n_reg }` | `{ beta: [f64], r_squared }` |
| `Econ.iv_2sls` | Pure | `{ x_endogenous: [f64], z_instruments: [f64], y: [f64], n_obs, n_reg, n_instr }` | `{ beta: [f64], r_squared }` |
| `Econ.logistic_mle` | Pure | `{ x: [f64], y: [f64], n_obs, n_reg, max_iter, tolerance }` | `{ beta: [f64] }` |
| `Econ.eigenvector_centrality` | Pure | `{ adjacency: [f64], n, max_iter, tolerance }` | `{ centrality: [f64] }` |
| `Econ.degree_centrality` | Pure | `{ adjacency: [f64], n }` | `{ centrality: [f64] }` |
| `Econ.interbank_clearing` | Pure | `{ exposures: [f64], capital: [f64], n, max_rounds, tolerance }` | `{ clearing_payments: [f64] }` |
| `Econ.leontief_inverse` | Pure | `{ matrix: [f64], n, max_rounds, tolerance }` | `{ inverse: [f64] }` |
| `Econ.output_multipliers` | Pure | `{ leontief_inverse: [f64], n }` | `{ multipliers: [f64] }` |
| `Econ.validate_scalar_constraint` | Pure | `{ value, min, max }` | `{ valid }` |
| `Econ.aggregate_wealth` | Pure | — | `{ status }` (requires Agent structs) |
| `Econ.agent_based_aggregate_wealth` | Pure | — | `{ status }` (requires Agent structs) |
| `Econ.aggregate_paper_fills` | Pure | — | `{ status }` (requires Fill structs) |

### 11.3 Multi-agent orchestration bindings

Post-0.1 extensions exposing the agent runtime orchestration layer via `capability.invoke` IDs in the `Orchestration.*` namespace.

| Binding | Effect | Input | Output |
|---|---|---|---|
| `Orchestration.session_create` | External | `{ id: string, task: string }` | `{ id, task, status }` |
| `Orchestration.session_plan` | External | `{ id: string }` | `{ id, step_count, status }` |
| `Orchestration.session_execute` | External | `{ id: string }` | `{ id, status, node_count, results: [{node_id, node_name, success, agent_id}] }` |
| `Orchestration.session_status` | External | `{ id: string }` | `Record` (summary fields) |
| `Orchestration.roster_register` | External | `{ session_id, agent_id, did, name, role?, capabilities?: [string] }` | `{ session_id, agent_id, registered }` |
| `Orchestration.roster_list` | External | `{ session_id: string }` | `{ session_id, agents: [Record], count }` |
| `Orchestration.roster_capabilities` | External | `{ session_id: string }` | `{ session_id, capabilities: [string], count }` |

### 11.4 Asset persistence bindings

Post-0.1 extensions exposing the persistent asset store via `capability.invoke` IDs in the `Asset.persist_*` namespace. These complement the existing transient `Asset.create`/`add_temporal`/`add_topic`/`set_spatial` bindings.

| Binding | Effect | Input | Output |
|---|---|---|---|
| `Asset.persist` | External | `{ asset_id: string, kind: string, owner_did?: string }` | `{ asset_id, persisted }` |
| `Asset.persist_create` | External | `{ asset_id: string, kind: string, owner_did?: string }` | `{ asset_id, kind, persisted }` |
| `Asset.persist_add_temporal` | External | `{ asset_id, kind_iri, seconds, nanos?, duration_secs?, asserted_by?, confidence? }` | `{ asset_id, added }` |
| `Asset.persist_add_topic` | External | `{ asset_id: string, topic: string }` | `{ asset_id, topic, added }` |
| `Asset.persist_set_spatial` | External | `{ asset_id, anchor_iri, latitude?, longitude?, altitude? }` | `{ asset_id, anchor_iri, set }` |
| `Asset.persist_compile` | External | `{ asset_id: string }` | `{ asset_id, quin_count }` |
| `Asset.persist_temporal_span` | External | `{ asset_id: string }` | `{ asset_id, span_seconds }` |
| `Asset.persist_query_aspects` | External | `{ asset_id: string, kind_iri: string }` | `{ asset_id, kind_iri, aspects: [Record], count }` |
| `Asset.resolve` | Pure | `{ asset_id: string }` | `Record` (asset or null) |
| `Asset.resolve_by_spatial` | Pure | `{ anchor_iri: string }` | `{ assets: [Record], count }` |
| `Asset.resolve_by_topic` | Pure | `{ topic: string }` | `{ assets: [Record], count }` |
| `Asset.resolve_by_temporal` | Pure | `{ kind_iri: string }` | `{ assets: [Record], count }` |
| `Asset.list` | Pure | `{}` | `{ asset_ids: [string], count }` |
| `Asset.count` | Pure | `{}` | `{ count }` |

---

## 12. Normative examples (MUST parse and type-check)

### 12.1 Pure cell (HCF host only)

```vibe
= math.max(0, math.min(100, score))
```

### 12.2 Module: query and alert

```vibe
module <https://qualiadb.org/modules/clinic_alerts>;
prefix clinic: <https://qualiadb.org/clinic/>;
prefix snomed: <http://snomed.info/id/>;

requires [
    capability("graph.read", context: clinic:telemetry),
    capability("graph.write", context: clinic:alerts),
    capability("aura.validate"),
    capability("pulse.publish", topic: "clinic/alerts")
];

effect fn raise_alert(sensor: Iri, value: f64) budget(steps: 20000) -> Result<Receipt, string> {
    if value <= 85.0 {
        return Ok(receipt_empty());
    }

    let proposition = <<( sensor clinic:emitsAlert clinic:Overheat )>>;
    let stated = << sensor clinic:emitsAlert clinic:Overheat ~ clinic:claim_1 >>;

    transaction {
        graph.stage(stated);
        aura.validate(clinic:claim_1, clinic:EmergencyAlertShape)?;
        graph.commit()?;
    };

    effect pulse.publish("clinic/alerts", { sensor: sensor, value: value })?;
    return Ok(receipt_empty());
}

on pulse:message(topic: string, value: f64) budget(steps: 20000) -> Result<Receipt, string> {
    return raise_alert(clinic:sensor_1, value);
}
```

`receipt_empty` is a host Pure helper returning an empty `Receipt`. Implementations MAY name it `receipt.none`.

### 12.3 Bounded query

```vibe
pure fn count_conditions(kind: Iri) -> Result<i64, string> {
    let rows = graph.query(?s, clinic:hasCondition, kind, take: 64)?;
    let mut n: i64 = 0;
    for row in rows {
        n = n + 1;
    }
    return Ok(n);
}
```

---

## 13. Negative fixtures (MUST reject)

| Id | Source fragment | Why |
|---|---|---|
| N1 | `a<b` | relational `<` without spaces |
| N2 | `<< id \| s p o >>` | not RDF 1.2 |
| N3 | `<<[ s p o g prov ]>>` | raw Quin literal forbidden |
| N4 | cell `= pulse.publish("t", 1)` | External in Pure cell |
| N5 | `while true { }` | unbounded loop |
| N6 | module that calls `graph.commit` without `capability("graph.write")` | authority |
| N7 | `on tick() { graph.query(?s, ?p, ?o, take: 1); }` | hot path must not query |

---

## 14. Implementation placement

| Piece | Crate | Reason |
|---|---|---|
| Lexer, parser, AST, type/effect check, AST interpreter | new workspace crate `poet-vibe` | Must not grow `qualia-core-db` monolith |
| Quin seal, graph snapshot, SHACL, telemetry | `qualia-core-db` / `qualia-client-core` | Existing truth |
| Desktop host (load module, grant caps, dispatch `on`) | `webizen-desktop` + studio pane | UI is the test harness |
| WASM | `poet-vibe` with `wasm32-unknown-unknown` | Same interpreter |

Parser implementation: custom (e.g. `logos` + recursive descent or `chumsky`). **Do not** embed Rhai as the Vibe surface — RDF literals, `requires`, and effect checking would become a second language (Gemini’s Rhai path is recorded as a rejected 0.1 option).

Extensions later: WASM Component Model for sandboxed codecs; **child processes** (not Start-menu apps) for GPU/HSM, registered into the capability table (consult PDF). 0.1 does not load extensions.

---

## 15. Conformance bar for “0.1 implemented”

1. Every §12 example parses, type-checks, and evaluates on `native-desktop` and `wasm32`.
2. Every §13 fixture is rejected with a stable diagnostic code.
3. No public API writes Quin `parity` or metadata overlays.
4. Pure cells cannot reach Pulse, graph write, or time.
5. `graph.query` without `take` is a type/effect error.
6. Tests live in `poet-vibe` and do not require a GPU.

Until those six hold, documents MUST say **0.1-draft**, not v1.0.

---

## 16. Out of 0.1 (explicit)

Closures; generics beyond `Option`/`Result`/`List`; interpolation; Unicode identifiers; package signatures; SHACL-AF / ShEx as complete engines; Bao HMC v2; “mathematically perfect” speech; `no_std` edge; enumerating Qualia’s full capability matrix as keywords. (Note: `vibe-bc-0.1` bytecode was originally listed here but has been implemented as an optional stack-based VM with binary codec — see §1.)
