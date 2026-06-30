---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# financial Index

## Functionality Overview
Comprehensive index of functionality for `financial`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `economics.rs`
  - `fn simulate_gbm_path`
  - `fn run_monte_carlo_var`
  - `fn simulate_macroeconomic_flow`
  - `struct SystemContext`
  - `fn get_current_system_context`
  - `fn calculate_bandwidth_liability`
  - `fn propagate_supply_shock`
  - `fn resilience_resource_pricing`
  - `fn supply_shock_propagates_to_dependent_sectors`
  - `fn supply_shock_rejects_bad_dimensions`
  - `fn resilience_pricing_prioritizes_survival`
- 📄 `mod.rs`
- 📄 `tax_schema.rs`
  - `struct TaxRuleSchema`
  - `struct TaxRule`
  - `impl TaxRuleSchema`
  - `fn new_au_gst`
  - `fn new_eu_vat`
  - `fn new_us_sales_tax`
  - `fn new_zero_rated`
  - `fn evaluate`
  - `struct TaxLineItem`
  - `struct JurisdictionLiability`
  - `struct ClearingResult`
  - `struct TaxClearingHouse`
  - `impl TaxClearingHouse`
  - `fn new`
  - `fn with_schema`
  - *(...and 8 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
