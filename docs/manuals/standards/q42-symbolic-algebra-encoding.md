# Q42 Symbolic Algebra Encoding & Numeric Algebra Surface

**Version:** 0.1.1
**Date:** 2026-08-15
**Status:** Draft Standard
**Repository:** https://github.com/mediaprophet/qualiaDB/tree/0.0.33

## Abstract

This standard defines (a) a textual grammar for symbolic algebraic expressions, (b) a
canonical, lossless encoding of those expressions into a sequence of 48-byte `NQuin`
frames so that symbolic results can be **stored in the graph and cited with provenance**,
and (c) the interoperable operation surface (MCP tools + SHACL shapes) for the symbolic
(CAS) and numeric algebra primitives. It complements the
[Q42 10D Tensor Standard](q42-10d-tensor-standard.md) and the
[Q42 unified volume format](q42-format-internal-draft.md).

Reference implementation: `crates/qualia-core-db/src/specialized_libs/symbolic_algebra.rs`
(CAS) and `.../linear_algebra.rs` (numeric). The CAS layer is intentionally distinct from
`solvers/symbolic_logic` (which is a SAT / defeasible-logic engine, not computer algebra).

## 1. Expression model

A symbolic expression `Expr` is a finite tree over the following node kinds:

| Kind | Arity | Meaning |
|------|-------|---------|
| `Const(f64)` | 0 | A real literal |
| `Var(name)` | 0 | A free variable, identified by a UTF-8 name |
| `Add(a, b)` | 2 | `a + b` |
| `Sub(a, b)` | 2 | `a − b` |
| `Mul(a, b)` | 2 | `a · b` |
| `Div(a, b)` | 2 | `a / b` |
| `Pow(a, e:i32)` | 1 | `a` raised to an **integer** exponent `e` |
| `Neg(a)` | 1 | `−a` |
| `Sqrt(a)` | 1 | principal square root `√a` |

Two expressions are equal iff their trees are structurally identical.

## 2. Textual grammar (NORMATIVE)

The parser accepts the following grammar (whitespace insignificant):

```
expr   = term (('+' | '-') term)*
term   = factor (('*' | '/') factor)*
factor = unary ('^' ['-'] integer)?       // exponent is an integer literal
unary  = '-' unary | base
base   = number | ident | unop '(' expr ')' | '(' expr ')'
unop   = 'sqrt' | 'exp' | 'ln' | 'sin' | 'cos' | 'tan'
number = digits ['.' digits]
ident  = (alpha | '_') (alphanumeric | '_')*
```

`*` binds tighter than `+`/`-`; `^` binds tighter than unary minus on the base. `sqrt` is
reserved identifiers. Examples: `x^3 - 2*x^2 + 5`, `sqrt(b^2 - 4*a*c)`,
`sin(x) + exp(2*x)`, `(price - 4) / 2`.

Malformed input MUST be rejected with an error (never a panic).

## 3. NQuin tree encoding (NORMATIVE)

An expression is serialised to a `Vec<NQuin>` in **post-order**: a node appears AFTER all
of its children, so **the root is the LAST element**. Each node occupies exactly one quin
and references its children by their **zero-based index** in the sequence.

Per-node layout (the `subject` field carries the node's own index for readability; it is
not load-bearing for decode):

| Kind | `predicate` (= `q_hash(tag)`) | `object` | `context` | `metadata` |
|------|------------------------------|----------|-----------|------------|
| Const | `cas:const` | f64 bits (`f64::to_bits`) | 0 | 0 |
| Var | `cas:var` | name packed (§3.1) | 0 | name byte length |
| Add | `cas:add` | left child index | right child index | 0 |
| Sub | `cas:sub` | left index | right index | 0 |
| Mul | `cas:mul` | left index | right index | 0 |
| Div | `cas:div` | left index | right index | 0 |
| Pow | `cas:pow` | base child index | 0 | exponent (`i32` as `u64`) |
| Neg | `cas:neg` | child index | 0 | 0 |
| Sqrt | `cas:sqrt` | child index | 0 | 0 |
| Exp | `cas:exp` | child index | 0 | 0 |
| Ln | `cas:ln` | child index | 0 | 0 |
| Sin | `cas:sin` | child index | 0 | 0 |
| Cos | `cas:cos` | child index | 0 | 0 |
| Tan | `cas:tan` | child index | 0 | 0 |

Node-kind tags are FNV-1a `q_hash` of the strings in the table. `parity` MUST be the
five-field fold `subject ^ predicate ^ object ^ context ^ metadata`
(`NQuin::calculate_parity`). Decode MAY ignore `parity`; a unified v3 volume verify
MUST NOT. A four-field fold (metadata omitted) is a verify Fail.

### 3.1 Variable-name packing

A variable name is packed little-endian into the 64-bit `object` field, up to **8 bytes**;
`metadata` holds the byte length actually stored. Names longer than 8 bytes are truncated
to 8 (implementations SHOULD warn). This keeps single-quin nodes zero-indirection for the
common short-name case; a future revision MAY add an overflow form for long names.

### 3.2 Round-trip guarantee

For any expression whose variable names are ≤ 8 bytes, `from_quins(to_quins(e)) == e`.
Every Quin produced by `to_quins` MUST satisfy `verify_ecc_parity()`. Decoding starts
at the last element and recurses through child indices. An out-of-range child index or
an unknown predicate tag MUST be a decode error.

## 4. Operations surface (NORMATIVE interface names)

### 4.1 Symbolic (CAS) — MCP tool `cas`

`op ∈ { differentiate, simplify, expand, evaluate, solve_quadratic, factor }`.

| op | inputs | output |
|----|--------|--------|
| `differentiate` | `expr`, `var` | `derivative` (text) |
| `simplify` | `expr` | `simplified` (text) |
| `expand` | `expr` | `expanded` (text) |
| `evaluate` | `expr`, `env:{var→number}` | `value` (number) |
| `solve_quadratic` | `a`, `b`, `c` | `roots` (two root expressions) |
| `factor` | `a`, `b`, `c`, `var` | `factored` (text) or null if no real factorisation |

Semantics: `simplify` performs constant folding + identity elimination (`x+0`, `x·1`,
`x·0`, `x⁰`, `x¹`, `−(−x)`, `x/1`, `x−x`, `x/x`, `x+x→2x`) to a bounded fixpoint;
`differentiate` applies the sum / product / quotient / power / chain / sqrt rules;
`expand` distributes products and small (≤ 8) integer powers over sums;
`factor` factors a real quadratic `a·x²+b·x+c` into `a·(x−r₁)(x−r₂)` (None when the
discriminant is negative).

### 4.2 Numeric algebra — MCP tools

- `algebra_solve_polynomial` — input `coeffs` (DESCENDING), output all real + complex
  `roots` (`{re, im}`). Quadratics use a numerically-stable closed form; general degree
  uses the Durand–Kerner iteration.
- `algebra_matrix_analyze` — `op ∈ { determinant, eigenvalues, eigen_symmetric, svd }`
  over a row-major `rows×cols` `data` array. `determinant` via LU with partial pivoting;
  `eigen_symmetric` via cyclic Jacobi (eigenvalues + eigenvectors); `eigenvalues` (general)
  via the Faddeev–LeVerrier characteristic polynomial; `svd` via the eigendecomposition of
  `AᵀA` (`A = U·Σ·Vᵀ`).

### 4.3 SHACL configuration shapes

Validation shapes live in `shapes/specialized-libraries.shacl.ttl` (namespace
`https://webizen.org/q42#`) and mirror `specialized_libs_shacl.rs`:
`q42:PolynomialSolveShape`, `q42:SingularValueDecompositionShape`,
`q42:DeterminantShape`, `q42:SymbolicExpressionShape` (operators ∈ `{add, sub, mul, div,
pow, neg, sqrt}`), `q42:SymbolicOperationShape` (op ∈ `{differentiate, simplify, expand,
evaluate, solve, factor}`). The shape file MUST parse under RDF 1.1 Turtle (validated with
`rdflib`).

## 5. Numeric precision & ZK

- Numeric algebra is `f64`; root-finding and eigensolvers are iterative with documented
  tolerances. Implementations SHOULD treat near-zero discriminants/off-diagonals within a
  relative epsilon.
- The privacy-preserving matrix product (`private_matrix_multiply`) proves `A·B = C` in
  zero knowledge over a **fixed-point** encoding (scale `1e6`): real-valued matrices are
  supported to ~`1e-6` precision; integer matrices are exact. The Groth16 circuit attests
  the exact scaled-integer identity `Σ a'·b' = C'`; the result is `C'/S²`.

## 6. Provenance & citation

`expr_citation_hash(e)` = `q_hash` of the canonical `Display` form of `e`; structurally
equal expressions hash equally. Combined with §3, a computed/derived expression (e.g. a
universalised duty rule or an `amendedText` transform) can be stored as a quin subgraph and
cited by hash — see `core-ontologies/PLAN.md` §19.

## 7. Conformance & test vectors

A conforming implementation MUST pass:

- **Parse/derivative:** `differentiate("x^3 - 2*x^2 + 5", x)` evaluates to `4` at `x = 2`.
- **Quadratic (numeric):** `x² − 5x + 6` → roots `{2, 3}`; `x² + 1` → `{±i}`.
- **General roots:** `x⁴ − 1` → `{1, −1, i, −i}` (`|root| = 1`).
- **Eigen/SVD:** `[[2,1],[1,2]]` → eigenvalues `{1, 3}` with `A·v = λv`;
  SVD reconstruction `‖A − UΣVᵀ‖ < 1e-9`.
- **Encoding round-trip:** `from_quins(to_quins(parse("x^2 + 3*x + 2"))) == parse(...)`.
- **Factor inverts expand:** `factor(1,-5,6,x)` evaluates equal to `x²−5x+6` everywhere.

Reference tests: `specialized_libs::symbolic_algebra::tests` and
`specialized_libs::linear_algebra::tests`.

## 8. References

- [Q42 10D Tensor Standard](q42-10d-tensor-standard.md)
- [Q42 unified volume (v3)](q42-format-internal-draft.md)
- `ALGEBRA_MANIFOLD_PLAN.md` — implementation plan & status
- `core-ontologies/PLAN.md` §19 — ontology-layer considerations
