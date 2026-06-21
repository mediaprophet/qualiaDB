use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::ops::{Add, Mul, Sub};
use serde::{Deserialize, Serialize};
use crate::solvers::SolversError;

use super::storage::*;
use super::computation::*;
use super::optimization::*;
use super::privacy::*;
use super::performance::*;


/// Matrix metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixMetadata {
    pub matrix_id: String,
    pub rows: usize,
    pub cols: usize,
    pub data_type: DataType,
    pub storage_format: StorageFormat,
    pub compression: CompressionType,
    pub created_at: u64,
    pub last_accessed: u64,
    pub access_count: u64,
}


/// Data types for matrices
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataType {
    Float32,
    Float64,
    Complex32,
    Complex64,
    Integer32,
    Integer64,
}


/// Storage formats for matrices
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StorageFormat {
    RowMajor,
    ColumnMajor,
    Blocked,
    CompressedSparseRow,
    CompressedSparseColumn,
}


/// Compression types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    LZ4,
    ZSTD,
    Custom(String),
}


/// Matrix representation
#[derive(Debug, Clone)]
pub struct Matrix {
    pub matrix_id: String,
    pub rows: usize,
    pub cols: usize,
    pub data_type: DataType,
    pub data: Vec<f64>, // Simplified to f64 for demonstration
    pub storage_format: StorageFormat,
    pub metadata: MatrixMetadata,
}


/// Linear algebra result
#[derive(Debug, Clone)]
pub struct LinearAlgebraResult<T> {
    pub result: T,
    pub execution_time: u64,
    pub memory_usage: u64,
    pub operations_used: Vec<String>,
    pub privacy_preserved: bool,
}


/// Linear algebra error types
#[derive(Debug, Clone)]
pub enum LinearAlgebraError {
    InvalidDimensions(String),
    SingularMatrix(String),
    StorageError(String),
    ComputationError(String),
    PrivacyError(String),
    OptimizationError(String),
}


impl std::fmt::Display for LinearAlgebraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinearAlgebraError::InvalidDimensions(msg) => write!(f, "Invalid dimensions: {}", msg),
            LinearAlgebraError::SingularMatrix(msg) => write!(f, "Singular matrix: {}", msg),
            LinearAlgebraError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            LinearAlgebraError::ComputationError(msg) => write!(f, "Computation error: {}", msg),
            LinearAlgebraError::PrivacyError(msg) => write!(f, "Privacy error: {}", msg),
            LinearAlgebraError::OptimizationError(msg) => write!(f, "Optimization error: {}", msg),
        }
    }
}


impl std::error::Error for LinearAlgebraError {}

