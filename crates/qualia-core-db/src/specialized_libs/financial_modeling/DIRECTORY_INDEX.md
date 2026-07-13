---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# financial_modeling Index

## Functionality Overview
Comprehensive index of functionality for `financial_modeling`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `mod.rs`
  - `struct FinancialModelingLibrary`
  - `struct PortfolioManager`
  - `struct PortfolioStorage`
  - `struct Portfolio`
  - `struct Asset`
  - `enum AssetType`
  - `struct RiskProfile`
  - `enum RiskTolerance`
  - `enum TimeHorizon`
  - `enum LiquidityNeeds`
  - `enum InvestmentStrategy`
  - `struct PortfolioMetadata`
  - `enum Permission`
  - `enum ComplianceFlag`
  - `struct PortfolioAccessControl`
  - *(...and 383 more)*
- 📄 `portfolio_risk.rs`
  - `fn returns_from_prices`
  - `fn quantile_sorted`
  - `fn portfolio_returns`
  - `fn compute_risk_metrics`
  - `fn asset`
  - `fn portfolio`
  - `fn single_asset_matches_hand_computation`
  - `fn value_weighting_blends_drawdown_between_components`
  - `fn refuses_without_history`
  - `fn refuses_misaligned_histories`
  - `fn refuses_too_short_history`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
