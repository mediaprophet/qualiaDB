---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# dimensionality Index

## Functionality Overview
Comprehensive index of functionality for `dimensionality`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `mod.rs`
- 📄 `pca.rs`
  - `struct Pca`
  - `impl Pca`
  - `fn transform`
  - `fn fit`
  - `fn one_dominant_direction`
  - `fn diagonal_correlation_axis`
  - `fn transform_projects_and_decorrelates`
  - `fn total_explained_variance_equals_total_variance`
  - `fn guards`
- 📄 `som.rs`
  - `struct Som`
  - `struct Lcg`
  - `impl Lcg`
  - `fn unit`
  - `impl Som`
  - `fn idx`
  - `fn bmu`
  - `fn train`
  - `fn separated_clusters_map_to_distinct_regions`
  - `fn preserves_1d_order`
  - `fn guards`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
