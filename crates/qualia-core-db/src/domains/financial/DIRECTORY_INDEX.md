---
created: 2026-06-30
updated: 2026-07-07
update_scope: Focused
---

# financial Index

Financial domain engines for economics kernels and tax-schema clearing.

## Subdirectories

- `economics/` - purpose-defined economics kernels with compatibility
  re-exports from `mod.rs`.

## Files

- `mod.rs`
  - `pub mod economics`
  - `pub mod tax_schema`
- `tax_schema.rs`
  - `TaxRuleSchema`
  - `TaxRule`
  - `TaxLineItem`
  - `JurisdictionLiability`
  - `ClearingResult`
  - `TaxClearingHouse`

## Changelog

- **2026-07-07**: Split the old single `economics.rs` file into the
  `economics/` subdirectory and added deterministic caller-buffered stochastic
  kernels.
- **2026-06-30**: Automated full index generation.
