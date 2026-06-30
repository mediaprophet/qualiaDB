---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# exact Index

## Functionality Overview
Comprehensive index of functionality for `exact`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `bigint.rs`
  - `struct BigInt`
  - `impl BigInt`
  - `fn zero`
  - `fn one`
  - `fn from_i64`
  - `fn from_u64`
  - `fn from_str`
  - `fn to_string`
  - `fn is_zero`
  - `fn is_negative`
  - `fn signum`
  - `fn abs`
  - `fn neg`
  - `fn normalize`
  - `fn cmp_mag`
  - *(...and 31 more)*
- 📄 `mod.rs`
- 📄 `rational.rs`
  - `struct BigRational`
  - `impl BigRational`
  - `fn zero`
  - `fn one`
  - `fn from_bigint`
  - `fn from_i64`
  - `fn from_i64s`
  - `fn new`
  - `fn numerator`
  - `fn denominator`
  - `fn is_zero`
  - `fn signum`
  - `fn abs`
  - `fn neg`
  - `fn recip`
  - *(...and 21 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
