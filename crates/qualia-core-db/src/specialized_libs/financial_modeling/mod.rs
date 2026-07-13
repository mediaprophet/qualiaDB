//! Financial Modeling Library - Secure Financial Computing and Risk Analysis
//!
//! This module provides high-performance financial modeling operations leveraging Phase 2 enhancements:
//! - Fiduciary Cryptography (ML-DSA) for secure financial transactions
//! - Zero-Knowledge Semantic Proofs for privacy-preserving financial analysis
//! - Hardware-Sympathetic Storage (ZNS) for zero-copy financial data
//! - Statistical Computing Library for advanced financial analytics

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// Real return-based portfolio risk metrics (volatility, historical VaR/CVaR,
/// Sharpe, Sortino, max-drawdown) computed from each asset's price history.
/// Split into its own library submodule (PROJECT RULE §11) with its own tests
/// against hand-computed statistics.
pub mod portfolio_risk;

mod results;
mod portfolio;
mod assets;
mod risk;
mod rebalancing;
mod execution;
mod settlement;
mod performance;
mod pricing;
mod trading;
mod compliance;
mod reporting;
mod library;

pub use results::*;
pub use portfolio::*;
pub use assets::*;
pub use risk::*;
pub use rebalancing::*;
pub use execution::*;
pub use settlement::*;
pub use performance::*;
pub use pricing::*;
pub use trading::*;
pub use compliance::*;
pub use reporting::*;
pub use library::*;

#[cfg(test)]
mod tests;
