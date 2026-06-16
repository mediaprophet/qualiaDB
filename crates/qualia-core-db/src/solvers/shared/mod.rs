//! Shared utilities for solvers
//!
//! This module provides zero-heap utilities for solver implementations,
//! including convergence checking and numerical stability utilities.

pub mod convergence;

pub use convergence::{
    ConvergenceChecker, ConvergenceCriteria, ConvergenceStatus,
    DEFAULT_MAX_ITERATIONS, DEFAULT_TOLERANCE
};