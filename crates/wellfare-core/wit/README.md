# Wellfare component contract

`wellfare.wit` is the versioned WebAssembly Component Model contract for the
Wellfare data surface. It is a contract, not an additional runtime export yet.
The active browser interface remains `src/wasm.rs`, generated with
`wasm-bindgen` for `wasm32-unknown-unknown`.

## Boundary rules

- Quin values cross this boundary as five exact `u64` fields. They must never be
  represented as `f64` because JavaScript numbers cannot retain all `u64` bits.
- Both stores require caller-selected capacities; operations return
  `capacity-exceeded` or `result-limit-exceeded` instead of growing without a
  bound.
- Health, vault, validation, and rule operations receive an `access-context`.
  A future adapter must verify that context before reading, modifying, or
  returning restricted data.
- `document` holds explicitly labelled bytes for JSON/SPARQL result documents.
  It avoids conflating structured records with an unspecified string encoding.

## Integration status

Before this world can be shipped, add a separate component adapter using a
compatible `wit-bindgen` release, build it for a Component Model target, and add
conformance tests that compare each exported operation with the existing Rust
implementation. Do not expose the current unbounded `QualiaStore` as this
contract without its capacity and access checks.
