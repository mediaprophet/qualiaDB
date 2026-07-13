//! `MLError` type and trait impls.

#[allow(unused_imports)]
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::collections::HashMap;

/// ML error types
#[derive(Debug, Clone)]
pub enum MLError {
    ModelError(String),
    InferenceError(String),
    TrainingError(String),
    OptimizationError(String),
    DataError(String),
    ResourceError(String),
    ValidationError(String),
}

impl std::fmt::Display for MLError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MLError::ModelError(msg) => write!(f, "Model error: {}", msg),
            MLError::InferenceError(msg) => write!(f, "Inference error: {}", msg),
            MLError::TrainingError(msg) => write!(f, "Training error: {}", msg),
            MLError::OptimizationError(msg) => write!(f, "Optimization error: {}", msg),
            MLError::DataError(msg) => write!(f, "Data error: {}", msg),
            MLError::ResourceError(msg) => write!(f, "Resource error: {}", msg),
            MLError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for MLError {}
