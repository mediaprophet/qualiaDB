---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# simd_kernel Index

## Functionality Overview
Comprehensive index of functionality for `simd_kernel`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `mod.rs`
- 📄 `operations.rs`
  - `fn geometric_product`
  - `fn outer_product`
  - `fn rotor_from_angle_axis`
  - `fn apply_rotor`
  - `fn translator_from_displacement`
  - `fn apply_translator`
  - `fn is_simd_available`
  - `fn test_outer_product`
  - `fn test_rotor_creation`
  - `fn test_translator`
  - `fn test_simd_availability`
- 📄 `simd_backend.rs`
  - `struct GaKernel`
  - `impl GaKernel`
  - `fn init`
  - `fn multivector_geometric_product`
  - `fn multivector_outer_product`
  - `fn vec3`
  - `fn test_geometric_product_vectors`
  - `fn test_outer_product_self_is_zero`
- 📄 `types.rs`
  - `struct Multivector`
  - `enum Grade`
  - `struct Rotor`
  - `struct Translator`
  - `impl Default`
  - `fn default`
  - `impl Multivector`
  - `fn zero`
  - `fn scalar`
  - `fn vector`
  - `fn bivector`
  - `fn trivector`
  - `fn from_rotor`
  - `fn from_translator`
  - `fn from_vector`
  - *(...and 24 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
