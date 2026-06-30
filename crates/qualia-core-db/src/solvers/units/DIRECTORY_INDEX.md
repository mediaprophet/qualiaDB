---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# units Index

## Functionality Overview
Comprehensive index of functionality for `units`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `constants.rs`
  - `fn constants_carry_correct_dimensions`
  - `fn gas_constant_is_avogadro_times_boltzmann`
  - `fn photon_energy_e_equals_h_nu_is_dimensionally_energy`
  - `fn thermal_energy_kt_is_energy`
- 📄 `conversion.rs`
  - `struct Unit`
  - `impl Unit`
  - `fn to_si`
  - `fn from_si`
  - `fn quantity`
  - `fn convert`
  - `fn length_conversions`
  - `fn temperature_is_affine`
  - `fn energy_conversion`
  - `fn cross_dimension_conversion_fails_closed`
- 📄 `dimension.rs`
  - `struct Dimension`
  - `impl Dimension`
  - `fn is_dimensionless`
  - `fn mul`
  - `fn div`
  - `fn powi`
  - `fn products_and_quotients_compose`
  - `fn powers_scale_exponents`
  - `fn dimensionless_detection`
- 📄 `mod.rs`
  - `enum UnitsError`
  - `impl core`
  - `fn fmt`
  - `impl std`
- 📄 `quantity.rs`
  - `struct Quantity`
  - `impl Quantity`
  - `fn add`
  - `fn sub`
  - `fn mul`
  - `fn div`
  - `fn scale`
  - `fn powi`
  - `fn compatible_with`
  - `fn metres`
  - `fn seconds`
  - `fn adding_like_dimensions_works_unlike_fails`
  - `fn products_derive_new_dimensions`
  - `fn kinetic_energy_is_dimensionally_consistent`
  - `fn divide_by_zero_fails_closed`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
